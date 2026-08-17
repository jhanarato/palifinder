use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

pub fn pali_files(location: &Path) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(location)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| is_pali_root_text_file(e.path()).unwrap())
        .map(DirEntry::into_path)
}

fn is_pali_root_text_file(path: &Path) -> Result<bool> {
    if !path.metadata()?.is_file() {
        return Ok(false);
    }
    let file_stem = path.file_stem().context("File has missing stem")?;
    let stem_str = file_stem.to_str().context("Error obtaining stem string")?;
    Ok(stem_str.ends_with("root-pli-ms"))
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
