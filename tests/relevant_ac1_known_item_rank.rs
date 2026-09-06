//! AC1 (P0): known-item retrieval ranks Fleet Build Automation at rank 1 for
//! "how does build start on redbaron", showing title, relative path, and a
//! one-line summary.

use std::path::PathBuf;

fn fixture_vault() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("relevant")
        .join("vault")
}

#[test]
fn ac1_known_item_ranks_first() {
    let root = fixture_vault();
    let opts = summa::relevant::RelevantOpts { limit: 5, explain: false };
    let pages = summa::relevant::run(&root, "how does build start on redbaron", &opts)
        .expect("relevant failed");

    assert!(!pages.is_empty(), "expected at least one ranked page");
    let top = &pages[0];
    assert_eq!(top.title, "Fleet Build Automation", "wrong page at rank 1: {:?}", pages);
    assert_eq!(
        top.path,
        "wiki/entities/Fleet Build Automation.md",
        "relative path wrong"
    );
    assert!(!top.summary.is_empty(), "summary should be non-empty");
}
