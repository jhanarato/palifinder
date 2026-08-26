use anyhow::Result;
use rusqlite::Connection;

#[derive(Debug)]
struct HeadwordFields {
    stem: String,
}

#[derive(Debug)]
struct LookupFields {
    headwords: String,
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
    pub fn stem(&self, term: &str) -> Result<String> {
        let stem = self.connection.query_one(
            "SELECT stem from dpd_headwords where lemma_1 == (?1)",
            [term],
            |row| Ok(HeadwordFields { stem: row.get(0)? }),
        )?;
        Ok(stem.stem)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn lookup(&self, term: &str) -> Result<Vec<usize>> {
        let lookup = self.connection.query_one(
            "SELECT headwords FROM lookup WHERE lookup_key == (?1) ",
            [term],
            |row| Ok(LookupFields { headwords: row.get(0)? }),
        )?;
        let ids: Vec<usize> = serde_json::from_str(lookup.headwords.as_str())?;
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {}
