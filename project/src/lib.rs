//! Project format for Split Office (.awb files).
//!
//! # Phase 2 — Transformation Engine (Spec §project)
//!
//! Defines the Analytical Workbench (.awb) project format for saving/loading
//! complete analytical sessions: dataset references, workflow, UI state.
//!
//! Never stores full datasets — only references to source files.

use core::DatasetId;
use serde::{Deserialize, Serialize};
use transform::TransformList;

/// An Analytical Workbench project.
///
/// Spec §Project Format: dataset references, workflow graph, profiles, UI state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Project metadata.
    pub meta: ProjectMeta,
    /// Reference to the source dataset.
    pub dataset: DatasetRef,
    /// The transformation pipeline.
    pub transforms: TransformList,
    /// UI state that survives across sessions.
    #[serde(default)]
    pub ui_state: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    /// Display name.
    pub name: String,
    /// Format version for forward compatibility.
    #[serde(default = "default_version")]
    pub version: String,
    /// ISO-8601 timestamp of last save.
    #[serde(default)]
    pub saved_at: String,
}

fn default_version() -> String {
    "0.1.0".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRef {
    pub id: DatasetId,
    /// Path to the source Parquet/CSV file (relative to project).
    pub path: String,
    /// Display name.
    pub name: String,
}

impl Project {
    pub fn new(name: impl Into<String>, dataset_path: impl Into<String>) -> Self {
        Self {
            meta: ProjectMeta {
                name: name.into(),
                version: default_version(),
                saved_at: String::new(),
            },
            dataset: DatasetRef {
                id: DatasetId(0),
                path: dataset_path.into(),
                name: String::new(),
            },
            transforms: TransformList::new(),
            ui_state: serde_json::Value::Null,
        }
    }

    /// Serialize to a .awb JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from a .awb JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_roundtrip() {
        let proj = Project::new("My Analysis", "data/sales.parquet");
        let json = proj.to_json().unwrap();
        let restored = Project::from_json(&json).unwrap();
        assert_eq!(restored.meta.name, "My Analysis");
        assert_eq!(restored.dataset.path, "data/sales.parquet");
        assert!(restored.transforms.is_empty());
    }
}
