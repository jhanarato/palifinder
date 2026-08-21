use anyhow::Result;
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
struct PaliText {
    segments: Vec<Segment>,
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
struct Segment {
    uid: String,
    text: String,
}

impl PaliText {
    fn parse(json: &str) -> Result<Self> {
        let entries: BTreeMap<String, String> = serde_json::from_str(json)?;
        let segments: Vec<Segment> = entries
            .iter()
            .map(|(k, v)| Segment { uid: k.clone(), text: v.clone() })
            .collect();
        Ok(Self { segments })
    }
}

#[allow(clippy::missing_errors_doc)]
pub fn segments(content: &str) -> Result<Vec<String>> {
    let entries: BTreeMap<String, String> = serde_json::from_str(content)?;
    let segments: Vec<String> = entries.values().cloned().collect();
    Ok(segments)
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
    fn test_get_segments() -> Result<()> {
        assert_eq!(
            segments(MN1_SEGMENTS)?,
            vec!("Majjhima Nikāya 1 ", "Mūlapariyāyasutta ", "Evaṁ me sutaṁ—")
        );
        Ok(())
    }

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
