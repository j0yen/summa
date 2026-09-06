use anyhow::Result;
use clap::{Parser, Subcommand};
use summa::{ingest, index, links, lint, log, page, vault};

#[derive(Parser)]
#[command(
    name = "summa",
    version,
    about = "CLI mechanics layer for an LLM-maintained wiki on Obsidian vaults"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Classify and extract a source (PDF, URL, or Markdown), stage it, and print JSON stub
    Ingest {
        /// Path to a file or a URL to fetch
        source: String,
        /// Destination directory for staged sources (relative to vault root)
        #[arg(long, default_value = "Clippings")]
        dest: String,
    },
    /// Regenerate index.md between summa anchors (idempotent)
    Index,
    /// Append a timestamped line to log.md
    Log {
        /// Kind of event (e.g. ingest, answer, lint)
        kind: String,
        /// Subject or topic (use "-" for none)
        subject: String,
        /// Note text (free text)
        note: Vec<String>,
    },
    /// Parse all wikilinks in the vault and report orphans, dangling, malformed
    Links {
        /// Output JSON instead of human-readable text
        #[arg(long)]
        json: bool,
    },
    /// Run vault health checks (orphans, dangling, malformed, missing-index, stale, un-ingested)
    Lint {
        /// Apply mechanical repairs (malformed links, index regeneration)
        #[arg(long)]
        fix: bool,
        /// Also repair malformed links in human-authored notes (requires --fix)
        #[arg(long)]
        include_human: bool,
        /// Output JSON instead of human-readable text
        #[arg(long)]
        json: bool,
    },
    /// Mint or update wiki pages (entity, summary, answer)
    Page {
        #[command(subcommand)]
        kind: PageCommands,
    },
}

#[derive(Subcommand)]
enum PageCommands {
    /// Create or update an entity page
    Entity {
        /// Title of the entity (Title Case)
        title: String,
        /// Optional alias for the entity
        #[arg(long)]
        alias: Option<String>,
        /// Mention to append: "[[SourceLink]] — claim"
        #[arg(long)]
        mention: Option<String>,
    },
    /// Write or overwrite a source-summary page
    Summary {
        /// Path to the staged source (relative to vault root)
        #[arg(long)]
        source: String,
        /// Title of the source
        #[arg(long)]
        title: String,
        /// Path to a file containing the TL;DR text
        #[arg(long)]
        tldr: String,
        /// Entity wikilinks to associate (e.g. [[Model Context Protocol]])
        #[arg(long = "entity")]
        entities: Vec<String>,
    },
    /// Write an answer page
    Answer {
        /// The question being answered
        #[arg(long)]
        question: String,
        /// Slug for the filename
        #[arg(long)]
        slug: String,
        /// Path to a file containing the answer body
        #[arg(long)]
        body: String,
        /// Pages cited (e.g. [[MCP Analysis]])
        #[arg(long = "cite")]
        cites: Vec<String>,
    },
    /// File a decision entry on an entity page (create-or-append, idempotent)
    Decision {
        /// Title of the entity (Title Case)
        title: String,
        /// Entry text to log verbatim; newlines are folded to spaces
        #[arg(long)]
        entry: Option<String>,
        /// Mention to append alongside the entry: "[[SourceLink]] — claim"
        #[arg(long)]
        mention: Option<String>,
        /// Override the entry's date (ISO, e.g. 2026-09-01); default is today
        #[arg(long)]
        date: Option<String>,
    },
}

fn main() -> Result<()> {
    // SIGPIPE convention: must be first line in main()
    sigpipe::reset();

    let cli = Cli::parse();
    let vault_root = vault::resolve_root()?;

    match cli.command {
        Commands::Ingest { source, dest } => {
            let stub = ingest::run(&vault_root, &source, &dest)?;
            println!("{}", serde_json::to_string_pretty(&stub)?);
        }
        Commands::Index => {
            index::run(&vault_root)?;
        }
        Commands::Log { kind, subject, note } => {
            let note_str = note.join(" ");
            log::run(&vault_root, &kind, &subject, &note_str)?;
        }
        Commands::Links { json } => {
            let report = links::run(&vault_root)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                links::print_human(&report);
            }
        }
        Commands::Lint { fix, include_human, json } => {
            let opts = lint::LintOpts { fix, include_human, json };
            let report = lint::run(&vault_root, &opts)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                lint::print_human(&report);
            }
        }
        Commands::Page { kind } => match kind {
            PageCommands::Entity { title, alias, mention } => {
                page::entity(&vault_root, &title, alias.as_deref(), mention.as_deref())?;
            }
            PageCommands::Summary { source, title, tldr, entities } => {
                page::summary(&vault_root, &source, &title, &tldr, &entities)?;
            }
            PageCommands::Answer { question, slug, body, cites } => {
                page::answer(&vault_root, &question, &slug, &body, &cites)?;
            }
            PageCommands::Decision { title, entry, mention, date } => {
                if entry.is_none() && mention.is_none() && date.is_none() {
                    // Bare form: print the existing log, newest first.
                    let log = page::decision_log(&vault_root, &title)?;
                    for line in log {
                        println!("{}", line);
                    }
                } else {
                    let appended = page::decision(
                        &vault_root,
                        &title,
                        entry.as_deref(),
                        mention.as_deref(),
                        date.as_deref(),
                    )?;
                    if entry.is_some() && !appended {
                        println!("duplicate entry, not appended");
                    }
                }
            }
        },
    }

    Ok(())
}
