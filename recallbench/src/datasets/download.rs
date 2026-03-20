use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::io::AsyncWriteExt;

/// Default cache directory for downloaded datasets.
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("recallbench")
        .join("data")
}

/// Download a file from a URL to the cache directory.
///
/// - Skips download if the file already exists and is non-empty.
/// - Shows a progress bar during download.
/// - Returns the path to the cached file.
pub async fn download_dataset(url: &str, filename: &str, force: bool) -> Result<PathBuf> {
    let dir = cache_dir();
    tokio::fs::create_dir_all(&dir).await
        .context("Failed to create cache directory")?;

    let dest = dir.join(filename);

    if !force && is_cached(&dest).await {
        tracing::info!("Using cached dataset: {}", dest.display());
        return Ok(dest);
    }

    tracing::info!("Downloading {} ...", url);

    let client = reqwest::Client::new();
    let response = client.get(url).send().await
        .context("Failed to send download request")?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("Download failed with status {status}: {url}");
    }

    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{wide_bar:.cyan/blue} {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("##-"),
    );
    pb.set_message(format!("Downloading {filename}"));

    let temp_path = dest.with_extension("part");
    let mut file = tokio::fs::File::create(&temp_path).await
        .context("Failed to create temp file")?;

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Error reading download stream")?;
        file.write_all(&chunk).await
            .context("Error writing to file")?;
        pb.inc(chunk.len() as u64);
    }

    file.flush().await?;
    drop(file);

    tokio::fs::rename(&temp_path, &dest).await
        .context("Failed to move downloaded file into place")?;

    pb.finish_with_message(format!(
        "Downloaded {filename} ({:.1} MB)",
        dest.metadata().map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0)
    ));

    Ok(dest)
}

/// Check if a file is already cached (exists and >1KB).
async fn is_cached(path: &Path) -> bool {
    match tokio::fs::metadata(path).await {
        Ok(meta) => meta.len() > 1024,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_dir_is_valid() {
        let dir = cache_dir();
        assert!(dir.to_str().unwrap().contains("recallbench"));
        assert!(dir.to_str().unwrap().contains("data"));
    }

    #[tokio::test]
    async fn is_cached_nonexistent() {
        assert!(!is_cached(Path::new("/nonexistent/file.json")).await);
    }
}
