//! summa relevant — lexical ranking of wiki pages against a query.
//!
//! Deterministic, offline, no embeddings: scores are a weighted sum of
//! title/alias hits (highest weight), body term frequency (log-scaled),
//! and a small recency tiebreak from the page's `updated:` frontmatter
//! field. Ranks only pages under `wiki/` (entities, answers, sources) —
//! that's where summa's own pages live.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{frontmatter, vault};

/// Files larger than this are skipped rather than scanned (rogue-file cap).
const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;
/// Queries shorter than this many characters return no results.
const MIN_QUERY_CHARS: usize = 3;

const TITLE_WEIGHT: f64 = 10.0;
const ALIAS_WEIGHT: f64 = 8.0;
const BODY_WEIGHT: f64 = 2.0;
/// Max recency bonus for a page updated "now"; decays to 0 over RECENCY_HALF_LIFE_DAYS.
const RECENCY_MAX_BONUS: f64 = 3.0;
const RECENCY_HALF_LIFE_DAYS: f64 = 60.0;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RankedPage {
    pub title: String,
    pub path: String,
    pub summary: String,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain: Option<ScoreExplain>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ScoreExplain {
    pub title: f64,
    pub alias: f64,
    pub body: f64,
    pub recency: f64,
}

pub struct RelevantOpts {
    pub limit: usize,
    pub explain: bool,
}

impl Default for RelevantOpts {
    fn default() -> Self {
        RelevantOpts { limit: 5, explain: false }
    }
}

/// Rank pages under `<root>/wiki` against `query`. Never errors on a
/// malformed page — such pages are skipped and the run still succeeds.
/// A query shorter than MIN_QUERY_CHARS (after trimming) yields no results.
pub fn run(root: &Path, query: &str, opts: &RelevantOpts) -> Result<Vec<RankedPage>> {
    let trimmed = query.trim();
    if trimmed.chars().count() < MIN_QUERY_CHARS {
        return Ok(Vec::new());
    }

    let terms = tokenize(trimmed);
    if terms.is_empty() {
        return Ok(Vec::new());
    }

    let wiki_root = root.join("wiki");
    if !wiki_root.exists() {
        return Ok(Vec::new());
    }

    let md_files = vault::enumerate_md(&wiki_root);
    let mut scored: Vec<RankedPage> = Vec::new();

    for path in &md_files {
        let meta = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.len() > MAX_FILE_BYTES {
            continue; // rogue-file cap
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (fm_str, body) = frontmatter::split_frontmatter(&content);
        let fm: Option<frontmatter::SummaFrontmatter> = match fm_str {
            Some(s) => match serde_yaml::from_str(s) {
                Ok(f) => Some(f),
                Err(_) => continue, // malformed frontmatter: skip page entirely
            },
            None => None,
        };

        let rel = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel.to_string_lossy().into_owned();

        let title = fm
            .as_ref()
            .and_then(|f| f.title.clone())
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string()
            });
        let aliases: Vec<String> = fm
            .as_ref()
            .and_then(|f| f.aliases.clone())
            .unwrap_or_default();
        let updated = fm.as_ref().and_then(|f| f.updated.clone());

        let title_lc = title.to_lowercase();
        let mut title_hits = 0usize;
        for t in &terms {
            title_hits += term_count(&title_lc, t);
        }

        let mut alias_hits = 0usize;
        for a in &aliases {
            let a_lc = a.to_lowercase();
            for t in &terms {
                alias_hits += term_count(&a_lc, t);
            }
        }

        let body_lc = body.to_lowercase();
        let mut body_hits = 0usize;
        for t in &terms {
            body_hits += term_count(&body_lc, t);
        }

        if title_hits == 0 && alias_hits == 0 && body_hits == 0 {
            continue; // no match at all
        }

        let title_score = title_hits as f64 * TITLE_WEIGHT;
        let alias_score = alias_hits as f64 * ALIAS_WEIGHT;
        let body_score = (1.0 + body_hits as f64).ln() * BODY_WEIGHT;
        let recency_score = recency_bonus(updated.as_deref());

        let total = title_score + alias_score + body_score + recency_score;

        let summary = first_summary_line(body);

        scored.push(RankedPage {
            title,
            path: rel_str,
            summary,
            score: total,
            explain: if opts.explain {
                Some(ScoreExplain {
                    title: title_score,
                    alias: alias_score,
                    body: body_score,
                    recency: recency_score,
                })
            } else {
                None
            },
        });
    }

    // Highest score first; ties broken by title for determinism.
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.title.cmp(&b.title))
    });
    scored.truncate(opts.limit);

    Ok(scored)
}

fn tokenize(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn term_count(haystack: &str, term: &str) -> usize {
    if term.is_empty() {
        return 0;
    }
    haystack.matches(term).count()
}

/// First non-empty line of the body, trimmed and length-capped.
fn first_summary_line(body: &str) -> String {
    for line in body.lines() {
        let t = line.trim();
        if !t.is_empty() {
            let t = t.trim_start_matches('#').trim();
            return if t.chars().count() > 200 {
                t.chars().take(200).collect()
            } else {
                t.to_string()
            };
        }
    }
    String::new()
}

/// Small recency tiebreak: exponential decay from RECENCY_MAX_BONUS at
/// age 0 to ~0 by a few half-lives out. Pages with no/unparseable
/// `updated:` get no bonus (never penalized below the match score).
fn recency_bonus(updated: Option<&str>) -> f64 {
    let updated = match updated {
        Some(u) => u,
        None => return 0.0,
    };
    let dt = match chrono::DateTime::parse_from_rfc3339(updated) {
        Ok(dt) => dt,
        Err(_) => return 0.0,
    };
    let now = chrono::Utc::now();
    let age_days = (now.signed_duration_since(dt).num_seconds() as f64 / 86400.0).max(0.0);
    RECENCY_MAX_BONUS * 0.5f64.powf(age_days / RECENCY_HALF_LIFE_DAYS)
}

// ─── output ─────────────────────────────────────────────────────────────────

pub fn print_human(pages: &[RankedPage]) {
    for p in pages {
        println!("- {} ({})", p.title, p.path);
        if !p.summary.is_empty() {
            println!("  {}", p.summary);
        }
        if let Some(e) = &p.explain {
            println!(
                "  [explain] title={:.2} alias={:.2} body={:.2} recency={:.2}",
                e.title, e.alias, e.body, e.recency
            );
        }
    }
}
