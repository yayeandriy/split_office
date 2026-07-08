//! View registry — maps `ViewType` to metadata.
//!
//! Phase 1 view types (spec §View Registry):
//!
//! - SpreadsheetView
//! - DocumentView
//!
//! Ambient panels (Explorer, Inspector) are now floating windows outside
//! the split-view layout.  The registry is intentionally open for future
//! extension: new view types are added as enum variants and registered
//! once without touching any other part of the framework.

use serde::{Deserialize, Serialize};

// ── View type ─────────────────────────────────────────────────────────────────

/// The class of a view.  No view type receives special treatment.
///
/// Only tab-capable views that project workspace objects are registered here.
/// Ambient panels (Explorer, Inspector) are now floating windows outside the split layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViewType {
    /// Projects a CDM Dataset — shows a virtual-scrolling grid.
    Spreadsheet,
    /// Projects a CDM Document — shows block-structured rich text.
    Document,
}

impl ViewType {
    /// Short human-readable label (used in tab bars and menus).
    pub fn label(self) -> &'static str {
        match self {
            ViewType::Spreadsheet => "Spreadsheet",
            ViewType::Document    => "Document",
        }
    }

    /// Icon character rendered next to the label.
    /// Uses SF Pro-safe Unicode symbols (not emoji).
    pub fn icon(self) -> &'static str {
        match self {
            ViewType::Spreadsheet => "⊞",
            ViewType::Document    => "≡",
        }
    }

    /// All views in the split framework project a workspace object.
    pub fn requires_object(self) -> bool {
        true
    }
}

// ── View registry ─────────────────────────────────────────────────────────────

/// Tracks every registered view type for the lifetime of the application.
///
/// Phase 1 pre-registers the four built-in view types.  Future phases will
/// accept dynamic registration from plugins.
pub struct ViewRegistry {
    types: Vec<ViewType>,
}

impl ViewRegistry {
    /// Build the Phase 1 registry with all built-in view types.
    pub fn new() -> Self {
        Self {
            types: vec![
                ViewType::Spreadsheet,
                ViewType::Document,
            ],
        }
    }

    /// All registered view types in registration order.
    pub fn view_types(&self) -> &[ViewType] {
        &self.types
    }

    /// True if the given type is registered.
    pub fn is_registered(&self, vt: ViewType) -> bool {
        self.types.contains(&vt)
    }
}

impl Default for ViewRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_phase1_types_registered() {
        let reg = ViewRegistry::new();
        assert!(reg.is_registered(ViewType::Spreadsheet));
        assert!(reg.is_registered(ViewType::Document));
    }

    #[test]
    fn test_requires_object() {
        assert!(ViewType::Spreadsheet.requires_object());
        assert!(ViewType::Document.requires_object());
    }

    #[test]
    fn test_serde_roundtrip() {
        let vt = ViewType::Spreadsheet;
        let json = serde_json::to_string(&vt).unwrap();
        let restored: ViewType = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, vt);
    }
}
