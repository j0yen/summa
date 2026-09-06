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
// summa page decision — file a decision entry on an entity page
// ──────────────────────────────────────────────────────────────────────────────

/// The hand-made reference fixture: a copy of the real wiki/entities/Fleet
/// Build Automation.md page, used as ground truth for AC2 (never a fixture
/// the builder generates from its own output).
fn hand_made_decision_fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("decision")
}

fn temp_decision_vault() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    copy_dir_all(&hand_made_decision_fixture_dir(), tmp.path())
        .expect("failed to copy decision fixture vault");
    tmp
}

// AC1: no page for the title -> created page carries entity frontmatter, a
// lede placeholder, a Decision log with the dated entry, and Mentions, in
// that order.
#[test]
fn test_decision_creates_page_in_order() {
    let tmp = temp_vault();
    let root = tmp.path();

    let appended = summa::page::decision(
        root,
        "New Decision Entity",
        Some("adopted the new build gate"),
        None,
        Some("2026-09-06"),
    )
    .expect("decision create failed");
    assert!(appended, "expected a new entry to be appended");

    let path = root
        .join("wiki")
        .join("entities")
        .join("New Decision Entity.md");
    assert!(path.exists(), "page not created");
    let content = fs::read_to_string(&path).expect("read page");

    let fm_pos = content.find("summa: entity").expect("frontmatter missing");
    let log_pos = content.find("## Decision log").expect("Decision log missing");
    let mentions_pos = content.find("## Mentions").expect("Mentions missing");
    assert!(fm_pos < log_pos, "frontmatter should precede Decision log");
    assert!(log_pos < mentions_pos, "Decision log should precede Mentions");
    assert!(
        content.contains("- 2026-09-06: adopted the new build gate"),
        "dated entry missing: {content}"
    );
}

// AC2: appending an entry to a copy of the hand-made Fleet Build Automation
// page must diff from the original by exactly the new log line plus the
// `updated:` field.
#[test]
fn test_decision_preserves_hand_made_page() {
    let tmp = temp_decision_vault();
    let root = tmp.path();
    let path = root
        .join("wiki")
        .join("entities")
        .join("Fleet Build Automation.md");

    let original = fs::read_to_string(&path).expect("read original fixture");

    summa::page::decision(
        root,
        "Fleet Build Automation",
        Some("replaced polling timer with path-unit trigger"),
        None,
        Some("2026-09-06"),
    )
    .expect("decision append failed");

    let after = fs::read_to_string(&path).expect("read after append");

    // Everything before the splice point (frontmatter sans `updated:`, plus
    // the lede) is byte-identical.
    let original_head = original.split("## Mentions").next().unwrap();
    let after_head = after.split("## Decision log").next().unwrap();
    let original_head_no_updated: String = original_head
        .lines()
        .filter(|l| !l.starts_with("updated:"))
        .collect::<Vec<_>>()
        .join("\n");
    let after_head_no_updated: String = after_head
        .lines()
        .filter(|l| !l.starts_with("updated:"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        original_head_no_updated, after_head_no_updated,
        "frontmatter/lede bytes before the splice point were disturbed"
    );

    // Everything from `## Mentions` onward is byte-identical (untouched).
    let original_tail = &original[original.find("## Mentions").unwrap()..];
    let after_tail = &after[after.find("## Mentions").unwrap()..];
    assert_eq!(original_tail, after_tail, "Mentions section was disturbed");

    // Exactly the new dated entry was spliced in before Mentions.
    assert!(
        after.contains("## Decision log\n\n- 2026-09-06: replaced polling timer with path-unit trigger\n\n## Mentions"),
        "Decision log not spliced in before Mentions: {after}"
    );

    // `updated:` is the only frontmatter field that changed.
    assert_ne!(
        original.lines().find(|l| l.starts_with("updated:")),
        after.lines().find(|l| l.starts_with("updated:")),
        "updated field should have changed"
    );
}

// AC3: a page with Mentions but no Decision log gets the section inserted
// before Mentions; all other bytes preserved (covered structurally above,
// this test exercises the from-scratch splice path directly).
#[test]
fn test_decision_inserts_section_before_mentions() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    let entities_dir = root.join("wiki").join("entities");
    fs::create_dir_all(&entities_dir).unwrap();
    let path = entities_dir.join("No Log Yet.md");
    let original = "---\nsumma: entity\ntitle: No Log Yet\ncreated: 2026-01-01T00:00:00Z\nupdated: 2026-01-01T00:00:00Z\n---\n\nSome lede text.\n\n## Mentions\n\n- [[Src]] — a mention\n";
    fs::write(&path, original).unwrap();

    summa::page::decision(root, "No Log Yet", Some("first decision"), None, Some("2026-09-06"))
        .expect("decision failed");

    let after = fs::read_to_string(&path).unwrap();
    assert!(after.contains("Some lede text."), "lede text lost");
    assert!(
        after.contains("[[Src]] — a mention"),
        "existing mention lost"
    );
    assert!(
        after.find("## Decision log").unwrap() < after.find("## Mentions").unwrap(),
        "Decision log not inserted before Mentions"
    );
    assert!(after.contains("- 2026-09-06: first decision"));
}

