//! Femind native adapter — links directly to the femind crate.
//!
//! Enabled via the `femind-adapter` feature flag (on by default).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{NaiveDate, TimeZone, Utc};
use femind::context::ContextBudget;
use femind::embeddings::{ApiBackend, CandleNativeBackend, EmbeddingBackend, FallbackBackend};
use femind::engine::MemoryEngine;
use femind::memory::store::StoreResult;
use femind::traits::{MemoryMeta, MemoryRecord, MemoryType, ScoringStrategy};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::traits::MemorySystem;
use crate::types::{BenchmarkQuestion, ConversationSession, IngestStats, RetrievalResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMemory {
    pub id: Option<i64>,
    pub content: String,
    pub role: String,
    pub session_index: usize,
    pub turn_index: usize,
    pub session_date: String,
    pub created_at: chrono::DateTime<Utc>,
}

impl MemoryRecord for ConversationMemory {
    fn id(&self) -> Option<i64> { self.id }
    fn searchable_text(&self) -> String { self.content.clone() }
    fn memory_type(&self) -> MemoryType { MemoryType::Episodic }
    fn importance(&self) -> u8 { if self.role == "user" { 6 } else { 5 } }
    fn created_at(&self) -> chrono::DateTime<Utc> { self.created_at }
    fn category(&self) -> Option<&str> { Some(&self.role) }
    fn metadata(&self) -> HashMap<String, String> {
        let mut meta = HashMap::new();
        meta.insert("session_index".into(), self.session_index.to_string());
        meta.insert("turn_index".into(), self.turn_index.to_string());
        meta.insert("session_date".into(), self.session_date.clone());
        meta
    }
}

pub struct FemindAdapter {
    engine: Mutex<MemoryEngine<ConversationMemory>>,
    /// Accumulated records awaiting batch embedding. Flushed on retrieve_context().
    pending: Mutex<Vec<ConversationMemory>>,
    /// Reusable embedding backend — shared Arc avoids reloading model on reset().
    backend: std::sync::Arc<dyn EmbeddingBackend>,
    /// Assembly configuration (diversification, recency, etc.)
    assembly_config: femind::context::AssemblyConfig,
    /// Engine config toggles.
    engine_config: femind::engine::EngineConfig,
    /// Optional LLM callback for extraction-based ingest.
    /// When set, ingest_session() uses store_with_extraction() instead of chunking.
    llm: Option<Box<dyn femind::traits::LlmCallback>>,
    /// Human-readable system name.
    system_name: String,
    /// Ingest mode used for this adapter.
    ingest_mode: IngestMode,
    /// Optional stable tag for the extraction model, used in persistent cache paths.
    llm_cache_tag: Option<String>,
    /// Optional question-scoped persistent corpus cache configuration.
    persistent: Option<PersistentCorpusConfig>,
    /// Active persistent database path for the current prepared question corpus.
    active_database_path: Mutex<Option<PathBuf>>,
    /// Skip runner-driven live ingest after a prepared corpus is loaded.
    skip_live_ingest: AtomicBool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestMode {
    Chunk,
    Extract,
    Hybrid,
}

impl IngestMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Chunk => "chunk",
            Self::Extract => "extract",
            Self::Hybrid => "hybrid",
        }
    }
}

#[derive(Debug, Clone)]
struct PersistentCorpusConfig {
    cache_root: PathBuf,
    chunk_size: usize,
    min_turn_chars: usize,
}

#[derive(Debug)]
struct IdentityScorer;

impl ScoringStrategy for IdentityScorer {
    fn score_multiplier(&self, _record: &MemoryMeta, _query: &str, _base_score: f32) -> f32 {
        1.0
    }
}

fn benchmark_engine_config() -> femind::engine::EngineConfig {
    let mut config = femind::engine::EngineConfig::default();
    config.strict_grounding_enabled = false;
    config.query_alignment_enabled = false;
    config
}

impl FemindAdapter {
    /// Create with the default local CandleNativeBackend (all-MiniLM-L6-v2).
    pub fn new() -> Result<Self> {
        let backend: std::sync::Arc<dyn EmbeddingBackend> =
            std::sync::Arc::new(CandleNativeBackend::new()?);
        Self::with_backend(backend)
    }

