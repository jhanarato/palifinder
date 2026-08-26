use crate::texts::Segment;
use std::collections::HashSet;
use std::collections::hash_set::IntoIter;
use tantivy::tokenizer::{TextAnalyzer, TokenStream};

pub struct Vocabulary {
    analyzer: TextAnalyzer,
    tokens: HashSet<String>,
}

impl Vocabulary {
    #[must_use]
    pub fn new(segments: impl Iterator<Item = Segment>, analyzer: TextAnalyzer) -> Self {
        let mut vocabulary = Self {
            analyzer,
            tokens: HashSet::new(),
        };
        for segment in segments {
            vocabulary.add_text(segment.text.as_str());
        }
        vocabulary
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

        let analyzer = TextAnalyzer::builder(WhitespaceTokenizer::default()).build();

        let mut vocabulary: Vec<String> = Vocabulary::new(segments.into_iter(), analyzer)
            .into_iter()
            .collect();
        vocabulary.sort();

        assert_eq!(vocabulary, vec!("cat", "mat", "on", "sat", "the"));
    }
}
