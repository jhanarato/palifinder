use crate::table::TermStems;
use tantivy::tokenizer::{Token, TokenFilter, TokenStream, Tokenizer};

#[allow(unused)]
#[derive(Clone)]
pub struct PaliDictStemmer {
    term_stems: TermStems,
}

impl From<TermStems> for PaliDictStemmer {
    fn from(term_stems: TermStems) -> Self {
        Self { term_stems }
    }
}

impl TokenFilter for PaliDictStemmer {
    type Tokenizer<T: Tokenizer> = StemmerFilter<T>;

    fn transform<T: Tokenizer>(self, tokenizer: T) -> StemmerFilter<T> {
        StemmerFilter {
            inner: tokenizer,
        }
    }
}


#[allow(unused)]
#[derive(Clone)]
pub struct StemmerFilter<T> {
    inner: T,
}

impl<T: Tokenizer> Tokenizer for StemmerFilter<T> {
    type TokenStream<'a> = StemmerTokenStream<T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        StemmerTokenStream {
            tail: self.inner.token_stream(text),
            buffer: String::new()
        }
    }
}


#[allow(unused)]
pub struct StemmerTokenStream<T> {
    tail: T,
    buffer: String,
}

impl<T: TokenStream> TokenStream for StemmerTokenStream<T> {
    fn advance(&mut self) -> bool {
        if !self.tail.advance() {
            return false;
        }
        let token = self.tail.token_mut();
        token.text = String::from("Foobar");
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
    use crate::table::{TermStem, TermStems};
    use crate::tests::assert_token;
    use tantivy::tokenizer::{TextAnalyzer, Token, WhitespaceTokenizer};

    fn term_stems() -> TermStems {
        TermStems::from(vec![
            TermStem {
                term: String::from("jumped"),
                stem: Some(String::from("jump")),
            },
            TermStem {
                term: String::from("jumping"),
                stem: Some(String::from("jump")),
            },
            TermStem {
                term: String::from("frog"),
                stem: Some(String::from("frog")),
            },
            TermStem {
                term: String::from("xyz"),
                stem: None,
            },
        ])
    }

    fn token_stream_helper(text: &str) -> Vec<Token> {
        let mut token_stream = TextAnalyzer::builder(WhitespaceTokenizer::default())
            .filter(PaliDictStemmer::from(term_stems()))
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
    fn test_changes_token_to_stem_when_available() {
        let tokens = token_stream_helper("jumping jumped frog");
        assert_token(&tokens[0], 0, "jump", 0, 0);
        assert_token(&tokens[0], 0, "jump", 0, 0);
        assert_token(&tokens[0], 0, "frog", 0, 0);
    }

    #[test]
    fn test_token_unchanged_when_stem_unavailable() {
        let tokens = token_stream_helper("abc xyz frog");
        assert_token(&tokens[0], 0, "abc", 0, 0);
        assert_token(&tokens[0], 0, "xyz", 0, 0);
        assert_token(&tokens[0], 0, "frog", 0, 0);
    }
}
