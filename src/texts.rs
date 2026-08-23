use anyhow::{Error, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[derive(Clone, Debug)]
pub struct PaliFiles {
    location: PathBuf,
}

impl PaliFiles {
    #[must_use]
    pub fn new(location: PathBuf) -> Self {
        Self { location }
    }

    pub fn files(&self) -> impl Iterator<Item=PathBuf> {
        WalkDir::new(&self.location)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| Self::is_pali_file(e.path()))
            .map(DirEntry::into_path)
    }

    pub fn texts(&self) -> impl Iterator<Item=Result<PaliText>> {
        self.files().map(|file| PaliText::try_from(&file))
    }

    fn is_pali_file(path: &Path) -> bool {
        if let Ok(metadata) = path.metadata()
            && metadata.is_file()
            && let Some(stem) = path.file_stem()
            && let Some(stem) = stem.to_str()
        {
            return stem.ends_with("root-pli-ms");
        }
        false
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub struct PaliText {
    pub segments: Vec<Segment>,
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub struct Segment {
    pub uid: String,
    pub text: String,
}

impl PaliText {
    #[allow(clippy::missing_errors_doc)]
    pub fn parse(json: &str) -> Result<Self> {
        let entries: BTreeMap<String, String> = serde_json::from_str(json)?;
        let segments: Vec<Segment> = entries
            .iter()
            .map(|(k, v)| Segment { uid: k.clone(), text: v.clone() })
            .collect();
        Ok(Self { segments })
    }
}

impl IntoIterator for PaliText {
    type Item = Segment;
    type IntoIter = std::vec::IntoIter<Segment>;

    fn into_iter(self) -> Self::IntoIter {
        self.segments.into_iter()
    }
}

impl TryFrom<&PathBuf> for PaliText {
    type Error = Error;

    fn try_from(file: &PathBuf) -> std::result::Result<Self, Self::Error> {
        let json = std::fs::read_to_string(file)?;
        PaliText::parse(json.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub const TEXT_JSON: &str = r#"
    {
        "mn1:0.1": "Majjhima Nikāya 1 ",
        "mn1:0.2": "Mūlapariyāyasutta ",
        "mn1:1.1": "Evaṁ me sutaṁ—"
    }
    "#;

    fn expected_segments() -> Vec<Segment> {
        vec!(
            Segment {
                uid: String::from("mn1:0.1"),
                text: String::from("Majjhima Nikāya 1 ")
            },
            Segment {
                uid: String::from("mn1:0.2"),
                text: String::from("Mūlapariyāyasutta ")
            },
            Segment {
                uid: String::from("mn1:1.1"),
                text: String::from("Evaṁ me sutaṁ—")
            },
        )
    }

    #[test]
    fn test_parse_pali_json() {
        let text = PaliText::parse(TEXT_JSON).unwrap();
        assert_eq!(
            text.segments,
            expected_segments()
        );
    }

    #[test]
    fn test_pali_text_into_segment_iterator() {
        let text = PaliText::parse(TEXT_JSON).unwrap();
        let segments: Vec<Segment> = text.into_iter().collect();
        assert_eq!(segments, expected_segments());
    }
}
