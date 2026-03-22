//! Pre-computed embedding cache for benchmark datasets.
//!
//! Embeds all unique sessions in a dataset once, stores chunks + vectors in a
//! SQLite cache file. Subsequent benchmark runs load from cache instead of
//! re-embedding — turning minutes of API calls into seconds of SQLite reads.
//!
//! Defaults to API embedding (fast, cheap). Falls back to local Candle if no
//! API key is configured.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rusqlite::{params, Connection};

use crate::types::ConversationSession;

/// A cached chunk with its pre-computed embedding.
#[derive(Debug, Clone)]
pub struct CachedChunk {
    pub session_id: String,
    pub chunk_index: usize,
    pub text: String,
    pub embedding: Vec<f32>,
    pub session_date: String,
}

/// Embedding cache backed by a SQLite database.
pub struct EmbeddingCache {
    path: PathBuf,
}

impl EmbeddingCache {
    /// Cache directory for embeddings.
    fn cache_dir() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("recallbench")
            .join("embeddings")
    }

    /// Path to the cache file for a given dataset/variant/model combination.
    pub fn cache_path(dataset: &str, variant: &str, model_name: &str) -> PathBuf {
        // Sanitize model name for filesystem
        let safe_model = model_name.replace('/', "-");
        Self::cache_dir().join(format!("{dataset}-{variant}-{safe_model}.db"))
    }

    /// Check if a valid cache exists for this configuration.
    pub fn exists(dataset: &str, variant: &str, model_name: &str) -> bool {
        let path = Self::cache_path(dataset, variant, model_name);
        if !path.exists() {
            return false;
        }
        // Verify the cache is valid (has the meta table and matching config)
        match Connection::open(&path) {
            Ok(conn) => {
                let result: Result<String, _> = conn.query_row(
                    "SELECT value FROM cache_meta WHERE key = 'model_name'",
                    [],
                    |row| row.get(0),
                );
                match result {
                    Ok(cached_model) => cached_model == model_name,
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }

    /// Build the embedding cache for a dataset.
    ///
    /// Extracts all unique sessions, chunks them, embeds via the provided backend,
    /// and stores everything in a SQLite cache file.
    pub fn build(
        dataset: &str,
        variant: &str,
        questions: &[crate::types::BenchmarkQuestion],
        backend: &dyn mindcore::embeddings::EmbeddingBackend,
        chunk_size: usize,
        min_turn_chars: usize,
    ) -> Result<Self> {
        let path = Self::cache_path(dataset, variant, backend.model_name());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        tracing::info!("Building embedding cache at {}", path.display());

        // Collect unique sessions across all questions
        let mut unique_sessions: HashMap<String, &ConversationSession> = HashMap::new();
        for q in questions {
            for session in &q.sessions {
                unique_sessions.entry(session.id.clone()).or_insert(session);
            }
        }

        tracing::info!(
            "Found {} unique sessions across {} questions",
            unique_sessions.len(),
            questions.len()
        );

        // Chunk all sessions
        let mut all_chunks: Vec<(String, String, usize, String)> = Vec::new(); // (session_id, date, chunk_idx, text)
        for (session_id, session) in &unique_sessions {
            let session_date = session.date.clone().unwrap_or_default();
            let turns_iter = session.turns.iter().map(|t| (t.role.as_str(), t.content.as_str()));
            let chunks = mindcore::ingest::chunking::chunk_session(
                turns_iter, &session_date, chunk_size, min_turn_chars,
            );
            for (idx, chunk) in chunks.iter().enumerate() {
                all_chunks.push((session_id.clone(), session_date.clone(), idx, chunk.text.clone()));
            }
        }

        tracing::info!("Chunked into {} total chunks, embedding...", all_chunks.len());

        // Create the cache database
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cache_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS session_chunks (
                session_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                chunk_text TEXT NOT NULL,
                embedding BLOB NOT NULL,
                session_date TEXT NOT NULL DEFAULT '',
                PRIMARY KEY (session_id, chunk_index)
            );
            CREATE INDEX IF NOT EXISTS idx_session_id ON session_chunks(session_id);",
        )?;

        // Store metadata
        conn.execute(
            "INSERT OR REPLACE INTO cache_meta (key, value) VALUES ('model_name', ?1)",
            [backend.model_name()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO cache_meta (key, value) VALUES ('chunk_size', ?1)",
            [chunk_size.to_string()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO cache_meta (key, value) VALUES ('created_at', ?1)",
            [chrono::Utc::now().to_rfc3339()],
        )?;

        // Embed in batches and store
        let pb = ProgressBar::new(all_chunks.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg}\n{wide_bar:.cyan/dim} {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("##-"),
        );
        pb.set_message("Embedding dataset chunks...");

        const BATCH_SIZE: usize = 256;
        for batch_start in (0..all_chunks.len()).step_by(BATCH_SIZE) {
            let batch_end = (batch_start + BATCH_SIZE).min(all_chunks.len());
            let batch = &all_chunks[batch_start..batch_end];

            let texts: Vec<&str> = batch.iter().map(|(_, _, _, text)| text.as_str()).collect();
            let embeddings = backend.embed_batch(&texts)
                .context("Failed to embed batch")?;

            // Store in a transaction
            let tx = conn.unchecked_transaction()?;
            for ((session_id, date, idx, text), embedding) in batch.iter().zip(embeddings.iter()) {
                let blob = mindcore::embeddings::pooling::vec_to_bytes(embedding);
                tx.execute(
                    "INSERT OR REPLACE INTO session_chunks (session_id, chunk_index, chunk_text, embedding, session_date)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![session_id, idx, text, blob, date],
                )?;
            }
            tx.commit()?;

            pb.inc(batch.len() as u64);
        }

        pb.finish_with_message("Embedding cache built");

        let total_chunks: i64 = conn.query_row(
            "SELECT COUNT(*) FROM session_chunks", [], |row| row.get(0),
        )?;
        tracing::info!("Cache complete: {} chunks from {} sessions", total_chunks, unique_sessions.len());

        Ok(Self { path })
    }

    /// Open an existing cache.
    pub fn open(dataset: &str, variant: &str, model_name: &str) -> Result<Self> {
        let path = Self::cache_path(dataset, variant, model_name);
        if !path.exists() {
            anyhow::bail!("Embedding cache not found: {}", path.display());
        }
        Ok(Self { path })
    }

    /// Load cached chunks for a set of session IDs.
    pub fn load_sessions(&self, session_ids: &[&str]) -> Result<Vec<CachedChunk>> {
        let conn = Connection::open(&self.path)?;
        let mut chunks = Vec::new();

        for session_id in session_ids {
            let mut stmt = conn.prepare(
                "SELECT session_id, chunk_index, chunk_text, embedding, session_date
                 FROM session_chunks WHERE session_id = ?1 ORDER BY chunk_index",
            )?;

            let rows = stmt.query_map([session_id], |row| {
                let blob: Vec<u8> = row.get(3)?;
                Ok(CachedChunk {
                    session_id: row.get(0)?,
                    chunk_index: row.get(1)?,
                    text: row.get(2)?,
                    embedding: mindcore::embeddings::pooling::bytes_to_vec(&blob),
                    session_date: row.get(4)?,
                })
            })?;

            for row in rows {
                chunks.push(row?);
            }
        }

        Ok(chunks)
    }

    /// Path to the cache database.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_path_format() {
        let path = EmbeddingCache::cache_path("longmemeval", "small", "all-MiniLM-L6-v2");
        assert!(path.to_string_lossy().contains("longmemeval-small-all-MiniLM-L6-v2.db"));
    }

    #[test]
    fn cache_path_sanitizes_slashes() {
        let path = EmbeddingCache::cache_path("test", "v1", "sentence-transformers/all-MiniLM-L6-v2");
        assert!(path.to_string_lossy().contains("sentence-transformers-all-MiniLM-L6-v2"));
        assert!(!path.to_string_lossy().contains("//"));
    }
}
