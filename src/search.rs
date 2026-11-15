use crate::index::PaliIndex;
use crate::texts::TextUid;
use ansi_term::Colour;
use anyhow::Result;
use tabled::Tabled;
use tantivy::collector::TopDocs;
use tantivy::query::{Query, QueryParser, QueryParserError};
use tantivy::schema::{Field, Value};
use tantivy::snippet::{Snippet, SnippetGenerator};
use tantivy::{Index, IndexReader, ReloadPolicy, Score, TantivyDocument};
use pali_analysis::tokenizer::nfc;

/// Marks the parser reads as syntax, removed from a query that does not parse.
const SYNTAX: [char; 16] = [':', '[', ']', '<', '>', '{', '}', '(', ')', '*', '^', '~', '\'', '/', '-', '+'];

#[allow(unused)]
pub struct PaliSearcher {
    reader: IndexReader,
    query_parser: QueryParser,
    uid: Field,
    contents: Field,
}

impl PaliSearcher {
    pub fn new(index: &PaliIndex) -> Result<Self> {
        let contents = index.contents_field()?;
        Ok(Self {
            reader: Self::reader(index.as_ref())?,
            query_parser: QueryParser::for_index(index.as_ref(), vec![contents]),
            uid: index.uid_field()?,
            contents: index.contents_field()?,
        })
    }

    fn reader(index: &Index) -> Result<IndexReader> {
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        Ok(reader)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        let Some(query) = self.parse(&nfc(query)) else {
            return Ok(Vec::new());
        };
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10).order_by_score())?;

        let generator = SnippetGenerator::create(&searcher, &*query, self.contents)?;
        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(uid) = retrieved_doc.get_first(self.uid)
                && let Some(uid) = uid.as_str()
            {
                let snippet = Self::highlight(&generator.snippet_from_doc(&retrieved_doc));
                let result = SearchResult {
                    uid: TextUid::try_from(uid)?,
                    score,
                    snippet,
                };

                results.push(result);
            }
        }
        Ok(results)
    }

    /// A valid query means what the query language says. One that does not parse, as
    /// pasted Pali may not, is searched as plain words. Nothing but stop words parses to
    /// "everything", and should find nothing.
    fn parse(&self, query: &str) -> Option<Box<dyn Query>> {
        match self.query_parser.parse_query(query) {
            Ok(query) => return Some(query),
            Err(QueryParserError::AllButQueryForbidden) => return None,
            Err(_) => {}
        }
        let (query, errors) = self.query_parser.parse_query_lenient(&Self::clean_query(query));
        if errors.iter().any(|error| matches!(error, QueryParserError::AllButQueryForbidden)) {
            return None;
        }
        for error in errors {
            eprintln!("ignored part of the query: {error}");
        }
        Some(query)
    }

    /// The query as plain words. Matched quotes stay, so that phrase search still works.
    fn clean_query(query: &str) -> String {
        let query: String = query.chars().map(|c| if SYNTAX.contains(&c) { ' ' } else { c }).collect();
        if query.matches('"').count().is_multiple_of(2) {
            query
        } else {
            query.replace('"', " ")
        }
    }

    fn highlight(snippet: &Snippet) -> String {
        let mut result = String::new();
        let mut start_from = 0;

        for fragment_range in snippet.highlighted() {
            result.push_str(&snippet.fragment()[start_from..fragment_range.start]);
            let fragment = &snippet.fragment()[fragment_range.clone()];
            let red_fragment = Colour::Green.paint(fragment).to_string();
            result.push_str(&red_fragment);
            start_from = fragment_range.end;
        }

        result.push_str(&snippet.fragment()[start_from..]);
        result
    }
}

#[derive(Tabled)]
pub struct SearchResult {
    #[tabled(rename = "Text UID")]
    pub uid: TextUid,
    #[tabled(rename = "Relevance Score")]
    pub score: Score,
    #[tabled(rename = "Keywords in Context")]
    pub snippet: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pali_analysis::analyzers::AnalyzerConfig;
    use crate::index::Location;
    use crate::texts::{PaliText, TextUid};
    use crate::writer::PaliWriter;
    use tantivy::tokenizer::TextAnalyzer;

    fn searcher() -> PaliSearcher {
        let analyzer = TextAnalyzer::try_from(AnalyzerConfig::Algorithmic).unwrap();
        let index = PaliIndex::create(Location::InRam, analyzer).unwrap();
        let text = PaliText::for_test("mn1", "Saṁyuttaṁ nibbānassa dhammassa.");
        let mut writer = PaliWriter::new(&index).unwrap();
        writer.add_texts(std::iter::once(Ok(text))).unwrap();
        PaliSearcher::new(&index).unwrap()
    }

    fn found(query: &str) -> Vec<TextUid> {
        searcher().search(query).unwrap().into_iter().map(|r| r.uid).collect()
    }

    fn mn1() -> Vec<TextUid> {
        vec![TextUid::try_from("mn1").unwrap()]
    }

    #[test]
    fn test_inflections_find_each_other() {
        assert_eq!(found("nibbānaṁ"), mn1());
        assert_eq!(found("dhammo"), mn1());
    }

    #[test]
    fn test_either_niggahita_finds_the_text() {
        assert_eq!(found("saṃyutta"), mn1());
        assert_eq!(found("saṁyutta"), mn1());
    }

    #[test]
    fn test_decomposed_query_finds_the_text() {
        assert_eq!(found("nibba\u{304}nassa"), mn1());
    }

    #[test]
    fn test_punctuation_is_not_an_error() {
        assert_eq!(found("dhammassa: \"nibbānassa"), mn1());
    }

    #[test]
    fn test_a_query_that_does_not_parse_is_searched_as_words() {
        assert_eq!(found("<nibbānassa>"), mn1());
        assert_eq!(found("[dhammassa] nibbānassa"), mn1());
        assert_eq!(found("'dhammassa / nibbānassa"), mn1());
        assert_eq!(found("nibbānassa)"), mn1());
    }

    #[test]
    fn test_a_valid_query_keeps_its_meaning() {
        let none = Vec::<TextUid>::new();
        assert_eq!(found("dhammassa -nibbānassa"), none);
        assert_eq!(found("+dhammassa +xyzzy"), none);
        assert_eq!(found("xyzzy AND (dhammassa OR nibbānassa)"), none);
        assert_eq!(found("uid:mn1"), mn1());
    }

    #[test]
    fn test_stop_words_alone_find_nothing() {
        assert_eq!(found("na ca"), Vec::<TextUid>::new());
    }

    #[test]
    fn test_matched_quotes_search_a_phrase() {
        assert_eq!(found("\"nibbānassa dhammassa\""), mn1());
        assert_eq!(found("\"dhammassa nibbānassa\""), Vec::<TextUid>::new());
    }

    #[test]
    fn test_unknown_and_empty_queries_find_nothing() {
        assert_eq!(found("xyzzy"), Vec::<TextUid>::new());
        assert_eq!(found(""), Vec::<TextUid>::new());
    }
}
