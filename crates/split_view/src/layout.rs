//! Layout tree — the structural representation of the split view UI.
//!
//! Every possible UI configuration (any combination of splits, tabs, and
//! leaf views) is expressed as a `LayoutNode`.  The tree is pure data with
//! no rendering logic; the renderer walks it separately.
//!
//! # Spec mapping
//!
//! | Spec term       | Rust type          |
//! |-----------------|--------------------|
//! | LeafView        | `LayoutNode::Leaf` |
//! | HorizontalSplit | `LayoutNode::HSplit` |
//! | VerticalSplit   | `LayoutNode::VSplit` |
//! | TabContainer    | `LayoutNode::Tabs` |

use cdm::ObjectId;
use serde::{Deserialize, Serialize};

use crate::registry::ViewType;

// ── View identity ─────────────────────────────────────────────────────────────

/// Opaque, monotonically-increasing identifier for a view instance.
///
/// Distinct from `ObjectId` — a view is not a CDM object, it is a *projection*
/// of one.  Multiple views may project the same `ObjectId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ViewId(pub u64);

impl ViewId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Monotonically-increasing source of `ViewId`s.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ViewIdGen {
    next: u64,
}

impl ViewIdGen {
    pub fn new() -> Self {
        Self { next: 1 }
    }

    pub fn next(&mut self) -> ViewId {
        let id = ViewId(self.next);
        self.next += 1;
        id
    }
}

// ── Leaf view ─────────────────────────────────────────────────────────────────

/// A single view that projects a workspace object (or nothing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeafView {
    /// Stable identity used for focus tracking and persistence.
    pub view_id: ViewId,
    /// The kind of view being projected.
    pub view_type: ViewType,
    /// The workspace object this view projects, if any.
    ///
    /// - `SpreadsheetView` → `DatasetId`
    /// - `DocumentView`    → `DocumentId`
    pub object_ref: Option<ObjectId>,
    /// Human-readable label shown in tab bars and title bars.
    pub title: String,
}

impl LeafView {
    pub fn new(view_id: ViewId, view_type: ViewType, title: impl Into<String>) -> Self {
        Self {
            view_id,
            view_type,
            object_ref: None,
            title: title.into(),
        }
    }

    pub fn with_object(mut self, id: ObjectId) -> Self {
        self.object_ref = Some(id);
        self
    }
}

// ── Split node ────────────────────────────────────────────────────────────────

/// A binary split (horizontal or vertical) with an adjustable divider.
///
/// `ratio` is the fraction of available space given to `first`; `second`
/// receives `1 - ratio`.  Values are clamped to [0.1, 0.9].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitNode {
    /// Fraction [0.1, 0.9] of total space given to the first child.
    pub ratio: f32,
    pub first: Box<LayoutNode>,
    pub second: Box<LayoutNode>,
}

impl SplitNode {
    pub fn new(first: LayoutNode, second: LayoutNode, ratio: f32) -> Self {
        Self {
            ratio: ratio.clamp(0.1, 0.9),
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = ratio.clamp(0.1, 0.9);
    }
}

// ── Tab container ─────────────────────────────────────────────────────────────

/// A set of leaf views sharing the same screen region, with one active at a time.
///
/// The spec guarantees: only the active tab renders; hidden tabs are suspended.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabContainer {
    /// Index into `tabs` of the currently-displayed tab.
    pub active_tab: usize,
    /// Ordered tab list.  Must never be empty.
    pub tabs: Vec<LeafView>,
}

impl TabContainer {
    /// Create a tab container with a single initial leaf.
    pub fn new(leaf: LeafView) -> Self {
        Self {
            active_tab: 0,
            tabs: vec![leaf],
        }
    }

    pub fn push(&mut self, leaf: LeafView) {
        self.tabs.push(leaf);
        self.active_tab = self.tabs.len() - 1;
    }

    pub fn close(&mut self, index: usize) -> Option<LeafView> {
        if self.tabs.len() <= 1 || index >= self.tabs.len() {
            return None; // refuse to leave the container empty
        }
        let removed = self.tabs.remove(index);
        self.active_tab = self.active_tab.min(self.tabs.len() - 1);
        Some(removed)
    }

    pub fn active_leaf(&self) -> &LeafView {
        &self.tabs[self.active_tab]
    }

    /// Reorder tabs: move the tab at `from` to `to`.
    pub fn reorder(&mut self, from: usize, to: usize) {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
            return;
        }
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        // Keep active_tab tracking the same tab.
        if self.active_tab == from {
            self.active_tab = to;
        } else if from < self.active_tab && to >= self.active_tab {
            self.active_tab -= 1;
        } else if from > self.active_tab && to <= self.active_tab {
            self.active_tab += 1;
        }
    }

    /// Change the view type of a single tab.
    pub fn set_view_type(&mut self, index: usize, vt: ViewType) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.view_type = vt;
        }
    }
}

// ── Layout node ───────────────────────────────────────────────────────────────

