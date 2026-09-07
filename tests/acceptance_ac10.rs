//! AC10 (P0, PRD-summa-gate-debt, archived): "Given the finished diff,
//! When version and tests are checked, Then Cargo.toml is 0.6.1, cargo
//! test is fully green, and clippy is clean on the diff." True at ship
//! (c9498c9, tag v0.6.1). summa is a shared repo that keeps shipping
//! unrelated work on the same crate afterward, so a hardcoded equality
//! check breaks on every subsequent legitimate version bump -- observed
//! live: PRD-summa-lint-selfreview's archive tick hit this when v0.6.2
//! landed. A first attempt just loosened the check to `>= 0.6.1`, which
//! this repo's own reviewer-agent correctly blocked: that would let ANY
//! future version (including an unreviewed, unauthorized jump) sail
//! through silently, since nothing ties the version claim to a real
//! record of a reviewed release. This version instead requires genuine
//! provenance: the version must be >= 0.6.1 AND CHANGELOG.md must carry
//! a `## v<version>` heading for the CURRENT version -- the same
//! convention every legitimate ship in this repo's history already
//! follows (see the `## v0.6.1` / `## v0.6.2` etc. sections). A silent,
//! undocumented version bump with no CHANGELOG entry now fails exactly
//! as the reviewer's counter-attack test wanted, while a normal,
//! reviewed ship (which always adds its own CHANGELOG section) keeps
//! passing without ever needing to hardcode the next literal version.
//!
//! This test checks the one fact re-derivable from inside the crate
//! (the declared version, cross-checked against CHANGELOG.md). "cargo
//! test is fully green" is self-evidently true if this test binary is
//! running at all; "clippy is clean" is a separate, non-test invocation
//! (`cargo clippy --workspace -- -D warnings`), captured in this PRD's
//! own verification notes rather than re-implemented as a unit test
//! that would just shell out to clippy.
use std::fs;
use std::path::Path;

fn cargo_toml_version(manifest_dir: &Path) -> String {
    let path = manifest_dir.join("Cargo.toml");
    let body = fs::read_to_string(&path).expect("AC10: read Cargo.toml");
    let version_line = body
        .lines()
        .find(|l| l.trim().starts_with("version = \""))
        .expect("AC10: no [package].version line in Cargo.toml");
    version_line
        .trim()
        .trim_start_matches("version = \"")
        .trim_end_matches('"')
        .to_string()
}

#[test]
fn ac10_cargo_toml_version_is_at_least_0_6_1() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let version = cargo_toml_version(manifest_dir);
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

#[test]
fn ac10_version_has_changelog_provenance() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let version = cargo_toml_version(manifest_dir);
    let changelog =
        fs::read_to_string(manifest_dir.join("CHANGELOG.md")).expect("AC10: read CHANGELOG.md");
    let heading = format!("## v{version}");
    assert!(
        changelog.lines().any(|l| l.trim_start().starts_with(&heading)),
        "AC10: Cargo.toml's version {version} has no matching '{heading}' section in \
         CHANGELOG.md -- every reviewed release records its own CHANGELOG entry; a version \
         bump with none is either undocumented or unauthorized"
    );
}
