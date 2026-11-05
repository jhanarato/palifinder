use crate::dpd::Dictionary;
use crate::snowball;
use crate::vocabulary::Vocabulary;
use anyhow::{Context, Error, Result, bail};
use csv::{Reader, Writer};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions, remove_file, rename};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::vec::IntoIter;

#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct TermStem {
    pub term: String,
    pub dpd_stem: Option<String>,
    pub snowball_stem: String,
}

#[derive(Clone)]
pub struct StemTable {
    records: Vec<TermStem>,
}

impl StemTable {
    pub fn new(vocabulary: Vocabulary, dictionary: &Dictionary) -> Self {
        let mut entries = Vec::new();
        for term in vocabulary {
            let dpd_stem = Self::dpd_stem(term.as_str(), dictionary);
            let snowball_stem = snowball::pali_stem(term.as_str());
            entries.push(TermStem {
                term,
                dpd_stem,
                snowball_stem,
            });
        }
        Self { records: entries }
    }

    fn dpd_stem(term: &str, dictionary: &Dictionary) -> Option<String> {
        let stems = dictionary.stems(term);
        match stems {
            Ok(stems) => stems.first().cloned(),
            Err(_) => None,
        }
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

impl<T> TryFrom<Reader<T>> for StemTable
where
    T: std::io::Read,
{
    type Error = Error;
    fn try_from(mut reader: Reader<T>) -> Result<Self, Self::Error> {
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
term,dpd_stem,snowball_stem
jumped,jump,jump
jumping,jump,jump
frog,,frog";

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
                snowball_stem: String::from("jump")
            }
        );
        assert_eq!(
            term_stems[1],
            TermStem {
                term: String::from("jumping"),
                dpd_stem: Some(String::from("jump")),
                snowball_stem: String::from("jump")
            }
        );
        assert_eq!(
            term_stems[2],
            TermStem {
                term: String::from("frog"),
                dpd_stem: None,
                snowball_stem: String::from("frog")
            }
        );
    }

    fn table(csv: &str) -> Result<StemTable> {
        StemTable::try_from(Reader::from_reader(csv.as_bytes()))
    }

    #[test]
    fn test_an_empty_table_is_refused() {
        assert!(table("").is_err());
        assert!(table("term,dpd_stem,snowball_stem\n").is_err());
    }

    fn files_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> =
            std::fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }

    #[test]
    fn test_a_saved_table_reads_back() {
        let dir = temp_dir::TempDir::new().unwrap();
        let path = dir.child("stems.csv");
        table(STEM_DATA).unwrap().save(&path).unwrap();
        let read: Vec<TermStem> = StemTable::try_from(Reader::from_path(&path).unwrap()).unwrap().into_iter().collect();
        assert_eq!(read, table(STEM_DATA).unwrap().into_iter().collect::<Vec<_>>());
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
}
