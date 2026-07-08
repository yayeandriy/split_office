//! Focus manager — tracks the active view and routes keyboard commands.
//!
//! Spec requirements (§Focus Manager):
//!
//! - Exactly one view is "focused" at a time.
//! - Only the focused view receives keyboard commands.
//! - Focus cycles via Focus Next / Focus Previous commands.
//! - Focus is preserved across layout changes when the view still exists.

use serde::{Deserialize, Serialize};

use crate::layout::{LayoutNode, ViewId};

// ── Focus manager ─────────────────────────────────────────────────────────────

/// Owns the focused-view state and provides focus-routing helpers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusManager {
    /// The currently-focused view.  `None` only when the layout is empty
    /// (which should not occur in normal use).
    pub focused: Option<ViewId>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self { focused: None }
    }

    /// Set focus explicitly (e.g. on mouse click).
    pub fn set_focus(&mut self, id: ViewId) {
        self.focused = Some(id);
    }

    /// Clear focus (should be rare; prefer explicit routing).
    pub fn clear(&mut self) {
        self.focused = None;
    }

    /// True if `id` currently has focus.
    pub fn is_focused(&self, id: ViewId) -> bool {
        self.focused == Some(id)
    }

    /// Move focus to the next view in the layout tree (depth-first order).
    ///
    /// Wraps around at the end.
    pub fn focus_next(&mut self, root: &LayoutNode) {
        let ids = root.all_view_ids();
        self.step_focus(&ids, 1);
    }

    /// Move focus to the previous view (reverse depth-first order).
    ///
    /// Wraps around at the start.
    pub fn focus_prev(&mut self, root: &LayoutNode) {
        let ids = root.all_view_ids();
        self.step_focus(&ids, -1);
    }

    fn step_focus(&mut self, ids: &[ViewId], delta: i64) {
        if ids.is_empty() {
            self.focused = None;
            return;
        }
        let current_pos = self
            .focused
            .and_then(|fid| ids.iter().position(|id| *id == fid));

        let next_pos = match current_pos {
            Some(pos) => {
                let len = ids.len() as i64;
                ((pos as i64 + delta).rem_euclid(len)) as usize
            }
            None => 0,
        };
        self.focused = Some(ids[next_pos]);
    }

    /// Validate that the focused view still exists in `root`; if not, reset
    /// to the first available view.  Call this after any layout mutation.
    pub fn validate(&mut self, root: &LayoutNode) {
        let ids = root.all_view_ids();
        if let Some(fid) = self.focused {
            if !ids.contains(&fid) {
                self.focused = ids.first().copied();
            }
        } else {
            self.focused = ids.first().copied();
        }
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{LeafView, LayoutNode, SplitNode};
    use crate::registry::ViewType;

    fn leaf(id: u64) -> LayoutNode {
        LayoutNode::Leaf(LeafView::new(ViewId::new(id), ViewType::Document, "t"))
    }

    fn three_way_layout() -> LayoutNode {
        LayoutNode::VSplit(SplitNode::new(
            leaf(1),
            LayoutNode::VSplit(SplitNode::new(leaf(2), leaf(3), 0.5)),
            0.5,
        ))
    }

    #[test]
    fn test_focus_next_cycles() {
        let root = three_way_layout();
        let mut fm = FocusManager::new();
        fm.set_focus(ViewId::new(1));
        fm.focus_next(&root);
        assert_eq!(fm.focused, Some(ViewId::new(2)));
        fm.focus_next(&root);
        assert_eq!(fm.focused, Some(ViewId::new(3)));
        fm.focus_next(&root);
        assert_eq!(fm.focused, Some(ViewId::new(1))); // wraps
    }

    #[test]
    fn test_focus_prev_cycles() {
        let root = three_way_layout();
        let mut fm = FocusManager::new();
        fm.set_focus(ViewId::new(1));
        fm.focus_prev(&root);
        assert_eq!(fm.focused, Some(ViewId::new(3))); // wraps backwards
    }

    #[test]
    fn test_validate_resets_on_stale_id() {
        let root = leaf(42);
        let mut fm = FocusManager::new();
        fm.set_focus(ViewId::new(99)); // stale id
        fm.validate(&root);
        assert_eq!(fm.focused, Some(ViewId::new(42)));
    }

    #[test]
    fn test_validate_keeps_valid_focus() {
        let root = three_way_layout();
        let mut fm = FocusManager::new();
        fm.set_focus(ViewId::new(2));
        fm.validate(&root);
        assert_eq!(fm.focused, Some(ViewId::new(2)));
    }
}
