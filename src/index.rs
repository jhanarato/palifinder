use tantivy::Index;
use tantivy::schema::{STORED, Schema, TEXT};
use anyhow::Result;
use tantivy::tokenizer::TextAnalyzer;
use crate::analyzers::AnalyzerConfig;

#[allow(clippy::missing_errors_doc)]
pub fn index() -> Result<Index> {
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("uid", TEXT | STORED);
    schema_builder.add_text_field("contents", TEXT);
    let schema = schema_builder.build();

    let pli_stem = TextAnalyzer::try_from(AnalyzerConfig::Algorithmic)?;
    let index = Index::builder().schema(schema).create_in_ram()?;
    index.tokenizers().register("pli_stem", pli_stem);
    Ok(index)
}