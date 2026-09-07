//! AC9 (P1, PRD-summa-gate-debt): "Given the pushed workflow, When one CI
//! run completes on GitHub, Then a subsequent gate run's ci-checks
//! receipt reports run_count >= 1."
//!
//! Whether a GitHub Actions run has actually completed is a fact about
//! GitHub's state after this commit is pushed, not something knowable
//! from inside a `cargo test` invocation running before that push even
//! happens. This test only pins the precondition this crate controls:
//! the workflow file exists and is minimally well-formed (cargo test
//! wired in). The actual run_count evidence is the gate's own
//! ci-checks-receipt.json, captured in this PRD's `Receipts:` line.
use std::fs;
use std::path::Path;

#[test]
fn ac9_ci_workflow_exists_and_runs_cargo_test() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    let body = fs::read_to_string(&path).expect("AC9: .github/workflows/ci.yml is missing");
    assert!(
        body.contains("cargo test"),
        "AC9: ci.yml does not run cargo test"
    );
}
