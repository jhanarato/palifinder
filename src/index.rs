use crate::analyzers::AnalyzerConfig;
use crate::texts::PaliText;
use anyhow::Result;
use std::path::PathBuf;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions};
use tantivy::tokenizer::TextAnalyzer;
use tantivy::{Document, Index, IndexWriter, ReloadPolicy, TantivyDocument};

struct PaliIndex {
    schema: Schema,
    index: Index,
    writer: IndexWriter,
}

pub enum Location {
    InRam,
    InDir { path: PathBuf },
}

#[allow(unused)]
impl PaliIndex {
    const PLI_STEM: &str = "pli_stem";

    pub fn create(location: Location, config: AnalyzerConfig) -> Result<Self> {
        let schema = Self::schema();

        let builder = Index::builder().schema(schema.clone());

        let index = match location {
            Location::InRam => builder.create_in_ram()?,
            Location::InDir { path } => builder.create_in_dir(path)?,
        };

        let analyzer = TextAnalyzer::try_from(config)?;
        index.tokenizers().register(Self::PLI_STEM, analyzer);

        let mut writer: IndexWriter = index.writer(50_000_000)?;

        Ok(Self {
            schema,
            index,
            writer,
        })
    }

    #[allow(unused)]
    fn schema() -> Schema {
        let mut schema_builder = Schema::builder();

        let uid_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("raw")
                    .set_index_option(IndexRecordOption::Basic),
            )
            .set_stored();

        schema_builder.add_text_field("uid", uid_options);

        let contents_options = TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(Self::PLI_STEM)
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        );

        schema_builder.add_text_field("contents", contents_options);
        schema_builder.build()
    }

    pub fn add_text(&mut self, text: &PaliText) -> Result<()> {
        let uid = self.schema.get_field("uid")?;
        let contents = self.schema.get_field("contents")?;
        let mut document = TantivyDocument::default();
        document.add_text(uid, text.uid.clone());
        for segment in text.segments.clone() {
            document.add_text(contents, segment.text);
        }
        self.writer.add_document(document)?;
        self.writer.commit()?;
        Ok(())
    }

    fn search(&self, query: &str) -> Result<Vec<String>> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let searcher = reader.searcher();
        let contents = self.schema.get_field("contents")?;
        let query_parser = QueryParser::for_index(&self.index, vec![contents]);
        let query = query_parser.parse_query(query)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10).order_by_score())?;

        let mut json_documents = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            json_documents.push(retrieved_doc.to_json(&self.schema));
        }
        Ok(json_documents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texts::Segment;

    #[test]
    fn test_create_index_with_document() {
        let mut index = PaliIndex::create(Location::InRam, AnalyzerConfig::Algorithmic).unwrap();
        let text = PaliText {
            uid: String::from("mn1"),
            segments: vec![Segment {
                uid: String::from("mn1:1.1"),
                text: String::from("Evaṁ me sutaṁ—"),
            }],
        };
        index.add_text(&text).unwrap();
        let results = index.search("Evaṁ").unwrap();
        assert_eq!(results, vec![String::from(r#"{"uid":["mn1"]}"#)]);
    }
}
