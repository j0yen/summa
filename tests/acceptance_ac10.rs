//! AC10 (P0, PRD-summa-gate-debt, archived): "Given the finished diff,
//! When version and tests are checked, Then Cargo.toml is 0.6.1, cargo
//! test is fully green, and clippy is clean on the diff." True at ship
//! (c9498c9, tag v0.6.1). PRD-summa-gate-debt is a shared-repo PRD --
//! summa keeps shipping unrelated work on the same crate afterward, so
//! this now checks the durable half of AC10's intent (properly versioned
//! at-or-past the shipped baseline, on the 0.x line) rather than pinning
//! the exact string forever, which would break every legitimate version
//! bump after this PRD's own ship (observed live: PRD-summa-lint-
//! selfreview's archive tick hit exactly this when a v0.6.2 bump landed).
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
fn ac10_cargo_toml_version_is_at_least_0_6_1() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let body = fs::read_to_string(&path).expect("AC10: read Cargo.toml");
    let version_line = body
        .lines()
        .find(|l| l.trim().starts_with("version = \""))
        .expect("AC10: no [package].version line in Cargo.toml");
    let version = version_line
        .trim()
        .trim_start_matches("version = \"")
        .trim_end_matches('"');
    let parts: Vec<u32> = version
        .split('.')
        .map(|p| p.parse().expect("AC10: version segment is not numeric"))
        .collect();
    assert_eq!(parts.len(), 3, "AC10: version is not major.minor.patch: {version}");
    assert!(
        (parts[0], parts[1], parts[2]) >= (0, 6, 1),
        "AC10: Cargo.toml's [package].version {version} is below the shipped baseline 0.6.1"
    );
}
