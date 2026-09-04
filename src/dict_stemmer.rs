use crate::table::TermStem;
use anyhow::Error;
use csv::Reader;
use std::collections::HashMap;
use tantivy::tokenizer::{Token, TokenFilter, TokenStream, Tokenizer};

#[allow(unused)]
#[derive(Clone)]
pub struct DictionaryStemmer {
    term_stems: HashMap<String, String>,
}

impl TryFrom<&str> for DictionaryStemmer {
    type Error = Error;

    fn try_from(data: &str) -> Result<Self, Self::Error> {
        let mut term_stems: HashMap<String, String> = HashMap::new();
        let mut reader = Reader::from_reader(data.as_bytes());
        for record in reader.deserialize() {
            let term_stem: TermStem = record?;
            if let Some(stem) = term_stem.dpd_stem {
                term_stems.insert(term_stem.term, stem);
            }
        }
        Ok(Self { term_stems })
    }
}

impl TokenFilter for DictionaryStemmer {
    type Tokenizer<T: Tokenizer> = StemmerFilter<T>;

    fn transform<T: Tokenizer>(self, tokenizer: T) -> StemmerFilter<T> {
        StemmerFilter {
            inner: tokenizer,
            term_stems: self.term_stems,
        }
    }
}

#[allow(unused)]
#[derive(Clone)]
pub struct StemmerFilter<T> {
    inner: T,
    term_stems: HashMap<String, String>,
}

impl<T: Tokenizer> Tokenizer for StemmerFilter<T> {
    type TokenStream<'a> = StemmerTokenStream<'a, T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        StemmerTokenStream {
            tail: self.inner.token_stream(text),
            buffer: String::new(),
            term_stems: &mut self.term_stems,
        }
    }
}

#[allow(unused)]
pub struct StemmerTokenStream<'a, T> {
    tail: T,
    buffer: String,
    term_stems: &'a mut HashMap<String, String>,
}

impl<T: TokenStream> TokenStream for StemmerTokenStream<'_, T> {
    fn advance(&mut self) -> bool {
        if !self.tail.advance() {
            return false;
        }
        let token = self.tail.token_mut();
        if let Some(stem) = self.term_stems.get(&token.text)
        {
            token.text = stem.clone();
        }
        true
    }

    fn token(&self) -> &Token {
        self.tail.token()
    }

    fn token_mut(&mut self) -> &mut Token {
        self.tail.token_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::assert_token;
    use tantivy::tokenizer::{TextAnalyzer, Token, WhitespaceTokenizer};

    const STEM_DATA: &str = "\
term,dpd_stem
jumped,jump
jumping,jump
frog,";

    fn token_stream_helper(text: &str) -> Vec<Token> {
        let stemmer = DictionaryStemmer::try_from(STEM_DATA).unwrap();
        let mut token_stream = TextAnalyzer::builder(WhitespaceTokenizer::default())
            .filter(stemmer)
            .build();

        let mut token_stream = token_stream.token_stream(text);
        let mut tokens = vec![];
        let mut add_token = |token: &Token| {
            tokens.push(token.clone());
        };
        token_stream.process(&mut add_token);
        tokens
    }

    #[test]
    fn test_from_str() {
        let stemmer = DictionaryStemmer::try_from(STEM_DATA).unwrap();
        assert_eq!(stemmer.term_stems.get("jumped").unwrap(), "jump");
        assert_eq!(stemmer.term_stems.get("jumping").unwrap(), "jump");
        assert_eq!(stemmer.term_stems.get("frog"), None);
    }

    #[test]
    fn test_changes_token_to_stem_when_available() {
        let tokens = token_stream_helper("jumping jumped frog");
        assert_token(&tokens[0], 0, "jump", 0, 7);
        assert_token(&tokens[1], 1, "jump", 8, 14);
        assert_token(&tokens[2], 2, "frog", 15, 19);
    }

    #[test]
    fn test_token_unchanged_when_stem_unavailable() {
        let tokens = token_stream_helper("abc xyz frog");
        assert_token(&tokens[0], 0, "abc", 0, 3);
        assert_token(&tokens[1], 1, "xyz", 4, 7);
        assert_token(&tokens[2], 2, "frog", 8, 12);
    }
}
