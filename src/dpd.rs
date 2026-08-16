use rusqlite::Connection;
use anyhow::{Result};

#[derive(Debug)]
struct Stem {
    stem: String,
}

/// # Errors
///
/// Returns `anyhow::Error` when the stem is not found or there's a problem reading the database.
pub fn stem(conn: &mut Connection, word: &str) -> Result<String> {
    let stem = conn.query_one(
        "SELECT stem from dpd_headwords where lemma_1 == (?1)",
        [word],
        |row| Ok(Stem { stem: row.get(0)? }),
    )?;
    Ok(stem.stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_stem() {
        let mut conn = Connection::open("data/dpd.db").unwrap();
        assert_eq!(
            stem(&mut conn, "bhagavā").unwrap(),
            String::from("!bhagav")
        );
    }
}