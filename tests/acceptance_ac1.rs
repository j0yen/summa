//! AC1 (P0, PRD-summa-gate-debt): scripts/run-metrics.sh exists so
//! `autobuilder loop --iteration 0 --trace` no longer aborts on a missing
//! harness script (the root cause of the intake/proof-receipt/session-trace
//! block on every summa gate tick before this PRD).
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[test]
fn ac1_run_metrics_script_exists_and_is_executable() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/run-metrics.sh");
    let meta = fs::metadata(&path)
        .unwrap_or_else(|e| panic!("AC1: scripts/run-metrics.sh missing: {e}"));
    assert!(
        meta.permissions().mode() & 0o111 != 0,
        "AC1: scripts/run-metrics.sh exists but is not executable"
    );
    let body = fs::read_to_string(&path).expect("AC1: read scripts/run-metrics.sh");
    assert!(
        body.contains("target/autobuilder/metrics.json"),
        "AC1: scripts/run-metrics.sh does not write the expected metrics.json path"
    );
}
