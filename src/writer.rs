use crate::texts::{PaliFiles, PaliText};
use anyhow::Result;
use tantivy::schema::Field;
use tantivy::{Index, IndexWriter, TantivyDocument};

pub struct PaliWriter {
    writer: IndexWriter,
    uid: Field,
    contents: Field,
}

impl PaliWriter {
    pub fn new(index: &Index) -> Result<Self> {
        Ok(Self {
            writer: index.writer(50_000_000)?,
            uid: index.schema().get_field("uid")?,
            contents: index.schema().get_field("contents")?,
        })
    }

    pub fn index_files(&mut self, files: &PaliFiles) -> Result<()> {
        for text in files.texts() {
            match text {
                Ok(text) => self.add_text(&text)?,
                Err(e) => println!("Error: {e:#?}"),
            }
        }
        Ok(())
    }

    fn add_text(&mut self, text: &PaliText) -> Result<()> {
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
