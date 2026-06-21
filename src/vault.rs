use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Resolve vault root from $SUMMA_VAULT env var, else ~/Notes
pub fn resolve_root() -> Result<PathBuf> {
    if let Ok(v) = std::env::var("SUMMA_VAULT") {
        let p = PathBuf::from(v);
        return Ok(p);
    }
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join("Notes"))
}

/// Skip directories we never want to traverse
fn should_skip(name: &str) -> bool {
    matches!(name, ".obsidian" | ".git" | ".trash" | ".DS_Store" | "target")
}

/// Enumerate all .md files in the vault, skipping hidden dirs
pub fn enumerate_md(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_str().unwrap_or("");
                !should_skip(name)
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().and_then(|s| s.to_str()) == Some("md")
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Enumerate all files (any type) in the vault, skipping hidden dirs
pub fn enumerate_all(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_str().unwrap_or("");
                !should_skip(name)
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Classify a file into md, pdf, or other
pub enum FileKind {
    Md,
    Pdf,
    Other,
}

pub fn classify_file(path: &Path) -> FileKind {
    match path.extension().and_then(|s| s.to_str()) {
        Some("md") => FileKind::Md,
        Some("pdf") => FileKind::Pdf,
        _ => FileKind::Other,
    }
}
