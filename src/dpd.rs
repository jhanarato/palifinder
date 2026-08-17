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

    fn connection() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE dpd_headwords (
                lemma_1 VARCHAR NOT NULL,
                stem VARCHAR NOT NULL
            )",
            (),
        ).unwrap();
        conn
    }

    #[test]
    fn test_stem_with_lemma_1() {
        let mut conn = connection();
        conn.execute(
            "INSERT INTO dpd_headwords (lemma_1, stem) VALUES (?1, ?2)",
            ("bhagavā", "!bhagav")
        ).unwrap();

        assert_eq!(stem(&mut conn, "bhagavā").unwrap(), String::from("!bhagav"));
    }
}