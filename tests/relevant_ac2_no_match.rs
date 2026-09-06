//! AC2 (P0): a query matching nothing yields empty output, exit 0 (never an error).
//! AC6 (P0, shared shape): a machine without a vault (wiki/ absent) also yields
//! empty output rather than an error.

use std::path::PathBuf;

fn fixture_vault() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("relevant")
        .join("vault")
}

#[test]
fn ac2_no_match_is_empty_ok() {
    let root = fixture_vault();
    let opts = summa::relevant::RelevantOpts { limit: 5, explain: false };
    let pages = summa::relevant::run(&root, "zzznonexistentqueryterm999", &opts)
        .expect("relevant should not error on no match");
    assert!(pages.is_empty(), "expected no matches: {:?}", pages);
}

#[test]
fn ac2_short_query_is_empty_ok() {
    let root = fixture_vault();
    let opts = summa::relevant::RelevantOpts { limit: 5, explain: false };
    // Under the 3-char minimum
    let pages = summa::relevant::run(&root, "ab", &opts).expect("relevant should not error");
    assert!(pages.is_empty(), "queries under 3 chars must yield nothing");
}

#[test]
fn ac6_missing_vault_is_empty_ok() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let opts = summa::relevant::RelevantOpts { limit: 5, explain: false };
    let pages = summa::relevant::run(tmp.path(), "how does build start on redbaron", &opts)
        .expect("relevant should not error when wiki/ is absent");
    assert!(pages.is_empty(), "expected no matches without a vault: {:?}", pages);
}
