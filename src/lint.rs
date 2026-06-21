//! summa lint — vault health checks with optional mechanical repair.
//!
//! Checks:
//!  1. orphan        — summa-owned pages with 0 inbound links
//!  2. dangling      — [[X]] wikilinks with no matching X.md
//!  3. malformed     — links using \| or all-digit alias (--fix repairs these)
//!  4. missing-index — summa pages absent from index.md anchored section
//!  5. stale-vs-src  — source-summary ingested: older than source file mtime
//!  6. un-ingested   — raw PDFs / Clippings/*.md with no source-summary pointing at them

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::SystemTime;

use crate::{frontmatter, index, links, log, vault};

// ─── output types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LintReport {
    pub orphans: Vec<String>,
    pub dangling: Vec<DanglingItem>,
    pub malformed: Vec<MalformedItem>,
    pub missing_index: Vec<String>,
    pub stale_vs_source: Vec<StaleItem>,
    pub un_ingested: Vec<String>,
    pub fixed: Vec<FixRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DanglingItem {
    pub target: String,
    #[serde(rename = "in")]
    pub in_files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MalformedItem {
    pub file: String,
    pub raw: String,
    pub fix: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StaleItem {
    pub file: String,
    pub ingested: String,
    pub source: String,
    pub source_mtime: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FixRecord {
    pub file: String,
    pub change: String,
}

// ─── options ────────────────────────────────────────────────────────────────

pub struct LintOpts {
    pub fix: bool,
    pub include_human: bool,
    pub json: bool,
}

// ─── public entry point ─────────────────────────────────────────────────────

pub fn run(root: &Path, opts: &LintOpts) -> Result<LintReport> {
    let mut report = LintReport::default();

    // Build the links graph (reuse links module)
    let links_report = links::run(root)?;

    // 1. Orphans
    report.orphans = links_report.orphans.clone();

    // 2. Dangling
    report.dangling = links_report
        .dangling
        .iter()
        .map(|d| DanglingItem {
            target: d.target.clone(),
            in_files: d.in_files.clone(),
        })
        .collect();

    // 3. Malformed (with optional --fix)
    report.malformed = links_report
        .malformed
        .iter()
        .map(|m| MalformedItem {
            file: m.file.clone(),
            raw: m.raw.clone(),
            fix: m.fix.clone(),
        })
        .collect();

    if opts.fix {
        let fixes = apply_malformed_fixes(root, &report.malformed, opts.include_human)?;
        report.fixed.extend(fixes);
    }

    // 4. Missing-index
    report.missing_index = check_missing_index(root)?;
    if opts.fix && !report.missing_index.is_empty() {
        index::run(root).context("failed to regenerate index.md")?;
        for page in &report.missing_index {
            report.fixed.push(FixRecord {
                file: "index.md".to_string(),
                change: format!("added missing page: {}", page),
            });
        }
        // Re-check after fix (should now be empty)
        report.missing_index = check_missing_index(root)?;
    }

    // 5. Stale-vs-source
    report.stale_vs_source = check_stale_vs_source(root)?;

    // 6. Un-ingested
    report.un_ingested = check_un_ingested(root)?;

    // Append lint line to log.md
    let fixed_malformed = report
        .fixed
        .iter()
        .filter(|f| f.change.starts_with("fixed:"))
        .count();
    let fixed_index = report
        .fixed
        .iter()
        .filter(|f| f.change.starts_with("added missing"))
        .count();
    let note = format!(
        "fixed {} malformed links, {} missing-index pages; orphans={} dangling={} stale={} un-ingested={}",
        fixed_malformed,
        fixed_index,
        report.orphans.len(),
        report.dangling.len(),
        report.stale_vs_source.len(),
        report.un_ingested.len(),
    );
    // Best-effort; don't fail the whole run if log write fails
    let _ = log::run(root, "lint", "-", &note);

    Ok(report)
}

// ─── check: missing-index ───────────────────────────────────────────────────

fn check_missing_index(root: &Path) -> Result<Vec<String>> {
    let index_path = root.join("index.md");
    let index_content = if index_path.exists() {
        std::fs::read_to_string(&index_path).context("read index.md")?
    } else {
        String::new()
    };

    // Extract content between anchors
    let anchored = extract_between_anchors(&index_content);

    let md_files = vault::enumerate_md(root);
    let excluded = ["index.md", "log.md", "CLAUDE.md"];
    let mut missing = Vec::new();

    for path in &md_files {
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if excluded.contains(&file_name) {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if !frontmatter::is_summa_owned(&content) {
            continue;
        }

        // Get the title for index lookup
        let title = get_page_title(&content, path);
        // Check if title appears in the anchored section
        if !anchored.contains(&title) {
            let rel = path.strip_prefix(root).unwrap_or(path);
            missing.push(rel.to_string_lossy().into_owned());
        }
    }

    missing.sort();
    Ok(missing)
}

fn extract_between_anchors(content: &str) -> String {
    const START: &str = "<!-- summa:index-start -->";
    const END: &str = "<!-- summa:index-end -->";
    if let (Some(s), Some(e)) = (content.find(START), content.find(END)) {
        content[s + START.len()..e].to_string()
    } else {
        String::new()
    }
}

fn get_page_title(content: &str, path: &Path) -> String {
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

// ─── check: stale-vs-source ─────────────────────────────────────────────────

fn check_stale_vs_source(root: &Path) -> Result<Vec<StaleItem>> {
    let md_files = vault::enumerate_md(root);
    let mut stale = Vec::new();

    for path in &md_files {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let fm = match frontmatter::parse_frontmatter(&content)? {
            Some(fm) => fm,
            None => continue,
        };

        // Only source-summary pages
        if fm.summa.as_deref() != Some("source-summary") {
            continue;
        }

        let ingested_str = match &fm.ingested {
            Some(s) => s.clone(),
            None => continue,
        };

        let source_rel = match &fm.source {
            Some(s) => s.clone(),
            None => continue,
        };

        let source_path = root.join(&source_rel);
        if !source_path.exists() {
            // Source file absent — can't check
            continue;
        }

        // Get source mtime
        let source_mtime = match source_path.metadata().and_then(|m| m.modified()) {
            Ok(mt) => mt,
            Err(_) => continue,
        };

        // Parse ingested timestamp
        let ingested_time = match parse_iso8601(&ingested_str) {
            Some(t) => t,
            None => continue,
        };

        if ingested_time < source_mtime {
            let rel = path.strip_prefix(root).unwrap_or(path);
            let source_mtime_str = format_system_time(source_mtime);
            stale.push(StaleItem {
                file: rel.to_string_lossy().into_owned(),
                ingested: ingested_str,
                source: source_rel,
                source_mtime: source_mtime_str,
            });
        }
    }

    stale.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(stale)
}

fn parse_iso8601(s: &str) -> Option<SystemTime> {
    // Parse ISO-8601 UTC string like "2026-01-01T00:00:00Z"
    use chrono::DateTime;
    let dt = DateTime::parse_from_rfc3339(s).ok()?;
    let secs = dt.timestamp();
    if secs < 0 {
        return None;
    }
    SystemTime::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(secs as u64))
}

fn format_system_time(t: SystemTime) -> String {
    use chrono::{DateTime, Utc};
    let dt: DateTime<Utc> = t.into();
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

// ─── check: un-ingested ─────────────────────────────────────────────────────

fn check_un_ingested(root: &Path) -> Result<Vec<String>> {
    let all_files = vault::enumerate_all(root);
    let md_files = vault::enumerate_md(root);

    // Collect all `source:` fields from source-summary pages
    let mut sourced: HashSet<String> = HashSet::new();
    for path in &md_files {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Ok(Some(fm)) = frontmatter::parse_frontmatter(&content) {
            if fm.summa.as_deref() == Some("source-summary") {
                if let Some(src) = fm.source {
                    // Normalize: strip leading "./" if any
                    sourced.insert(src.trim_start_matches("./").to_string());
                }
            }
        }
    }

    let mut un_ingested = Vec::new();

    for path in &all_files {
        let rel = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel.to_string_lossy().into_owned();

        // Check raw PDFs anywhere in vault
        if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            if !sourced.contains(&rel_str) {
                un_ingested.push(rel_str);
            }
            continue;
        }

        // Check Clippings/*.md without summa: frontmatter
        if rel_str.starts_with("Clippings/") && path.extension().and_then(|s| s.to_str()) == Some("md") {
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if !frontmatter::is_summa_owned(&content) && !sourced.contains(&rel_str) {
                un_ingested.push(rel_str);
            }
        }
    }

    un_ingested.sort();
    Ok(un_ingested)
}

// ─── fix: malformed links ───────────────────────────────────────────────────

fn apply_malformed_fixes(
    root: &Path,
    malformed: &[MalformedItem],
    include_human: bool,
) -> Result<Vec<FixRecord>> {
    // Group by file
    let mut by_file: HashMap<String, Vec<(&MalformedItem, )>> = HashMap::new();
    for item in malformed {
        by_file.entry(item.file.clone()).or_default().push((item,));
    }

    let mut fixes = Vec::new();

    for (rel_file, items) in &by_file {
        let abs_path = root.join(rel_file);
        let content = match std::fs::read_to_string(&abs_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Check ownership
        let is_owned = frontmatter::is_summa_owned(&content);
        if !is_owned && !include_human {
            // Skip human files unless --include-human
            continue;
        }

        let mut new_content = content.clone();
        for (item,) in items {
            new_content = new_content.replace(&item.raw, &item.fix);
            fixes.push(FixRecord {
                file: rel_file.clone(),
                change: format!("fixed: {} → {}", item.raw, item.fix),
            });
        }

        if new_content != content {
            std::fs::write(&abs_path, &new_content)
                .with_context(|| format!("failed to write {}", rel_file))?;
        }
    }

    Ok(fixes)
}

// ─── human-readable output ──────────────────────────────────────────────────

pub fn print_human(report: &LintReport) {
    println!("=== summa lint ===");

    println!("\nOrphans ({}):", report.orphans.len());
    for o in &report.orphans {
        println!("  {}", o);
    }

    println!("\nDangling ({}):", report.dangling.len());
    for d in &report.dangling {
        println!("  [[{}]] referenced in:", d.target);
        for f in &d.in_files {
            println!("    {}", f);
        }
    }

    println!("\nMalformed ({}):", report.malformed.len());
    for m in &report.malformed {
        println!("  {} : {} → {}", m.file, m.raw, m.fix);
    }

    println!("\nMissing from index ({}):", report.missing_index.len());
    for p in &report.missing_index {
        println!("  {}", p);
    }

    println!("\nStale-vs-source ({}):", report.stale_vs_source.len());
    for s in &report.stale_vs_source {
        println!(
            "  {} (ingested: {} | source mtime: {})",
            s.file, s.ingested, s.source_mtime
        );
    }

    println!("\nUn-ingested ({}):", report.un_ingested.len());
    for u in &report.un_ingested {
        println!("  {}", u);
    }

    if !report.fixed.is_empty() {
        println!("\nFixed ({}):", report.fixed.len());
        for f in &report.fixed {
            println!("  {}: {}", f.file, f.change);
        }
    }
}
