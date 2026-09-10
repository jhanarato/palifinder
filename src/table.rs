use crate::dpd::Dictionary;
use crate::vocabulary::Vocabulary;
use anyhow::Result;
use csv::Writer;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct TermStem {
    pub term: String,
    pub dpd_stem: Option<String>,
}

#[derive(Clone)]
pub struct TermStems {
    entries: Vec<TermStem>,
}

impl TermStems {
    pub fn new(vocabulary: Vocabulary, dictionary: &Dictionary) -> Self {
        let mut entries = Vec::new();
        for term in vocabulary {
            let stems = dictionary.stems(term.as_str());
            match stems {
                Err(_) => entries.push(TermStem {
                    term: term.clone(),
                    dpd_stem: None,
                }),
                Ok(stems) => match stems.first() {
                    Some(stem) => entries.push(TermStem {
                        term: term.clone(),
                        dpd_stem: Some(stem.clone()),
                    }),
                    None => entries.push(TermStem {
                        term: term.clone(),
                        dpd_stem: None,
                    }),
                },
            }
        }

        Self { entries }
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn save(&self, path: &Path) -> Result<()> {
        let mut writer = Writer::from_path(path)?;
        for record in &self.entries {
            writer.serialize(record)?;
        }
        Ok(())
    }
}
