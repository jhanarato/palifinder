use crate::dpd::Dictionary;
use crate::vocabulary::Vocabulary;
use anyhow::{Error, Result};
use csv::{Reader, Writer};
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
    records: Vec<TermStem>,
}

impl StemTable {
    pub fn new(vocabulary: Vocabulary, dictionary: &Dictionary) -> Self {
        let mut entries = Vec::new();
        for term in vocabulary {
            let dpd_stem = Self::dpd_stem(term.as_str(), dictionary);
            let snowball_stem = snowball::pali_stem(term.as_str());
            entries.push(TermStem { term, dpd_stem, snowball_stem });
        }
        Self { records: entries }
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
        for record in &self.records {
            writer.serialize(record)?;
        }
        Ok(())
    }
}

impl<T> TryFrom<Reader<T>> for StemTable
where
    T: std::io::Read,
{
    type Error = Error;
    fn try_from(mut reader: Reader<T>) -> Result<Self, Self::Error> {
        let mut records: Vec<TermStem> = Vec::new();
        for record in reader.deserialize() {
            records.push(record?);
        }
        Ok(Self { records })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv::Reader;

    const STEM_DATA: &str = "\
term,dpd_stem,snowball_stem
jumped,jump,jump
jumping,jump,jump
frog,,frog";

    #[test]
    fn test_try_from_reader() {
        let reader = Reader::from_reader(STEM_DATA.as_bytes());
        let _table = StemTable::try_from(reader);
    }
}