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
            // One thread keeps document order, and so the order of tied results, the
            // same from build to build.
            writer: index.as_ref().writer_with_num_threads(1, 50_000_000)?,
            uid: index.uid_field()?,
            contents: index.contents_field()?,
        })
    }

    /// Add the texts and commit, returning how many there were.
    pub fn add_texts(&mut self, texts: impl Iterator<Item = Result<PaliText>>) -> Result<usize> {
        let mut count = 0;
        for text in texts {
            let text = text?;
            let doc = self.create_document(&text);
            self.writer.add_document(doc)?;
            count += 1;
        }
        self.writer.commit()?;
        Ok(count)
    }

    /// Wait for background merges, which would otherwise be abandoned half done.
    pub fn finish(self) -> Result<()> {
        self.writer.wait_merging_threads()?;
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