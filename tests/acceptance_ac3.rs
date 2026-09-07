//! AC3 (P0, PRD-summa-gate-debt): declared rust-version matches what the
//! locked deps actually require (1.88), so msrv-verify's `cargo +1.88
//! check` targets a toolchain that's really available and flake-audit
//! stops flagging the MSRV mismatch.
use std::fs;
use std::path::Path;

#[test]
fn ac3_declared_rust_version_is_1_88() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let body = fs::read_to_string(&path).expect("AC3: read Cargo.toml");
    assert!(
        body.lines()
            .any(|l| l.trim() == "rust-version = \"1.88\""),
        "AC3: Cargo.toml does not declare rust-version = \"1.88\""
    );
}
