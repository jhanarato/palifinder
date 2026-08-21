use anyhow::Result;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};
use crate::tokenizer::tokenize;

#[derive(Clone, Debug)]
pub struct PaliFiles {
    location: PathBuf,
}

impl PaliFiles {
    #[must_use]
    pub fn new(location: PathBuf) -> Self {
        Self { location }
    }

    pub fn files(&self) -> impl Iterator<Item = PathBuf> {
        WalkDir::new(&self.location)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| Self::is_pali_file(e.path()))
            .map(DirEntry::into_path)
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

    #[allow(clippy::missing_errors_doc)]
    pub fn vocabulary(&self) -> Result<HashSet<String>> {
        let mut vocabulary: HashSet<String> = HashSet::new();
        for file in self.files() {
            let json = std::fs::read_to_string(file)?;
            let text = PaliText::parse(json.as_str())?;
            for segment in text.segments {
                for token in tokenize(segment.text.as_str()) {
                    vocabulary.insert(token);
                }
            }
        }
        Ok(vocabulary)
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

#[cfg(test)]
mod tests {
    use super::*;

    pub const MN1_SEGMENTS: &str = r#"
    {
        "mn1:0.1": "Majjhima Nikāya 1 ",
        "mn1:0.2": "Mūlapariyāyasutta ",
        "mn1:1.1": "Evaṁ me sutaṁ—"
    }
    "#;

    #[test]
    fn test_parse_pali_json() {
        let file = PaliText::parse(MN1_SEGMENTS).unwrap();
        assert_eq!(
            file.segments,
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
        );
    }
}