    /// Create with DeepInfra API backend (all-MiniLM-L6-v2, fast remote embedding).
    pub fn with_deepinfra_api(api_key: &str) -> Result<Self> {
        let backend: std::sync::Arc<dyn EmbeddingBackend> =
            std::sync::Arc::new(ApiBackend::deepinfra_minilm(api_key));
        Self::with_backend(backend)
    }

    /// Create with API-first, local-fallback pattern.
    pub fn with_api_and_local_fallback(api_key: &str) -> Result<Self> {
        let api = Box::new(ApiBackend::deepinfra_minilm(api_key)) as Box<dyn EmbeddingBackend>;
        let local = Box::new(CandleNativeBackend::new()?) as Box<dyn EmbeddingBackend>;
        let backend: std::sync::Arc<dyn EmbeddingBackend> =
            std::sync::Arc::new(FallbackBackend::api_with_local_fallback(api, local));
        Self::with_backend(backend)
    }

    /// Create with a custom embedding backend (API, local, or fallback).
    pub fn with_backend(backend: std::sync::Arc<dyn EmbeddingBackend>) -> Result<Self> {
        Self::with_backend_config_named(backend, benchmark_engine_config(), "femind")
    }

    /// Create with a custom embedding backend and engine config.
    pub fn with_backend_config_named(
        backend: std::sync::Arc<dyn EmbeddingBackend>,
        engine_config: femind::engine::EngineConfig,
        system_name: impl Into<String>,
    ) -> Result<Self> {
        let engine = MemoryEngine::<ConversationMemory>::builder()
            .scoring(IdentityScorer)
            .config(engine_config.clone())
            .embedding_backend_arc(std::sync::Arc::clone(&backend))
            .build()?;
        Ok(Self {
            engine: Mutex::new(engine),
            pending: Mutex::new(Vec::new()),
            backend,
            assembly_config: femind::context::AssemblyConfig::default(),
            engine_config,
            llm: None,
            system_name: system_name.into(),
            ingest_mode: IngestMode::Chunk,
            llm_cache_tag: None,
            persistent: None,
            active_database_path: Mutex::new(None),
            skip_live_ingest: AtomicBool::new(false),
        })
    }

    /// Set the assembly configuration (diversification, recency, etc.)
    pub fn with_assembly_config(mut self, config: femind::context::AssemblyConfig) -> Self {
        self.assembly_config = config;
        self
    }

    /// Enable persistent question-scoped corpora under the default recallbench cache root.
    pub fn with_persistent_corpora(mut self) -> Self {
        let cache_root = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("recallbench")
            .join("femind-corpora");
        self.persistent = Some(PersistentCorpusConfig {
            cache_root,
            chunk_size: 1000,
            min_turn_chars: 10,
        });
        self
    }

    /// Switch ingest mode.
    pub fn with_ingest_mode(mut self, ingest_mode: IngestMode) -> Self {
        self.ingest_mode = ingest_mode;
        self
    }

    /// Get mutable access to the engine for config changes.
    pub fn engine_mut(&mut self) -> &mut MemoryEngine<ConversationMemory> {
        self.engine.get_mut().expect("engine lock poisoned")
    }

    /// Enable LLM extraction during ingest.
    /// When set, ingest_session() extracts individual facts via LLM
    /// instead of chunking raw text.
    pub fn with_llm(mut self, llm: Box<dyn femind::traits::LlmCallback>) -> Self {
        self.llm = Some(llm);
        self
    }

    /// Enable LLM extraction with a stable cache tag for persistent corpora.
    pub fn with_llm_tagged(
        mut self,
        llm: Box<dyn femind::traits::LlmCallback>,
        cache_tag: impl Into<String>,
    ) -> Self {
        self.llm = Some(llm);
        self.llm_cache_tag = Some(cache_tag.into());
        self
    }

    fn build_engine_for_path(&self, path: Option<&Path>) -> Result<MemoryEngine<ConversationMemory>> {
        let mut builder = MemoryEngine::<ConversationMemory>::builder()
            .scoring(IdentityScorer)
            .config(self.engine_config.clone())
            .embedding_backend_arc(std::sync::Arc::clone(&self.backend));
        if let Some(path) = path {
            builder = builder.database(path.to_string_lossy().to_string());
        }
        Ok(builder.build()?)
    }

