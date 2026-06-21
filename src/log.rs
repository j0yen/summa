use anyhow::{Context, Result};
use chrono::Utc;
use std::path::Path;

pub fn run(root: &Path, kind: &str, subject: &str, note: &str) -> Result<()> {
    let log_path = root.join("log.md");

    // Create log.md if it doesn't exist
    if !log_path.exists() {
        std::fs::write(&log_path, "# Log\n\n")
            .context("failed to create log.md")?;
    }

    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    // Format: <ISO-8601-Z>  <kind>  <subject>  <note>  (two spaces as field sep)
    let line = format!("{}  {}  {}  {}\n", now, kind, subject, note);

    // Append-only
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&log_path)
        .context("failed to open log.md for append")?;
    file.write_all(line.as_bytes())
        .context("failed to write to log.md")?;

    Ok(())
}
