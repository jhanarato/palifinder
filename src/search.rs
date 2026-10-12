use crate::index::PaliIndex;
use crate::texts::TextUid;
use anyhow::Result;
use tabled::Tabled;
use tantivy::collector::TopDocs;
use tantivy::query::{Query, QueryParser};
use tantivy::schema::{Field, Value};
use tantivy::snippet::SnippetGenerator;
use tantivy::{Index, IndexReader, ReloadPolicy, Score, TantivyDocument};

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
        let query = self.query_parser.parse_query(query)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10).order_by_score())?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(uid) = retrieved_doc.get_first(self.uid)
                && let Some(uid) = uid.as_str()
            {
                let snippet = self.snippet(&*query, &retrieved_doc)?;
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

    fn snippet(&self, query: &dyn Query, document: &TantivyDocument) -> Result<String> {
        let generator = SnippetGenerator::create(&self.reader.searcher(), query, self.contents)?;
        let snippet = generator.snippet_from_doc(document);
        println!("{snippet:#?}");
        Ok(snippet.to_html())
        // format!("Snippet text with {} highlighting", Style::new().italic().paint("italic"))
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