// AC4: an entry identical to an existing one (after the date prefix) is not
// appended twice, and the command exits 0 (i.e. returns Ok(false)).
#[test]
fn test_decision_dedup() {
    let tmp = temp_vault();
    let root = tmp.path();

    let first = summa::page::decision(
        root,
        "Dedup Entity",
        Some("same entry text"),
        None,
        Some("2026-09-01"),
    )
    .expect("first decision failed");
    assert!(first);

    let second = summa::page::decision(
        root,
        "Dedup Entity",
        Some("same entry text"),
        None,
        Some("2026-09-06"),
    )
    .expect("second decision failed");
    assert!(!second, "duplicate entry should not be appended");

    let path = root.join("wiki").join("entities").join("Dedup Entity.md");
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(
        content.matches("same entry text").count(),
        1,
        "entry appended twice"
    );
}

// AC5: --entry and --mention together land both in one invocation.
#[test]
fn test_decision_entry_and_mention_together() {
    let tmp = temp_vault();
    let root = tmp.path();

    summa::page::decision(
        root,
        "Combo Entity",
        Some("decided the thing"),
        Some("[[Some Source]] — claim"),
        Some("2026-09-06"),
    )
    .expect("decision failed");

    let path = root.join("wiki").join("entities").join("Combo Entity.md");
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("- 2026-09-06: decided the thing"), "entry missing");
    assert!(content.contains("[[Some Source]] — claim"), "mention missing");
}

// AC6: --date overrides today's date.
#[test]
fn test_decision_date_override() {
    let tmp = temp_vault();
    let root = tmp.path();

    summa::page::decision(
        root,
        "Backfilled Entity",
        Some("backfilled decision"),
        None,
        Some("2026-09-01"),
    )
    .expect("decision failed");

    let path = root.join("wiki").join("entities").join("Backfilled Entity.md");
    let content = fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("- 2026-09-01: backfilled decision"),
        "override date not used: {content}"
    );
}

// AC7: the bare form (no flags) prints the existing log newest first.
#[test]
fn test_decision_log_newest_first() {
    let tmp = temp_vault();
    let root = tmp.path();

    summa::page::decision(root, "Timeline Entity", Some("first"), None, Some("2026-09-01"))
        .expect("decision 1 failed");
    summa::page::decision(root, "Timeline Entity", Some("second"), None, Some("2026-09-02"))
        .expect("decision 2 failed");
    summa::page::decision(root, "Timeline Entity", Some("third"), None, Some("2026-09-03"))
        .expect("decision 3 failed");

    let log = summa::page::decision_log(root, "Timeline Entity").expect("decision_log failed");
    assert_eq!(
        log,
        vec![
            "2026-09-03: third".to_string(),
            "2026-09-02: second".to_string(),
            "2026-09-01: first".to_string(),
        ],
        "expected newest-first order, got {:?}",
        log
    );
}

// AC8: entry text containing a newline is folded to one line with spaces.
#[test]
fn test_decision_entry_newline_folded() {
    let tmp = temp_vault();
    let root = tmp.path();

    summa::page::decision(
        root,
        "Multiline Entity",
        Some("first part\nsecond part"),
        None,
        Some("2026-09-06"),
    )
    .expect("decision failed");

    let path = root.join("wiki").join("entities").join("Multiline Entity.md");
    let content = fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("- 2026-09-06: first part second part"),
        "newline not folded to space: {content}"
    );
    // The logged bullet is exactly one line — no embedded newline survived.
    let bullet_line = content
        .lines()
        .find(|l| l.starts_with("- 2026-09-06:"))
        .expect("bullet line missing");
    assert_eq!(
        bullet_line, "- 2026-09-06: first part second part",
        "entry should be folded onto a single line"
    );
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
