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
    pub fn stems(&self, term: &str) ->  Result<Vec<String>> {
        let mut stems: Vec<String> = Vec::new();
        let ids = self.lookup(term)?;
        for id in ids {
            let stem = self.stem_for_id(id)?;
            stems.push(stem);
        }
        Ok(stems)
    }

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
    pub fn stem_for_id(&self, headword_id: u32) -> Result<String>{
        let stem = self.connection.query_one(
            "SELECT stem from dpd_headwords where id == (?1)",
            [headword_id],
            |row| Ok(HeadwordFields { stem: row.get(0)? }),
        )?;
        Ok(stem.stem)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn lookup(&self, term: &str) -> Result<Vec<u32>> {
        let lookup = self.connection.query_one(
            "SELECT headwords FROM lookup WHERE lookup_key == (?1) ",
            [term],
            |row| Ok(LookupFields { headwords: row.get(0)? }),
        )?;
        let ids: Vec<u32> = serde_json::from_str(lookup.headwords.as_str())?;
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {}
