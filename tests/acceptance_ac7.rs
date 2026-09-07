//! AC7 (P0, PRD-summa-gate-debt): backfilled annotated tags v0.5.0 and
//! v0.6.0 exist, so receipt base_ref resolution stops mis-attributing
//! sibling diffs to v0.4.0.
use std::path::Path;
use std::process::Command;

#[test]
fn ac7_v0_5_0_and_v0_6_0_tags_exist_and_are_annotated() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for tag in ["v0.5.0", "v0.6.0"] {
        let out = Command::new("git")
            .args(["cat-file", "-t", tag])
            .current_dir(root)
            .output()
            .unwrap_or_else(|e| panic!("AC7: failed to run git cat-file for {tag}: {e}"));
        let kind = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            kind.trim(),
            "tag",
            "AC7: {tag} does not exist as an annotated tag object (got {kind:?})"
        );
    }
}
