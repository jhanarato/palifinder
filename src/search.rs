use anyhow::Result;
use tantivy::schema::{Field, Value};
use tantivy::{Index, IndexReader, ReloadPolicy, TantivyDocument};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;

#[allow(unused)]
pub struct PaliSearcher {
    reader: IndexReader,
    query_parser: QueryParser,
    uid: Field,
    contents: Field,
}

impl PaliSearcher {
    pub fn new(index: &Index) -> Result<Self> {
        let reader = Self::reader(index)?;
        let uid = index.schema().get_field("uid")?;
        let contents = index.schema().get_field("contents")?;
        let query_parser = QueryParser::for_index(index, vec![contents]);

        Ok(Self {
            reader,
            query_parser,
            uid,
            contents,
        })
    }

    fn reader(index: &Index) ->  Result<IndexReader>{
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        Ok(reader)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn search(&self, query: &str) -> Result<Vec<String>> {
        let searcher = self.reader.searcher();
        let query = self.query_parser.parse_query(query)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10).order_by_score())?;

        let mut uids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(uid) = retrieved_doc.get_first(self.uid)
                && let Some(uid) = uid.as_str()
            {
                uids.push(String::from(uid));
            }
        }
        Ok(uids)
    }
}
