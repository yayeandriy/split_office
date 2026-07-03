// use std::path::Path; // reserved for future use
use std::sync::Arc;

use anyhow::{Context, Result};
use arrow::array::RecordBatch;
use duckdb::Connection;
use parking_lot::Mutex;
use tracing::{debug, instrument};

use core::{FilterExpr, SortSpec, Viewport};
use storage::DatasetHandle;

/// Parameters for a single page fetch.
#[derive(Debug, Clone)]
pub struct QueryParams {
    pub filter: FilterExpr,
    pub sort: Vec<SortSpec>,
    pub viewport: Viewport,
}

impl QueryParams {
    pub fn new(viewport: Viewport) -> Self {
        Self {
            filter: FilterExpr::None,
            sort: Vec::new(),
            viewport,
        }
    }
}

/// DuckDB-backed query engine for a single Parquet dataset.
///
/// Thread-safe via internal `Arc<Mutex<Connection>>` — clone is cheap.
#[derive(Clone)]
pub struct QueryEngine {
    conn: Arc<Mutex<Connection>>,
    parquet_path: String,
    // table_alias reserved for future named-table support
    #[allow(dead_code)]
    table_alias: String,
}

impl QueryEngine {
    /// Open a query engine for the given dataset handle.
    pub fn open(handle: &DatasetHandle) -> Result<Self> {
        let parquet_path = handle
            .parquet_path
            .to_str()
            .context("non-UTF-8 parquet path")?
            .to_string();

        let conn = Connection::open_in_memory().context("failed to open DuckDB connection")?;

        // Verify the parquet file is readable immediately.
        conn.execute(
            &format!("SELECT COUNT(*) FROM parquet_scan('{}')", parquet_path),
            [],
        )
        .context("DuckDB could not open parquet file")?;

        let table_alias = handle
            .dataset
            .source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("data")
            .to_string();

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            parquet_path,
            table_alias,
        })
    }

    /// Fetch a page of Arrow data for the given params.
    ///
    /// Returns an Arrow `RecordBatch` containing exactly `viewport.visible_rows` rows
    /// (or fewer at the end of the dataset).
    #[instrument(skip(self, params), fields(
        first_row = params.viewport.first_row,
        visible_rows = params.viewport.visible_rows,
    ))]
    pub fn fetch_page(&self, params: &QueryParams) -> Result<RecordBatch> {
        let sql = self.build_page_query(params);
        debug!(sql = %sql, "fetch_page");

        let conn = self.conn.lock();
        let mut stmt = conn.prepare(&sql).context("failed to prepare query")?;
        let rbs: Vec<RecordBatch> = stmt
            .query_arrow([])
            .context("arrow query failed")?
            .collect();

        if rbs.is_empty() {
            // Return an empty record batch with the correct schema.
            let schema = Arc::new(arrow::datatypes::Schema::empty());
            return Ok(RecordBatch::new_empty(schema));
        }

        // Concatenate batches (DuckDB may return multiple small batches).
        if rbs.len() == 1 {
            Ok(rbs.into_iter().next().unwrap())
        } else {
            arrow::compute::concat_batches(&rbs[0].schema(), &rbs)
                .context("failed to concatenate arrow batches")
        }
    }

    /// Count total rows matching the given filter (for scrollbar sizing).
    #[instrument(skip(self, filter))]
    pub fn count_rows(&self, filter: &FilterExpr) -> Result<usize> {
        let where_clause = filter.to_sql();
        let sql = if where_clause.is_empty() {
            format!(
                "SELECT COUNT(*) AS n FROM parquet_scan('{}')",
                self.parquet_path
            )
        } else {
            format!(
                "SELECT COUNT(*) AS n FROM parquet_scan('{}') WHERE {}",
                self.parquet_path, where_clause
            )
        };

        debug!(sql = %sql, "count_rows");
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([])?;
        let row = rows.next()?.context("count query returned no rows")?;
        let count: i64 = row.get(0)?;
        Ok(count as usize)
    }

    // ---- private helpers ----

    fn build_page_query(&self, params: &QueryParams) -> String {
        let mut sql = format!("SELECT * FROM parquet_scan('{}')", self.parquet_path);

        let where_clause = params.filter.to_sql();
        if !where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", where_clause));
        }

        if !params.sort.is_empty() {
            let order_parts: Vec<String> = params.sort.iter().map(|s| s.to_sql()).collect();
            sql.push_str(&format!(" ORDER BY {}", order_parts.join(", ")));
        }

        sql.push_str(&format!(
            " LIMIT {} OFFSET {}",
            params.viewport.visible_rows, params.viewport.first_row
        ));

        sql
    }
}
