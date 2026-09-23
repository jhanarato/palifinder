use crate::analyzers::AnalyzerConfig;
use anyhow::Result;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions};
use tantivy::tokenizer::TextAnalyzer;
use tantivy::{Document, Index, IndexWriter, ReloadPolicy, TantivyDocument, doc};

const PLI_ALGORITHMIC: &str = "pli_algorithmic";

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
            .set_tokenizer(PLI_ALGORITHMIC)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );

    schema_builder.add_text_field("contents", contents_options);
    schema_builder.build()
}

#[allow(unused)]
fn register_analyzer(index: &Index) -> Result<()> {
    let pli_stem = TextAnalyzer::try_from(AnalyzerConfig::Algorithmic)?;
    index.tokenizers().register(PLI_ALGORITHMIC, pli_stem);
    Ok(())
}

#[allow(unused)]
fn add_document(index: &Index, schema: &Schema) -> Result<()> {
    let mut index_writer: IndexWriter = index.writer(50_000_000)?;

    let uid = schema.get_field("uid")?;
    let contents = schema.get_field("contents")?;

    index_writer.add_document(doc!(
        uid => "mn1",
        contents => "Evaṁ me sutaṁ—",
        contents => "ekaṁ samayaṁ bhagavā ukkaṭṭhāyaṁ viharati subhagavane sālarājamūle. ",
    ))?;

    index_writer.commit()?;
    Ok(())
}

#[allow(unused)]
fn search(index: &Index, schema: &Schema) -> Result<Vec<String>> {
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()?;

    let searcher = reader.searcher();
    let contents = schema.get_field("contents")?;
    let query_parser = QueryParser::for_index(index, vec![contents]);
    let query = query_parser.parse_query("vihar")?;
    let top_docs = searcher.search(&query, &TopDocs::with_limit(10).order_by_score())?;

    let mut json_documents = Vec::new();
    for (_score, doc_address) in top_docs {
        let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
        json_documents.push(retrieved_doc.to_json(schema));
    }
    Ok(json_documents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_index_in_ram_with_document() {
        let schema = schema();
        let index = Index::builder()
            .schema(schema.clone())
            .create_in_ram()
            .unwrap();
        register_analyzer(&index).unwrap();
        add_document(&index, &schema).unwrap();
        let results = search(&index, &schema).unwrap();
        assert_eq!(results, vec![String::from(r#"{"uid":["mn1"]}"#)]);
    }
}