    fn reset_engine_for_path(&self, path: Option<&Path>) -> Result<()> {
        let new_engine = self.build_engine_for_path(path)?;
        let mut guard = self.engine.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        *guard = new_engine;
        let mut pending = self.pending.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        pending.clear();
        Ok(())
    }

    fn persistent_db_path(
        &self,
        persistent: &PersistentCorpusConfig,
        dataset: &str,
        variant: &str,
        question: &BenchmarkQuestion,
    ) -> PathBuf {
        let fingerprint = benchmark_question_fingerprint(question);
        let embedding_tag = sanitize_path_component(self.backend.model_name());
        let llm_tag = sanitize_path_component(
            self.llm_cache_tag
                .as_deref()
                .unwrap_or("none"),
        );
        persistent.cache_root
            .join(format!(
                "{}-{}-{}-emb-{}-llm-{}-graph{}",
                dataset,
                variant,
                self.ingest_mode.as_str(),
                embedding_tag,
                llm_tag,
                self.assembly_config.graph_depth
            ))
            .join(format!("{fingerprint}.db"))
    }

    fn persistent_corpus_ready(path: &Path) -> Result<bool> {
        if !path.exists() {
            return Ok(false);
        }
        let conn = Connection::open(path)?;
        let ready: Option<String> = conn
            .query_row(
                "SELECT value FROM femind_meta WHERE key = ?1",
                params!["recallbench_ready"],
                |row| row.get(0),
            )
            .optional()?;
        Ok(matches!(ready.as_deref(), Some("1")))
    }

    fn set_persistent_corpus_ready(path: &Path, ready: bool) -> Result<()> {
        let conn = Connection::open(path)?;
        conn.execute(
            "INSERT INTO femind_meta(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params!["recallbench_ready", if ready { "1" } else { "0" }],
        )?;
        Ok(())
    }

    fn clear_partial_persistent_corpus(path: &Path) -> Result<()> {
        for suffix in ["", "-wal", "-shm"] {
            let sidecar = PathBuf::from(format!("{}{}", path.display(), suffix));
            if sidecar.exists() {
                std::fs::remove_file(sidecar)?;
            }
        }
        Ok(())
    }

    /// Load pre-computed chunks from the embedding cache.
    ///
    /// Inserts text into the memories table (triggering FTS5 indexing) and
    /// pre-computed vectors into memory_vectors — skips the embedding backend entirely.
    /// Load pre-computed chunks with their embeddings into the engine.
    ///
    /// Each chunk is a (text, embedding, session_date, chunk_index) tuple.
    /// Inserts text into memories (triggering FTS5), vectors into memory_vectors,
    /// bypassing the embedding backend entirely.
    pub fn load_precomputed(
        &self,
        chunks: &[(String, Vec<f32>, String, usize)],  // (text, embedding, session_date, chunk_index)
    ) -> Result<usize> {
        let engine = self.engine.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let db = engine.database();
        let model_name = self.backend.model_name();
        let mut stored = 0usize;

        let store = femind::memory::MemoryStore::<ConversationMemory>::new();

        for (text, embedding, session_date, chunk_index) in chunks {
            let record = ConversationMemory {
                id: None,
                content: text.clone(),
                role: "chunk".to_string(),
                session_index: 0,
                turn_index: *chunk_index,
                session_date: session_date.clone(),
                created_at: parse_session_date(session_date),
            };

            match store.store(db, &record) {
                Ok(StoreResult::Added(id)) => {
                    let hash = {
                        use sha2::Digest;
                        format!("{:x}", sha2::Sha256::digest(text.as_bytes()))
                    };
                    femind::search::VectorSearch::store_vector(
                        db, id, embedding, model_name, &hash,
                    )?;
                    // Update embedding status
                    db.with_writer(|conn| {
                        conn.execute(
                            "UPDATE memories SET embedding_status = 'success' WHERE id = ?1",
                            [id],
                        )?;
                        Ok(())
                    })?;
                    stored += 1;
                }
                Ok(StoreResult::Duplicate(_)) => {}
                Err(e) => tracing::warn!("Failed to store cached chunk: {e}"),
            }
        }

        Ok(stored)
    }

