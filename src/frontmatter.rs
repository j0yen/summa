use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum PageType {
    Entity,
    SourceSummary,
    Answer,
    Unknown(String),
}

impl PageType {
    #[allow(clippy::should_implement_trait)] // named to match the `summa:` value, not std::str::FromStr — infallible, not the FromStr trait
    pub fn from_str(s: &str) -> Self {
        match s {
            "entity" => PageType::Entity,
            "source-summary" => PageType::SourceSummary,
            "answer" => PageType::Answer,
            other => PageType::Unknown(other.to_string()),
        }
    }
}

/// Minimal frontmatter structure for summa ownership detection
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct SummaFrontmatter {
    pub summa: Option<String>,
    pub title: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub source: Option<String>,
    pub ingested: Option<String>,
    pub entities: Option<Vec<String>>,
    pub question: Option<String>,
    pub asked: Option<String>,
    pub cites: Option<Vec<String>>,
}

/// Split a markdown file into (frontmatter_str, body_str).
/// Returns (None, full_content) if no frontmatter found.
pub fn split_frontmatter(content: &str) -> (Option<&str>, &str) {
    if !content.starts_with("---") {
        return (None, content);
    }
    // Find closing ---
    let after_open = &content[3..];
    // Skip optional newline
    let after_open = after_open.trim_start_matches('\n');
    if let Some(close_pos) = after_open.find("\n---") {
        let fm = &after_open[..close_pos];
        let body = &after_open[close_pos + 4..]; // skip \n---
        let body = body.trim_start_matches('\n');
        (Some(fm), body)
    } else {
        (None, content)
    }
}

/// Check if a file is summa-owned (has a `summa:` frontmatter key)
pub fn is_summa_owned(content: &str) -> bool {
    let (fm, _) = split_frontmatter(content);
    if let Some(fm_str) = fm {
        if let Ok(fm_parsed) = serde_yaml::from_str::<serde_yaml::Value>(fm_str) {
            return fm_parsed.get("summa").is_some();
        }
    }
    false
}

/// Parse summa frontmatter from content
pub fn parse_frontmatter(content: &str) -> Result<Option<SummaFrontmatter>> {
    let (fm, _) = split_frontmatter(content);
    if let Some(fm_str) = fm {
        let fm: SummaFrontmatter = serde_yaml::from_str(fm_str)?;
        Ok(Some(fm))
    } else {
        Ok(None)
    }
}

/// Get page type from content
pub fn page_type(content: &str) -> Option<PageType> {
    let (fm, _) = split_frontmatter(content);
    let fm_str = fm?;
    let fm: SummaFrontmatter = serde_yaml::from_str(fm_str).ok()?;
    let summa_val = fm.summa?;
    Some(PageType::from_str(&summa_val))
}

/// Read a file and return is_summa_owned
pub fn file_is_summa_owned(path: &Path) -> bool {
    if let Ok(content) = std::fs::read_to_string(path) {
        is_summa_owned(&content)
    } else {
        false
    }
}
