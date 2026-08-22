use crate::texts::{PaliFiles, PaliText, Segment};
use anyhow::{Error, Result};
use std::collections::HashSet;
use std::collections::hash_set::IntoIter;
use tantivy::tokenizer::{SimpleTokenizer, TokenStream, Tokenizer};

#[derive(Debug)]
pub struct Vocabulary {
    #[allow(dead_code)]
    tokens: HashSet<String>,
}

impl Default for Vocabulary {
    fn default() -> Self {
        Self::new()
    }
}

impl Vocabulary {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            tokens: HashSet::new(),
        }
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

impl From<Segment> for Vocabulary {
    fn from(segment: Segment) -> Self {
        let mut vocabulary = Self::new();
        vocabulary.add_text(segment.text.as_str());
        vocabulary
    }
}

impl From<PaliText> for Vocabulary {
    fn from(pali_text: PaliText) -> Self {
        let mut vocabulary = Self::new();
        for segment in pali_text.segments {
            vocabulary.add_text(segment.text.as_str());
        }
        vocabulary
    }
}

impl TryFrom<PaliFiles> for Vocabulary {
    type Error = Error;
    fn try_from(pali_files: PaliFiles) -> Result<Self, Self::Error> {
        let mut vocabulary = Self::new();
        for file in pali_files.files() {
            let json = std::fs::read_to_string(file)?;
            let text = PaliText::parse(json.as_str())?;
            for segment in text.segments {
                vocabulary.add_text(segment.text.as_str());
            }
        }
        Ok(vocabulary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_iterator() {
        let mut vocabulary = Vocabulary::new();
        vocabulary.add_text("Evaṁ me sutaṁ—");
        let mut words: Vec<String> = vocabulary.into_iter().collect();
        words.sort();
        assert_eq!(words, vec!("Evaṁ", "me", "sutaṁ"));
    }

    #[test]
    fn test_from_segment() {
        let segment = Segment {
            uid: String::from("mn1:0.1"),
            text: String::from("Evaṁ me sutaṁ—"),
        };

        let vocabulary = Vocabulary::from(segment);

        let mut words: Vec<String> = vocabulary.into_iter().collect();
        words.sort();
        assert_eq!(words, vec!("Evaṁ", "me", "sutaṁ"));
    }

    #[test]
    fn test_from_pali_text() {
        let json = r#"
        {
            "mn1:0.2": "Mūlapariyāyasutta ",
            "mn1:1.1": "Evaṁ me sutaṁ—"
        }
        "#;
        let text = PaliText::parse(json).unwrap();
        let vocabulary = Vocabulary::from(text);
        let mut words: Vec<String> = vocabulary.into_iter().collect();
        words.sort();
        assert_eq!(words, vec!("Evaṁ", "Mūlapariyāyasutta", "me", "sutaṁ"));
    }
}
