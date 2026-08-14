use anyhow::Result;
use rusqlite::Connection;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tantivy::tokenizer::{SimpleTokenizer, TokenStream, Tokenizer};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug)]
struct Stem {
    stem: String,
}

fn get_stem(conn: &mut Connection, word: &str) -> Result<String> {
    let stem = conn.query_one(
        "SELECT stem from dpd_headwords where lemma_1 == (?1)",
        [word],
        |row| Ok(Stem { stem: row.get(0)? }),
    )?;
    Ok(stem.stem)
}

fn get_files(location: &Path) -> impl Iterator<Item=PathBuf> {
    WalkDir::new(location)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| is_pali_root_text_file(e.path()).expect("Not a Pali file."))
        .map(DirEntry::into_path)
}

fn get_segments(content: &str) -> Result<Vec<String>> {
    let entries: BTreeMap<String, String> = serde_json::from_str(content)?;
    let segments: Vec<String> = entries.values().cloned().collect();
    Ok(segments)
}

fn tokenize(segment: &str) -> Vec<String> {
    let mut tokenizer = SimpleTokenizer::default();
    let mut stream = tokenizer.token_stream(segment);
    let mut tokens = Vec::new();
    stream.process(&mut |token| {
        tokens.push(token.text.clone());
    });

    tokens
}

fn is_pali_root_text_file(path: &Path) -> Result<bool> {
    if !path.metadata()?.is_file() { return Ok(false) }
    if path.file_stem().unwrap().to_str().unwrap().ends_with("root-pli-ms") { return Ok(true) }
    Ok(false)
}

fn main() -> Result<()> {
    let db_path = Path::new("data/dpd.db");
    let mut conn = Connection::open(db_path)?;
    let pali_dir = Path::new("/opt/sc/sc-flask/sc-data/sc_bilara_data/root/pli/ms");
    for file in get_files(pali_dir) {
        let contents = std::fs::read_to_string(file)?;
        for segment in get_segments(contents.as_str())? {
            for token in tokenize(segment.as_str()) {
                let stem = get_stem(&mut conn, token.as_str());
                match stem {
                    Ok(stem) => println!("{token} {stem}"),
                    Err(_) => println!("{token} NA"),
                }
            }
        }
    }
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
    fn test_get_stem() {
        let mut conn = Connection::open("data/dpd.db").unwrap();
        assert_eq!(
            get_stem(&mut conn, "bhagavā").unwrap(),
            String::from("!bhagav")
        );
    }

    #[test]
    fn test_get_segments() -> Result<()> {
        assert_eq!(
            get_segments(MN1_SEGMENTS)?,
            vec!("Majjhima Nikāya 1 ", "Mūlapariyāyasutta ", "Evaṁ me sutaṁ—")
        );
        Ok(())
    }
}
