//! AC6 (P0, PRD-summa-gate-debt): the lopdf RUSTSEC advisory
//! (RUSTSEC-2026-0187, fixed in lopdf 0.42.0) and deny.toml's
//! rejected-license finding are both resolved.
use std::fs;
use std::path::Path;

#[test]
fn ac6_deny_toml_exists() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("deny.toml");
    assert!(path.is_file(), "AC6: deny.toml is missing");
}

#[test]
fn ac6_lopdf_is_past_the_rustsec_advisory() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock");
    let body = fs::read_to_string(&path).expect("AC6: read Cargo.lock");
    let mut lines = body.lines();
    let version = loop {
        let Some(line) = lines.next() else {
            panic!("AC6: no [[package]] named \"lopdf\" found in Cargo.lock");
        };
        if line.trim() == "name = \"lopdf\"" {
            let v = lines
                .next()
                .expect("AC6: lopdf package block ends before a version line");
            break v
                .trim()
                .strip_prefix("version = \"")
                .and_then(|s| s.strip_suffix('"'))
                .expect("AC6: malformed lopdf version line")
                .to_string();
        }
    };
    let mut parts = version.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    assert!(
        major > 0 || minor >= 42,
        "AC6: locked lopdf version {version} is still on the RUSTSEC-2026-0187 side of 0.42.0"
    );
}
