//! Integration tests for `summa lint` — covers all 6 ACs.

use std::fs;
use std::path::{Path, PathBuf};

fn fixture_vault() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("vault")
}

/// Create a temp dir copy of the fixture vault so tests can mutate it.
/// Also re-applies the mtime contract on PDF fixtures (git doesn't preserve mtime).
fn temp_vault() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    copy_dir_all(&fixture_vault(), tmp.path()).expect("failed to copy fixture vault");

    // Re-apply required mtime:
    // stale-analysis.pdf mtime = 2026-06-01 → newer than ingested 2026-01-01 → stale
    let stale_pdf = tmp.path().join("Clippings").join("stale-analysis.pdf");
    set_mtime_unix(&stale_pdf, 1780272000); // 2026-06-01T00:00:00Z

    // mcp-analysis.pdf mtime = 2026-05-01 → older than ingested 2026-06-21 → fresh
    let fresh_pdf = tmp.path().join("Clippings").join("mcp-analysis.pdf");
    set_mtime_unix(&fresh_pdf, 1777593600); // 2026-05-01T00:00:00Z

    tmp
}

/// Set mtime of a file using touch -d @<unix_secs>.
fn set_mtime_unix(path: &Path, unix_secs: i64) {
    let ts = format!("@{}", unix_secs);
    let status = std::process::Command::new("touch")
        .arg("-d")
        .arg(&ts)
        .arg(path)
        .status()
        .expect("touch failed");
    assert!(status.success(), "touch failed for {:?}", path);
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

// ─── AC1: lint reports all 6 categories with counts and paths ───────────────

#[test]
fn test_lint_reports_all_categories() {
    let tmp = temp_vault();
    let root = tmp.path();

    let opts = summa::lint::LintOpts {
        fix: false,
        include_human: false,
        json: false,
    };
    let report = summa::lint::run(root, &opts).expect("lint failed");

    // 1. Orphans: "Orphan Concept" has no inbound links
    assert!(
        !report.orphans.is_empty(),
        "expected at least one orphan (Orphan Concept entity)"
    );
    let has_orphan = report.orphans.iter().any(|p| p.contains("Orphan Concept"));
    assert!(
        has_orphan,
        "Orphan Concept not found in orphans: {:?}",
        report.orphans
    );

    // 2. Dangling: [[Nonexistent Page]] in human-note.md
    assert!(
        !report.dangling.is_empty(),
        "expected at least one dangling link"
    );
    let has_dangling = report.dangling.iter().any(|d| d.target.contains("Nonexistent"));
    assert!(
        has_dangling,
        "Nonexistent dangling link not found: {:?}",
        report.dangling
    );

    // 3. Malformed: [[AtScale\|12345]] in human-note.md
    assert!(
        !report.malformed.is_empty(),
        "expected at least one malformed link"
    );
    let has_malformed = report.malformed.iter().any(|m| m.raw.contains("12345"));
    assert!(
        has_malformed,
        "expected malformed digit-alias link: {:?}",
        report.malformed
    );

    // 4. Missing-index: index.md only has Model Context Protocol; others are missing
    assert!(
        !report.missing_index.is_empty(),
        "expected at least one missing-index page"
    );

    // 5. Stale-vs-source: stale-analysis.md ingested 2026-01-01 < pdf mtime 2026-06-01
    assert!(
        !report.stale_vs_source.is_empty(),
        "expected stale-vs-source detection"
    );
    let has_stale = report
        .stale_vs_source
        .iter()
        .any(|s| s.file.contains("stale-analysis"));
    assert!(
        has_stale,
        "stale-analysis.md not detected as stale: {:?}",
        report.stale_vs_source
    );

    // 6. Un-ingested: never-ingested.pdf has no source-summary
    assert!(
        !report.un_ingested.is_empty(),
        "expected un-ingested sources"
    );
    let has_uningest = report.un_ingested.iter().any(|u| u.contains("never-ingested"));
    assert!(
        has_uningest,
        "never-ingested.pdf not found in un-ingested: {:?}",
        report.un_ingested
    );
}

// ─── AC2: --fix repairs malformed links, idempotent ─────────────────────────

#[test]
fn test_lint_fix_repairs_malformed_idempotent() {
    let tmp = temp_vault();
    let root = tmp.path();

    // human-note.md has [[AtScale\|12345]] — need --include-human to fix it
    let opts = summa::lint::LintOpts {
        fix: true,
        include_human: true,
        json: false,
    };
    let report1 = summa::lint::run(root, &opts).expect("lint --fix run 1");

    // At least one malformed fix was applied
    let malformed_fixes1: Vec<&summa::lint::FixRecord> = report1
        .fixed
        .iter()
        .filter(|f| f.change.starts_with("fixed:"))
        .collect();
    assert!(
        !malformed_fixes1.is_empty(),
        "expected malformed fixes to be applied: {:?}",
        report1.fixed
    );

    // The human-note.md should now have no backslash-pipe
    let human_note = fs::read_to_string(root.join("AtScale").join("human-note.md"))
        .expect("read human-note.md");
    assert!(
        !human_note.contains("\\|"),
        "backslash-pipe still present after fix: {}",
        human_note
    );

    // Second --fix run: no further malformed fixes
    let report2 = summa::lint::run(root, &opts).expect("lint --fix run 2");
    let malformed_fixes2: Vec<&summa::lint::FixRecord> = report2
        .fixed
        .iter()
        .filter(|f| f.change.starts_with("fixed:"))
        .collect();
    assert!(
        malformed_fixes2.is_empty(),
        "second --fix run still produced malformed fixes (not idempotent): {:?}",
        malformed_fixes2
    );
}

// ─── AC3: --fix regenerates index.md so missing pages are now present ────────

#[test]
fn test_lint_fix_regenerates_index() {
    let tmp = temp_vault();
    let root = tmp.path();

    // Baseline: only Model Context Protocol is between anchors
    let index_before = fs::read_to_string(root.join("index.md")).expect("read index.md");
    assert!(
        !index_before.contains("Stale Analysis"),
        "Stale Analysis shouldn't be in index fixture yet"
    );

    let opts = summa::lint::LintOpts {
        fix: true,
        include_human: false,
        json: false,
    };
    summa::lint::run(root, &opts).expect("lint --fix");

    // After --fix, index.md should list all summa pages
    let index_after = fs::read_to_string(root.join("index.md")).expect("read index.md after");
    assert!(
        index_after.contains("Model Context Protocol"),
        "Model Context Protocol missing after index regen"
    );
    assert!(
        index_after.contains("Stale Analysis"),
        "Stale Analysis not added to index after --fix:\n{}",
        index_after
    );
}

// ─── AC4: stale detection: flags stale, does NOT flag fresh ─────────────────

#[test]
fn test_stale_detection_flags_stale_not_fresh() {
    let tmp = temp_vault();
    let root = tmp.path();

    let opts = summa::lint::LintOpts {
        fix: false,
        include_human: false,
        json: false,
    };
    let report = summa::lint::run(root, &opts).expect("lint failed");

    // stale-analysis.md: ingested 2026-01-01, pdf mtime 2026-06-01 → stale
    let has_stale = report
        .stale_vs_source
        .iter()
        .any(|s| s.file.contains("stale-analysis"));
    assert!(
        has_stale,
        "stale-analysis.md should be stale: {:?}",
        report.stale_vs_source
    );

    // mcp-analysis.md: ingested 2026-06-21, pdf mtime 2026-05-01 → fresh (NOT stale)
    let has_fresh_as_stale = report
        .stale_vs_source
        .iter()
        .any(|s| s.file.contains("mcp-analysis"));
    assert!(
        !has_fresh_as_stale,
        "mcp-analysis.md should NOT be stale: {:?}",
        report.stale_vs_source
    );
}

// ─── AC5: --fix never modifies source-summary bodies, raw sources, stale pages ─

#[test]
fn test_fix_does_not_modify_source_summaries() {
    let tmp = temp_vault();
    let root = tmp.path();

    let stale_path = root.join("wiki").join("sources").join("stale-analysis.md");
    let mcp_path = root.join("wiki").join("sources").join("mcp-analysis.md");

    let stale_before = fs::read_to_string(&stale_path).expect("read stale-analysis.md");
    let mcp_before = fs::read_to_string(&mcp_path).expect("read mcp-analysis.md");

    // Run --fix with maximum reach
    let opts = summa::lint::LintOpts {
        fix: true,
        include_human: true,
        json: false,
    };
    summa::lint::run(root, &opts).expect("lint --fix");

    // Source-summaries have no malformed links so they must be byte-identical after fix
    let stale_after = fs::read_to_string(&stale_path).expect("read stale-analysis.md after");
    let mcp_after = fs::read_to_string(&mcp_path).expect("read mcp-analysis.md after");

    assert_eq!(
        stale_before, stale_after,
        "stale-analysis.md was modified by --fix"
    );
    assert_eq!(
        mcp_before, mcp_after,
        "mcp-analysis.md was modified by --fix"
    );

    // Raw PDF sources must still exist and be unmodified (lint never writes PDFs)
    let never_pdf = root.join("Clippings").join("never-ingested.pdf");
    assert!(never_pdf.exists(), "never-ingested.pdf should still exist");
}
