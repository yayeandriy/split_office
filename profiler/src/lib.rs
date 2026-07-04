//! Data profiling and analysis for Split Office.
//!
//! # Architecture
//!
//! ```text
//! Polars DataFrame
//!   ↓
//! profile_dataframe()
//!   ├── stats.rs         → per-column statistics + quantiles
//!   ├── distribution.rs  → histogram summaries
//!   ├── outliers.rs      → IQR outlier detection
//!   ├── correlation.rs   → Pearson correlation matrix
//!   ├── relationships.rs → FK/hierarchy detection
//!   └── profile.rs       → orchestrator + quality assessment
//!   ↓
//! DatasetProfile
//!   ├── ColumnProfile[]
//!   ├── CorrelationMatrix
//!   ├── Relationship[]
//!   └── DatasetQuality
//! ```
//!
//! All analysis is synchronous; background execution is the caller's responsibility.

pub mod cardinality;
pub mod correlation;
pub mod distribution;
pub mod inference;
pub mod nulls;
pub mod outliers;
pub mod profile;
pub mod relationships;
pub mod stats;
pub mod types;

pub use profile::profile_dataframe;
pub use types::*;

/// Load a Parquet file with Polars and run a full dataset profile.
///
/// This is the **primary entry point** for the Application layer.
/// It encapsulates all Polars IO so that no other crate (especially `app`)
/// needs to touch `polars::prelude::LazyFrame` directly.
///
/// # Errors
/// Returns an error if the file cannot be opened, parsed, or profiled.
pub fn profile_from_parquet(
    path: &str,
    dataset_name: &str,
) -> anyhow::Result<DatasetProfile> {
    use polars::prelude::{LazyFrame, ScanArgsParquet};

    let lf = LazyFrame::scan_parquet(path, ScanArgsParquet::default())
        .map_err(|e| anyhow::anyhow!("Polars scan: {e}"))?;

    let df = lf
        .collect()
        .map_err(|e| anyhow::anyhow!("Polars collect: {e}"))?;

    profile_dataframe(&df, dataset_name)
        .map_err(|e| anyhow::anyhow!("Profiling: {e}"))
}

use core::DataType;

/// Convert a Polars DataType to core::DataType.
pub fn arrow_to_core_dtype(dt: &polars::prelude::DataType) -> DataType {
    use polars::prelude::DataType as P;
    match dt {
        P::Boolean => DataType::Boolean,
        P::Int8 => DataType::Int8,
        P::Int16 => DataType::Int16,
        P::Int32 => DataType::Int32,
        P::Int64 => DataType::Int64,
        P::UInt8 => DataType::UInt8,
        P::UInt16 => DataType::UInt16,
        P::UInt32 => DataType::UInt32,
        P::UInt64 => DataType::UInt64,
        P::Float32 => DataType::Float32,
        P::Float64 => DataType::Float64,
        P::String => DataType::Utf8,
        P::Date => DataType::Date32,
        P::Datetime(_, _) => DataType::TimestampMillis,
        _ => DataType::Other(format!("{dt:?}")),
    }
}
