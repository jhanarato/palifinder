use crate::texts::PaliText;
use anyhow::Result;
use tantivy::schema::Field;
use tantivy::{Index, IndexWriter, TantivyDocument};

#[allow(unused)]
struct PaliIndexWriter {
    writer: IndexWriter,
    uid: Field,
    contents: Field,
}

#[allow(unused)]
impl PaliIndexWriter {
    fn new(index: &Index) -> Result<Self> {
        Ok(Self {
            writer: index.writer(50_000_000)?,
            uid: index.schema().get_field("uid")?,
            contents: index.schema().get_field("contents")?,
        })
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn add_text(&mut self, text: &PaliText) -> Result<()> {
        let mut document = TantivyDocument::default();
        document.add_text(self.uid, &text.uid);
        for segment in text.segments.clone() {
            document.add_text(self.contents, segment.text);
        }

        self.writer.add_document(document)?;
        self.writer.commit()?;
        Ok(())
    }
}
