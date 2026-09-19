use crate::analyzers::AnalyzerConfig;
use anyhow::Result;
use tantivy::Index;
use tantivy::schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions};
use tantivy::tokenizer::TextAnalyzer;

#[allow(clippy::missing_errors_doc)]
pub fn index() -> Result<Index> {
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
            .set_tokenizer("pli_stem")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );

    schema_builder.add_text_field("contents", contents_options);
    let schema = schema_builder.build();

    let pli_stem = TextAnalyzer::try_from(AnalyzerConfig::Algorithmic)?;
    let index = Index::builder().schema(schema).create_in_ram()?;
    index.tokenizers().register("pli_stem", pli_stem);
    
    Ok(index)
}
