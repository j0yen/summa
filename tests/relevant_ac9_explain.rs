//! AC9 (P2): --explain shows each listed page's score broken into
//! title/body/recency components.

use std::path::PathBuf;

fn fixture_vault() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("relevant")
        .join("vault")
}

#[test]
fn ac9_explain_breaks_down_score() {
    let root = fixture_vault();
    let opts = summa::relevant::RelevantOpts { limit: 5, explain: true };
    let pages = summa::relevant::run(&root, "how does build start on redbaron", &opts)
        .expect("relevant failed");
    assert!(!pages.is_empty());
    for p in &pages {
        let e = p.explain.as_ref().expect("explain populated when requested");
        // Components should sum to (approximately) the total score.
        let sum = e.title + e.alias + e.body + e.recency;
        assert!((sum - p.score).abs() < 1e-6, "components don't sum to score: {:?}", p);
    }
}
