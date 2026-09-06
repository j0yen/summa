# Changelog

## v0.5.0 — 2026-09-06

summa lint hardening — the wiki gets tended, not just written. `summa lint` no longer aborts on one malformed frontmatter file (reports it as a `malformed` finding with path + parser error and keeps checking everything else); dangling links now split into `repairable` (case/spacing/alias-normalizable, repaired to the page's canonical title by `--fix`) and `truly-dangling` (survive verbatim as the wiki's to-do list); `--json` carries the new fields. Live-vault backlog cleared as part of this ship: missing_index 1544→0, dangling repaired down to 471 truly-dangling with 0 repairable remaining, malformed_frontmatter=3 reported and fixed by hand. A guarded `summa lint --fix --json` pass is wired into /self-review (no-op when `summa` is absent from PATH), journaling one `summa:` counts line per day.

## v0.4.0 — 2026-09-06

summa page decision — file today's decision in the wiki with one command: `summa page decision <TITLE> --entry "..."` creates the entity page if absent (frontmatter, lede placeholder, Decision log, Mentions) or splices a `## Decision log` section in before `## Mentions` on an existing page, preserving every other byte. Supports `--mention` alongside `--entry`, `--date` backfill, a duplicate guard (byte-identical entries after the date prefix are skipped), and a bare form that prints the log newest-first.

## v0.3.0 — 2026-09-06

The wiki is write-only today: 200+ pages under ~/Notes/wiki that no Claude session consults unless a human remembers to ask. A new `summa relevant <query>` subcommand ranks wiki pages for a query, and a UserPromptSubmit hook injects the top matches into each session's first prompt, the same way recall surfaces memories. Written knowledge starts coming back on its own.

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
