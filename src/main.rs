use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;

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

fn main() -> Result<()> {
    let mut conn = Connection::open("data/dpd.db")?;
    let stem = get_stem(&mut conn, "bhagavā")?;
    println!("Stem of bhagavā is {stem}");

    let file_data = std::fs::read_to_string("/opt/sc/sc-flask/sc-data/sc_bilara_data/root/pli/ms/sutta/mn/mn1_root-pli-ms.json")?;
    let segments: HashMap<String, String> = serde_json::from_str(file_data.as_str())?;
    for (uid, content) in segments {
        println!("Segment {uid} is {content}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_stem() {
        let mut conn = Connection::open("data/dpd.db").unwrap();
        assert_eq!(
            get_stem(&mut conn, "bhagavā").unwrap(),
            String::from("!bhagav")
        );
    }
}
