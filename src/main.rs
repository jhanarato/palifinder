use rusqlite::{Connection, Result};

#[derive(Debug)]
struct Stem {
    stem: String,
}

#[allow(dead_code)]
fn get_stem(conn: &mut Connection, word: &str) -> Result<String> {
    let stem = conn.query_row(
        "SELECT stem from dpd_headwords where lemma_1 == (?1)",
        [word],
        |row| Ok(Stem { stem: row.get(0)? })
    )?;
    Ok(stem.stem)
}

fn main() -> Result<()> {
    let conn = Connection::open("data/dpd.db")?;
    let mut stmt = conn.prepare("SELECT stem from dpd_headwords where lemma_1 == 'bhagavā'")?;
    let stems = stmt.query_map([], |row| {
        Ok(
            Stem {
                stem: row.get(0)?,
            }
        )
    })?;

    for stem in stems {
        println!("Found stem {:?}", stem?.stem);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_stem() {
        let mut conn = Connection::open("data/dpd.db").unwrap();
        assert_eq!(get_stem(&mut conn, "bhagavā").unwrap(), String::from("!bhagav"));
    }
}