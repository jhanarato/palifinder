use crate::snowball;
use std::borrow::Cow;
use std::mem;
use tantivy::tokenizer::{Token, TokenFilter, TokenStream, Tokenizer};

#[derive(Clone)]
pub struct AlgorithmicStemmer;

impl TokenFilter for AlgorithmicStemmer {
    type Tokenizer<T: Tokenizer> = StemmerFilter<T>;

    fn transform<T: Tokenizer>(self, tokenizer: T) -> StemmerFilter<T> {
        StemmerFilter {
            inner: tokenizer,
        }
    }
}

#[derive(Clone)]
#[allow(unused)]
pub struct StemmerFilter<T> {
    inner: T,
}

impl<T: Tokenizer> Tokenizer for StemmerFilter<T> {
    type TokenStream<'a> = StemmerTokenStream<T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        StemmerTokenStream {
            tail: self.inner.token_stream(text),
            buffer: String::new(),
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

        match snowball::pali_stem(&token.text) {
            Cow::Owned(stemmed_str) => token.text = stemmed_str,
            Cow::Borrowed(stemmed_str) => {
                self.buffer.clear();
                self.buffer.push_str(stemmed_str);
                mem::swap(&mut token.text, &mut self.buffer);
            }
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
    use crate::algo_stemmer::AlgorithmicStemmer;
    use crate::tests::assert_token;
    use tantivy::tokenizer::{TextAnalyzer, Token, WhitespaceTokenizer};

    fn token_stream_helper(text: &str) -> Vec<Token> {
        let stemmer = AlgorithmicStemmer;
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
    fn test_algorithmic_stemmer() {
        let tokens = token_stream_helper("Tatra kho bhagavā");
        assert_token(&tokens[0], 0, "Tatr", 0, 5);
        assert_token(&tokens[1], 1, "kh", 6, 9);
        assert_token(&tokens[2], 2, "bhagav", 10, 18);
    }
}