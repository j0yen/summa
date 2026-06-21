use anyhow::{Context, Result};
use chrono::Utc;
use std::path::Path;

use crate::frontmatter;

/// Create or update an entity page.
/// - Creates wiki/entities/<Title>.md if absent.
/// - Appends mention bullet under ## Mentions if present (dedup on source link).
/// - Bumps `updated:` when modifying.
pub fn entity(root: &Path, title: &str, alias: Option<&str>, mention: Option<&str>) -> Result<()> {
    let entities_dir = root.join("wiki").join("entities");
    std::fs::create_dir_all(&entities_dir)?;

    let file_path = entities_dir.join(format!("{}.md", title));
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    if !file_path.exists() {
        // Create new entity page
        let aliases_yaml = if let Some(a) = alias {
            format!("aliases: [{}]\n", a)
        } else {
            String::new()
        };

        let mut content = format!(
            "---\nsumma: entity\ntitle: {}\n{}created: {}\nupdated: {}\n---\n\n",
            title, aliases_yaml, now, now
        );

        if let Some(m) = mention {
            content.push_str("## Mentions\n\n");
            content.push_str(&format!("- {}\n", m));
        } else {
            content.push_str("## Mentions\n\n");
        }

        std::fs::write(&file_path, content)
            .with_context(|| format!("failed to write {}", file_path.display()))?;
    } else {
        // Update existing entity page
        let existing = std::fs::read_to_string(&file_path)
            .with_context(|| format!("failed to read {}", file_path.display()))?;

        if let Some(m) = mention {
            // Check for dedup: does this mention's source link already appear?
            let source_link = extract_source_link(m);
            if source_link.is_empty() || !existing.contains(&source_link) {
                // Append the mention bullet
                let updated = append_mention(&existing, m, &now);
                std::fs::write(&file_path, updated)
                    .with_context(|| format!("failed to write {}", file_path.display()))?;
            }
            // else: already present, skip (dedup)
        } else if let Some(a) = alias {
            // Just add alias if not present
            let _ = a; // currently no update path for alias-only
        }
    }

    Ok(())
}

/// Append a mention bullet under ## Mentions and bump `updated:`
fn append_mention(content: &str, mention: &str, now: &str) -> String {
    let bullet = format!("- {}", mention);

    // Update the `updated:` frontmatter field
    let content = update_frontmatter_field(content, "updated", now);

    // Find ## Mentions section and append
    if let Some(pos) = content.find("## Mentions") {
        // Find the end of the mentions section (next ## or end of file)
        let after = &content[pos..];
        let section_start = pos + after.find('\n').map(|p| p + 1).unwrap_or(after.len());

        // Find where to insert (at the end of mentions section)
        let rest = &content[section_start..];
        let next_section = rest.find("\n## ").map(|p| section_start + p + 1);

        match next_section {
            Some(ns) => {
                let before = &content[..ns];
                let after = &content[ns..];
                // Trim trailing whitespace from before, add bullet
                let before = before.trim_end_matches('\n');
                format!("{}\n{}\n\n{}", before, bullet, after)
            }
            None => {
                // End of file
                let trimmed = content.trim_end_matches('\n');
                format!("{}\n{}\n", trimmed, bullet)
            }
        }
    } else {
        // No ## Mentions section — append one
        let trimmed = content.trim_end_matches('\n');
        format!("{}\n\n## Mentions\n\n{}\n", trimmed, bullet)
    }
}

/// Extract the source link from a mention string like "[[Source]] — claim"
fn extract_source_link(mention: &str) -> String {
    if let Some(end) = mention.find("]]") {
        mention[..end + 2].to_string()
    } else {
        String::new()
    }
}

/// Update a single frontmatter field value
fn update_frontmatter_field(content: &str, field: &str, value: &str) -> String {
    let (fm, body) = frontmatter::split_frontmatter(content);
    if let Some(fm_str) = fm {
        // Simple line-by-line replacement
        let updated_fm: String = fm_str
            .lines()
            .map(|line| {
                if line.starts_with(&format!("{}:", field)) {
                    format!("{}: {}", field, value)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("---\n{}\n---\n\n{}", updated_fm, body)
    } else {
        content.to_string()
    }
}

/// Write or overwrite a source-summary page.
pub fn summary(
    root: &Path,
    source: &str,
    title: &str,
    tldr_path: &str,
    entities: &[String],
) -> Result<()> {
    let sources_dir = root.join("wiki").join("sources");
    std::fs::create_dir_all(&sources_dir)?;

    let slug = title_to_slug(title);
    let file_path = sources_dir.join(format!("{}.md", slug));
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let tldr = std::fs::read_to_string(tldr_path)
        .with_context(|| format!("failed to read tldr file: {}", tldr_path))?;

    let entities_yaml = if entities.is_empty() {
        "entities: []\n".to_string()
    } else {
        let list: Vec<String> = entities
            .iter()
            .map(|e| format!("\"{}\"", e))
            .collect();
        format!("entities: [{}]\n", list.join(", "))
    };

    let content = format!(
        "---\nsumma: source-summary\nsource: {}\ningested: {}\ntitle: {}\n{}---\n\n{}\n",
        source, now, title, entities_yaml, tldr.trim()
    );

    std::fs::write(&file_path, content)
        .with_context(|| format!("failed to write {}", file_path.display()))?;

    Ok(())
}

/// Write an answer page.
pub fn answer(
    root: &Path,
    question: &str,
    slug: &str,
    body_path: &str,
    cites: &[String],
) -> Result<()> {
    let answers_dir = root.join("wiki").join("answers");
    std::fs::create_dir_all(&answers_dir)?;

    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let file_name = format!("{}-{}.md", date, slug);
    let file_path = answers_dir.join(&file_name);
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let body = std::fs::read_to_string(body_path)
        .with_context(|| format!("failed to read body file: {}", body_path))?;

    let cites_yaml = if cites.is_empty() {
        "cites: []\n".to_string()
    } else {
        let list: Vec<String> = cites
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect();
        format!("cites: [{}]\n", list.join(", "))
    };

    let content = format!(
        "---\nsumma: answer\nquestion: \"{}\"\nasked: {}\n{}---\n\n{}\n",
        question.replace('"', "\\\""),
        now,
        cites_yaml,
        body.trim()
    );

    std::fs::write(&file_path, content)
        .with_context(|| format!("failed to write {}", file_path.display()))?;

    Ok(())
}

/// Convert a title to a slug (lowercase, hyphenated)
fn title_to_slug(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
