use std::path::{Path, PathBuf};
use std::fs;
use std::env;

fn fixture_vault() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("vault")
}

/// Create a temp dir copy of the fixture vault so tests can mutate it
fn temp_vault() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    copy_dir_all(&fixture_vault(), tmp.path()).expect("failed to copy fixture vault");
    tmp
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

// ──────────────────────────────────────────────────────────────────────────────
// AC3: summa index writes index.md between anchors, idempotent
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_index_idempotent() {
    let tmp = temp_vault();
    let root = tmp.path();

    // Run index once
    summa::index::run(root).expect("index run 1 failed");
    let after_first = fs::read_to_string(root.join("index.md")).expect("read index.md");

    // Run index again
    summa::index::run(root).expect("index run 2 failed");
    let after_second = fs::read_to_string(root.join("index.md")).expect("read index.md 2");

    // Should be identical
    assert_eq!(after_first, after_second, "index not idempotent");

    // Should contain anchors
    assert!(after_first.contains("<!-- summa:index-start -->"), "missing start anchor");
    assert!(after_first.contains("<!-- summa:index-end -->"), "missing end anchor");

    // Should preserve the preamble
    assert!(after_first.contains("hand-written preamble"), "preamble lost");
}

#[test]
fn test_index_contains_summa_pages() {
    let tmp = temp_vault();
    let root = tmp.path();

    summa::index::run(root).expect("index failed");
    let content = fs::read_to_string(root.join("index.md")).expect("read index.md");

    // Should list the entity we have
    assert!(content.contains("Model Context Protocol"), "entity not in index");
}

// ──────────────────────────────────────────────────────────────────────────────
// AC4: summa log appends one correctly-formatted line
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_log_appends() {
    let tmp = temp_vault();
    let root = tmp.path();

    let before = fs::read_to_string(root.join("log.md")).expect("read log.md");
    let before_lines: usize = before.lines().count();

    summa::log::run(root, "ingest", "AtScale", "test entry").expect("log failed");

    let after = fs::read_to_string(root.join("log.md")).expect("read log.md after");
    let after_lines: usize = after.lines().count();

    // One more line
    assert_eq!(after_lines, before_lines + 1, "expected exactly one new line");

    // The new line should match the format
    let new_line = after.lines().last().expect("no lines");
    assert!(new_line.contains("ingest"), "kind missing");
    assert!(new_line.contains("AtScale"), "subject missing");
    assert!(new_line.contains("test entry"), "note missing");

    // Check two-space separators
    assert!(new_line.contains("  ingest  "), "two-space separators missing");
}

#[test]
fn test_log_creates_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    // No log.md initially

    summa::log::run(root, "lint", "-", "fixed 0 links").expect("log run failed");

    let content = fs::read_to_string(root.join("log.md")).expect("log.md not created");
    assert!(content.contains("lint"), "log content wrong");
}

// ──────────────────────────────────────────────────────────────────────────────
// AC5: summa links correctly enumerates orphans, dangling, malformed
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_links_malformed() {
    let vault = fixture_vault();
    let report = summa::links::run(&vault).expect("links failed");

    // The human-note.md has [[AtScale\|12345]] which is malformed
    assert!(
        !report.malformed.is_empty(),
        "expected malformed links, got none"
    );
    // Check it's the right one
    let has_backslash = report.malformed.iter().any(|m| m.raw.contains("\\|"));
    assert!(has_backslash, "backslash-pipe malformed link not found");
}

#[test]
fn test_links_dangling() {
    let vault = fixture_vault();
    let report = summa::links::run(&vault).expect("links failed");

    // [[Nonexistent Page]] should be dangling
    let has_nonexistent = report.dangling.iter().any(|d| d.target == "Nonexistent Page");
    assert!(has_nonexistent, "[[Nonexistent Page]] not in dangling: {:?}", report.dangling);
}

#[test]
fn test_links_orphan() {
    let vault = fixture_vault();
    let report = summa::links::run(&vault).expect("links failed");

    // Orphan Concept has no inbound links
    let has_orphan = report.orphans.iter().any(|o| o.contains("Orphan Concept"));
    assert!(has_orphan, "Orphan Concept not in orphans: {:?}", report.orphans);
}

#[test]
fn test_links_json_structure() {
    let vault = fixture_vault();
    let report = summa::links::run(&vault).expect("links failed");

    // JSON serialization should work
    let json = serde_json::to_string(&report).expect("json serialization failed");
    let reparsed: serde_json::Value = serde_json::from_str(&json).expect("json parse failed");

    assert!(reparsed.get("orphans").is_some());
    assert!(reparsed.get("dangling").is_some());
    assert!(reparsed.get("malformed").is_some());
    assert!(reparsed.get("stats").is_some());
    assert!(reparsed["stats"]["pages"].as_u64().unwrap() > 0);
}

