use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use tracing::{info, instrument};

use core::{Dataset, DatasetId};
use crate::schema_convert::polars_schema_to_core;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> DatasetId {
    DatasetId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
}

/// A fully loaded dataset handle, ready for querying.
///
/// The Parquet file at `parquet_path` is the single source of truth.
/// All queries go through DuckDB's `parquet_scan()`.
#[derive(Debug, Clone)]
pub struct DatasetHandle {
    pub dataset: Dataset,
    /// Absolute path to the Parquet file DuckDB will query.
    pub parquet_path: PathBuf,
}

/// Load a Parquet file directly.
#[instrument(skip_all, fields(path = %path.as_ref().display()))]
pub fn load_parquet(path: impl AsRef<Path>) -> Result<DatasetHandle> {
    let path = path.as_ref().to_path_buf();
    let abs = path.canonicalize().context("cannot canonicalize parquet path")?;

    let name = abs
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("dataset")
        .to_string();

    // Use Polars lazy scan to get schema + row count cheaply.
    let mut lf = polars::prelude::LazyFrame::scan_parquet(
        abs.to_str().context("non-UTF-8 path")?,
        polars::prelude::ScanArgsParquet::default(),
    )
    .context("failed to open parquet file")?;

    let schema = lf.collect_schema().context("failed to read parquet schema")?;
    let core_schema = polars_schema_to_core(&schema);

    // Row count: collect only row count (cheap with parquet metadata).
    let row_count = count_rows_parquet(&abs).context("failed to count rows")?;

    info!(name = %name, rows = row_count, cols = core_schema.column_count(), "loaded parquet");

    let dataset = Dataset::new(next_id(), name, core_schema, row_count, abs.clone());
    Ok(DatasetHandle {
        dataset,
        parquet_path: abs,
    })
}

/// Load a CSV file: infer schema, write to a temp Parquet, return handle.
///
/// Auto-detects separator by sniffing the first line — supports `,`, `;`, `\t`, `|`.
#[instrument(skip_all, fields(path = %path.as_ref().display()))]
pub fn load_csv(path: impl AsRef<Path>) -> Result<DatasetHandle> {
    use polars::prelude::*;

    let path = path.as_ref().to_path_buf();
    let abs = path.canonicalize().context("cannot canonicalize csv path")?;

    let name = abs
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("dataset")
        .to_string();

    // ── Sniff separator from the first line ───────────────────────────────
    let sep = detect_separator(&abs).context("failed to detect CSV separator")?;
    let sep_char = char::from(sep).to_string();
    info!(name = %name, sep = %sep_char, "detected CSV separator");

    // ── Read CSV into a DataFrame ─────────────────────────────────────────
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .with_parse_options(
            CsvParseOptions::default()
                .with_separator(sep)
                .with_quote_char(Some(b'"'))
                // Ignore rows that can't be parsed rather than hard-failing.
                .with_truncate_ragged_lines(true),
        )
        // Infer schema from more rows to handle mixed-type columns safely.
        .with_infer_schema_length(Some(1000))
        .with_ignore_errors(true)
        .try_into_reader_with_file_path(Some(abs.clone()))?
        .finish()
        .context("failed to parse CSV")?;

    // ── Write to Parquet alongside the CSV ───────────────────────────────
    let parquet_path = abs.with_extension("parquet");
    let mut file = std::fs::File::create(&parquet_path)
        .context("failed to create parquet output")?;
    ParquetWriter::new(&mut file)
        .finish(&mut df.clone())
        .context("failed to write parquet")?;

    info!(name = %name, rows = df.height(), cols = df.width(), sep = %sep_char, "converted csv → parquet");

    let schema = polars_schema_to_core(df.schema());
    let row_count = df.height();

    let dataset = Dataset::new(next_id(), name, schema, row_count, parquet_path.clone());
    Ok(DatasetHandle {
        dataset,
        parquet_path,
    })
}

/// Sniff the separator from the first non-empty line of the file.
///
/// Counts occurrences of `;`, `\t`, `|`, `,` in the header and returns
/// whichever appears most often. Falls back to `,`.
fn detect_separator(path: &Path) -> Result<u8> {
    use std::io::{BufRead, BufReader};

    let file = std::fs::File::open(path).context("cannot open file for separator detection")?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line).context("cannot read first line")?;

    let candidates: &[(u8, &str)] = &[
        (b';', ";"),
        (b'\t', "\t"),
        (b'|', "|"),
        (b',', ","),
    ];

    let best = candidates
        .iter()
        .max_by_key(|(_, s)| line.matches(s).count())
        .map(|(sep, _)| *sep)
        .unwrap_or(b',');

    Ok(best)
}


/// Count rows in a Parquet file using DuckDB (reads metadata, not data).
fn count_rows_parquet(path: &Path) -> Result<usize> {
    let conn = duckdb::Connection::open_in_memory()
        .context("failed to open DuckDB for row count")?;
    let count: i64 = conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM parquet_scan('{}')",
                path.to_str().context("non-UTF-8 path")?
            ),
            [],
            |row| row.get(0),
        )
        .context("DuckDB count query failed")?;
    Ok(count as usize)
}
