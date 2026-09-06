//! AC7 (P1): --stdin returns the same ranking as the argv form for an
//! equivalent query.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn fixture_vault() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("relevant")
        .join("vault")
}

#[test]
fn ac7_stdin_matches_argv() {
    let root = fixture_vault();
    let query = "how does build start on redbaron";

    let argv_out = Command::new(env!("CARGO_BIN_EXE_summa"))
        .env("SUMMA_VAULT", &root)
        .args(["relevant", query, "--format", "json"])
        .output()
        .expect("argv invocation failed");
    assert!(argv_out.status.success());

    let mut child = Command::new(env!("CARGO_BIN_EXE_summa"))
        .env("SUMMA_VAULT", &root)
        .args(["relevant", "--stdin", "--format", "json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("stdin invocation failed to spawn");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(query.as_bytes())
        .unwrap();
    let stdin_out = child.wait_with_output().expect("stdin invocation failed");
    assert!(stdin_out.status.success());

    assert_eq!(
        String::from_utf8_lossy(&argv_out.stdout),
        String::from_utf8_lossy(&stdin_out.stdout),
        "stdin form must match argv form for an equivalent query"
    );
}
