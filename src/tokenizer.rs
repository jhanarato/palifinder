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
            alphabet: &self.alphabet,
            text,
            chars: text.char_indices(),
            token: &mut self.token,
        }
    }
}

pub struct PaliTokenStream<'a> {
    alphabet: &'a HashSet<char>,
    text: &'a str,
    chars: CharIndices<'a>,
    token: &'a mut Token,
}

impl PaliTokenStream<'_> {
    fn search_token_end(&mut self) -> usize {
        (&mut self.chars)
            .filter(|(_, c)| !self.alphabet.contains(c))
            .map(|(offset, _)| offset)
            .next()
            .unwrap_or(self.text.len())
    }
}

impl TokenStream for PaliTokenStream<'_> {
    fn advance(&mut self) -> bool {
        self.token.text.clear();
        self.token.position = self.token.position.wrapping_add(1);
        while let Some((offset_from, c)) = self.chars.next() {
            if self.alphabet.contains(&c) {
                let offset_to = self.search_token_end();
                self.token.offset_from = offset_from;
                self.token.offset_to = offset_to;
                self.token.text.push_str(&self.text[offset_from..offset_to]);
                return true
            }
        }
        false
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
    fn test_tokenize_pali_words() {
        let tokens = token_stream_helper("Evaṁ me sutaṁ—");
        assert_eq!(tokens.len(), 3);
        assert_token(&tokens[0], 0, "Evaṁ", 0, 6);
        assert_token(&tokens[1], 1, "me", 7, 9);
        assert_token(&tokens[2], 2, "sutaṁ", 10, 17);
    }
}
