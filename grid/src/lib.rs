//! Virtualized grid renderer for Split Office.
//!
//! # Key principles
//!
//! - Only visible rows are ever painted.
//! - Uses `egui::Painter` directly — no widgets, no nested layouts, no egui tables.
//! - Column widths stored in a `HashMap<usize, f32>` — never derived from data per frame.
//! - Header clicks emit sort signals; resize handles drag column widths.

pub mod cell;
pub mod renderer;
pub mod state;

pub use renderer::GridRenderer;
pub use state::{GridAction, GridState};
