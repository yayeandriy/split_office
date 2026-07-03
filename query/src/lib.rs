//! Query engine: DuckDB-backed SQL execution over Parquet files.
//!
//! # Architecture
//!
//! ```text
//! Parquet file
//!   ↓ parquet_scan(...)
//! DuckDB in-process
//!   ↓ SELECT ... WHERE ... ORDER BY ... LIMIT ? OFFSET ?
//! Arrow RecordBatch
//!   ↓
//! Grid renderer
//! ```
//!
//! One `QueryEngine` per open dataset — each owns a DuckDB connection.
//! All public methods are synchronous; threading is the caller's responsibility.

pub mod engine;
pub mod stats;

pub use engine::{QueryEngine, QueryParams};
pub use stats::{ColumnStats, DatasetStats};
