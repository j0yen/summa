//! AC8 (P0): --format json parses as a JSON array of {title, path, summary, score}.

use std::path::PathBuf;
use std::process::Command;

fn fixture_vault() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("relevant")
        .join("vault")
}

#[test]
fn ac8_json_output_shape() {
    let root = fixture_vault();
    let out = Command::new(env!("CARGO_BIN_EXE_summa"))
        .env("SUMMA_VAULT", &root)
        .args(["relevant", "how does build start on redbaron", "--format", "json"])
        .output()
        .expect("invocation failed");
    assert!(out.status.success());

    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    let arr = parsed.as_array().expect("top-level array");
    assert!(!arr.is_empty(), "expected at least one ranked page");
    for item in arr {
        assert!(item.get("title").is_some(), "missing title: {item}");
        assert!(item.get("path").is_some(), "missing path: {item}");
        assert!(item.get("summary").is_some(), "missing summary: {item}");
        assert!(item.get("score").is_some(), "missing score: {item}");
    }
}
