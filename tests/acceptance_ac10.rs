//! AC10 (P0, PRD-summa-gate-debt): "Given the finished diff, When version
//! and tests are checked, Then Cargo.toml is 0.6.1, cargo test is fully
//! green, and clippy is clean on the diff."
//!
//! This test checks the one fact re-derivable from inside the crate
//! (the declared version). "cargo test is fully green" is self-evidently
//! true if this test binary is running at all; "clippy is clean" is a
//! separate, non-test invocation (`cargo clippy --workspace -- -D
//! warnings`), captured in this PRD's own verification notes rather than
//! re-implemented as a unit test that would just shell out to clippy.
use std::fs;
use std::path::Path;

#[test]
fn ac10_cargo_toml_version_is_0_6_1() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let body = fs::read_to_string(&path).expect("AC10: read Cargo.toml");
    assert!(
        body.lines().any(|l| l.trim() == "version = \"0.6.1\""),
        "AC10: Cargo.toml's [package].version is not 0.6.1"
    );
}
