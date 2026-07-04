//! Column lineage tracking for Split Office.
//!
//! # Phase 2 — Transformation Engine (Spec §lineage)
//!
//! Tracks the provenance of every derived column through the workflow.
//! Used for impact analysis: "If I delete column X, what breaks?"
//!
//! # Architecture
//!
//! ```text
//! Profit ──► depends on ──► Revenue
//!                          Cost
//! ```
//!
//! Every derived artifact tracks its origin columns.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A lineage graph mapping derived columns to their source columns.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LineageGraph {
    /// column → set of columns it depends on
    edges: HashMap<String, HashSet<String>>,
}

impl LineageGraph {
    pub fn new() -> Self {
        Self { edges: HashMap::new() }
    }

    /// Record that `derived` depends on `source`.
    pub fn add_dependency(&mut self, derived: impl Into<String>, source: impl Into<String>) {
        self.edges
            .entry(derived.into())
            .or_default()
            .insert(source.into());
    }

    /// Record multiple dependencies at once.
    pub fn add_dependencies(
        &mut self,
        derived: impl Into<String>,
        sources: impl IntoIterator<Item = impl Into<String>>,
    ) {
        let entry = self.edges.entry(derived.into()).or_default();
        for s in sources {
            entry.insert(s.into());
        }
    }

    /// Get all direct dependencies of a column.
    pub fn dependencies_of(&self, column: &str) -> Vec<&String> {
        self.edges
            .get(column)
            .map(|s| s.iter().collect())
            .unwrap_or_default()
    }

    /// Impact analysis: if `column` changes, which other columns are affected?
    /// Returns all transitive dependents.
    pub fn impacted_by(&self, column: &str) -> Vec<String> {
        let mut result = Vec::new();
        for (derived, sources) in &self.edges {
            if sources.contains(column) || self.is_transitively_dependent(derived, column) {
                result.push(derived.clone());
            }
        }
        result.sort();
        result.dedup();
        result
    }

    fn is_transitively_dependent(&self, derived: &str, target: &str) -> bool {
        if let Some(sources) = self.edges.get(derived) {
            for source in sources {
                if source == target {
                    return true;
                }
                if self.is_transitively_dependent(source, target) {
                    return true;
                }
            }
        }
        false
    }

    /// All columns that have lineage information.
    pub fn all_derived_columns(&self) -> Vec<&String> {
        let mut cols: Vec<&String> = self.edges.keys().collect();
        cols.sort();
        cols
    }

    pub fn len(&self) -> usize {
        self.edges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_dependency() {
        let mut g = LineageGraph::new();
        g.add_dependency("profit", "revenue");
        g.add_dependency("profit", "cost");

        let deps = g.dependencies_of("profit");
        assert!(deps.contains(&&"revenue".to_string()));
        assert!(deps.contains(&&"cost".to_string()));
    }

    #[test]
    fn test_impact_analysis() {
        let mut g = LineageGraph::new();
        // profit = revenue - cost
        g.add_dependency("profit", "revenue");
        g.add_dependency("profit", "cost");
        // margin = profit / revenue
        g.add_dependency("margin", "profit");
        g.add_dependency("margin", "revenue");

        let impacted = g.impacted_by("cost");
        // cost affects profit, and transitively margin
        assert!(impacted.contains(&"profit".to_string()));
        assert!(impacted.contains(&"margin".to_string()));

        let impacted_rev = g.impacted_by("revenue");
        assert!(impacted_rev.contains(&"profit".to_string()));
        assert!(impacted_rev.contains(&"margin".to_string()));
    }

    #[test]
    fn test_no_dependency() {
        let g = LineageGraph::new();
        assert!(g.impacted_by("nothing").is_empty());
    }
}
