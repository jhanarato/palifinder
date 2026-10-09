use crate::index::PaliIndex;
use crate::texts::PaliText;
use anyhow::Result;
use tantivy::schema::Field;
use tantivy::{IndexWriter, TantivyDocument};

pub struct PaliWriter {
    writer: IndexWriter,
    uid: Field,
    contents: Field,
}

impl PaliWriter {
    pub fn new(index: &PaliIndex) -> Result<Self> {
        Ok(Self {
            writer: index.as_ref().writer(50_000_000)?,
            uid: index.uid_field()?,
            contents: index.contents_field()?,
        })
    }

    pub fn add_texts(&mut self, texts: impl Iterator<Item = Result<PaliText>>) -> Result<()> {
        for text in texts {
            match text {
                Ok(text) => self.add_text(&text)?,
                Err(e) => println!("Error: {e:#?}"),
            }
        }
        Ok(())
    }

    fn add_text(&mut self, text: &PaliText) -> Result<()> {
        let doc = self.create_document(text);
        self.writer.add_document(doc)?;
        self.writer.commit()?;
        Ok(())
    }

    fn create_document(&mut self, text: & PaliText) -> TantivyDocument {
        let mut document = TantivyDocument::default();
        document.add_text(self.uid, &text.uid);
        for segment in text.segments.clone() {
            document.add_text(self.contents, segment.text);
        }
        document
    }
}