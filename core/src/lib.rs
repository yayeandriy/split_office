//! Core data model for Split Office.
//!
//! # Design principles
//!
//! - No `Workbook`, `Sheet`, or `Cell` types.
//! - Primary primitives: `Dataset`, `Column`, `View`, `Selection`.
//! - Rendering details (cell painting) live in the `grid` crate.

pub mod dataset;
pub mod filter;
pub mod selection;
pub mod sort;
pub mod types;
pub mod viewport;

pub use dataset::{Column, Dataset, DatasetId, Schema};
pub use filter::FilterExpr;
pub use selection::Selection;
pub use sort::{SortDirection, SortSpec};
pub use types::DataType;
pub use viewport::Viewport;
