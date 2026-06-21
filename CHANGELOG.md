# Changelog

## v0.2.0 — 2026-06-21

### Added
- `summa lint [--fix] [--include-human] [--json]` subcommand with 6 vault health checks:
  1. **orphan** — summa-owned pages with 0 inbound links (report-only)
  2. **dangling** — `[[X]]` wikilinks with no matching X.md (report-only)
  3. **malformed** — links using `\|` or all-digit alias; `--fix` repairs summa-owned files (add `--include-human` to also fix human notes)
  4. **missing-index** — summa pages absent from `index.md` anchored section; `--fix` runs index regeneration
  5. **stale-vs-source** — source-summary pages whose `ingested:` timestamp predates their source file's mtime (report-only)
  6. **un-ingested** — raw PDFs or `Clippings/*.md` with no matching source-summary (report-only)
- After each lint run, appends a line to `log.md` summarising counts
- `--json` output for all 6 categories plus fixed records
- `--fix` is idempotent: a second run produces no further diff
- 5 new integration tests covering all ACs (25 tests total, all green)

## v0.1.0 — 2026-06-21

Initial release: `summa ingest`, `summa index`, `summa log`, `summa links`, `summa page entity/summary/answer`.
