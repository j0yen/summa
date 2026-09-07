# PRD: summa-gate-debt — pay the repo-wide receipts blocking every summa ship

- Status: queued
- build_target: rust-extend
- build_into: /home/jsy/repos/summa
- build_priority: high
- build_version_bump: patch
- publish: none
- PM: Joe
- Drafted: 2026-09-06
- Vision: visions/buildloop-operations.md
- Engineering target: extend ~/repos/summa in place (v0.6.0 → v0.6.1) with the scaffold, config, and toolchain fixes the extend-gate has flagged on every tick since v0.4.0; no feature code.

## TL;DR

Three summa PRDs have finished code and green tests but cannot tag or archive: the gate reports ~10 blocking receipts that are repo debt, not PRD defects. Verified on the redbaron checkout: no `scripts/` dir (run-metrics.sh and audit.sh missing, which kills intake/proof-receipt/session-trace and risk-gate), no `.github/workflows/` (ci-checks counts 0 runs), `rust-version = "1.85"` while locked deps need 1.88 (msrv-verify and flake-audit red), no fixtures lane in `agent/proof-lanes.toml` (vti-plan flags non-.rs fixture files), no ac-traceability source in the repo, cargo-deny findings (lopdf advisory via pdf-extract, a rejected license), and tags stopped at v0.4.0 so receipt base_refs mis-attribute sibling diffs. Fix them once, at the repo level.

## Problem

Every gate tick on summa re-confirms the same inherited blocks (receipts under `target/autobuilder/receipts/`, journaled repeatedly in PRD-summa-relevant-recall's Blocked log). Until block-attribution changes ship (PRD-build-gate-delta-baseline) or the debt is paid, nothing in this repo can reach `built`. This PRD is the payment.

## Requirements

P0 (build these)
1. Scaffold `scripts/run-metrics.sh` and `scripts/audit.sh` matching the autobuilder loop's expectations (model them on a repo where intake/proof-receipt/risk-gate pass, e.g. mcphost's), so `autobuilder loop --iteration 0 --trace` completes.
2. Bump `rust-version` to 1.88 in Cargo.toml so the declared MSRV matches the Cargo.lock reality (icu_*/idna_adapter/time need 1.86–1.88); msrv-verify and flake-audit run against the declared toolchain.
3. Add a fixtures lane to `agent/proof-lanes.toml` covering non-`.rs` files under `tests/fixtures/**` so vti-plan routes them.
4. Provide the ac-traceability source the gate documents (extended-gates.toml or in-repo PRD copy — whichever the gate's own docs specify) so the receipt stops reporting "no PRD-*.md found".
5. Resolve the cargo-deny blocks: update lopdf past the RUSTSEC advisory (>=0.42 via pdf-extract or direct pin) and handle the rejected-license finding in deny.toml with a scoped, commented exception.
6. Backfill annotated tags `v0.5.0` and `v0.6.0` at the commits where Cargo.toml reached those versions, so receipt base_ref diffs attribute each sibling's commits correctly.

P1
7. Add a minimal `.github/workflows/ci.yml` (cargo test on push) and push it so ci-checks has runs to settle against.

## Acceptance criteria

1. P0 — Given the extended repo, When `autobuilder loop --iteration 0 --trace` runs, Then it no longer aborts on missing `scripts/run-metrics.sh` and the intake, proof-receipt, and session-trace receipts are produced.
2. P0 — Given `scripts/audit.sh` exists, When the gate's risk-gate producer runs, Then the risk-gate receipt is produced instead of reported missing.
3. P0 — Given `rust-version = "1.88"`, When msrv-verify runs its toolchain check three times, Then all runs exit 0 and the flake-audit receipt stops flagging the MSRV check.
4. P0 — Given the fixtures lane in proof-lanes.toml, When `autobuilder vti-plan --project .` runs at HEAD, Then `tests/fixtures/vault/**` paths route with nonzero confidence and vti-plan reports unrouted=0.
5. P0 — Given the ac-traceability source is in place, When that receipt producer runs, Then it stops reporting "no PRD-*.md found in project root".
6. P0 — Given the dependency and deny.toml changes, When `cargo deny check advisories licenses` runs, Then the lopdf advisory and the rejected-license finding are both resolved, and `cargo test` stays fully green.
7. P0 — Given the backfilled tags, When `git describe --tags` runs at the v0.6.0 commit and at HEAD, Then base_ref resolution yields v0.6.0 (not v0.4.0), and both tags are annotated and pushed.
8. P0 — Given all P0 fixes landed, When `extend-gate.sh /home/jsy/repos/summa --head <landed sha>` runs, Then the blocking set contains none of: intake, proof-receipt, session-trace, risk-gate, msrv-verify, flake-audit, vti-plan, ac-traceability (ci-checks and reviewer-agent may still block pending CI history).
9. P1 — Given the pushed workflow, When one CI run completes on GitHub, Then a subsequent gate run's ci-checks receipt reports run_count >= 1.
10. P0 — Given the finished diff, When version and tests are checked, Then Cargo.toml is 0.6.1, `cargo test` is fully green, and clippy is clean on the diff.

## Non-goals

- Changing gate semantics (PRD-build-gate-delta-baseline owns that).
- Touching the three sibling PRDs' feature code or re-litigating their reviewer findings.
- Fixing reviewer-agent's cross-sibling attribution (scheduler-level; follow-on).

## Deploy notes

Lands directly in ~/repos/summa on redbaron; after it tags, re-run the gate for the three parked summa siblings — with this debt paid (and/or a recorded baseline), their next tick should ship them.
