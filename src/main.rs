pub mod dpd;
pub mod texts;

use anyhow::Result;
use csv::Writer;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tantivy::tokenizer::{TokenStream, Tokenizer, WhitespaceTokenizer};

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
    for file in texts::pali_files(pali_dir) {
        let contents = std::fs::read_to_string(file)?;
        for segment in texts::segments(contents.as_str())? {
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

