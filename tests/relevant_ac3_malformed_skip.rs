//! AC3 (P0): a page with unparseable frontmatter is skipped, and the run
//! still succeeds and still reports the other pages.

use std::path::PathBuf;

fn fixture_vault() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("relevant")
        .join("vault")
}

#[test]
fn ac3_malformed_page_skipped_not_fatal() {
    let root = fixture_vault();
    let opts = summa::relevant::RelevantOpts { limit: 10, explain: false };
    // "bad title" only matches the malformed page's body text; the run must
    // still succeed and must never surface the malformed page.
    let pages = summa::relevant::run(&root, "unparseable frontmatter crash", &opts)
        .expect("relevant must not error on malformed frontmatter");
    assert!(
        pages.iter().all(|p| !p.path.contains("Malformed Page")),
        "malformed page must never be ranked: {:?}",
        pages
    );

    // And a normal query still finds the healthy pages despite the malformed
    // one sitting in the same directory.
    let pages = summa::relevant::run(&root, "how does build start on redbaron", &opts)
        .expect("relevant must not error");
    assert!(!pages.is_empty(), "malformed file must not silence the whole run");
}
