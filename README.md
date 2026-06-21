# summa

CLI mechanics layer for an LLM-maintained wiki on Obsidian vaults (the [`/summa` skill](https://github.com/j0yen/autobuilder-private) calls these commands; the skill adds LLM synthesis on top).

## TL;DR

summa provides deterministic, testable operations that belong in a CLI rather than an LLM skill: extract text from a dropped source, regenerate the vault index, append to the timeline log, and compute the wikilink graph (orphans + dangling targets). These operations have no LLM judgment involved — they're pure mechanics.

## Subcommands

| Command | What it does |
|---------|-------------|
| `summa ingest <path\|url>` | Classify source (PDF/URL/MD), extract text, stage under `Clippings/`, emit JSON stub |
| `summa index` | Regenerate `index.md` between `<!-- summa:index-start/end -->` anchors (idempotent) |
| `summa log <kind> <subject> <note>` | Append one timestamped line to `log.md` (append-only) |
| `summa links [--json]` | Parse all `[[wikilinks]]`, report orphans / dangling / malformed |
| `summa page entity <Title> [--mention "..."]` | Create/update an entity page (dedup on source link) |
| `summa page summary --source ... --title ... --tldr ...` | Write/overwrite a source-summary page |
| `summa page answer --question ... --slug ... --body ...` | Write an answer page |

## JSON contracts

### `summa ingest`
```json
{"source":"<path>","kind":"pdf|url|md","staged_path":"Clippings/x.pdf",
 "extracted_text_path":"/tmp/summa-<sha8>.txt","suggested_title":"X","bytes":1234}
```

### `summa links --json`
```json
{"orphans":["wiki/entities/Foo.md"],
 "dangling":[{"target":"Bar","in":["wiki/sources/x.md"]}],
 "malformed":[{"file":"AtScale/y.md","raw":"[[A\\|123]]","fix":"[[A]]"}],
 "stats":{"pages":520,"links":1840}}
```

## Acceptance criteria

1. `summa ingest <fixture.pdf>` extracts non-empty text, stages source, prints valid JSON with `source`, `staged_path`, `extracted_text_path` keys
2. `summa ingest <fixture.md>` passes markdown through + emits stub JSON; URL ingestion fails cleanly on network error
3. `summa index` writes `index.md` with content between anchors, preserves preamble, idempotent (second run = no diff)
4. `summa log ingest AtScale "test entry"` appends exactly one correctly-formatted timestamped line
5. `summa links --json` correctly enumerates orphans, dangling, malformed `\|` links (asserted on fixture vault)
6. `summa page entity <Title> --mention "..."` creates entity page + appends deduped mention bullet (round-trip tested)
7. `cargo test --release` passes (20 tests); `summa --help` lists all 5 subcommands

## Configuration

- Vault root: `$SUMMA_VAULT` env var, else `~/Notes`
- PDF extraction: `pdf-extract` (pure Rust, no `pdftotext` dependency)

## Install

```sh
cargo install --path .
# or from source:
cargo build --release
install -Dm755 target/release/summa ~/.local/bin/summa
```

## Requirements

- Rust 1.85+ (MSRV)
- No system dependencies (pure Rust PDF extraction via `pdf-extract`)

## Ownership invariant

summa writes ONLY files it owns (files with a `summa:` frontmatter key), all under `~/Notes/wiki/`, plus the managed `index.md` and `log.md`. It never modifies human notes or raw sources.

## License

MIT — Joe Yen
