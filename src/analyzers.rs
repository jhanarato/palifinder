use crate::algo_stemmer::AlgorithmicStemmer;
use crate::dict_stemmer::DictionaryStemmer;
use crate::stop_words::stop_word_filter;
use crate::table::StemTable;
use crate::tokenizer::PaliTokenizer;
use anyhow::Error;
use csv::Reader;
use std::path::PathBuf;
use tantivy::tokenizer::{LowerCaser, TextAnalyzer};

pub enum AnalyzerConfig {
    Algorithmic,
    Dictionary { stem_file: PathBuf },
}

impl TryFrom<AnalyzerConfig> for TextAnalyzer {
    type Error = Error;

    fn try_from(config: AnalyzerConfig) -> Result<Self, Self::Error> {
        match config {
            AnalyzerConfig::Algorithmic => {
                Ok(TextAnalyzer::builder(PaliTokenizer::default())
                    .filter(LowerCaser)
                    .filter(stop_word_filter())
                    .filter(AlgorithmicStemmer)
                    .build())
            }
            AnalyzerConfig::Dictionary { stem_file } => {
                let reader = Reader::from_path(stem_file)?;
                let table = StemTable::try_from(reader)?;
                Ok(TextAnalyzer::builder(PaliTokenizer::default())
                    .filter(LowerCaser)
                    .filter(stop_word_filter())
                    .filter(DictionaryStemmer::from(table))
                    .build())
            }
        }
    }
}
