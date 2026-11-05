use crate::dpd::Dictionary;
use crate::snowball;
use crate::vocabulary::Vocabulary;
use anyhow::{Context, Error, Result, bail};
use csv::{Reader, Writer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions, remove_file, rename};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::vec::IntoIter;

const COLUMNS: [&str; 5] = ["term", "dpd_stem", "snowball_stem", "family", "fallback"];

#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct TermStem {
    pub term: String,
    /// The word DPD says this term belongs to.
    pub dpd_stem: Option<String>,
    pub snowball_stem: String,
    /// The index term shared by the verbal forms of the word's root.
    pub family: Option<String>,
    /// The commonest word with this Snowball stem, for terms DPD does not have.
    pub fallback: Option<String>,
}

#[derive(Clone)]
pub struct StemTable {
    records: Vec<TermStem>,
}

impl StemTable {
    #[allow(clippy::missing_errors_doc)]
    pub fn new(vocabulary: Vocabulary, dictionary: &Dictionary) -> Result<Self> {
        let mut terms: HashMap<String, u64> = dictionary.lemmas()?.into_iter().map(|t| (t, 0)).collect();
        terms.extend(vocabulary);
        let mut terms: Vec<(String, u64)> = terms.into_iter().collect();
        terms.sort_unstable();
        let mut entries = dictionary.entries(&terms).context("resolving terms against DPD")?;

        let mut tallies: HashMap<String, HashMap<String, u64>> = HashMap::new();
        let mut wordless: HashMap<String, u64> = HashMap::new();
        let mut records = Vec::with_capacity(terms.len());
        for (term, count) in terms {
            let entry = entries.remove(&term).unwrap_or_default();
            let snowball_stem = snowball::pali_stem(&term);
            if let Some(word) = &entry.word {
                *tallies
                    .entry(snowball_stem.clone())
                    .or_default()
                    .entry(word.clone())
                    .or_default() += count;
            } else {
                *wordless.entry(snowball_stem.clone()).or_default() += count;
            }
            records.push(TermStem {
                term,
                dpd_stem: entry.word,
                snowball_stem,
                family: entry.family,
                fallback: None,
            });
        }
        for record in &mut records {
            let unknown = wordless.get(&record.snowball_stem).copied().unwrap_or_default();
            record.fallback = tallies.get(&record.snowball_stem).and_then(|tally| commonest(tally, unknown));
        }
        Ok(Self { records })
    }

    /// Write the table beside `path` and then move it into place, so that a failure
    /// leaves any old table whole. Only `path` itself is replaced.
    #[allow(clippy::missing_errors_doc)]
    pub fn save(&self, path: &Path) -> Result<()> {
        let (file, building) = new_file_beside(path)?;
        let saved = self.write_to(file).and_then(|()| Ok(rename(&building, path)?));
        if saved.is_err() {
            // The first failure is the one to report, whether or not this works.
            let _ = remove_file(&building);
        }
        saved
    }

    fn write_to(&self, file: File) -> Result<()> {
        let mut writer = Writer::from_writer(file);
        for record in &self.records {
            writer.serialize(record)?;
        }
        writer.into_inner().map_err(csv::IntoInnerError::into_error)?.sync_all()?;
        Ok(())
    }
}

