use crate::dpd::Dictionary;
use crate::vocabulary::Vocabulary;
use anyhow::Result;
use csv::Writer;
use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::snowball;

#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct TermStem {
    pub term: String,
    pub dpd_stem: Option<String>,
    pub snowball_stem: String,
}

#[derive(Clone)]
pub struct StemTable {
    entries: Vec<TermStem>,
}

impl StemTable {
    pub fn new(vocabulary: Vocabulary, dictionary: &Dictionary) -> Self {
        let mut entries = Vec::new();
        for term in vocabulary {
            let dpd_stem = Self::dpd_stem(term.as_str(), dictionary);
            let snowball_stem = snowball::pali_stem(term.as_str());
            entries.push(TermStem { term, dpd_stem, snowball_stem });
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
