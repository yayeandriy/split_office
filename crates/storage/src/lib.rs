//! Storage layer: load CSV / Parquet files into a `DatasetHandle`.
//!
//! CSV files are converted to Parquet on import and then treated as Parquet.
//! All persistent data lives as Parquet on disk.

pub mod loader;
pub mod schema_convert;

pub use loader::{DatasetHandle, load_csv, load_parquet};