/// A node in the layout tree.
///
/// The tree is recursive: `HSplit` and `VSplit` contain any `LayoutNode` as
/// children, enabling arbitrarily deep nesting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutNode {
    /// A single view occupying the full available rect.
    Leaf(LeafView),
    /// Two children stacked top-to-bottom (first on top).
    HSplit(SplitNode),
    /// Two children placed left-to-right (first on left).
    VSplit(SplitNode),
    /// Multiple views occupying the same rect, shown one at a time.
    Tabs(TabContainer),
}

impl LayoutNode {
    /// Collect all `ViewId`s reachable from this node.
    pub fn all_view_ids(&self) -> Vec<ViewId> {
        let mut out = Vec::new();
        self.collect_ids(&mut out);
        out
    }

    fn collect_ids(&self, out: &mut Vec<ViewId>) {
        match self {
            LayoutNode::Leaf(l) => out.push(l.view_id),
            LayoutNode::HSplit(s) | LayoutNode::VSplit(s) => {
                s.first.collect_ids(out);
                s.second.collect_ids(out);
            }
            LayoutNode::Tabs(t) => {
                for tab in &t.tabs {
                    out.push(tab.view_id);
                }
            }
        }
    }

    /// Return the first `LeafView` found (depth-first).
    pub fn first_leaf(&self) -> Option<&LeafView> {
        match self {
            LayoutNode::Leaf(l) => Some(l),
            LayoutNode::HSplit(s) | LayoutNode::VSplit(s) => {
                s.first.first_leaf().or_else(|| s.second.first_leaf())
            }
            LayoutNode::Tabs(t) => t.tabs.first(),
        }
    }

    /// Find a leaf by `ViewId`, depth-first.
    pub fn find_leaf(&self, id: ViewId) -> Option<&LeafView> {
        match self {
            LayoutNode::Leaf(l) if l.view_id == id => Some(l),
            LayoutNode::Leaf(_) => None,
            LayoutNode::HSplit(s) | LayoutNode::VSplit(s) => {
                s.first.find_leaf(id).or_else(|| s.second.find_leaf(id))
            }
            LayoutNode::Tabs(t) => t.tabs.iter().find(|l| l.view_id == id),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ViewType;

    fn make_leaf(id: u64, vt: ViewType) -> LayoutNode {
        LayoutNode::Leaf(LeafView::new(ViewId::new(id), vt, "test"))
    }

    #[test]
    fn test_leaf_creation() {
        let leaf = LeafView::new(ViewId::new(1), ViewType::Spreadsheet, "Sheet A");
        assert_eq!(leaf.view_id, ViewId::new(1));
        assert!(leaf.object_ref.is_none());
    }

    #[test]
    fn test_split_ratio_clamped() {
        let s = SplitNode::new(
            make_leaf(1, ViewType::Spreadsheet),
            make_leaf(2, ViewType::Document),
            1.5,
        );
        assert_eq!(s.ratio, 0.9);

        let s2 = SplitNode::new(
            make_leaf(3, ViewType::Spreadsheet),
            make_leaf(4, ViewType::Document),
            -0.5,
        );
        assert_eq!(s2.ratio, 0.1);
    }

    #[test]
    fn test_tab_container_push_and_close() {
        let leaf1 = LeafView::new(ViewId::new(1), ViewType::Document, "Doc A");
        let leaf2 = LeafView::new(ViewId::new(2), ViewType::Document, "Doc B");
        let mut tabs = TabContainer::new(leaf1);
        tabs.push(leaf2);
        assert_eq!(tabs.tabs.len(), 2);
        assert_eq!(tabs.active_tab, 1);

        // Close last tab — new active is 0.
        let removed = tabs.close(1).unwrap();
        assert_eq!(removed.view_id, ViewId::new(2));
        assert_eq!(tabs.active_tab, 0);
    }

    #[test]
    fn test_tab_container_refuses_last_tab() {
        let leaf = LeafView::new(ViewId::new(1), ViewType::Document, "Doc");
        let mut tabs = TabContainer::new(leaf);
        assert!(tabs.close(0).is_none());
    }

    #[test]
    fn test_all_view_ids_nested() {
        let root = LayoutNode::VSplit(SplitNode::new(
            make_leaf(1, ViewType::Spreadsheet),
            LayoutNode::HSplit(SplitNode::new(
                make_leaf(2, ViewType::Document),
                make_leaf(3, ViewType::Spreadsheet),
                0.5,
            )),
            0.7,
        ));
        let ids = root.all_view_ids();
        assert_eq!(ids, vec![ViewId::new(1), ViewId::new(2), ViewId::new(3)]);
    }

    #[test]
    fn test_serde_roundtrip() {
        let root = LayoutNode::VSplit(SplitNode::new(
            make_leaf(1, ViewType::Spreadsheet),
            make_leaf(2, ViewType::Document),
            0.6,
        ));
        let json = serde_json::to_string(&root).unwrap();
        let restored: LayoutNode = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.all_view_ids(), vec![ViewId::new(1), ViewId::new(2)]);
    }
}
