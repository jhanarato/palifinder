use crate::analyzers::AnalyzerConfig;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions};
use tantivy::tokenizer::TextAnalyzer;
use tantivy::{Document, Index, IndexWriter, ReloadPolicy, TantivyDocument};

const PLI_ALGORITHMIC: &str = "pli_algorithmic";

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
#[allow(clippy::missing_panics_doc)]
pub fn create_index_in_ram_with_document() {
    let schema = schema();
    let index = Index::builder()
        .schema(schema.clone())
        .create_in_ram()
        .unwrap();

    let pli_stem = TextAnalyzer::try_from(AnalyzerConfig::Algorithmic).unwrap();
    index.tokenizers().register(PLI_ALGORITHMIC, pli_stem);

    let mut index_writer: IndexWriter = index.writer(50_000_000).unwrap();

    let uid = schema.get_field("uid").unwrap();
    let contents = schema.get_field("contents").unwrap();

    let mut document = TantivyDocument::default();
    document.add_text(uid, "mn1");
    document.add_text(contents, "Evaṁ me sutaṁ—");
    document.add_text(
        contents,
        "ekaṁ samayaṁ bhagavā ukkaṭṭhāyaṁ viharati subhagavane sālarājamūle. ",
    );

    index_writer.add_document(document);
    index_writer.commit();

    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()
        .unwrap();

    let searcher = reader.searcher();

    let query_parser = QueryParser::for_index(&index, vec![uid, contents]);

    let query = query_parser.parse_query("vihar").unwrap();

    let top_docs = searcher
        .search(&query, &TopDocs::with_limit(10).order_by_score())
        .unwrap();

    for (_score, doc_address) in top_docs {
        let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();
        println!("{}", retrieved_doc.to_json(&schema));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_index_in_ram_with_document() {
        create_index_in_ram_with_document();
    }
}