    /// Extract fact triples from stored memories and build graph edges.
    ///
    /// Scans all memories for structured fact patterns, detects conflicting facts,
    /// and creates SupersededBy edges between outdated and current facts.
    fn build_graph_from_facts(&self) -> Result<()> {
        use femind::ingest::fact_extraction;
        use femind::memory::{GraphMemory, RelationType};

        let engine = self.engine.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let db = engine.database();

        // Load all stored memory texts
        let memories: Vec<(i64, String)> = db.with_reader(|conn| {
            let mut stmt = conn.prepare("SELECT id, searchable_text FROM memories ORDER BY id")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
        })?;

        // Concatenate all memory texts and extract facts
        let all_text = memories.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>().join("\n");
        let (triples, conflicts) = fact_extraction::extract_facts(&all_text);

        if triples.is_empty() {
            return Ok(());
        }

        // Build a map from fact text → memory_id (find which memory contains each fact)
        let mut fact_to_memory: std::collections::HashMap<usize, i64> = std::collections::HashMap::new();
        for (mem_id, text) in &memories {
            for triple in &triples {
                if text.contains(&triple.original) {
                    fact_to_memory.insert(triple.index, *mem_id);
                }
            }
        }

        // Create SupersededBy edges for conflicting facts
        let mut edges_created = 0;
        for conflict in &conflicts {
            let old_mem = fact_to_memory.get(&conflict.old_fact.index);
            let new_mem = fact_to_memory.get(&conflict.new_fact.index);
            if let (Some(&old_id), Some(&new_id)) = (old_mem, new_mem) {
                if old_id != new_id {
                    // Old fact's memory is superseded by new fact's memory
                    let _ = GraphMemory::relate(db, old_id, new_id, &RelationType::SupersededBy);
                    edges_created += 1;
                }
            }
        }

        // NOTE: RelatedTo edges disabled — they add too much noise by connecting
        // all facts mentioning the same entity, flooding context with irrelevant facts.
        // Only SupersededBy edges are useful for conflict resolution.
        let related_edges = 0;

        tracing::info!(
            "Graph built: {} facts extracted, {} conflicts → {} superseded edges, {} related edges",
            triples.len(), conflicts.len(), edges_created, related_edges
        );

        Ok(())
    }

    /// Flush all pending records into the engine via a single store_batch().
    fn flush_pending(&self) -> Result<(usize, usize)> {
        let records: Vec<ConversationMemory> = {
            let mut pending = self.pending.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
            std::mem::take(&mut *pending)
        };

        if records.is_empty() {
            return Ok((0, 0));
        }

        tracing::info!("Flushing {} accumulated chunks for batch embedding", records.len());
        let engine = self.engine.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let results = engine.store_batch(&records)?;
        let stored = results.iter().filter(|r| matches!(r, StoreResult::Added(_))).count();
        let dupes = results.iter().filter(|r| matches!(r, StoreResult::Duplicate(_))).count();
        Ok((stored, dupes))
    }
}

