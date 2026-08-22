use std::collections::HashSet;
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
        Self { tokens: HashSet::new() }
    }

    pub fn add_text(& mut self, text: &str) {
        let mut tokenizer = SimpleTokenizer::default();
        let mut stream = tokenizer.token_stream(text);
        stream.process(&mut |token| {
            self.tokens.insert(token.text.clone());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_text() {
        let mut vocabulary = Vocabulary::new();
        vocabulary.add_text("Evaṁ me sutaṁ—");
        assert_eq!(vocabulary.tokens.iter().len(), 3);
        assert!(vocabulary.tokens.contains("Evaṁ"));
        assert!(vocabulary.tokens.contains("me"));
        assert!(vocabulary.tokens.contains("sutaṁ"));
    }
}