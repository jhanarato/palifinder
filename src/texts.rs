use anyhow::Result;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

pub fn pali_files(location: &Path) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(location)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| is_pali_file(e.path()))
        .map(DirEntry::into_path)
}

fn is_pali_file(path: &Path) -> bool {
    match path.metadata() {
        Ok(metadata) => {
            if metadata.is_file() {
                 match path.file_stem() {
                     Some(stem) => {
                         match stem.to_str() {
                             Some(stem_str) => {
                                 stem_str.ends_with("root-pli-ms")
                             },
                             None => false
                         }
                     },
                     None => false,
                 }
            } else {
                false
            }
        },
        Err(_) => false
    }
}

/// # Errors
///
/// Returns `anyhow::Error` if JSON cannot be parsed.
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
}