#[async_trait]
impl MemorySystem for FemindAdapter {
    fn name(&self) -> &str { &self.system_name }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }

    fn supports_precomputed(&self) -> bool { true }

    fn load_precomputed(&self, chunks: &[(String, Vec<f32>, String, usize)]) -> Result<usize> {
        self.load_precomputed(chunks)
    }

    async fn reset(&self) -> Result<()> {
        let active_path = self
            .active_database_path
            .lock()
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .clone();
        self.reset_engine_for_path(active_path.as_deref())
    }

    async fn prepare_question(
        &self,
        dataset: &str,
        variant: &str,
        question: &BenchmarkQuestion,
    ) -> Result<()> {
        let Some(persistent) = &self.persistent else {
            self.skip_live_ingest.store(false, Ordering::Relaxed);
            return Ok(());
        };

        let db_path = self.persistent_db_path(persistent, dataset, variant, question);
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        {
            let mut active = self
                .active_database_path
                .lock()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            *active = Some(db_path.clone());
        }

        if Self::persistent_corpus_ready(&db_path)? {
            self.skip_live_ingest.store(true, Ordering::Relaxed);
            return Ok(());
        }

        if db_path.exists() {
            Self::clear_partial_persistent_corpus(&db_path)?;
        }

        self.skip_live_ingest.store(false, Ordering::Relaxed);
        self.reset_engine_for_path(Some(&db_path))?;
        Self::set_persistent_corpus_ready(&db_path, false)?;

        for session in &question.sessions {
            self.ingest_session(session).await?;
        }

        let (stored, _dupes) = self.flush_pending()?;
        if stored > 0 && self.assembly_config.graph_depth > 0 {
            self.build_graph_from_facts()?;
        }

        Self::set_persistent_corpus_ready(&db_path, true)?;
        self.skip_live_ingest.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn ingest_session(&self, session: &ConversationSession) -> Result<IngestStats> {
        if self.skip_live_ingest.load(Ordering::Relaxed) {
            return Ok(IngestStats {
                memories_stored: 0,
                duplicates_skipped: 0,
                duration_ms: 0,
            });
        }

        let start = std::time::Instant::now();
        let session_date = session.date.clone().unwrap_or_default();
        let session_created_at = parse_session_date(&session_date);

        let chunk_count = if matches!(self.ingest_mode, IngestMode::Chunk | IngestMode::Hybrid) {
            let turns_iter = session.turns.iter().map(|t| (t.role.as_str(), t.content.as_str()));
            let chunk_size = self
                .persistent
                .as_ref()
                .map(|p| p.chunk_size)
                .unwrap_or(1000);
            let min_turn_chars = self
                .persistent
                .as_ref()
                .map(|p| p.min_turn_chars)
                .unwrap_or(10);
            let chunks = femind::ingest::chunking::chunk_session(
                turns_iter,
                &session_date,
                chunk_size,
                min_turn_chars,
            );

            let records: Vec<ConversationMemory> = chunks.iter().enumerate()
                .map(|(idx, chunk)| ConversationMemory {
                    id: None,
                    content: chunk.text.clone(),
                    role: "chunk".to_string(),
                    session_index: 0,
                    turn_index: idx,
                    session_date: session_date.clone(),
                    created_at: session_created_at,
                })
                .collect();

            let count = records.len();
            let mut pending = self.pending.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
            pending.extend(records);
            count
        } else {
            0
        };

        let mut extracted_count = 0usize;
        let mut duplicate_count = 0usize;
        if matches!(self.ingest_mode, IngestMode::Extract | IngestMode::Hybrid) {
            if let Some(ref llm) = self.llm {
                let full_text: String = session.turns.iter()
                    .map(|t| format!("{}: {}", t.role, t.content))
                    .collect::<Vec<_>>()
                    .join("\n");

                let engine = self.engine.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
                let before_max_id = max_memory_id(&engine)?;
                let result = engine.store_with_extraction(&full_text, llm.as_ref())?;
                stamp_new_memories_created_at(&engine, before_max_id, session_created_at)?;
                extracted_count = result.memories_stored;
                duplicate_count = result.duplicates_skipped;
            }
        }

        Ok(IngestStats {
            memories_stored: chunk_count + extracted_count,
            duplicates_skipped: duplicate_count,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn retrieve_context(&self, query: &str, _query_date: Option<&str>, token_budget: usize) -> Result<RetrievalResult> {
        // Flush all accumulated chunks in a single batch (one embed_batch() call)
        let (stored, _dupes) = self.flush_pending()?;
        if stored > 0 {
            tracing::info!("Batch embedded {stored} chunks");

            // If graph expansion is enabled, extract facts and build graph edges
            if self.assembly_config.graph_depth > 0 {
                self.build_graph_from_facts()?;
            }
        }

        let start = std::time::Instant::now();
        let engine = self.engine.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let budget = ContextBudget::new(token_budget);
        let assembly = engine.assemble_context_with_config(query, &budget, &self.assembly_config)?;
        let context: String = assembly.items.iter().map(|item| item.content.as_str()).collect::<Vec<_>>().join("\n");

        Ok(RetrievalResult {
            context,
            items_retrieved: assembly.items.len(),
            tokens_used: assembly.total_tokens,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

fn parse_session_date(session_date: &str) -> chrono::DateTime<Utc> {
    if session_date.is_empty() {
        return Utc::now();
    }

    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(session_date) {
        return parsed.with_timezone(&Utc);
    }

    if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(session_date, "%Y/%m/%d (%a) %H:%M") {
        return Utc.from_utc_datetime(&parsed);
    }

    if let Ok(date) = NaiveDate::parse_from_str(session_date, "%Y-%m-%d") {
        if let Some(dt) = date.and_hms_opt(0, 0, 0) {
            return Utc.from_utc_datetime(&dt);
        }
    }

    tracing::warn!("Failed to parse session date '{session_date}', falling back to current time");
    Utc::now()
}

fn sanitize_path_component(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            _ => '-',
        })
        .collect()
}

fn benchmark_question_fingerprint(question: &BenchmarkQuestion) -> String {
    use sha2::Digest;

    let mut hasher = sha2::Sha256::new();
    hasher.update(question.id.as_bytes());
    hasher.update([0]);
    for session in &question.sessions {
        hasher.update(session.id.as_bytes());
        hasher.update([0]);
        if let Some(date) = &session.date {
            hasher.update(date.as_bytes());
        }
        hasher.update([0]);
        for turn in &session.turns {
            hasher.update(turn.role.as_bytes());
            hasher.update([0]);
            hasher.update(turn.content.as_bytes());
            hasher.update([0]);
        }
    }
    format!("{:x}", hasher.finalize())
}

fn max_memory_id(engine: &MemoryEngine<ConversationMemory>) -> Result<i64> {
    Ok(engine.database().with_reader(|conn| {
        let max_id = conn.query_row("SELECT COALESCE(MAX(id), 0) FROM memories", [], |row| {
            row.get::<_, i64>(0)
        })?;
        Ok(max_id)
    })?)
}

fn stamp_new_memories_created_at(
    engine: &MemoryEngine<ConversationMemory>,
    before_max_id: i64,
    created_at: chrono::DateTime<Utc>,
) -> Result<()> {
    let created_at = created_at.to_rfc3339();
    Ok(engine.database().with_writer(|conn| {
        conn.execute(
            "UPDATE memories
             SET created_at = ?1,
                 updated_at = CASE
                     WHEN updated_at < ?1 THEN ?1
                     ELSE updated_at
                 END
             WHERE id > ?2",
            rusqlite::params![created_at, before_max_id],
        )?;
        Ok(())
    })?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use femind::embeddings::NoopBackend;
    use crate::types::Turn;

    #[tokio::test]
    async fn ingest_and_retrieve() {
        let adapter = FemindAdapter::new().unwrap();
        let session = ConversationSession {
            id: "s1".to_string(), date: Some("2024-01-15".to_string()),
            turns: vec![
                Turn { role: "user".to_string(), content: "My favorite color is blue and I really enjoy painting landscapes".to_string() },
                Turn { role: "assistant".to_string(), content: "That sounds wonderful! Blue is a calming color for landscapes.".to_string() },
            ],
        };
        let stats = adapter.ingest_session(&session).await.unwrap();
        assert!(stats.memories_stored >= 1, "should store at least 1 chunk");
        // Retrieval triggers flush + embedding
        let result = adapter.retrieve_context("favorite color", None, 16384).await.unwrap();
        assert!(result.context.contains("blue"), "should find 'blue' in context");
    }

    #[tokio::test]
    async fn reset_clears() {
        let adapter = FemindAdapter::new().unwrap();
        let session = ConversationSession {
            id: "s1".to_string(), date: None,
            turns: vec![Turn { role: "user".to_string(), content: "Hello world, this is a test message with enough content".to_string() }],
        };
        adapter.ingest_session(&session).await.unwrap();
        adapter.reset().await.unwrap();
        let result = adapter.retrieve_context("hello", None, 16384).await.unwrap();
        assert_eq!(result.items_retrieved, 0);
    }

    #[test]
    fn parse_session_date_accepts_plain_dates() {
        let parsed = parse_session_date("2024-01-15");
        assert_eq!(parsed, Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap());
    }

    #[test]
    fn parse_session_date_accepts_longmemeval_timestamps() {
        let parsed = parse_session_date("2023/07/12 (Wed) 10:06");
        assert_eq!(parsed, Utc.with_ymd_and_hms(2023, 7, 12, 10, 6, 0).unwrap());
    }

    #[tokio::test]
    async fn persistent_question_corpus_is_reused_without_live_ingest() {
        let tempdir = tempfile::tempdir().unwrap();
        let mut adapter = FemindAdapter::with_backend_config_named(
            Arc::new(NoopBackend::new(16)),
            femind::engine::EngineConfig::default(),
            "femind-enhanced",
        )
        .unwrap()
        .with_ingest_mode(IngestMode::Chunk);
        adapter.persistent = Some(PersistentCorpusConfig {
            cache_root: tempdir.path().to_path_buf(),
            chunk_size: 1000,
            min_turn_chars: 10,
        });

        let question = BenchmarkQuestion {
            id: "q-persist".to_string(),
            question_type: "multi-session".to_string(),
            question: "What color did the user prefer?".to_string(),
            ground_truth: vec!["blue".to_string()],
            question_date: Some("2024-01-15".to_string()),
            sessions: vec![ConversationSession {
                id: "s1".to_string(),
                date: Some("2024-01-15".to_string()),
                turns: vec![
                    Turn { role: "user".to_string(), content: "My favorite color is blue and I enjoy painting landscapes.".to_string() },
                    Turn { role: "assistant".to_string(), content: "Blue works well for landscape palettes.".to_string() },
                ],
            }],
            is_abstention: false,
            metadata: HashMap::new(),
        };

        adapter
            .prepare_question("longmemeval", "small", &question)
            .await
            .unwrap();

        let active_path = adapter
            .active_database_path
            .lock()
            .unwrap()
            .clone()
            .unwrap();
        assert!(active_path.exists(), "prepared corpus database should exist");

        let memory_count_1: i64 = {
            let conn = rusqlite::Connection::open(&active_path).unwrap();
            conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0)).unwrap()
        };
        assert!(memory_count_1 > 0, "prepared corpus should contain stored memories");

        adapter.reset().await.unwrap();
        let ingest = adapter
            .ingest_session(&question.sessions[0])
            .await
            .unwrap();
        assert_eq!(ingest.memories_stored, 0, "prepared corpora should skip live ingest");

        adapter
            .prepare_question("longmemeval", "small", &question)
            .await
            .unwrap();
        let memory_count_2: i64 = {
            let conn = rusqlite::Connection::open(&active_path).unwrap();
            conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0)).unwrap()
        };
        assert_eq!(memory_count_1, memory_count_2, "reused corpus should not duplicate stored memories");
    }

    #[tokio::test]
    async fn partial_persistent_corpus_is_rebuilt() {
        let tempdir = tempfile::tempdir().unwrap();
        let mut adapter = FemindAdapter::with_backend_config_named(
            Arc::new(NoopBackend::new(16)),
            femind::engine::EngineConfig::default(),
            "femind-enhanced",
        )
        .unwrap()
        .with_ingest_mode(IngestMode::Chunk);
        adapter.persistent = Some(PersistentCorpusConfig {
            cache_root: tempdir.path().to_path_buf(),
            chunk_size: 1000,
            min_turn_chars: 10,
        });

        let question = BenchmarkQuestion {
            id: "q-partial".to_string(),
            question_type: "multi-session".to_string(),
            question: "What color did the user prefer?".to_string(),
            ground_truth: vec!["blue".to_string()],
            question_date: Some("2024-01-15".to_string()),
            sessions: vec![ConversationSession {
                id: "s1".to_string(),
                date: Some("2024-01-15".to_string()),
                turns: vec![
                    Turn { role: "user".to_string(), content: "My favorite color is blue and I enjoy painting landscapes.".to_string() },
                    Turn { role: "assistant".to_string(), content: "Blue works well for landscape palettes.".to_string() },
                ],
            }],
            is_abstention: false,
            metadata: HashMap::new(),
        };

        let db_path = adapter.persistent_db_path(adapter.persistent.as_ref().unwrap(), "longmemeval", "small", &question);
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute("CREATE TABLE femind_meta(key TEXT PRIMARY KEY, value TEXT NOT NULL)", []).unwrap();
        conn.execute("INSERT INTO femind_meta(key, value) VALUES('schema_version', '1')", []).unwrap();
        drop(conn);

        adapter
            .prepare_question("longmemeval", "small", &question)
            .await
            .unwrap();

        assert!(FemindAdapter::persistent_corpus_ready(&db_path).unwrap());
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let memory_count: i64 = conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0)).unwrap();
        assert!(memory_count > 0, "rebuilt corpus should contain memories");
    }
}
