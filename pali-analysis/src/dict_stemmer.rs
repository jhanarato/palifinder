use crate::snowball;
use crate::table::StemTable;
use std::collections::HashMap;
use std::sync::Arc;
use tantivy::tokenizer::{Token, TokenFilter, TokenStream, Tokenizer};

/// How `DictionaryStemmer` treats terms beyond plain dictionary stems.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DictionaryMode {
    /// A term DPD lacks stays as it is.
    Plain,
    /// A term DPD lacks takes the commonest word with its Snowball stem, or that stem.
    Hybrid,
    /// As `Hybrid`, and the verbal forms of a root share one term.
    Family,
}

#[derive(Clone)]
pub struct DictionaryStemmer {
    term_stems: Arc<HashMap<String, String>>,
    /// For a query form absent from the table: its index term by Snowball stem.
    fallback_stems: Option<Arc<HashMap<String, String>>>,
}

impl DictionaryStemmer {
    #[must_use]
    pub fn new(table: StemTable, mode: DictionaryMode) -> Self {
        let records: Vec<_> = table.into_iter().collect();
        let families: HashMap<String, String> = if mode == DictionaryMode::Family {
            records
                .iter()
                .filter_map(|record| Some((record.dpd_stem.clone()?, record.family.clone()?)))
                .collect()
        } else {
            HashMap::new()
        };
        let index_term = |word: &String| families.get(word).unwrap_or(word).clone();

        let mut term_stems = HashMap::new();
        let mut fallback_stems = HashMap::new();
        for record in &records {
            let fallback = record.fallback.as_ref().map_or_else(|| record.snowball_stem.clone(), index_term);
            fallback_stems.insert(record.snowball_stem.clone(), fallback.clone());
            let stem = match (&record.dpd_stem, mode) {
                (Some(word), _) => Some(index_term(word)),
                (None, DictionaryMode::Plain) => None,
                (None, _) => Some(fallback),
            };
            if let Some(stem) = stem {
                term_stems.insert(record.term.clone(), stem);
            }
        }
        Self {
            term_stems: Arc::new(term_stems),
            fallback_stems: (mode != DictionaryMode::Plain).then(|| Arc::new(fallback_stems)),
        }
    }
}

impl TokenFilter for DictionaryStemmer {
    type Tokenizer<T: Tokenizer> = StemmerFilter<T>;

    fn transform<T: Tokenizer>(self, tokenizer: T) -> StemmerFilter<T> {
        StemmerFilter {
            inner: tokenizer,
            term_stems: self.term_stems,
            fallback_stems: self.fallback_stems,
        }
    }
}

#[allow(unused)]
#[derive(Clone)]
pub struct StemmerFilter<T> {
    inner: T,
    term_stems: Arc<HashMap<String, String>>,
    fallback_stems: Option<Arc<HashMap<String, String>>>,
}

impl<T: Tokenizer> Tokenizer for StemmerFilter<T> {
    type TokenStream<'a> = StemmerTokenStream<'a, T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        StemmerTokenStream {
            tail: self.inner.token_stream(text),
            buffer: String::new(),
            term_stems: &self.term_stems,
            fallback_stems: self.fallback_stems.as_deref(),
        }
    }
}

#[allow(unused)]
pub struct StemmerTokenStream<'a, T> {
    tail: T,
    buffer: String,
    term_stems: &'a HashMap<String, String>,
    fallback_stems: Option<&'a HashMap<String, String>>,
}

impl<T: TokenStream> TokenStream for StemmerTokenStream<'_, T> {
    fn advance(&mut self) -> bool {
        if !self.tail.advance() {
            return false;
        }
        let token = self.tail.token_mut();
        if let Some(stem) = self.term_stems.get(&token.text) {
            token.text.clone_from(stem);
        } else if let Some(fallback_stems) = self.fallback_stems {
            // Only an inflected query form absent from the texts can be missing here.
            let stem = snowball::pali_stem(&token.text);
            token.text = fallback_stems.get(&stem).cloned().unwrap_or(stem);
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
    use csv::Reader;
    use super::*;
    use crate::table::StemTable;
    use crate::tests::assert_token;
    use tantivy::tokenizer::{TextAnalyzer, Token, WhitespaceTokenizer};

    const STEM_DATA: &str = "\
term,dpd_stem,snowball_stem,family,fallback
jumped,jump,jump,,jump
jumping,jump,jump,,jump
frog,,frog,,";

    const FAMILY_DATA: &str = "\
term,dpd_stem,snowball_stem,family,fallback
jumped,jump,jum,family:jump,jump
frog,,frogstem,,";

    fn stems(text: &str, mode: DictionaryMode) -> Vec<String> {
        let table = StemTable::try_from(Reader::from_reader(FAMILY_DATA.as_bytes())).unwrap();
        let mut analyzer = TextAnalyzer::builder(WhitespaceTokenizer::default())
            .filter(DictionaryStemmer::new(table, mode))
            .build();
        let mut stream = analyzer.token_stream(text);
        let mut tokens = Vec::new();
        stream.process(&mut |token: &Token| tokens.push(token.text.clone()));
        tokens
    }

    fn token_stream_helper(text: &str) -> Vec<Token> {
        let reader = Reader::from_reader(STEM_DATA.as_bytes());
        let table = StemTable::try_from(reader).unwrap();
        let stemmer = DictionaryStemmer::new(table, DictionaryMode::Plain);
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
    fn test_changes_token_to_stem_when_available() {
        let tokens = token_stream_helper("jumping jumped frog");
        assert_token(&tokens[0], 0, "jump", 0, 7);
        assert_token(&tokens[1], 1, "jump", 8, 14);
        assert_token(&tokens[2], 2, "frog", 15, 19);
    }

    #[test]
    fn test_snowball_stems_a_term_missing_from_the_table() {
        let tokens = stems("frog dhammassa", DictionaryMode::Hybrid);
        assert_eq!(tokens, vec!("frogstem", "dhamm"));
    }

    #[test]
    fn test_unseen_form_joins_the_commonest_word_with_its_snowball_stem() {
        let table = "term,dpd_stem,snowball_stem,family,fallback\ndhammo,dhamma,dhamm,,dhamma\n";
        let table = StemTable::try_from(Reader::from_reader(table.as_bytes())).unwrap();
        let mut analyzer = TextAnalyzer::builder(WhitespaceTokenizer::default())
            .filter(DictionaryStemmer::new(table, DictionaryMode::Hybrid))
            .build();
        let mut stream = analyzer.token_stream("dhammassa");
        let mut tokens = Vec::new();
        stream.process(&mut |token: &Token| tokens.push(token.text.clone()));
        assert_eq!(tokens, vec!("dhamma"));
    }

    #[test]
    fn test_family_mode_prefers_the_family() {
        assert_eq!(stems("jumped frog", DictionaryMode::Family), vec!("family:jump", "frogstem"));
        assert_eq!(stems("jumped", DictionaryMode::Hybrid), vec!("jump"));
    }

    #[test]
    fn test_token_unchanged_when_stem_unavailable() {
        let tokens = token_stream_helper("abc xyz frog");
        assert_token(&tokens[0], 0, "abc", 0, 3);
        assert_token(&tokens[1], 1, "xyz", 4, 7);
        assert_token(&tokens[2], 2, "frog", 8, 12);
    }
}
