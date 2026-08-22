use std::collections::HashSet;
use std::collections::hash_set::IntoIter;
use tantivy::tokenizer::{SimpleTokenizer, TokenStream, Tokenizer};

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
}
