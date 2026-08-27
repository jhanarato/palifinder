use crate::dpd::Dictionary;
use crate::vocabulary::Vocabulary;
use anyhow::Result;
use csv::Writer;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize)]
struct TermStem {
    term: String,
    stem: Option<String>,
}

pub struct TermStems {
    entries: HashMap<String, Option<String>>,
}

impl TermStems {
    pub fn new(vocabulary: Vocabulary, dictionary: &Dictionary) -> Self {
        let mut entries: HashMap<String, Option<String>> = HashMap::new();
        for term in vocabulary {
            let stems = dictionary.stems(term.as_str());
            match stems {
                Err(_) => entries.insert(term.clone(), None),
                Ok(stems) => {
                    match stems.first() {
                        Some(stem) => entries.insert(term.clone(), Some(stem.clone())),
                        None => entries.insert(term.clone(), None),
                    }
                }
            };
        }

        Self { entries }
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn save_as(&self, path: &PathBuf) -> Result<()> {
        let mut writer = Writer::from_path(path)?;
        for (key, value) in &self.entries {
            let term_stem = TermStem {
                term: key.clone(),
                stem: value.clone(),
            };
            writer.serialize(term_stem)?;
        }
        Ok(())
    }
}