/// A file of its own beside `path`, created only if nothing has that name, so that no
/// other file is truncated or carried away by the rename.
fn new_file_beside(path: &Path) -> Result<(File, PathBuf)> {
    let name = path.file_name().with_context(|| format!("{} names no file for the stem table", path.display()))?;
    let mut attempt = 0u32;
    loop {
        let building = path.with_file_name(format!(".{}.{}.{attempt}.building", name.to_string_lossy(), std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&building) {
            Ok(file) => return Ok((file, building)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists && attempt < 1000 => attempt += 1,
            Err(error) => return Err(error).with_context(|| format!("creating {}", building.display())),
        }
    }
}

/// The word most terms with one Snowball stem belong to, unless terms DPD does not
/// know outnumber it. Those are then more likely over-stripped sandhi forms than
/// inflections of the word: nahetuṁ should not become nahāna.
fn commonest(tally: &HashMap<String, u64>, unknown: u64) -> Option<String> {
    tally
        .iter()
        .max_by_key(|(word, count)| (**count, std::cmp::Reverse((*word).clone())))
        .filter(|(_, count)| **count >= unknown)
        .map(|(word, _)| word.clone())
}

impl<T> TryFrom<Reader<T>> for StemTable
where
    T: std::io::Read,
{
    type Error = Error;
    fn try_from(mut reader: Reader<T>) -> Result<Self, Self::Error> {
        let headers = reader.headers()?;
        for column in COLUMNS {
            if !headers.iter().any(|header| header == column) {
                bail!("the stem table has no {column} column; run stem-table again");
            }
        }
        let mut records: Vec<TermStem> = Vec::new();
        for record in reader.deserialize() {
            records.push(record?);
        }
        if records.is_empty() {
            bail!("the stem table is empty; run stem-table again");
        }
        Ok(Self { records })
    }
}

impl IntoIterator for StemTable {
    type Item = TermStem;
    type IntoIter = IntoIter<TermStem>;

    fn into_iter(self) -> Self::IntoIter {
        self.records.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv::Reader;

    const STEM_DATA: &str = "\
term,dpd_stem,snowball_stem,family,fallback
jumped,jump,jump,,jump
jumping,jump,jump,,jump
frog,,frog,,";

    #[test]
    fn test_try_from_reader() {
        let reader = Reader::from_reader(STEM_DATA.as_bytes());
        let table = StemTable::try_from(reader).unwrap();
        let term_stems: Vec<TermStem> = table.into_iter().collect();
        assert_eq!(term_stems.len(), 3);
        assert_eq!(
            term_stems[0],
            TermStem {
                term: String::from("jumped"),
                dpd_stem: Some(String::from("jump")),
                snowball_stem: String::from("jump"),
                family: None,
                fallback: Some(String::from("jump")),
            }
        );
        assert_eq!(
            term_stems[1],
            TermStem {
                term: String::from("jumping"),
                dpd_stem: Some(String::from("jump")),
                snowball_stem: String::from("jump"),
                family: None,
                fallback: Some(String::from("jump")),
            }
        );
        assert_eq!(
            term_stems[2],
            TermStem {
                term: String::from("frog"),
                dpd_stem: None,
                snowball_stem: String::from("frog"),
                family: None,
                fallback: None,
            }
        );
    }

    fn table(csv: &str) -> Result<StemTable> {
        StemTable::try_from(Reader::from_reader(csv.as_bytes()))
    }

    #[test]
    fn test_a_table_of_the_old_shape_is_refused() {
        let error = table("term,dpd_stem,snowball_stem\nfrog,frog,frog\n").err().unwrap();
        assert!(error.to_string().contains("family"), "{error}");
    }

    #[test]
    fn test_an_empty_table_is_refused() {
        assert!(table("").is_err());
        assert!(table("term,dpd_stem,snowball_stem,family,fallback\n").is_err());
    }

    fn dictionary() -> Dictionary {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE dpd_headwords (id INTEGER, lemma_1 TEXT, pos TEXT, root_key TEXT,
                family_root TEXT, root_base TEXT, construction TEXT);
            CREATE TABLE lookup (lookup_key TEXT, headwords TEXT, deconstructor TEXT);
            INSERT INTO dpd_headwords VALUES (1, 'dhamma', 'masc', '', '', '', ''),
                (2, 'nibbuta', 'pp', '', '', '', '');
            INSERT INTO lookup VALUES ('dhammo', '[1]', ''), ('dhamma', '[1]', '');",
        )
        .unwrap();
        Dictionary::from(conn)
    }

    fn built(terms: &str) -> Vec<TermStem> {
        let analyzer = tantivy::tokenizer::TextAnalyzer::builder(crate::tokenizer::PaliTokenizer::default()).build();
        let vocabulary = Vocabulary::new([terms], analyzer);
        StemTable::new(vocabulary, &dictionary()).unwrap().into_iter().collect()
    }

    #[test]
    fn test_a_new_table_is_sorted_and_holds_terms_and_headwords() {
        let terms: Vec<String> = built("dhammo vvvvv dhammo").into_iter().map(|record| record.term).collect();
        assert_eq!(terms, ["dhamma", "dhammo", "nibbuta", "vvvvv"]);
    }

    #[test]
    fn test_a_new_table_counts_the_texts_for_its_fallbacks() {
        let records = built("dhammo dhammo dhammāti");
        let unknown = records.iter().find(|record| record.term == "dhammāti").unwrap();
        assert_eq!(unknown.dpd_stem, None);
        assert_eq!(unknown.fallback.as_deref(), Some("dhamma"));
    }

    #[test]
    fn test_a_saved_table_reads_back() {
        let dir = temp_dir::TempDir::new().unwrap();
        let path = dir.child("stems.csv");
        let records = built("dhammo vvvvv");
        let analyzer = tantivy::tokenizer::TextAnalyzer::builder(crate::tokenizer::PaliTokenizer::default()).build();
        let vocabulary = Vocabulary::new(["dhammo vvvvv"], analyzer);
        StemTable::new(vocabulary, &dictionary()).unwrap().save(&path).unwrap();
        let read: Vec<TermStem> = StemTable::try_from(Reader::from_path(&path).unwrap()).unwrap().into_iter().collect();
        assert_eq!(read, records);
    }

    fn files_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> =
            std::fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }

    #[test]
    fn test_saving_leaves_every_other_file_alone() {
        let dir = temp_dir::TempDir::new().unwrap();
        let path = dir.child("stems.csv");
        let first = dir.child(format!(".stems.csv.{}.0.building", std::process::id()));
        for other in [dir.child("stems.building"), first.clone()] {
            std::fs::write(&other, "unrelated").unwrap();
        }
        table(STEM_DATA).unwrap().save(&path).unwrap();
        for other in [dir.child("stems.building"), first] {
            assert_eq!(std::fs::read_to_string(other).unwrap(), "unrelated");
        }
        assert_eq!(files_in(dir.path()).len(), 3, "{:?}", files_in(dir.path()));
        assert_eq!(table(&std::fs::read_to_string(&path).unwrap()).unwrap().into_iter().count(), 3);
    }

    #[test]
    fn test_saves_side_by_side_do_not_meet() {
        let dir = temp_dir::TempDir::new().unwrap();
        let paths: Vec<PathBuf> = (0..8).map(|n| dir.child(format!("stems{n}.csv"))).collect();
        std::thread::scope(|scope| {
            for path in &paths {
                scope.spawn(|| table(STEM_DATA).unwrap().save(path).unwrap());
            }
        });
        for path in &paths {
            assert_eq!(table(&std::fs::read_to_string(path).unwrap()).unwrap().into_iter().count(), 3);
        }
        assert_eq!(files_in(dir.path()).len(), paths.len(), "nothing is left behind");
    }

    #[test]
    fn test_a_failed_save_keeps_the_old_table_and_leaves_nothing() {
        let dir = temp_dir::TempDir::new().unwrap();
        let path = dir.child("stems.csv");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("kept"), "").unwrap();
        assert!(table(STEM_DATA).unwrap().save(&path).is_err());
        assert!(path.join("kept").exists());
        assert_eq!(files_in(dir.path()), ["stems.csv"]);
    }

    #[test]
    fn test_equal_tallies_take_the_first_word_alphabetically() {
        let tally = HashMap::from([(String::from("b"), 5), (String::from("a"), 5)]);
        assert_eq!(commonest(&tally, 0).as_deref(), Some("a"));
    }

    #[test]
    fn test_unknown_terms_can_outweigh_a_word() {
        let tally = HashMap::from([(String::from("nahāna"), 10)]);
        assert_eq!(commonest(&tally, 1000), None);
        assert_eq!(commonest(&tally, 5).as_deref(), Some("nahāna"));
    }
}
