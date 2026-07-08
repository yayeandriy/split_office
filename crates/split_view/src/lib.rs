//! Split View Framework — platform-level view composition engine.
//!
//! This crate is the universal workspace window manager for Split Office.
//! It owns the layout tree, manages focus, and drives the recursive egui
//! renderer. No individual view type receives special treatment.
//!
//! # Public API
//!
//! ```text
//! LayoutManager  — CRUD on the layout tree
//! ViewRegistry   — maps ViewType → render factory
//! FocusManager   — tracks active view, routes keyboard
//! LayoutState    — serialisable snapshot for persistence
//! render_layout  — top-level egui render entry point
//! ```

pub mod focus;
pub mod layout;
pub mod manager;
pub mod persistence;
pub mod registry;
pub mod renderer;

pub use focus::FocusManager;
pub use layout::{LayoutNode, LeafView, SplitNode, TabContainer, ViewId};
pub use manager::{default_tab_name, LayoutManager};
pub use persistence::LayoutState;
pub use registry::{ViewRegistry, ViewType};
pub use renderer::{render_layout, tab_button, TabAction, ViewContext, BAR_PADDING};
