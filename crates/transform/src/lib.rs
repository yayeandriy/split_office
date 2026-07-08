//! Transformation definitions for Split Office.
//!
//! # Phase 2 — Transformation Engine (Spec §transform)
//!
//! Defines the transformations users can apply to datasets:
//! - Filter — row-level predicates
//! - Sort — ordering specifications
//! - DerivedColumn — computed columns via expressions
//! - Aggregate — group-by + aggregate functions
//! - Join — merge two datasets
//!
//! # Architecture
//!
//! Each transformation is a self-contained struct that knows how to:
//! 1. Validate itself against a schema
//! 2. Generate the SQL/execution plan
//! 3. Track which columns it adds/removes

use core::SortSpec;
use serde::{Deserialize, Serialize};

// ── Transformation ───────────────────────────────────────────────────────────

/// A single transform step in a workflow pipeline.
///
/// Spec §transform: transformation definitions with validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transform {
    /// Filter rows by a predicate expression.
    Filter(FilterTransform),
    /// Sort rows by column specifications.
    Sort(SortTransform),
    /// Compute a new column from an expression.
    DerivedColumn(DerivedColumnTransform),
    /// Group rows and compute aggregates.
    Aggregate(AggregateTransform),
    /// Join two datasets.
    Join(JoinTransform),
}

impl Transform {
    /// Human-readable label for the workflow sidebar.
    pub fn label(&self) -> &'static str {
        match self {
            Transform::Filter(_) => "Filter",
            Transform::Sort(_) => "Sort",
            Transform::DerivedColumn(_) => "Derived",
            Transform::Aggregate(_) => "Aggregate",
            Transform::Join(_) => "Join",
        }
    }

    /// Icon for the workflow sidebar.
    pub fn icon(&self) -> &'static str {
        match self {
            Transform::Filter(_) => "▽",
            Transform::Sort(_) => "⇅",
            Transform::DerivedColumn(_) => "ƒ",
            Transform::Aggregate(_) => "Σ",
            Transform::Join(_) => "⋈",
        }
    }

    /// Summary text for the workflow sidebar.
    pub fn summary(&self) -> String {
        match self {
            Transform::Filter(t) => t.summary(),
            Transform::Sort(t) => t.summary(),
            Transform::DerivedColumn(t) => t.summary(),
            Transform::Aggregate(t) => t.summary(),
            Transform::Join(t) => t.summary(),
        }
    }
}

// ── Filter ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterTransform {
    /// The expression to evaluate per row. Must be boolean-typed.
    pub expression: String,
    /// Whether this filter is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool { true }

impl FilterTransform {
    pub fn new(expression: impl Into<String>) -> Self {
        Self { expression: expression.into(), enabled: true }
    }

    fn summary(&self) -> String {
        if self.expression.is_empty() {
            "—".into()
        } else {
            self.expression.clone()
        }
    }
}

// ── Sort ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortTransform {
    pub specs: Vec<SortSpec>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl SortTransform {
    pub fn new(specs: Vec<SortSpec>) -> Self {
        Self { specs, enabled: true }
    }

    fn summary(&self) -> String {
        if self.specs.is_empty() {
            "—".into()
        } else {
            let labels: Vec<String> = self.specs.iter()
                .map(|s| format!("{} {}", s.column, s.direction.arrow_label()))
                .collect();
            labels.join(", ")
        }
    }
}

// ── Derived Column ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedColumnTransform {
    /// Name of the new column.
    pub name: String,
    /// Expression computing the column value.
    pub expression: String,
    /// Whether this transform is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl DerivedColumnTransform {
    pub fn new(name: impl Into<String>, expression: impl Into<String>) -> Self {
        Self { name: name.into(), expression: expression.into(), enabled: true }
    }

    fn summary(&self) -> String {
        format!("{} = {}", self.name, self.expression)
    }
}

// ── Aggregate ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateTransform {
    /// Columns to group by.
    pub group_by: Vec<String>,
    /// Aggregate expressions: ("total", "sum(revenue)").
    pub aggregates: Vec<(String, String)>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl AggregateTransform {
    pub fn new(group_by: Vec<String>, aggregates: Vec<(String, String)>) -> Self {
        Self { group_by, aggregates, enabled: true }
    }

    fn summary(&self) -> String {
        let groups = if self.group_by.is_empty() {
            "all rows".into()
        } else {
            self.group_by.join(", ")
        };
        let aggs: Vec<String> = self.aggregates.iter()
            .map(|(name, _)| name.clone())
            .collect();
        format!("GROUP BY {} → {}", groups, aggs.join(", "))
    }
}

