use rusqlite::{Connection, Result};

#[derive(Debug)]
struct Stem {
    stem: String,
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
