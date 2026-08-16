pub mod dpd;

use anyhow::Result;
use csv::Writer;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use tantivy::tokenizer::{TokenStream, Tokenizer, WhitespaceTokenizer};
use walkdir::{DirEntry, WalkDir};

fn pali_files(location: &Path) -> impl Iterator<Item=PathBuf> {
    WalkDir::new(location)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| is_pali_root_text_file(e.path()).expect("Not a Pali file."))
        .map(DirEntry::into_path)
}

fn is_pali_root_text_file(path: &Path) -> Result<bool> {
    if !path.metadata()?.is_file() { return Ok(false) }
    if path.file_stem().unwrap().to_str().unwrap().ends_with("root-pli-ms") { return Ok(true) }
    Ok(false)
}

fn segments(content: &str) -> Result<Vec<String>> {
    let entries: BTreeMap<String, String> = serde_json::from_str(content)?;
    let segments: Vec<String> = entries.values().cloned().collect();
    Ok(segments)
}

fn tokenize(segment: &str) -> Vec<String> {
    let mut tokenizer = WhitespaceTokenizer::default();
    let mut stream = tokenizer.token_stream(segment);
    let mut tokens = Vec::new();
    stream.process(&mut |token| {
        tokens.push(token.text.clone());
    });

    tokens
}

fn vocabulary(pali_dir: &Path) -> Result<HashSet<String>> {
    let mut vocabulary: HashSet<String> = HashSet::new();
    for file in pali_files(pali_dir) {
        let contents = std::fs::read_to_string(file)?;
        for segment in segments(contents.as_str())? {
            for token in tokenize(segment.as_str()) {
                vocabulary.insert(token);
            }
        }
    }
    Ok(vocabulary)
}

fn stem_table(conn: &mut Connection, vocabulary: &HashSet<String>) -> HashMap<String, Option<String>> {
    let mut table: HashMap<String, Option<String>> = HashMap::new();
    for term in vocabulary {
        let term_stem = dpd::stem(conn, term);
        match term_stem {
            Ok(stem) => { table.insert(term.clone(), Some(stem)); },
            Err(_) => { table.insert(term.clone(), None); },
        }
    }
    table
}

#[derive(Serialize)]
struct TermStem {
    term: String,
    stem: Option<String>,
}

fn save_table(table: &HashMap<String, Option<String>>, path: &Path) -> Result<()> {
    let mut writer = Writer::from_path(path)?;
    for (key, value) in table {
        let term_stem = TermStem { term: key.clone(), stem: value.clone() };
        writer.serialize(term_stem)?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let pali_dir = Path::new("/opt/sc/sc-flask/sc-data/sc_bilara_data/root/pli/ms");
    let vocabulary = vocabulary(pali_dir)?;

    let mut conn = Connection::open("data/dpd.db")?;
    let table = stem_table(&mut conn, &vocabulary);

    let csv_file = Path::new("data/term_stems.csv");
    save_table(&table, csv_file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    pub const MN1_SEGMENTS: &str = r#"
    {
        "mn1:0.1": "Majjhima Nikāya 1 ",
        "mn1:0.2": "Mūlapariyāyasutta ",
        "mn1:1.1": "Evaṁ me sutaṁ—"
    }
    "#;

    #[test]
    fn test_get_segments() -> Result<()> {
        assert_eq!(
            segments(MN1_SEGMENTS)?,
            vec!("Majjhima Nikāya 1 ", "Mūlapariyāyasutta ", "Evaṁ me sutaṁ—")
        );
        Ok(())
    }
}
