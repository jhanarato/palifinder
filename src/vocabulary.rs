use crate::texts::Segment;
use std::collections::HashSet;
use std::collections::hash_set::IntoIter;
use tantivy::tokenizer::{TokenStream, Tokenizer};

#[derive(Debug)]
pub struct Vocabulary<T: Tokenizer> {
    tokenizer: T,
    tokens: HashSet<String>,
}

impl<T: Tokenizer> Vocabulary<T> {
    #[must_use]
    pub fn new(segments: impl Iterator<Item = Segment>, tokenizer: T) -> Self {
        let mut vocabulary = Self {
            tokenizer,
            tokens: HashSet::new(),
        };
        for segment in segments {
            vocabulary.add_text(segment.text.as_str());
        }
        vocabulary
    }

    pub fn add_text(&mut self, text: &str) {
        let mut stream = self.tokenizer.token_stream(text);
        stream.process(&mut |token| {
            self.tokens.insert(token.text.clone());
        });
    }
}

impl<T: Tokenizer> IntoIterator for Vocabulary<T> {
    type Item = String;
    type IntoIter = IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use tantivy::tokenizer::WhitespaceTokenizer;
    use super::*;

    #[test]
    fn test_construct_from_segments() {
        let segments = vec![
            Segment {
                uid: String::from("a"),
                text: String::from("the cat sat"),
            },
            Segment {
                uid: String::from("b"),
                text: String::from("on the mat"),
            },
        ];

        let mut vocabulary: Vec<String> =
            Vocabulary::new(segments.into_iter(), WhitespaceTokenizer::default())
                .into_iter()
                .collect();
        vocabulary.sort();

        assert_eq!(vocabulary, vec!("cat", "mat", "on", "sat", "the"));
    }
}
