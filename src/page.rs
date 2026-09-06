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

/// Create or update a decision-log entry on an entity page.
/// - Creates wiki/entities/<Title>.md if absent, in the shape hand-built pages use:
///   entity frontmatter, a lede placeholder, `## Decision log` with the dated
///   entry, then `## Mentions`.
/// - If the page exists without a `## Decision log` section, the section is
///   spliced in before `## Mentions` (or appended at the end of the body if
///   there is no Mentions section); every other byte of the page is preserved.
/// - If the page already has a `## Decision log`, the entry is appended at the
///   end of that section, same append-at-section-end behavior as mentions.
/// - Dedup: an entry byte-identical (after the date prefix) to one already in
///   the log is not appended twice.
/// - `mention`, if given, is delegated to the same mention-append machinery
///   `entity()` uses, so one call can file both.
///
/// Returns `true` if a new decision-log line was appended, `false` if no
/// `entry` was given or the entry was a duplicate (skipped).
pub fn decision(
    root: &Path,
    title: &str,
    entry: Option<&str>,
    mention: Option<&str>,
    date: Option<&str>,
) -> Result<bool> {
    let entities_dir = root.join("wiki").join("entities");
    std::fs::create_dir_all(&entities_dir)?;

    let file_path = entities_dir.join(format!("{}.md", title));
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let date_str = date
        .map(|d| d.to_string())
        .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());

    let mut appended = false;

    if let Some(entry_text) = entry {
        let normalized_entry = normalize_entry(entry_text);

        if !file_path.exists() {
            let content = format!(
                "---\nsumma: entity\ntitle: {}\ncreated: {}\nupdated: {}\n---\n\n## Decision log\n\n- {}: {}\n\n## Mentions\n\n",
                title, now, now, date_str, normalized_entry
            );
            std::fs::write(&file_path, content)
                .with_context(|| format!("failed to write {}", file_path.display()))?;
            appended = true;
        } else {
            let existing = std::fs::read_to_string(&file_path)
                .with_context(|| format!("failed to read {}", file_path.display()))?;

            if decision_log_contains(&existing, &normalized_entry) {
                // Duplicate: skip silently, caller reports it.
            } else {
                let updated = append_decision_entry(&existing, &date_str, &normalized_entry, &now);
                std::fs::write(&file_path, updated)
                    .with_context(|| format!("failed to write {}", file_path.display()))?;
                appended = true;
            }
        }
    }

    if let Some(m) = mention {
        entity(root, title, None, Some(m))?;
    }

    Ok(appended)
}

/// Print the Decision log entries for a page, newest first (each without the
/// leading `- `). Returns an empty vec if the page or its log doesn't exist.
pub fn decision_log(root: &Path, title: &str) -> Result<Vec<String>> {
    let file_path = root
        .join("wiki")
        .join("entities")
        .join(format!("{}.md", title));

    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&file_path)
        .with_context(|| format!("failed to read {}", file_path.display()))?;

    let mut entries = decision_log_bullets(&content);
    entries.reverse();
    Ok(entries)
}

/// Fold an entry's newlines to spaces so the log stays a list of one-liners.
fn normalize_entry(entry: &str) -> String {
    entry
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract the raw bullet lines (without leading `- `) under `## Decision log`,
/// in file order (oldest first).
fn decision_log_bullets(content: &str) -> Vec<String> {
    let Some(pos) = content.find("## Decision log") else {
        return Vec::new();
    };
    let after = &content[pos..];
    let section_start = pos + after.find('\n').map(|p| p + 1).unwrap_or(after.len());
    let rest = &content[section_start..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    let section_body = &rest[..end];

    section_body
        .lines()
        .filter_map(|l| l.trim().strip_prefix("- ").map(|s| s.to_string()))
        .collect()
}

/// Strip a leading `YYYY-MM-DD: ` date prefix from a bullet, if present.
fn strip_date_prefix(bullet: &str) -> &str {
    let bytes = bullet.as_bytes();
    let looks_like_date = bytes.len() >= 12
        && bytes[..10].iter().enumerate().all(|(i, &b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
        && &bullet[10..12] == ": ";
    if looks_like_date {
        &bullet[12..]
    } else {
        bullet
    }
}

/// Does the Decision log already contain this entry (compared after its date
/// prefix is stripped)?
fn decision_log_contains(content: &str, entry: &str) -> bool {
    decision_log_bullets(content)
        .iter()
        .any(|b| strip_date_prefix(b) == entry)
}

/// Append a dated bullet to `## Decision log`, refreshing `updated:`.
/// - If the section exists, append at its end (same section-end semantics as
///   `append_mention`), reformatting nothing else in it.
/// - If it doesn't exist but `## Mentions` does, splice the section in
///   immediately before Mentions.
/// - If neither exists, append the section at the end of the body.
fn append_decision_entry(content: &str, date: &str, entry: &str, now: &str) -> String {
    let content = update_frontmatter_field(content, "updated", now);
    let bullet = format!("- {}: {}", date, entry);

    if content.contains("## Decision log") {
        append_bullet_to_section(&content, "## Decision log", &bullet)
    } else if let Some(mentions_pos) = content.find("## Mentions") {
        let before = content[..mentions_pos].trim_end_matches('\n');
        let after = &content[mentions_pos..];
        format!("{}\n\n## Decision log\n\n{}\n\n{}", before, bullet, after)
    } else {
        let trimmed = content.trim_end_matches('\n');
        format!("{}\n\n## Decision log\n\n{}\n", trimmed, bullet)
    }
}

/// Append a bullet at the end of a named section (before the next `## `
/// heading, or end of file), same placement rule `append_mention` uses.
fn append_bullet_to_section(content: &str, section_header: &str, bullet: &str) -> String {
    let pos = content
        .find(section_header)
        .expect("section_header must be present in content");
    let after = &content[pos..];
    let section_start = pos + after.find('\n').map(|p| p + 1).unwrap_or(after.len());

    let rest = &content[section_start..];
    let next_section = rest.find("\n## ").map(|p| section_start + p + 1);

    match next_section {
        Some(ns) => {
            let before = &content[..ns];
            let after = &content[ns..];
            let before = before.trim_end_matches('\n');
            format!("{}\n{}\n\n{}", before, bullet, after)
        }
        None => {
            let trimmed = content.trim_end_matches('\n');
            format!("{}\n{}\n", trimmed, bullet)
        }
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
