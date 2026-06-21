use anyhow::{Context, Result};
use std::path::Path;

use crate::frontmatter;
use crate::vault;

const INDEX_START: &str = "<!-- summa:index-start -->";
const INDEX_END: &str = "<!-- summa:index-end -->";

pub fn run(root: &Path) -> Result<()> {
    let index_path = root.join("index.md");

    // Read existing content or start fresh
    let existing = if index_path.exists() {
        std::fs::read_to_string(&index_path).context("failed to read index.md")?
    } else {
        String::new()
    };

    // Build new index content between anchors
    let new_content = build_index_section(root)?;

    // Replace content between anchors
    let updated = replace_between_anchors(&existing, &new_content);

    // Only write if changed (idempotency)
    if updated != existing {
        std::fs::write(&index_path, &updated).context("failed to write index.md")?;
    }

    Ok(())
}

fn build_index_section(root: &Path) -> Result<String> {
    let md_files = vault::enumerate_md(root);

    let mut entities: Vec<String> = Vec::new();
    let mut sources: Vec<(String, String)> = Vec::new(); // (title, ingested)
    let mut answers: Vec<String> = Vec::new();
    let mut human_dirs: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();

    for path in &md_files {
        let rel = match path.strip_prefix(root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let rel_str = rel.to_string_lossy();

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Skip managed files
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if matches!(file_name, "index.md" | "log.md" | "CLAUDE.md") {
            continue;
        }

        if frontmatter::is_summa_owned(&content) {
            match frontmatter::page_type(&content) {
                Some(frontmatter::PageType::Entity) => {
                    // Get title from frontmatter or stem
                    let title = get_title(&content, path);
                    entities.push(title);
                }
                Some(frontmatter::PageType::SourceSummary) => {
                    let title = get_title(&content, path);
                    let ingested = get_field(&content, "ingested").unwrap_or_default();
                    let date = ingested.get(..10).unwrap_or("").to_string();
                    sources.push((title, date));
                }
                Some(frontmatter::PageType::Answer) => {
                    let title = get_title(&content, path);
                    answers.push(title);
                }
                _ => {}
            }
        } else {
            // Human note — catalog at top-level folder granularity
            let parts: Vec<&str> = rel_str.split('/').collect();
            if parts.len() > 1 {
                let dir = parts[0].to_string();
                // Skip wiki/ subtree (those are summa-managed)
                if dir != "wiki" {
                    *human_dirs.entry(dir).or_insert(0) += 1;
                }
            }
        }
    }

    entities.sort();
    sources.sort_by(|a, b| a.0.cmp(&b.0));
    answers.sort();

    let mut lines = Vec::new();

    if !entities.is_empty() {
        lines.push("## Entities".to_string());
        for e in &entities {
            lines.push(format!("- [[{}]]", e));
        }
    }

    if !sources.is_empty() {
        lines.push("## Sources".to_string());
        for (title, date) in &sources {
            if date.is_empty() {
                lines.push(format!("- [[{}]]", title));
            } else {
                lines.push(format!("- [[{}]] — {}", title, date));
            }
        }
    }

    if !answers.is_empty() {
        lines.push("## Answers".to_string());
        for a in &answers {
            lines.push(format!("- [[{}]]", a));
        }
    }

    if !human_dirs.is_empty() {
        lines.push("## Human notes (by folder)".to_string());
        for (dir, count) in &human_dirs {
            lines.push(format!("- **{}/** — {} notes", dir, count));
        }
    }

    Ok(lines.join("\n"))
}

fn get_title(content: &str, path: &Path) -> String {
    if let Ok(Some(fm)) = frontmatter::parse_frontmatter(content) {
        if let Some(t) = fm.title {
            return t;
        }
    }
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string()
}

fn get_field(content: &str, field: &str) -> Option<String> {
    let (fm, _) = frontmatter::split_frontmatter(content);
    let fm_str = fm?;
    let val: serde_yaml::Value = serde_yaml::from_str(fm_str).ok()?;
    val.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn replace_between_anchors(existing: &str, new_content: &str) -> String {
    let start_marker = INDEX_START;
    let end_marker = INDEX_END;

    if let (Some(start_pos), Some(end_pos)) = (existing.find(start_marker), existing.find(end_marker)) {
        // Replace content between anchors
        let before = &existing[..start_pos + start_marker.len()];
        let after = &existing[end_pos..];
        format!("{}\n{}\n{}", before, new_content, after)
    } else {
        // No anchors found — create full file
        format!("# Index\n\n{}\n{}\n{}\n{}\n", start_marker, new_content, end_marker, "")
    }
}
