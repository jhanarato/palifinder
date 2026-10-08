use crate::analyzers::AnalyzerConfig;
use anyhow::Result;
use std::path::{Path, PathBuf};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions, Value};
use tantivy::tokenizer::TextAnalyzer;
use tantivy::{Index, ReloadPolicy, TantivyDocument};

pub struct PaliIndex {
    index: Index,
}

pub enum Location {
    InRam,
    InDir { index_path: PathBuf },
}

impl PaliIndex {
    const PLI_STEM: &str = "pli_stem";

    #[allow(clippy::missing_errors_doc)]
    pub fn create(location: Location, config: AnalyzerConfig) -> Result<Self> {
        let schema = Self::schema();

        let builder = Index::builder().schema(schema.clone());

        let index = match location {
            Location::InRam => builder.create_in_ram()?,
            Location::InDir { index_path } => builder.create_in_dir(index_path)?,
        };

        let analyzer = TextAnalyzer::try_from(config)?;
        index.tokenizers().register(Self::PLI_STEM, analyzer);

        Ok(Self { index })
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn open(path: &Path, config: AnalyzerConfig) -> Result<Self> {
        let index = Index::open_in_dir(path)?;
        let analyzer = TextAnalyzer::try_from(config)?;
        index.tokenizers().register(Self::PLI_STEM, analyzer);
        Ok(Self { index })
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

    #[allow(clippy::missing_errors_doc)]
    pub fn search(&self, query: &str) -> Result<Vec<String>> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let searcher = reader.searcher();
        let uid = self.index.schema().get_field("uid")?;
        let contents = self.index.schema().get_field("contents")?;
        let query_parser = QueryParser::for_index(&self.index, vec![contents]);
        let query = query_parser.parse_query(query)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10).order_by_score())?;

        let mut uids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(uid) = retrieved_doc.get_first(uid)
                && let Some(uid) = uid.as_str()
            {
                uids.push(String::from(uid));
            }
        }
        Ok(uids)
    }
}

impl AsRef<Index> for PaliIndex {
    fn as_ref(&self) -> &Index {
        &self.index
    }
}