// ── Join ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinTransform {
    /// Path to the right-side dataset.
    pub right_dataset: String,
    /// Join type.
    pub join_type: JoinType,
    /// Left column = Right column pairs.
    pub on: Vec<(String, String)>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
}

impl JoinTransform {
    fn summary(&self) -> String {
        let keys: Vec<String> = self.on.iter()
            .map(|(l, r)| format!("{l}={r}"))
            .collect();
        format!("{:?} JOIN {} ON {}", self.join_type, self.right_dataset, keys.join(", "))
    }
}

// ── Workflow (list of transforms) ────────────────────────────────────────────

/// An ordered list of transformations forming a pipeline.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransformList {
    transforms: Vec<Transform>,
}

impl TransformList {
    pub fn new() -> Self {
        Self { transforms: Vec::new() }
    }

    pub fn add(&mut self, transform: Transform) -> usize {
        let index = self.transforms.len();
        self.transforms.push(transform);
        index
    }

    pub fn remove(&mut self, index: usize) -> Option<Transform> {
        if index < self.transforms.len() {
            Some(self.transforms.remove(index))
        } else {
            None
        }
    }

    pub fn move_up(&mut self, index: usize) -> bool {
        if index > 0 && index < self.transforms.len() {
            self.transforms.swap(index, index - 1);
            true
        } else {
            false
        }
    }

    pub fn move_down(&mut self, index: usize) -> bool {
        if index + 1 < self.transforms.len() {
            self.transforms.swap(index, index + 1);
            true
        } else {
            false
        }
    }

    pub fn toggle(&mut self, index: usize) -> bool {
        if let Some(t) = self.transforms.get_mut(index) {
            match t {
                Transform::Filter(ft) => { ft.enabled = !ft.enabled; true }
                Transform::Sort(st) => { st.enabled = !st.enabled; true }
                Transform::DerivedColumn(dc) => { dc.enabled = !dc.enabled; true }
                Transform::Aggregate(at) => { at.enabled = !at.enabled; true }
                Transform::Join(jt) => { jt.enabled = !jt.enabled; true }
            }
        } else {
            false
        }
    }

    pub fn len(&self) -> usize { self.transforms.len() }

    pub fn is_empty(&self) -> bool { self.transforms.is_empty() }

    pub fn iter(&self) -> impl Iterator<Item = &Transform> {
        self.transforms.iter()
    }

    pub fn get(&self, index: usize) -> Option<&Transform> {
        self.transforms.get(index)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_list_add_remove() {
        let mut list = TransformList::new();
        let idx = list.add(Transform::Filter(FilterTransform::new("revenue > 100")));
        assert_eq!(list.len(), 1);
        assert_eq!(idx, 0);

        let removed = list.remove(0);
        assert!(removed.is_some());
        assert!(list.is_empty());
    }

    #[test]
    fn test_transform_list_reorder() {
        let mut list = TransformList::new();
        list.add(Transform::Filter(FilterTransform::new("a > 0")));
        list.add(Transform::Sort(SortTransform::new(vec![])));

        assert!(list.move_down(0)); // filter moves to position 1
        assert!(!list.move_down(1)); // already at bottom
        assert!(list.move_up(1)); // back to position 0
    }

    #[test]
    fn test_transform_list_toggle() {
        let mut list = TransformList::new();
        list.add(Transform::Filter(FilterTransform::new("x > 0")));
        assert!(list.toggle(0));
        match list.get(0).unwrap() {
            Transform::Filter(ft) => assert!(!ft.enabled),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_summaries() {
        let f = Transform::Filter(FilterTransform::new("revenue > 100"));
        assert!(f.summary().contains("revenue"));

        let s = Transform::Sort(SortTransform::new(vec![SortSpec::desc("date")]));
        assert!(s.summary().contains("date"));

        let d = Transform::DerivedColumn(DerivedColumnTransform::new("profit", "revenue - cost"));
        assert!(d.summary().contains("profit"));
        assert!(d.summary().contains("revenue"));

        let a = Transform::Aggregate(AggregateTransform::new(
            vec!["country".into()],
            vec![("total".into(), "sum(revenue)".into())],
        ));
        assert!(a.summary().contains("country"));
        assert!(a.summary().contains("total"));
    }
}
