//! Raw session cache — stores dataset sessions without chunking or embedding.
//!
//! Built automatically when a dataset is first loaded. Stores raw session text,
//! turns, dates, and session IDs in a lightweight SQLite database. Serves as the
//! foundation for embedding caches at any chunk size.
//!
//! Cache location: ~/.cache/recallbench/sessions/{dataset}-{variant}.db

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::types::{BenchmarkQuestion, ConversationSession, Turn};

/// Raw session cache backed by SQLite.
pub struct SessionCache {
    path: PathBuf,
}

impl SessionCache {
    fn cache_dir() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("recallbench")
            .join("sessions")
    }

    /// Path to the cache file for a dataset/variant.
    pub fn cache_path(dataset: &str, variant: &str) -> PathBuf {
        Self::cache_dir().join(format!("{dataset}-{variant}.db"))
    }

    /// Check if a session cache exists.
    pub fn exists(dataset: &str, variant: &str) -> bool {
        Self::cache_path(dataset, variant).exists()
    }

    /// Build the session cache from a loaded dataset.
    ///
    /// Extracts all unique sessions and stores raw turns.
    /// Fast — no embedding or chunking, just text storage.
    pub fn build(
        dataset: &str,
        variant: &str,
        questions: &[BenchmarkQuestion],
    ) -> Result<Self> {
        let path = Self::cache_path(dataset, variant);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        tracing::info!("Building session cache at {}", path.display());

        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cache_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                session_date TEXT NOT NULL DEFAULT '',
                turn_count INTEGER NOT NULL DEFAULT 0,
                total_chars INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS turns (
                session_id TEXT NOT NULL,
                turn_index INTEGER NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                PRIMARY KEY (session_id, turn_index),
                FOREIGN KEY (session_id) REFERENCES sessions(session_id)
            );
            CREATE TABLE IF NOT EXISTS question_sessions (
                question_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                is_answer_session INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (question_id, session_id)
            );
            CREATE INDEX IF NOT EXISTS idx_turns_session ON turns(session_id);
            CREATE INDEX IF NOT EXISTS idx_qs_question ON question_sessions(question_id);
            CREATE INDEX IF NOT EXISTS idx_qs_session ON question_sessions(session_id);",
        )?;

        // Store metadata
        conn.execute(
            "INSERT OR REPLACE INTO cache_meta (key, value) VALUES ('dataset', ?1)",
            [dataset],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO cache_meta (key, value) VALUES ('variant', ?1)",
            [variant],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO cache_meta (key, value) VALUES ('created_at', ?1)",
            [chrono::Utc::now().to_rfc3339()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO cache_meta (key, value) VALUES ('total_questions', ?1)",
            [questions.len().to_string()],
        )?;

        // Collect unique sessions and question-session mappings
        let mut unique_sessions: std::collections::HashMap<String, &ConversationSession> =
            std::collections::HashMap::new();

        let tx = conn.unchecked_transaction()?;

        for q in questions {
            let answer_ids: std::collections::HashSet<String> = q.metadata
                .get("answer_session_ids")
                .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
                .unwrap_or_default()
                .into_iter()
                .collect();

            for session in &q.sessions {
                unique_sessions.entry(session.id.clone()).or_insert(session);

                let is_answer = if answer_ids.contains(&session.id) { 1 } else { 0 };
                tx.execute(
                    "INSERT OR IGNORE INTO question_sessions (question_id, session_id, is_answer_session)
                     VALUES (?1, ?2, ?3)",
                    params![q.id, session.id, is_answer],
                )?;
            }
        }

        // Store sessions and turns
        for (session_id, session) in &unique_sessions {
            let total_chars: usize = session.turns.iter().map(|t| t.content.len()).sum();
            let session_date = session.date.clone().unwrap_or_default();

            tx.execute(
                "INSERT OR IGNORE INTO sessions (session_id, session_date, turn_count, total_chars)
                 VALUES (?1, ?2, ?3, ?4)",
                params![session_id, session_date, session.turns.len(), total_chars],
            )?;

            for (idx, turn) in session.turns.iter().enumerate() {
                tx.execute(
                    "INSERT OR IGNORE INTO turns (session_id, turn_index, role, content)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![session_id, idx, turn.role, turn.content],
                )?;
            }
        }

        tx.commit()?;

        let session_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions", [], |row| row.get(0),
        )?;
        let turn_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM turns", [], |row| row.get(0),
        )?;

        conn.execute(
            "INSERT OR REPLACE INTO cache_meta (key, value) VALUES ('total_sessions', ?1)",
            [session_count.to_string()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO cache_meta (key, value) VALUES ('total_turns', ?1)",
            [turn_count.to_string()],
        )?;

        tracing::info!(
            "Session cache complete: {} sessions, {} turns from {} questions",
            session_count, turn_count, questions.len()
        );

        Ok(Self { path })
    }

    /// Open an existing session cache.
    pub fn open(dataset: &str, variant: &str) -> Result<Self> {
        let path = Self::cache_path(dataset, variant);
        if !path.exists() {
            anyhow::bail!("Session cache not found: {}", path.display());
        }
        Ok(Self { path })
    }

    /// Get cache statistics.
    pub fn stats(&self) -> Result<CacheStats> {
        let conn = Connection::open(&self.path)?;

        let get_meta = |key: &str| -> String {
            conn.query_row(
                "SELECT value FROM cache_meta WHERE key = ?1",
                [key],
                |row| row.get(0),
            ).unwrap_or_default()
        };

        Ok(CacheStats {
            dataset: get_meta("dataset"),
            variant: get_meta("variant"),
            total_questions: get_meta("total_questions").parse().unwrap_or(0),
            total_sessions: get_meta("total_sessions").parse().unwrap_or(0),
            total_turns: get_meta("total_turns").parse().unwrap_or(0),
            created_at: get_meta("created_at"),
        })
    }

    /// Load sessions for a list of session IDs.
    pub fn load_sessions(&self, session_ids: &[&str]) -> Result<Vec<ConversationSession>> {
        let conn = Connection::open(&self.path)?;
        let mut sessions = Vec::new();

        for sid in session_ids {
            let date: Option<String> = conn.query_row(
                "SELECT session_date FROM sessions WHERE session_id = ?1",
                [sid],
                |row| row.get(0),
            ).ok();

            let mut stmt = conn.prepare(
                "SELECT role, content FROM turns WHERE session_id = ?1 ORDER BY turn_index",
            )?;
            let turns: Vec<Turn> = stmt.query_map([sid], |row| {
                Ok(Turn {
                    role: row.get(0)?,
                    content: row.get(1)?,
                })
            })?.filter_map(|r| r.ok()).collect();

            if !turns.is_empty() {
                sessions.push(ConversationSession {
                    id: sid.to_string(),
                    date: date.filter(|d| !d.is_empty()),
                    turns,
                });
            }
        }

        Ok(sessions)
    }

    /// Path to the cache database.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Statistics about a session cache.
#[derive(Debug)]
pub struct CacheStats {
    pub dataset: String,
    pub variant: String,
    pub total_questions: usize,
    pub total_sessions: usize,
    pub total_turns: usize,
    pub created_at: String,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}: {} questions, {} sessions, {} turns ({})",
            self.dataset, self.variant, self.total_questions,
            self.total_sessions, self.total_turns, self.created_at)
    }
}
