use crate::algo_stemmer::AlgorithmicStemmer;
use crate::dict_stemmer::{DictionaryMode, DictionaryStemmer};
use crate::table::StemTable;
use crate::tokenizer::PaliTokenizer;
use anyhow::Error;
use csv::Reader;
use tantivy::tokenizer::{LowerCaser, StopWordFilter, TextAnalyzer};

const STOP_WORDS: [&str; 10] = ["ca", "ti", "na", "pe", "vā", "kho", "hoti", "bhikkhave", "b", "so"];

/// Stop words never reach the index, so they count for nothing elsewhere either.
#[must_use]
pub fn stop_word_filter() -> StopWordFilter {
    StopWordFilter::remove(STOP_WORDS.map(String::from))
}

pub enum AnalyzerConfig {
    Algorithmic,
    Dictionary { stem_table: Vec<u8>, mode: DictionaryMode },
}

impl TryFrom<AnalyzerConfig> for TextAnalyzer {
    type Error = Error;

    fn try_from(config: AnalyzerConfig) -> Result<Self, Self::Error> {
        let stop_word_filter = stop_word_filter();
            
        match config {
            AnalyzerConfig::Algorithmic => {
                Ok(TextAnalyzer::builder(PaliTokenizer::default())
                    .filter(LowerCaser)
                    .filter(stop_word_filter)
                    .filter(AlgorithmicStemmer)
                    .build())
            }
            AnalyzerConfig::Dictionary { stem_table, mode } => {
                let table = StemTable::try_from(Reader::from_reader(stem_table.as_slice()))?;
                let stemmer = DictionaryStemmer::new(table, mode);
                Ok(TextAnalyzer::builder(PaliTokenizer::default())
                    .filter(LowerCaser)
                    .filter(stop_word_filter)
                    .filter(stemmer)
                    .build())
            }
        }
    }
}