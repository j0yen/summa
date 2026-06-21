use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::vault;

#[derive(Debug, Serialize, Deserialize)]
pub struct DanglingLink {
    pub target: String,
    #[serde(rename = "in")]
    pub in_files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MalformedLink {
    pub file: String,
    pub raw: String,
    pub fix: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LinkStats {
    pub pages: usize,
    pub links: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LinksReport {
    pub orphans: Vec<String>,
    pub dangling: Vec<DanglingLink>,
    pub malformed: Vec<MalformedLink>,
    pub stats: LinkStats,
}

/// Parse wikilinks from text content.
/// Returns (valid_targets, malformed_links)
pub fn parse_wikilinks(content: &str) -> (Vec<String>, Vec<(String, String)>) {
    // Matches [[...]] and ![[...]]
    let re = Regex::new(r"!?\[\[([^\]]+)\]\]").unwrap();
    // Regex to detect backslash-pipe (malformed clipper escaping)
    let backslash_pipe_re = Regex::new(r"\\|").unwrap();

    let mut valid = Vec::new();
    let mut malformed = Vec::new();

    for cap in re.captures_iter(content) {
        let inner = cap[1].to_string();
        let raw = cap[0].to_string();

        // Check for \| (malformed clipper escaping)
        if inner.contains("\\|") {
            // Extract target (before \|)
            let target = inner.split("\\|").next().unwrap_or("").trim().to_string();
            // Alias is the part after \|
            let alias = inner.splitn(2, "\\|").nth(1).unwrap_or("").trim().to_string();
            // Fix: strip the alias entirely if all-digit, else keep as proper |
            let fix = if alias.chars().all(|c| c.is_ascii_digit()) && !alias.is_empty() {
                format!("[[{}]]", target)
            } else if alias.is_empty() {
                format!("[[{}]]", target)
            } else {
                format!("[[{}|{}]]", target, alias)
            };
            malformed.push((raw, fix));
            // Still treat target as a link for graph purposes
            // Extract target without section
            let target_clean = target.split('#').next().unwrap_or(&target).trim().to_string();
            if !target_clean.is_empty() {
                valid.push(target_clean);
            }
            continue;
        }

        // Check for | (alias separator — normal)
        let (target_part, alias_part) = if let Some(pipe_pos) = inner.find('|') {
            (&inner[..pipe_pos], Some(&inner[pipe_pos + 1..]))
        } else {
            (inner.as_str(), None)
        };

        // Check if alias is all-digits (malformed clipper ID)
        if let Some(alias) = alias_part {
            if alias.trim().chars().all(|c| c.is_ascii_digit()) && !alias.trim().is_empty() {
                // Malformed: all-digit alias
                let target = target_part.trim().to_string();
                let fix = format!("[[{}]]", target);
                malformed.push((raw.clone(), fix));
                // Still count the target as a link
                let target_clean = target.split('#').next().unwrap_or(&target).trim().to_string();
                if !target_clean.is_empty() {
                    valid.push(target_clean);
                }
                continue;
            }
        }

        // Normal link: extract target (before # for section)
        let target = target_part.split('#').next().unwrap_or(target_part).trim().to_string();
        if !target.is_empty() {
            valid.push(target);
        }
    }

    let _ = backslash_pipe_re; // suppress unused warning
    (valid, malformed)
}

/// Run the full links analysis on the vault
pub fn run(root: &Path) -> Result<LinksReport> {
    let md_files = vault::enumerate_md(root);

    // Build stem map: lowercase(stem) -> [paths]
    let mut stem_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut stem_map_case: HashMap<String, Vec<PathBuf>> = HashMap::new(); // case-sensitive

    for path in &md_files {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            stem_map
                .entry(stem.to_lowercase())
                .or_default()
                .push(path.clone());
            stem_map_case
                .entry(stem.to_string())
                .or_default()
                .push(path.clone());
        }
    }

    // Track inbound link counts per file
    let mut inbound: HashMap<PathBuf, usize> = HashMap::new();
    for path in &md_files {
        inbound.entry(path.clone()).or_insert(0);
    }

    // For dangling: target -> set of files that reference it
    let mut dangling_map: HashMap<String, HashSet<String>> = HashMap::new();
    // For malformed: (file, raw, fix)
    let mut malformed_links: Vec<MalformedLink> = Vec::new();

    let mut total_links = 0usize;

    for path in &md_files {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (targets, malformed) = parse_wikilinks(&content);
        total_links += targets.len() + malformed.len();

        // Record malformed
        let rel_path = path.strip_prefix(root).unwrap_or(path);
        for (raw, fix) in malformed {
            malformed_links.push(MalformedLink {
                file: rel_path.to_string_lossy().into_owned(),
                raw,
                fix,
            });
        }

        // Resolve targets
        for target in &targets {
            // Case-sensitive first
            if let Some(resolved) = stem_map_case.get(target).and_then(|v| v.first()) {
                *inbound.entry(resolved.clone()).or_insert(0) += 1;
            } else if let Some(resolved) = stem_map
                .get(&target.to_lowercase())
                .and_then(|v| v.first())
            {
                *inbound.entry(resolved.clone()).or_insert(0) += 1;
            } else {
                // Dangling
                dangling_map
                    .entry(target.clone())
                    .or_default()
                    .insert(rel_path.to_string_lossy().into_owned());
            }
        }
    }

    // Collect orphans: summa-owned pages (wiki/ subtree) with 0 inbound links
    // Exclude index.md, log.md, CLAUDE.md
    let excluded_names = ["index.md", "log.md", "CLAUDE.md"];
    let mut orphans: Vec<String> = Vec::new();
    for (path, count) in &inbound {
        if *count == 0 {
            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if excluded_names.contains(&file_name) {
                continue;
            }
            // Check if it's under wiki/ and summa-owned
            let rel = path.strip_prefix(root).unwrap_or(path);
            let rel_str = rel.to_string_lossy();
            if rel_str.starts_with("wiki/") || rel_str.starts_with("wiki\\") {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if crate::frontmatter::is_summa_owned(&content) {
                        orphans.push(rel_str.into_owned());
                    }
                }
            }
        }
    }
    orphans.sort();

    // Build dangling list
    let mut dangling: Vec<DanglingLink> = dangling_map
        .into_iter()
        .map(|(target, files)| {
            let mut in_files: Vec<String> = files.into_iter().collect();
            in_files.sort();
            DanglingLink {
                target,
                in_files,
            }
        })
        .collect();
    dangling.sort_by(|a, b| a.target.cmp(&b.target));

    Ok(LinksReport {
        orphans,
        dangling,
        malformed: malformed_links,
        stats: LinkStats {
            pages: md_files.len(),
            links: total_links,
        },
    })
}

pub fn print_human(report: &LinksReport) {
    println!("=== Link Report ===");
    println!("Pages: {}  Links: {}", report.stats.pages, report.stats.links);

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
}
