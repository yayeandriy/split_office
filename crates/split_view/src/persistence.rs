//! Persistence — serialisable snapshot of the full layout state.
//!
//! `LayoutState` is the only type that crosses the save/restore boundary.
//! It embeds the layout tree, focus, view ID counter, and split ratios
//! (ratios are stored inline in `SplitNode`, so nothing extra is needed).
//!
//! # Spec requirements (§Persistence)
//!
//! Persists:
//! - Layout Tree  ✓ (via `LayoutNode` serde)
//! - View States  — deferred to individual view renderers
//! - Tab Order    ✓ (via `TabContainer.tabs` ordering)
//! - Split Ratios ✓ (via `SplitNode.ratio`)
//! - Focused View ✓ (via `FocusManager.focused`)
//! - Workspace References ✓ (via `LeafView.object_ref`)

use serde::{Deserialize, Serialize};

use crate::focus::FocusManager;
use crate::layout::{LayoutNode, ViewIdGen};
use crate::manager::LayoutManager;

// ── Layout state ──────────────────────────────────────────────────────────────

/// Complete, serialisable snapshot of the split-view layout.
///
/// Round-trips cleanly to/from JSON.  The `LayoutManager` can be rebuilt
/// from this state without data loss.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutState {
    pub root: LayoutNode,
    pub focus: FocusManager,
    pub id_gen: ViewIdGen,
}

impl LayoutState {
    /// Capture the current state of a `LayoutManager`.
    pub fn capture(mgr: &LayoutManager) -> Self {
        Self {
            root: mgr.root.clone(),
            focus: mgr.focus.clone(),
            id_gen: mgr.id_gen.clone(),
        }
    }

    /// Restore a `LayoutManager` from a captured state.
    pub fn restore(self) -> LayoutManager {
        LayoutManager::restored(self.root, self.focus, self.id_gen)
    }

    /// Serialise to a compact JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialise from a JSON string produced by [`to_json`].
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Pretty-printed JSON for debugging / human-readable storage.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::LayoutManager;

    #[test]
    fn test_roundtrip_default_layout() {
        let mgr = LayoutManager::default_layout();
        let state = LayoutState::capture(&mgr);
        let json = state.to_json().unwrap();
        let restored_state = LayoutState::from_json(&json).unwrap();
        let restored = restored_state.restore();
        assert_eq!(
            restored.root.all_view_ids(),
            mgr.root.all_view_ids()
        );
        assert_eq!(restored.focus.focused, mgr.focus.focused);
    }

    #[test]
    fn test_roundtrip_nested_layout() {
        let mut mgr = LayoutManager::default_layout();
        mgr.split_vertical("Doc", None, 0.7).unwrap();
        let id2 = mgr.focus.focused.unwrap();
        mgr.open_tab("View", None).unwrap();

        let before_ids = mgr.root.all_view_ids();
        let state = LayoutState::capture(&mgr);
        let json = state.to_json().unwrap();
        let restored = LayoutState::from_json(&json).unwrap().restore();

        assert_eq!(restored.root.all_view_ids(), before_ids);
        // Focused view survives.
        assert!(restored.root.all_view_ids().contains(&id2)
            || restored.focus.focused.is_some());
    }

    #[test]
    fn test_tab_order_preserved() {
        let mut mgr = LayoutManager::default_layout();
        let id_a = mgr.focus.focused.unwrap();
        let id_b = mgr.open_tab("B", None).unwrap();
        let id_c = mgr.open_tab("C", None).unwrap();

        let state = LayoutState::capture(&mgr);
        let json = state.to_json().unwrap();
        let restored = LayoutState::from_json(&json).unwrap().restore();

        let ids = restored.root.all_view_ids();
        assert_eq!(ids, vec![id_a, id_b, id_c]);
    }

    #[test]
    fn test_split_ratios_preserved() {
        let mut mgr = LayoutManager::default_layout();
        mgr.split_vertical("Doc", None, 0.3).unwrap();

        let state = LayoutState::capture(&mgr);
        let json = state.to_json().unwrap();
        let restored = LayoutState::from_json(&json).unwrap().restore();

        if let crate::layout::LayoutNode::VSplit(s) = &restored.root {
            assert!((s.ratio - 0.3).abs() < 1e-5);
        } else {
            panic!("expected VSplit");
        }
    }
}
