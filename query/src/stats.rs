use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use storage::DatasetHandle;

/// Per-column statistics computed by Polars.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStats {
    pub name: String,
    pub null_count: usize,
    pub unique_count: Option<usize>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub mean: Option<f64>,
}

/// Statistics for an entire dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStats {
    pub columns: Vec<ColumnStats>,
}

/// Compute full dataset statistics using Polars (lazy scan).
#[instrument(skip(handle))]
pub fn compute_stats(handle: &DatasetHandle) -> Result<DatasetStats> {
    use polars::prelude::*;

    let path = handle
        .parquet_path
        .to_str()
        .context("non-UTF-8 parquet path")?;

    let df = LazyFrame::scan_parquet(path, ScanArgsParquet::default())
        .context("failed to scan parquet for stats")?
        .collect()
        .context("failed to collect dataframe for stats")?;

    let mut columns = Vec::new();

    for col_name in df.get_column_names() {
        // In Polars 0.46, df.column() returns &Column (a wrapper).
        // .as_series() gives us the underlying Series with all statistical methods.
        let col = df.column(col_name).context("column not found")?;
        let series = col.as_series().context("column has no series representation")?;

        let null_count = series.null_count();

        // Unique count — can be expensive; skip for very large datasets.
        let unique_count = if df.height() <= 5_000_000 {
            series.n_unique().ok()
        } else {
            None
        };

        // Min / max as strings via reduce.
        let min = series.min_reduce().ok().map(|s| format!("{}", s.value()));
        let max = series.max_reduce().ok().map(|s| format!("{}", s.value()));

        // Mean — only meaningful for numeric; returns None for non-numeric.
        let mean = series.mean();

        columns.push(ColumnStats {
            name: col_name.to_string(),
            null_count,
            unique_count,
            min,
            max,
            mean,
        });
    }

    Ok(DatasetStats { columns })
}
