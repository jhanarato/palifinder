use anyhow::{Context, Error, Result};
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

    pub fn files(&self) -> impl Iterator<Item = PathBuf> {
        WalkDir::new(&self.location)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| Self::is_pali_file(e.path()))
            .map(DirEntry::into_path)
    }

    pub fn texts(&self) -> impl Iterator<Item = Result<PaliText>> {
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

    pub fn segments(&self) -> impl Iterator<Item = Segment> {
        self.texts()
            .filter_map(Result::ok)
            .flat_map(|text| text.segments)
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub struct PaliText {
    pub uid: String,
    pub segments: Vec<Segment>,
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub struct Segment {
    pub uid: String,
    pub text: String,
}

impl PaliText {
    #[allow(clippy::missing_errors_doc)]
    pub fn parse(path: &Path, json: &str) -> Result<Self> {
        let uid = path
            .file_stem()
            .context("Bad file stem")?
            .to_str()
            .context("Bad string")?
            .to_string();
        let entries: BTreeMap<String, String> = serde_json::from_str(json)?;
        let segments: Vec<Segment> = entries
            .iter()
            .map(|(k, v)| Segment {
                uid: k.clone(),
                text: v.clone(),
            })
            .collect();
        Ok(Self { uid, segments })
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
        PaliText::parse(file, json.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use temp_dir::TempDir;

    pub const TEXT_JSON: &str = r#"
    {
        "mn1:0.1": "Majjhima Nikāya 1 ",
        "mn1:0.2": "Mūlapariyāyasutta ",
        "mn1:1.1": "Evaṁ me sutaṁ—"
    }
    "#;

    fn expected_segments() -> Vec<Segment> {
        vec![
            Segment {
                uid: String::from("mn1:0.1"),
                text: String::from("Majjhima Nikāya 1 "),
            },
            Segment {
                uid: String::from("mn1:0.2"),
                text: String::from("Mūlapariyāyasutta "),
            },
            Segment {
                uid: String::from("mn1:1.1"),
                text: String::from("Evaṁ me sutaṁ—"),
            },
        ]
    }

    fn pali_text_from_file() -> PaliText {
        let dir = TempDir::new().unwrap();
        let file = dir.child("mn1.json");
        std::fs::write(&file, TEXT_JSON).unwrap();
        PaliText::try_from(&file.as_path().to_path_buf()).unwrap()
    }

    #[test]
    fn test_parse_pali_json() {
        assert_eq!(pali_text_from_file().segments, expected_segments());
    }

    #[test]
    fn test_pali_text_into_segment_iterator() {
        let segments: Vec<Segment> = pali_text_from_file().into_iter().collect();
        assert_eq!(segments, expected_segments());
    }

    #[test]
    fn test_pali_text_has_uid() {
        assert_eq!(pali_text_from_file().uid, String::from("mn1"));
    }
}
