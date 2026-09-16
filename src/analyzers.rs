use crate::algo_stemmer::AlgorithmicStemmer;
use crate::dict_stemmer::DictionaryStemmer;
use crate::stop_words::stop_word_filter;
use crate::table::StemTable;
use crate::tokenizer::PaliTokenizer;
use tantivy::tokenizer::{LowerCaser, TextAnalyzer};

pub enum AnalyzerConfig {
    Algorithmic,
    Dictionary { table: StemTable },
}

impl AnalyzerConfig {
    #[must_use]
    pub fn build(self) -> TextAnalyzer {
        match self {
            Self::Algorithmic => TextAnalyzer::builder(PaliTokenizer::default())
                .filter(LowerCaser)
                .filter(stop_word_filter())
                .filter(AlgorithmicStemmer)
                .build(),
            Self::Dictionary { table } => TextAnalyzer::builder(PaliTokenizer::default())
                .filter(LowerCaser)
                .filter(stop_word_filter())
                .filter(DictionaryStemmer::from(table))
                .build(),
        }
    }
}
