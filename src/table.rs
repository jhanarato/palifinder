use crate::dpd;
use csv::Writer;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use anyhow::{Result};

#[allow(clippy::implicit_hasher)]
pub fn stem_table(
    conn: &mut Connection,
    vocabulary: &HashSet<String>,
) -> HashMap<String, Option<String>> {
    let mut table: HashMap<String, Option<String>> = HashMap::new();
    for term in vocabulary {
        let term_stem = dpd::stem(conn, term);
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

/// # Errors
///
/// Will return an `anyhow::Result` if there are errors writing the file.
#[allow(clippy::implicit_hasher)]
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