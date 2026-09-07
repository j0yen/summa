//! AC4 (P0): the first user prompt of a session injects at most 5 pages and
//! at most 1200 bytes, prefixed `summa: relevant wiki pages`.
//! AC5 (P0): a second prompt in the same session emits nothing.
//! AC6 (P0): a machine without summa on PATH emits nothing and exits 0.
//!
//! These exercise `~/.claude/scripts/summa-user-prompt.sh` directly (the
//! hook lands at a fixed machine path per the PRD's engineering target,
//! same convention as recall's hook family) rather than reimplementing its
//! bash logic in Rust.

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::io::Write;

const HOOK: &str = "/home/jsy/.claude/scripts/summa-user-prompt.sh";

fn fixture_vault() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("relevant")
        .join("vault")
}

fn run_hook(state_dir: &std::path::Path, summa_bin: &str, payload: &str) -> Output {
    let mut child = Command::new("bash")
        .arg(HOOK)
        .env("SUMMA_VAULT", fixture_vault())
        .env("SUMMA_BIN", summa_bin)
        .env("SUMMA_HOOK_STATE_DIR", state_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn hook");
    // The hook may exit (and close stdin) before ever reading it —
    // e.g. AC6 when SUMMA_BIN is not executable, the script exits at
    // its very first check, before `cat -` runs. That races this
    // write against the child closing the pipe, intermittently
    // producing BrokenPipe. A closed stdin is a valid outcome here
    // (the child chose not to read), not a test failure, so the
    // write error is ignored rather than unwrapped
    // (PRD-summa-gate-debt flake-audit).
    let _ = child.stdin.take().unwrap().write_all(payload.as_bytes());
    child.wait_with_output().expect("hook did not run")
}

#[test]
fn ac4_and_ac5_first_prompt_injects_second_is_silent() {
    if !PathBuf::from(HOOK).exists() {
        eprintln!("skipping: hook not installed at {HOOK} on this machine");
        return;
    }
    let state = tempfile::TempDir::new().expect("tempdir");
    let session = "test-session-ac4";
    let payload = format!(
        r#"{{"session_id":"{session}","prompt":"how does build start on redbaron"}}"#
    );

    let first = run_hook(state.path(), env!("CARGO_BIN_EXE_summa"), &payload);
    assert!(first.status.success());
    let out = String::from_utf8_lossy(&first.stdout);
    assert!(
        out.starts_with("summa: relevant wiki pages"),
        "expected prefix, got: {out:?}"
    );
    assert!(out.len() <= 1200 + 1, "output exceeds 1200-byte cap: {} bytes", out.len());
    let page_lines = out.lines().filter(|l| l.starts_with("- ")).count();
    assert!(page_lines <= 5, "more than 5 pages injected: {page_lines}");

    // AC5: same session, second prompt — must emit nothing.
    let second = run_hook(state.path(), env!("CARGO_BIN_EXE_summa"), &payload);
    assert!(second.status.success());
    assert!(
        second.stdout.is_empty(),
        "second prompt in same session must be silent, got: {:?}",
        String::from_utf8_lossy(&second.stdout)
    );
}

#[test]
fn ac6_no_summa_on_path_is_silent() {
    if !PathBuf::from(HOOK).exists() {
        eprintln!("skipping: hook not installed at {HOOK} on this machine");
        return;
    }
    let state = tempfile::TempDir::new().expect("tempdir");
    let session = "test-session-ac6";
    let payload = format!(
        r#"{{"session_id":"{session}","prompt":"how does build start on redbaron"}}"#
    );

    // Point SUMMA_BIN at a path that doesn't exist / isn't executable.
    let out = run_hook(state.path(), "/nonexistent/summa-binary-for-test", &payload);
    assert!(out.status.success(), "hook must exit 0 even without summa");
    assert!(
        out.stdout.is_empty(),
        "hook must be silent when summa is absent: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
}
