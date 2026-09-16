use crate::algo_stemmer::AlgorithmicStemmer;
use crate::dict_stemmer::DictionaryStemmer;
use crate::stop_words::stop_word_filter;
use crate::table::StemTable;
use crate::tokenizer::PaliTokenizer;
use csv::Reader;
use std::path::PathBuf;
use tantivy::tokenizer::{LowerCaser, TextAnalyzer};
use anyhow::Result;

pub enum AnalyzerConfig {
    Algorithmic,
    Dictionary { stem_file: PathBuf },
}

impl AnalyzerConfig {
    #[must_use]
    pub fn build(self) -> Result<TextAnalyzer> {
        match self {
            Self::Algorithmic => {
                Ok(TextAnalyzer::builder(PaliTokenizer::default())
                    .filter(LowerCaser)
                    .filter(stop_word_filter())
                    .filter(AlgorithmicStemmer)
                    .build())
            }
            Self::Dictionary { stem_file } => {
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
