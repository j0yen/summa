//! AC2 (P0, PRD-summa-gate-debt): scripts/audit.sh exists so the gate's
//! risk-gate producer stops reporting the receipt as missing.
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[test]
fn ac2_audit_script_exists_and_is_executable() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/audit.sh");
    let meta =
        fs::metadata(&path).unwrap_or_else(|e| panic!("AC2: scripts/audit.sh missing: {e}"));
    assert!(
        meta.permissions().mode() & 0o111 != 0,
        "AC2: scripts/audit.sh exists but is not executable"
    );
    let body = fs::read_to_string(&path).expect("AC2: read scripts/audit.sh");
    assert!(
        body.contains("target/autobuilder/receipts/risk-gate.json"),
        "AC2: scripts/audit.sh does not write the expected risk-gate.json receipt path"
    );
}
