use crate::dpd::Dictionary;
use crate::vocabulary::Vocabulary;
use anyhow::Result;
use csv::Writer;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize)]
pub struct TermStem {
    pub term: String,
    pub dpd_stem: Option<String>,
}

#[derive(Clone)]
pub struct TermStems {
    pub entries: HashMap<String, Option<String>>,
}

impl TermStems {
    pub fn new(vocabulary: Vocabulary, dictionary: &Dictionary) -> Self {
        let mut entries: HashMap<String, Option<String>> = HashMap::new();
        for term in vocabulary {
            let stems = dictionary.stems(term.as_str());
            match stems {
                Err(_) => entries.insert(term.clone(), None),
                Ok(stems) => match stems.first() {
                    Some(stem) => entries.insert(term.clone(), Some(stem.clone())),
                    None => entries.insert(term.clone(), None),
                },
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
                dpd_stem: value.clone(),
            };
            writer.serialize(term_stem)?;
        }
        Ok(())
    }
}

impl From<Vec<TermStem>> for TermStems {
    fn from(term_stems: Vec<TermStem>) -> Self {
        let mut entries = HashMap::new();
        for term_stem in term_stems {
            entries.insert(term_stem.term, term_stem.dpd_stem);
        }
        Self { entries }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_term_stems() {
        let term_stems = TermStems::from(vec![TermStem {
            term: String::from("jumped"),
            dpd_stem: Some(String::from("jump")),
        }]);

        assert_eq!(term_stems.entries.len(), 1);
        assert_eq!(
            term_stems.entries.get("jumped").unwrap(),
            &Some(String::from("jump"))
        );
    }
}
