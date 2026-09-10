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
            let dpd_stem = Self::dpd_stem(term.as_str(), dictionary);
            entries.push(TermStem { term, dpd_stem });
        }
        Self { entries }
    }

    fn dpd_stem(term: &str, dictionary: &Dictionary) -> Option<String> {
        let stems = dictionary.stems(term);
        match stems {
            Ok(stems) => stems.first().cloned(),
            Err(_) => None,
        }
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
