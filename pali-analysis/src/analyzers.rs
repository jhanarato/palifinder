use crate::algo_stemmer::AlgorithmicStemmer;
use crate::dict_stemmer::DictionaryStemmer;
use crate::table::StemTable;
use crate::tokenizer::PaliTokenizer;
use anyhow::Error;
use csv::Reader;
use tantivy::tokenizer::{LowerCaser, StopWordFilter, TextAnalyzer};

pub enum AnalyzerConfig {
    Algorithmic,
    Dictionary { stem_table: Vec<u8> },
}

impl TryFrom<AnalyzerConfig> for TextAnalyzer {
    type Error = Error;

    fn try_from(config: AnalyzerConfig) -> Result<Self, Self::Error> {
        let stop_words = vec!["ca", "ti", "na", "pe", "vā", "kho", "hoti", "bhikkhave", "b", "so"];
        let stop_word_filter = StopWordFilter::remove(stop_words.into_iter().map(String::from));
            
        match config {
            AnalyzerConfig::Algorithmic => {
                Ok(TextAnalyzer::builder(PaliTokenizer::default())
                    .filter(LowerCaser)
                    .filter(stop_word_filter)
                    .filter(AlgorithmicStemmer)
                    .build())
            }
            AnalyzerConfig::Dictionary { stem_table } => {
                let table = StemTable::try_from(Reader::from_reader(stem_table.as_slice()))?;
                Ok(TextAnalyzer::builder(PaliTokenizer::default())
                    .filter(LowerCaser)
                    .filter(stop_word_filter)
                    .filter(DictionaryStemmer::from(table))
                    .build())
            }
        }
    }
}