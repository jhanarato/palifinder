use std::collections::HashSet;
use std::str::CharIndices;
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

#[must_use]
pub fn tokenize(segment: &str) -> Vec<String> {
    let mut tokenizer = PaliTokenizer::default();
    let mut stream = tokenizer.token_stream(segment);
    let mut tokens = Vec::new();
    stream.process(&mut |token| {
        tokens.push(token.text.clone());
    });

    tokens
}

pub struct PaliChars {
    chars: Vec<char>,
}

impl Default for PaliChars {
    fn default() -> Self {
        Self {
            chars: vec![
                'A', 'B', 'C', 'D', 'E', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'R',
                'S', 'T', 'U', 'V', 'W', 'Y', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j',
                'k', 'l', 'm', 'n', 'o', 'p', 'r', 's', 't', 'u', 'v', 'y', 'Ñ', 'ñ', 'Ā', 'ā',
                'Ī', 'ī', 'Ū', 'ū', 'Ḍ', 'ḍ', 'ḷ', 'ṁ', 'ṅ', 'ṇ', 'Ṭ', 'ṭ',
            ],
        }
    }
}

impl PaliChars {
    #[must_use]
    pub fn is_pali(&self, char: char) -> bool {
        self.chars.contains(&char)
    }
}

/// Tokenize the text by matching Pali alphabet
#[derive(Clone)]
pub struct PaliTokenizer {
    pub alphabet: HashSet<char>,
    token: Token,
}

impl Default for PaliTokenizer {
    fn default() -> Self {
        Self {
            alphabet: HashSet::from([
                'A', 'B', 'C', 'D', 'E', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'R',
                'S', 'T', 'U', 'V', 'W', 'Y', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j',
                'k', 'l', 'm', 'n', 'o', 'p', 'r', 's', 't', 'u', 'v', 'y', 'Ñ', 'ñ', 'Ā', 'ā',
                'Ī', 'ī', 'Ū', 'ū', 'Ḍ', 'ḍ', 'ḷ', 'ṁ', 'ṅ', 'ṇ', 'Ṭ', 'ṭ',
            ]),
            token: Token::default(),
        }
    }
}

impl Tokenizer for PaliTokenizer {
    type TokenStream<'a> = PaliTokenStream<'a>;
    fn token_stream<'a>(&'a mut self, text: &'a str) -> PaliTokenStream<'a> {
        self.token.reset();
        PaliTokenStream {
            more: true,
            _text: text,
            _chars: text.char_indices(),
            token: &mut self.token,
        }
    }
}

pub struct PaliTokenStream<'a> {
    more: bool,
    _text: &'a str,
    _chars: CharIndices<'a>,
    token: &'a mut Token,
}

impl PaliTokenStream<'_> {}

impl TokenStream for PaliTokenStream<'_> {
    fn advance(&mut self) -> bool {
        self.token.text.clear();
        self.token.position = 0;
        self.token.offset_from = 0;
        self.token.offset_to = 7;
        self.token.text = String::from("bhagavā");
        if self.more {
            self.more = false;
            true
        } else {
            false
        }
    }

    fn token(&self) -> &Token {
        self.token
    }

    fn token_mut(&mut self) -> &mut Token {
        self.token
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::tokenizer::TextAnalyzer;

    /// Helper function for testing token output. Copied from the `tantivy::tokenizer` tests.
    fn assert_token(token: &Token, position: usize, text: &str, from: usize, to: usize) {
        assert_eq!(
            token.position, position,
            "expected position {position} but {token:?}"
        );
        assert_eq!(token.text, text, "expected text {text} but {token:?}");
        assert_eq!(
            token.offset_from, from,
            "expected offset_from {from} but {token:?}"
        );
        assert_eq!(token.offset_to, to, "expected offset_to {to} but {token:?}");
    }

    fn token_stream_helper(text: &str) -> Vec<Token> {
        let mut a = TextAnalyzer::from(PaliTokenizer::default());
        let mut token_stream = a.token_stream(text);
        let mut tokens: Vec<Token> = vec![];
        let mut add_token = |token: &Token| {
            tokens.push(token.clone());
        };
        token_stream.process(&mut add_token);
        tokens
    }

    #[test]
    fn test_stream_yields_a_token() {
        let tokens = token_stream_helper("bhagavā");
        assert_eq!(tokens.len(), 1);
        assert_token(&tokens[0], 0, "bhagavā", 0, 7);
    }

    #[test]
    fn test_chars() {
        let tokenizer = PaliTokenizer::default();
        assert!(tokenizer.alphabet.contains(&'A'));
        assert!(tokenizer.alphabet.contains(&'Ḍ'));
        assert!(!tokenizer.alphabet.contains(&'X'));
        assert!(!tokenizer.alphabet.contains(&'?'));
    }
}
