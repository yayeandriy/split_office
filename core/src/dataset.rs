use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::types::DataType;

/// Opaque handle for a loaded dataset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatasetId(pub u64);

impl std::fmt::Display for DatasetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dataset({})", self.0)
    }
}

/// A single column in a dataset schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub dtype: DataType,
    pub nullable: bool,
}

impl Column {
    pub fn new(name: impl Into<String>, dtype: DataType) -> Self {
        Self {
            name: name.into(),
            dtype,
            nullable: true,
        }
    }
}

/// The schema of a dataset — ordered list of columns.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Schema {
    pub columns: Vec<Column>,
}

impl Schema {
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn column_names(&self) -> Vec<&str> {
        self.columns.iter().map(|c| c.name.as_str()).collect()
    }
}

/// A loaded dataset handle.
///
/// The actual data lives on disk (Parquet). This is the metadata handle
/// that drives queries, rendering, and statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub id: DatasetId,
    pub name: String,
    pub schema: Schema,
    pub row_count: usize,
    /// Absolute path to the source Parquet file.
    pub source_path: PathBuf,
}

impl Dataset {
    pub fn new(
        id: DatasetId,
        name: impl Into<String>,
        schema: Schema,
        row_count: usize,
        source_path: PathBuf,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            schema,
            row_count,
            source_path,
        }
    }
}
