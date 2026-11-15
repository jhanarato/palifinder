use std::collections::HashSet;
use std::collections::hash_set::IntoIter;
use tantivy::tokenizer::{TextAnalyzer, TokenStream};

pub struct Vocabulary {
    analyzer: TextAnalyzer,
    tokens: HashSet<String>,
}

impl Vocabulary {
    #[must_use]
    pub fn new<T: AsRef<str>>(texts: impl IntoIterator<Item = T>, analyzer: TextAnalyzer) -> Self {
        let mut vocabulary = Self {
            analyzer,
            tokens: HashSet::new(),
        };
        for text in texts {
            vocabulary.add_text(text.as_ref());
        }
        vocabulary
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn add_text(&mut self, text: &str) {
        let mut stream = self.analyzer.token_stream(text);
        stream.process(&mut |token| {
            self.tokens.insert(token.text.clone());
        });
    }
}

impl IntoIterator for Vocabulary {
    type Item = String;
    type IntoIter = IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::tokenizer::WhitespaceTokenizer;

    #[test]
    fn test_construct_from_texts() {
        let texts = ["the cat sat", "on the mat"];

        let analyzer = TextAnalyzer::builder(WhitespaceTokenizer::default()).build();

        let mut vocabulary: Vec<String> = Vocabulary::new(texts, analyzer)
            .into_iter()
            .collect();
        vocabulary.sort();

        assert_eq!(vocabulary, vec!("cat", "mat", "on", "sat", "the"));
    }
}
