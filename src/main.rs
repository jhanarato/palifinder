use rusqlite::{Connection, Result};

#[derive(Debug)]
struct Stem {
    stem: String,
}

fn get_stem(conn: &mut Connection, word: &str) -> Result<String> {
    let stem = conn.query_one(
        "SELECT stem from dpd_headwords where lemma_1 == (?1)",
        [word],
        |row| Ok(Stem { stem: row.get(0)? })
    )?;
    Ok(stem.stem)
}

fn main() -> Result<()> {
    let mut conn = Connection::open("data/dpd.db")?;
    let stem = get_stem(&mut conn, "bhagavā")?;
    println!("Stem of bhagavā is {stem}");
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