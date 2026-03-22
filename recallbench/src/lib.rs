#![allow(dead_code)]

//! RecallBench — A universal benchmark harness for AI memory systems.
//!
//! This library exposes the core traits and types needed to implement
//! memory system adapters for benchmarking.
//!
//! # Implementing a Memory System
//!
//! ```rust,ignore
//! use recallbench::traits::MemorySystem;
//! use recallbench::types::{ConversationSession, IngestStats, RetrievalResult};
//!
//! struct MySystem;
//!
//! #[async_trait::async_trait]
//! impl MemorySystem for MySystem {
//!     fn name(&self) -> &str { "my-system" }
//!     fn version(&self) -> &str { "1.0.0" }
//!     async fn reset(&self) -> anyhow::Result<()> { Ok(()) }
//!     async fn ingest_session(&self, session: &ConversationSession) -> anyhow::Result<IngestStats> {
//!         Ok(IngestStats::default())
//!     }
//!     async fn retrieve_context(&self, query: &str, date: Option<&str>, budget: usize) -> anyhow::Result<RetrievalResult> {
//!         Ok(RetrievalResult { context: String::new(), items_retrieved: 0, tokens_used: 0, duration_ms: 0 })
//!     }
//! }
//! ```

pub mod checkpoint;
pub mod config;
pub mod datasets;
// embedding_cache is used by the binary (main.rs), not the library
// pub mod embedding_cache;
pub mod errors;
pub mod judge;
pub mod llm;
pub mod longevity;
pub mod metrics;
pub mod report;
pub mod resume;
pub mod runner;
pub mod sampling;
pub mod systems;
pub mod traits;
pub mod types;
pub mod verify;
pub mod web;
