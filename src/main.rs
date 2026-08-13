use anyhow::Result;
use rusqlite::Connection;
use std::collections::BTreeMap;
use tantivy::tokenizer::{SimpleTokenizer, TokenStream, Tokenizer};

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

fn main() -> Result<()> {
    let mut conn = Connection::open("data/dpd.db")?;
    
    let contents = std::fs::read_to_string(
        "/opt/sc/sc-flask/sc-data/sc_bilara_data/root/pli/ms/sutta/mn/mn1_root-pli-ms.json",
    )?;

    for segment in get_segments(contents.as_str())? {
        for token in tokenize(segment.as_str()) {
            let stem = get_stem(&mut conn, token.as_str());
            match stem {
                Ok(stem) => println!("{token} {stem}"),
                Err(_) => println!("{token} NA"),
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
