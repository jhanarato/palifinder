use crate::dpd;
use crate::vocabulary::Vocabulary;
use anyhow::Result;
use csv::Writer;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[allow(clippy::implicit_hasher)]
pub fn stem_table(
    conn: &mut Connection,
    vocabulary: Vocabulary,
) -> HashMap<String, Option<String>> {
    let mut table: HashMap<String, Option<String>> = HashMap::new();
    for term in vocabulary {
        let term_stem = dpd::stem(conn, term.as_str());
        match term_stem {
            Ok(stem) => {
                table.insert(term.clone(), Some(stem));
            }
            Err(_) => {
                table.insert(term.clone(), None);
            }
        }
    }
    table
}

#[derive(Serialize)]
struct TermStem {
    term: String,
    stem: Option<String>,
}

#[allow(clippy::implicit_hasher, clippy::missing_errors_doc)]
pub fn save_table(table: &HashMap<String, Option<String>>, path: &Path) -> Result<()> {
    let mut writer = Writer::from_path(path)?;
    for (key, value) in table {
        let term_stem = TermStem {
            term: key.clone(),
            stem: value.clone(),
        };
        writer.serialize(term_stem)?;
    }
    Ok(())
}