// ──────────────────────────────────────────────────────────────────────────────
// AC6: summa page entity creates page and round-trips (append + dedup)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_page_entity_creates() {
    let tmp = temp_vault();
    let root = tmp.path();

    summa::page::entity(root, "Test Entity", None, Some("[[source]] — test claim"))
        .expect("entity create failed");

    let entity_path = root.join("wiki").join("entities").join("Test Entity.md");
    assert!(entity_path.exists(), "entity file not created");

    let content = fs::read_to_string(&entity_path).expect("read entity");
    assert!(content.contains("summa: entity"), "missing summa frontmatter");
    assert!(content.contains("Test Entity"), "title missing");
    assert!(content.contains("test claim"), "mention missing");
    assert!(content.contains("## Mentions"), "mentions section missing");
}

#[test]
fn test_page_entity_append_mention() {
    let tmp = temp_vault();
    let root = tmp.path();

    // Create entity first
    summa::page::entity(root, "Test Entity 2", None, Some("[[src1]] — claim one"))
        .expect("entity create failed");

    // Append another mention
    summa::page::entity(root, "Test Entity 2", None, Some("[[src2]] — claim two"))
        .expect("entity append failed");

    let entity_path = root.join("wiki").join("entities").join("Test Entity 2.md");
    let content = fs::read_to_string(&entity_path).expect("read entity");
    assert!(content.contains("claim one"), "first mention missing");
    assert!(content.contains("claim two"), "second mention missing");
}

#[test]
fn test_page_entity_dedup_mention() {
    let tmp = temp_vault();
    let root = tmp.path();

    // Create entity
    summa::page::entity(root, "Test Entity 3", None, Some("[[src1]] — claim one"))
        .expect("entity create failed");

    // Append same source link again
    summa::page::entity(root, "Test Entity 3", None, Some("[[src1]] — updated claim"))
        .expect("entity dedup failed");

    let entity_path = root.join("wiki").join("entities").join("Test Entity 3.md");
    let content = fs::read_to_string(&entity_path).expect("read entity");

    // Should only have one [[src1]] mention
    let count = content.matches("[[src1]]").count();
    assert_eq!(count, 1, "dedup failed: [[src1]] appears {} times", count);
}

// ──────────────────────────────────────────────────────────────────────────────
// Wikilink parser unit tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_wikilink_parser_normal() {
    let (targets, malformed) = summa::links::parse_wikilinks("See [[Foo]] and [[Bar|Alias]]");
    assert_eq!(targets, vec!["Foo", "Bar"]);
    assert!(malformed.is_empty());
}

#[test]
fn test_wikilink_parser_backslash_pipe() {
    let (_targets, malformed) = summa::links::parse_wikilinks("[[AtScale\\|12345]]");
    assert!(!malformed.is_empty(), "backslash-pipe should be malformed");
    assert_eq!(malformed[0].1, "[[AtScale]]", "fix should strip digit alias");
}

#[test]
fn test_wikilink_parser_all_digit_alias() {
    let (_targets, malformed) = summa::links::parse_wikilinks("[[AtScale|99999]]");
    assert!(!malformed.is_empty(), "all-digit alias should be malformed");
    assert_eq!(malformed[0].1, "[[AtScale]]");
}

#[test]
fn test_wikilink_parser_embed() {
    let (targets, malformed) = summa::links::parse_wikilinks("![[image.png]]");
    assert_eq!(targets, vec!["image.png"]);
    assert!(malformed.is_empty());
}

#[test]
fn test_wikilink_parser_section() {
    let (targets, malformed) = summa::links::parse_wikilinks("[[Page#Section]]");
    assert_eq!(targets, vec!["Page"]);
    assert!(malformed.is_empty());
}

// ──────────────────────────────────────────────────────────────────────────────
// Frontmatter tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_frontmatter_summa_owned() {
    let content = "---\nsumma: entity\ntitle: Foo\n---\n\nbody";
    assert!(summa::frontmatter::is_summa_owned(content));
}

#[test]
fn test_frontmatter_not_owned() {
    let content = "---\ntitle: Foo\n---\n\nbody";
    assert!(!summa::frontmatter::is_summa_owned(content));
}

#[test]
fn test_frontmatter_no_frontmatter() {
    let content = "Just a plain note with no frontmatter.";
    assert!(!summa::frontmatter::is_summa_owned(content));
}

// ──────────────────────────────────────────────────────────────────────────────
// AC2: summa ingest (md passthrough)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_ingest_md() {
    let tmp = temp_vault();
    let root = tmp.path();

    // Use human-note.md as test input
    let src = fixture_vault().join("AtScale").join("human-note.md");
    let stub = summa::ingest::run(root, src.to_str().unwrap(), "Clippings")
        .expect("ingest md failed");

    assert_eq!(stub.kind, "md");
    assert!(!stub.staged_path.is_empty());
    assert!(!stub.extracted_text_path.is_empty());
    assert!(stub.bytes > 0);

    // Verify JSON keys
    let json = serde_json::to_string(&stub).expect("json serialize");
    let val: serde_json::Value = serde_json::from_str(&json).expect("json parse");
    assert!(val.get("source").is_some());
    assert!(val.get("staged_path").is_some());
    assert!(val.get("extracted_text_path").is_some());
}
