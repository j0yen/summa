//! AC5 (P0, PRD-summa-gate-debt): the ac-traceability source the gate
//! documents (extended-gates.toml's prd_path, resolved to a real PRD-*.md
//! at the repo root) is in place, so the receipt stops reporting "no
//! PRD-*.md found in project root".
use std::fs;
use std::path::Path;

#[test]
fn ac5_extended_gates_prd_path_resolves_to_a_real_file() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cfg = fs::read_to_string(root.join("extended-gates.toml"))
        .expect("AC5: read extended-gates.toml");
    let prd_path = cfg
        .lines()
        .find_map(|l| {
            let l = l.trim();
            l.strip_prefix("prd_path")
                .and_then(|rest| rest.trim_start().strip_prefix('='))
                .map(|v| v.trim().trim_matches('"').to_string())
        })
        .expect("AC5: extended-gates.toml has no prd_path key");
    let resolved = root.join(&prd_path);
    assert!(
        resolved.is_file(),
        "AC5: extended-gates.toml's prd_path ({prd_path}) does not resolve to a real file"
    );
    assert!(
        prd_path.starts_with("PRD-"),
        "AC5: prd_path ({prd_path}) is not this repo's own PRD-*.md, not a borrowed/decoy file"
    );
}
