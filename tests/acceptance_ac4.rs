//! AC4 (P0, PRD-summa-gate-debt): agent/proof-lanes.toml routes non-.rs
//! fixture files under tests/fixtures/** so vti-plan doesn't flag them
//! unrouted. (Landed earlier by commit 78ea2a5 — this test pins the
//! behavior so a future proof-lanes.toml edit can't silently drop it,
//! without pulling in a toml-parsing dev-dependency just for this check.)
use std::fs;
use std::path::Path;

#[test]
fn ac4_fixtures_lane_covers_non_rs_fixture_files() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("agent/proof-lanes.toml");
    let body = fs::read_to_string(&path).expect("AC4: read agent/proof-lanes.toml");
    assert!(
        body.contains("\"tests/fixtures/**\""),
        "AC4: no lane glob in agent/proof-lanes.toml covers tests/fixtures/**"
    );
}
