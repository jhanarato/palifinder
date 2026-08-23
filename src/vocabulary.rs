use crate::texts::Segment;
use std::collections::HashSet;
use std::collections::hash_set::IntoIter;
use tantivy::tokenizer::{SimpleTokenizer, TokenStream, Tokenizer};

#[derive(Debug)]
pub struct Vocabulary {
    tokens: HashSet<String>,
}

impl Vocabulary {
    #[must_use]
    pub fn new(segments: impl Iterator<Item = Segment>) -> Self {
        let mut vocabulary = Self {
            tokens: HashSet::new(),
        };
        for segment in segments {
            vocabulary.add_text(segment.text.as_str());
        }
        vocabulary
    }

    pub fn add_text(&mut self, text: &str) {
        let mut tokenizer = SimpleTokenizer::default();
        let mut stream = tokenizer.token_stream(text);
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

    #[test]
    fn test_construct_from_segments() {
        let segments = vec![
            Segment {
                uid: String::from("uid1"),
                text: String::from("the cat sat"),
            },
            Segment {
                uid: String::from("uid1"),
                text: String::from("on the mat"),
            },
        ];

        let mut vocabulary: Vec<String> =
            Vocabulary::new(segments.into_iter()).into_iter().collect();
        vocabulary.sort();

        assert_eq!(vocabulary, vec!("cat", "mat", "on", "sat", "the"));
    }
}