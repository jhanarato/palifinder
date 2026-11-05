use anyhow::Result;
use std::path::{Path, PathBuf};
use tantivy::Index;
use tantivy::schema::{Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions};
use tantivy::tokenizer::TextAnalyzer;

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
    pub fn create(location: Location, analyzer: TextAnalyzer) -> Result<Self> {
        let schema = Self::schema();

        let builder = Index::builder().schema(schema.clone());

        let index = match location {
            Location::InRam => builder.create_in_ram()?,
            Location::InDir { index_path } => builder.create_in_dir(index_path)?,
        };

        index.tokenizers().register(Self::PLI_STEM, analyzer);

        Ok(Self { index })
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn open(path: &Path, analyzer: TextAnalyzer) -> Result<Self> {
        let index = Index::open_in_dir(path)?;
        index.tokenizers().register(Self::PLI_STEM, analyzer);
        Ok(Self { index })
    }

    /// Whether a directory holds an index this program made: Tantivy can open it, so its
    /// meta.json is a full record with schema, opstamp and segments, and the schema is ours.
    #[must_use]
    pub fn is_ours(path: &Path) -> bool {
        Index::open_in_dir(path).is_ok_and(|index| index.schema() == Self::schema())
    }

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

        let contents_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer(Self::PLI_STEM)
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();

        schema_builder.add_text_field("contents", contents_options);
        schema_builder.build()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn uid_field(&self) -> Result<Field> {
        Ok(self.index.schema().get_field("uid")?)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn contents_field(&self) -> Result<Field> {
        Ok(self.index.schema().get_field("contents")?)
    }
}

impl AsRef<Index> for PaliIndex {
    fn as_ref(&self) -> &Index {
        &self.index
    }
}
