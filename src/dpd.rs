use anyhow::Result;
use rusqlite::Connection;

#[derive(Debug)]
struct Stem {
    stem: String,
}

#[derive(Debug)]
pub struct Dictionary {
    connection: Connection,
}

impl From<Connection> for Dictionary {
    fn from(connection: Connection) -> Self {
        Self { connection }
    }
}


impl Dictionary {
    #[allow(clippy::missing_errors_doc)]
    pub fn stem(&self, word: &str) -> Result<String> {
        let stem = self.connection.query_one(
            "SELECT stem from dpd_headwords where lemma_1 == (?1)",
            [word],
            |row| Ok(Stem { stem: row.get(0)? }),
        )?;
        Ok(stem.stem)
    }
}

#[cfg(test)]
mod tests {}
