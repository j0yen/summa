use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct IngestStub {
    pub source: String,
    pub kind: String,
    pub staged_path: String,
    pub extracted_text_path: String,
    pub suggested_title: String,
    pub bytes: u64,
}

pub fn run(vault_root: &Path, source: &str, dest: &str) -> Result<IngestStub> {
    if is_url(source) {
        ingest_url(vault_root, source, dest)
    } else {
        let path = PathBuf::from(source);
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "pdf" => ingest_pdf(vault_root, &path, dest),
            "md" | "txt" => ingest_md(vault_root, &path, dest),
            _ => ingest_md(vault_root, &path, dest),
        }
    }
}

fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

fn ingest_pdf(vault_root: &Path, path: &Path, dest: &str) -> Result<IngestStub> {
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    if !abs_path.exists() {
        return Err(anyhow!("File not found: {}", abs_path.display()));
    }

    let bytes = std::fs::metadata(&abs_path)?.len();
    let file_name = abs_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown.pdf");

    // Stage the file to dest
    let dest_dir = vault_root.join(dest);
    std::fs::create_dir_all(&dest_dir)?;
    let staged = dest_dir.join(file_name);
    if abs_path != staged {
        std::fs::copy(&abs_path, &staged)
            .with_context(|| format!("failed to stage {} to {}", abs_path.display(), staged.display()))?;
    }

    // Extract text using pdf-extract
    let text = pdf_extract::extract_text(&abs_path)
        .with_context(|| format!("pdf-extract failed on {}", abs_path.display()))?;

    // Write extracted text to temp file
    let sha8 = sha8_of_path(&abs_path.to_string_lossy());
    let text_path = std::env::temp_dir().join(format!("summa-{}.txt", sha8));
    std::fs::write(&text_path, &text)?;

    let suggested_title = stem_to_title(
        abs_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown"),
    );

    let staged_rel = staged
        .strip_prefix(vault_root)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| staged.to_string_lossy().into_owned());

    Ok(IngestStub {
        source: abs_path.to_string_lossy().into_owned(),
        kind: "pdf".to_string(),
        staged_path: staged_rel,
        extracted_text_path: text_path.to_string_lossy().into_owned(),
        suggested_title,
        bytes,
    })
}

fn ingest_md(vault_root: &Path, path: &Path, dest: &str) -> Result<IngestStub> {
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    if !abs_path.exists() {
        return Err(anyhow!("File not found: {}", abs_path.display()));
    }

    let bytes = std::fs::metadata(&abs_path)?.len();
    let content = std::fs::read_to_string(&abs_path)?;
    let file_name = abs_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown.md");

    // Stage the file
    let dest_dir = vault_root.join(dest);
    std::fs::create_dir_all(&dest_dir)?;
    let staged = dest_dir.join(file_name);
    if abs_path != staged {
        std::fs::copy(&abs_path, &staged)
            .with_context(|| format!("failed to stage {}", abs_path.display()))?;
    }

    // Write content to temp file
    let sha8 = sha8_of_path(&abs_path.to_string_lossy());
    let text_path = std::env::temp_dir().join(format!("summa-{}.txt", sha8));
    std::fs::write(&text_path, &content)?;

    let suggested_title = stem_to_title(
        abs_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown"),
    );

    let staged_rel = staged
        .strip_prefix(vault_root)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| staged.to_string_lossy().into_owned());

    Ok(IngestStub {
        source: abs_path.to_string_lossy().into_owned(),
        kind: "md".to_string(),
        staged_path: staged_rel,
        extracted_text_path: text_path.to_string_lossy().into_owned(),
        suggested_title,
        bytes,
    })
}

fn ingest_url(vault_root: &Path, url: &str, dest: &str) -> Result<IngestStub> {
    // Best-effort fetch; fail cleanly with non-zero exit on error
    let response = ureq::get(url)
        .timeout(std::time::Duration::from_secs(15))
        .call()
        .with_context(|| format!("failed to fetch URL: {}", url))?;

    let body = response.into_string().context("failed to read response body")?;

    // Convert HTML to markdown
    let markdown = html2text::from_read(body.as_bytes(), 80);

    let bytes = markdown.len() as u64;

    // Derive a slug from the URL
    let slug = url_to_slug(url);
    let file_name = format!("{}.md", slug);

    // Stage
    let dest_dir = vault_root.join(dest);
    std::fs::create_dir_all(&dest_dir)?;
    let staged = dest_dir.join(&file_name);
    std::fs::write(&staged, &markdown)?;

    // Write to temp file
    let sha8 = sha8_of_path(url);
    let text_path = std::env::temp_dir().join(format!("summa-{}.txt", sha8));
    std::fs::write(&text_path, &markdown)?;

    let suggested_title = slug.replace('-', " ");
    let suggested_title = title_case(&suggested_title);

    let staged_rel = staged
        .strip_prefix(vault_root)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| staged.to_string_lossy().into_owned());

    Ok(IngestStub {
        source: url.to_string(),
        kind: "url".to_string(),
        staged_path: staged_rel,
        extracted_text_path: text_path.to_string_lossy().into_owned(),
        suggested_title,
        bytes,
    })
}

fn sha8_of_path(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:08x}", hasher.finish())
}

fn stem_to_title(stem: &str) -> String {
    let s = stem.replace(['-', '_'], " ");
    title_case(&s)
}

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn url_to_slug(url: &str) -> String {
    // Strip scheme and strip trailing slashes
    let without_scheme = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    // Take path parts after the domain
    let parts: Vec<&str> = without_scheme.split('/').collect();
    if parts.len() > 1 {
        // Use last non-empty segment
        let last = parts
            .iter()
            .rev()
            .find(|p| !p.is_empty())
            .copied()
            .unwrap_or("page");
        // Strip query/fragment
        let last = last.split('?').next().unwrap_or(last);
        let last = last.split('#').next().unwrap_or(last);
        // Replace non-alphanumeric with -
        let slug: String = last
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect();
        slug.trim_matches('-').to_string()
    } else {
        // Just the domain
        let domain = parts[0].split('?').next().unwrap_or(parts[0]);
        domain.replace('.', "-")
    }
}
