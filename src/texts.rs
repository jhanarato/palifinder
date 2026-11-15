use pali_analysis::tokenizer::nfc;
use anyhow::{Context, Error, Result};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
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

    /// Fail on any directory that cannot be walked, which `files` would skip.
    #[allow(clippy::missing_errors_doc)]
    pub fn check_readable(&self) -> Result<()> {
        for entry in WalkDir::new(&self.location) {
            entry.with_context(|| format!("reading {}", self.location.display()))?;
        }
        Ok(())
    }

    pub fn texts(&self) -> impl Iterator<Item = Result<PaliText>> {
        self.files().map(|file| PaliText::new(&file))
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

    /// Every segment, failing on any text that cannot be read: a stem table built from
    /// part of the texts must not replace a complete one.
    #[allow(clippy::missing_errors_doc)]
    pub fn all_segments(&self) -> Result<Vec<Segment>> {
        self.check_readable()?;
        let mut segments = Vec::new();
        for text in self.texts() {
            segments.extend(text?.segments);
        }
        Ok(segments)
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub struct Segment {
    pub uid: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub struct TextUid(String);

impl TryFrom<&Path> for TextUid {
    type Error = Error;

    fn try_from(path: &Path) -> Result<TextUid, Error> {
        let stem = path
            .file_stem()
            .context("Bad file stem")?
            .to_str()
            .context("Bad string")?
            .to_string();

        TextUid::try_from(stem.as_str())
    }
}

impl TryFrom<&str> for TextUid {
    type Error = Error;

    fn try_from(value: &str) -> Result<TextUid, Error> {
        let uid = value
            .split('_')
            .next()
            .context("Failed to extract text UID from filename.")?;
        Ok(TextUid(uid.to_string()))
    }
}

impl AsRef<str> for TextUid {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for TextUid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub struct PaliText {
    pub uid: TextUid,
    pub segments: Vec<Segment>,
}

impl PaliText {
    #[allow(clippy::missing_errors_doc)]
    pub fn new(path: &Path) -> Result<Self> {
        let uid = TextUid::try_from(path)?;

        let json = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let entries: BTreeMap<String, String> =
            serde_json::from_str(json.as_str()).with_context(|| format!("parsing {}", path.display()))?;
        let segments: Vec<Segment> = entries
            .iter()
            .map(|(k, v)| Segment {
                uid: k.clone(),
                text: nfc(v),
            })
            .collect();
        Ok(Self { uid, segments })
    }
}

#[cfg(test)]
impl PaliText {
    #[must_use]
    pub fn for_test(uid: &str, text: &str) -> Self {
        Self {
            uid: TextUid(uid.to_string()),
            segments: vec![Segment {
                uid: format!("{uid}:1"),
                text: text.to_string(),
            }],
        }
    }
}

impl IntoIterator for PaliText {
    type Item = Segment;
    type IntoIter = std::vec::IntoIter<Segment>;

    fn into_iter(self) -> Self::IntoIter {
        self.segments.into_iter()
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
        let file = dir.child("mn1_root-pli-ms.json");
        std::fs::write(&file, TEXT_JSON).unwrap();
        PaliText::new(file.as_path()).unwrap()
    }

    #[test]
    fn test_a_broken_text_fails_the_whole_read() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.child("mn1_root-pli-ms.json"), TEXT_JSON).unwrap();
        std::fs::write(dir.child("mn2_root-pli-ms.json"), "{\"mn2:1").unwrap();
        let error = PaliFiles::new(dir.path().to_path_buf()).all_segments().unwrap_err();
        assert!(format!("{error:#}").contains("mn2_root-pli-ms.json"), "{error:#}");
    }

    #[test]
    fn test_text_is_composed_when_read() {
        let dir = TempDir::new().unwrap();
        let file = dir.child("mn1_root-pli-ms.json");
        std::fs::write(&file, "{\"mn1:1\": \"nibba\u{304}nassa\"}").unwrap();
        assert_eq!(PaliText::new(&file).unwrap().segments[0].text, "nibbānassa");
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
        assert_eq!(pali_text_from_file().uid, TextUid(String::from("mn1")));
    }

    #[test]
    fn test_uid_created_from_path() {
        let path = Path::new("root/pli/ms/sutta/mn/mn1_root-pli-ms.json");
        let uid = TextUid::try_from(path).unwrap();
        assert_eq!(uid.as_ref(), "mn1");
    }
}
