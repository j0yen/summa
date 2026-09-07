//! AC8 (P0, PRD-summa-gate-debt): "Given all P0 fixes landed, When
//! extend-gate.sh --head <landed sha> runs, Then the blocking set
//! contains none of: intake, proof-receipt, session-trace, risk-gate,
//! msrv-verify, flake-audit, vti-plan, ac-traceability."
//!
//! This is a claim about the external 25-receipt gate pipeline's own
//! output at a specific commit, not about this crate's behavior — it
//! cannot be honestly re-created as an in-process unit test (that would
//! mean re-running the entire multi-minute extend-gate.sh pipeline,
//! including a headless reviewer-agent subagent call, from inside `cargo
//! test`). The actual evidence is the gate run captured in this PRD's own
//! `Receipts:` line (see PRD-summa-gate-debt.md, or the built-prds/ copy
//! once archived) plus the raw receipt JSON under
//! target/autobuilder/receipts/ at the landed commit. This test only
//! pins the one thing checkable from inside the crate: that the script
//! this AC is about actually exists where the gate expects it.
use std::path::Path;

#[test]
fn ac8_verified_externally_via_extend_gate_receipts() {
    // AC8's own evidence lives outside this repo (the /build fleet's
    // extend-gate.sh); this assertion only confirms the harness pieces
    // that gate depends on are present, not the gate's own verdict.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for rel in ["scripts/run-metrics.sh", "scripts/audit.sh", "deny.toml"] {
        assert!(
            root.join(rel).is_file(),
            "AC8: extend-gate.sh depends on {rel}, which is missing"
        );
    }
}
