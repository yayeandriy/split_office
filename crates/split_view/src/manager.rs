//! Layout manager — CRUD operations on the layout tree.
//!
//! The `LayoutManager` owns the root `LayoutNode` and the `FocusManager`.
//! All structural mutations go through it so focus can always be kept valid.
//!
//! # Spec mapping (§Layout Manager)
//!
//! | Spec command   | Method                        |
//! |----------------|-------------------------------|
//! | Split Horizontal | `split_horizontal`          |
//! | Split Vertical   | `split_vertical`            |
//! | Close View       | `close_view`                |
//! | Move View        | `move_view` (Phase 2)       |
//! | Open Tab         | `open_tab`                  |
//! | Close Tab        | `close_tab`                 |
//! | Restore Layout   | `LayoutState::from_json`    |
//! | Persist Layout   | `LayoutState::to_json`      |

use cdm::ObjectId;
use serde::{Deserialize, Serialize};

use crate::focus::FocusManager;
use crate::layout::{LeafView, LayoutNode, SplitNode, TabContainer, ViewId, ViewIdGen};
use crate::registry::ViewType;

// ── Layout manager ────────────────────────────────────────────────────────────

/// Owns the root layout node and coordinates focus changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutManager {
    pub root: LayoutNode,
    pub focus: FocusManager,
    pub(crate) id_gen: ViewIdGen,
}

impl LayoutManager {
    // ── Constructors ─────────────────────────────────────────────────────

    /// Build the default split-office startup layout:
    /// a single Spreadsheet tab group occupying the whole area.
    ///
    /// Every pane is always a TabContainer (VS Code-style), even with a single tab.
    pub fn default_layout() -> Self {
        let mut id_gen = ViewIdGen::new();
        let leaf = LeafView::new(id_gen.next(), ViewType::Spreadsheet, default_tab_name(ViewType::Spreadsheet));
        let root = LayoutNode::Tabs(TabContainer::new(leaf));
        let mut focus = FocusManager::new();
        focus.validate(&root);
        Self { root, focus, id_gen }
    }

    /// Restore from a previously-serialised `LayoutManager`.
    ///
    /// Normalizes the tree to ensure every pane is a TabContainer (VS Code-style).
    pub fn restored(root: LayoutNode, focus: FocusManager, id_gen: ViewIdGen) -> Self {
        let mut root = root;
        LayoutManager::normalize_tree(&mut root);
        let mut mgr = Self { root, focus, id_gen };
        mgr.focus.validate(&mgr.root); // defend against stale ids in save data
        mgr
    }

    /// Recursively wrap every bare `Leaf` in a `TabContainer`.
    ///
    /// This ensures all panes have tab chrome, even layouts restored from
    /// pre-tab-era persistence.
    fn normalize_tree(node: &mut LayoutNode) {
        match node {
            LayoutNode::Leaf(leaf) => {
                let dummy = LeafView::new(ViewId::new(0), ViewType::Spreadsheet, "");
                let leaf = std::mem::replace(leaf, dummy);
                *node = LayoutNode::Tabs(TabContainer::new(leaf));
            }
            LayoutNode::HSplit(s) | LayoutNode::VSplit(s) => {
                Self::normalize_tree(&mut s.first);
                Self::normalize_tree(&mut s.second);
            }
            LayoutNode::Tabs(_) => { /* already a tab container */ }
        }
    }

    // ── View creation helpers ─────────────────────────────────────────────

    fn new_leaf(&mut self, vt: ViewType, title: impl Into<String>, obj: Option<ObjectId>) -> LeafView {
        let mut leaf = LeafView::new(self.id_gen.next(), vt, title);
        if let Some(id) = obj {
            leaf = leaf.with_object(id);
        }
        leaf
    }

    // ── Split operations ──────────────────────────────────────────────────

    /// Split the currently-focused view horizontally (top / bottom).
    ///
    /// The focused view moves to the top half; a new view of the **same type**
    /// is placed in the bottom half, wrapped in a TabContainer (VS Code-style).
    /// Returns the `ViewId` of the new view.
    pub fn split_horizontal(
        &mut self,
        title: impl Into<String>,
        obj: Option<ObjectId>,
        ratio: f32,
    ) -> Option<ViewId> {
        let focused = self.focus.focused?;
        let vt = self.view_type_of(focused)?;
        let new_leaf = self.new_leaf(vt, title, obj);
        let new_id = new_leaf.view_id;
        let new_node = LayoutNode::Tabs(TabContainer::new(new_leaf));
        let changed = Self::split_node(&mut self.root, focused, new_node, SplitDir::Horizontal, ratio);
        if changed {
            self.focus.validate(&self.root);
            Some(new_id)
        } else {
            None
        }
    }

    /// Split the currently-focused view vertically (left / right).
    ///
    /// The focused view stays on the left; a new view of the **same type**
    /// appears on the right, wrapped in a TabContainer (VS Code-style).
    pub fn split_vertical(
        &mut self,
        title: impl Into<String>,
        obj: Option<ObjectId>,
        ratio: f32,
    ) -> Option<ViewId> {
        let focused = self.focus.focused?;
        let vt = self.view_type_of(focused)?;
        let new_leaf = self.new_leaf(vt, title, obj);
        let new_id = new_leaf.view_id;
        let new_node = LayoutNode::Tabs(TabContainer::new(new_leaf));
        let changed = Self::split_node(&mut self.root, focused, new_node, SplitDir::Vertical, ratio);
        if changed {
            self.focus.validate(&self.root);
            Some(new_id)
        } else {
            None
        }
    }

    /// Open a new tab next to the currently-focused view — inheriting its type.
    /// The tab is inserted into the focused view's TabContainer (creating one if needed).
    pub fn open_tab(
        &mut self,
        title: impl Into<String>,
        obj: Option<ObjectId>,
    ) -> Option<ViewId> {
        let focused = self.focus.focused?;
        let vt = self.view_type_of(focused)?;
        let new_leaf = self.new_leaf(vt, title, obj);
        let new_id = new_leaf.view_id;
        let changed = Self::add_tab_node(&mut self.root, focused, new_leaf);
        if changed {
            self.focus.set_focus(new_id);
            Some(new_id)
        } else {
            None
        }
    }

    /// Close the tab at `index` inside the `TabContainer` that contains `view_id`.
    pub fn close_tab(&mut self, view_id: ViewId, tab_index: usize) -> bool {
        let changed = Self::remove_tab_node(&mut self.root, view_id, tab_index);
        if changed {
            self.focus.validate(&self.root);
        }
        changed
    }

    /// Close the view with `view_id`, collapsing the split if needed.
    ///
    /// Returns `false` if the view is the only one remaining (cannot close last).
    pub fn close_view(&mut self, view_id: ViewId) -> bool {
        if self.root.all_view_ids().len() <= 1 {
            return false; // protect last view
        }
        let changed = Self::remove_view_node(&mut self.root, view_id);
        if changed {
            self.focus.validate(&self.root);
        }
        changed
    }

    // ── Tab operations ───────────────────────────────────────────────────

    /// Close the tab at `tab_index` inside the tab group identified by `group_id`.
    pub fn close_tab_in_group(&mut self, group_id: ViewId, tab_index: usize) -> bool {
        Self::remove_tab_from_group(&mut self.root, group_id, tab_index)
    }

    /// Create a new tab in the group identified by `group_id`, inheriting the group's type.
    pub fn new_tab_in_group(&mut self, group_id: ViewId) -> Option<ViewId> {
        let vt = self.view_type_of(group_id)?;
        let name = default_tab_name(vt);
        let leaf = self.new_leaf(vt, name, None);
        let new_id = leaf.view_id;
        Self::push_tab_to_group(&mut self.root, group_id, leaf);
        self.focus.validate(&self.root);
        Some(new_id)
    }

    /// Create a new tab with an explicit view type.
    pub fn new_tab_with_type(&mut self, group_id: ViewId, vt: ViewType) -> Option<ViewId> {
        let name = default_tab_name(vt);
        let leaf = self.new_leaf(vt, name, None);
        let new_id = leaf.view_id;
        Self::push_tab_to_group(&mut self.root, group_id, leaf);
        self.focus.validate(&self.root);
        Some(new_id)
    }

    /// Switch the view type of a single tab in a group.
    pub fn switch_tab_type(&mut self, group_id: ViewId, tab_index: usize, new_type: ViewType) -> bool {
        Self::set_tab_type_in_group(&mut self.root, group_id, tab_index, new_type)
    }

    /// Reorder tabs within a group.
    pub fn reorder_tabs(&mut self, group_id: ViewId, from: usize, to: usize) -> bool {
        Self::reorder_tabs_in_group(&mut self.root, group_id, from, to)
    }

    // ── Focus delegation ──────────────────────────────────────────────────

    pub fn focus_next(&mut self) {
        let root = &self.root.clone(); // clone to satisfy borrow checker
        self.focus.focus_next(root);
    }

    pub fn focus_prev(&mut self) {
        let root = &self.root.clone();
        self.focus.focus_prev(root);
    }

    pub fn set_focus(&mut self, id: ViewId) {
        self.focus.set_focus(id);
    }

    // ── View type query ──────────────────────────────────────────────────

    /// Return the `ViewType` of the leaf with the given id.
    pub fn view_type_of(&self, id: ViewId) -> Option<ViewType> {
        self.root.find_leaf(id).map(|l| l.view_type)
    }

    // ── Ratio adjustment ──────────────────────────────────────────────────

    /// Update the divider ratio for the split that contains `view_id` as
    /// its first child.
    pub fn set_ratio_for(&mut self, view_id: ViewId, ratio: f32) {
        Self::set_ratio_node(&mut self.root, view_id, ratio);
    }

    // ── Internal recursive helpers ────────────────────────────────────────

    fn split_node(
        node: &mut LayoutNode,
        target: ViewId,
        new_node: LayoutNode,
        dir: SplitDir,
        ratio: f32,
    ) -> bool {
        match node {
            LayoutNode::Leaf(leaf) if leaf.view_id == target => {
                // Wrap the existing bare leaf in a TabContainer so both sides
                // of the split are tab groups (VS Code-style: every pane has tabs).
                let existing_tabs = LayoutNode::Tabs(TabContainer::new(
                    std::mem::replace(leaf, LeafView::new(
                        ViewId::new(0), ViewType::Spreadsheet, "",
                    ))
                ));
                *node = match dir {
                    SplitDir::Horizontal => LayoutNode::HSplit(SplitNode::new(existing_tabs, new_node, ratio)),
                    SplitDir::Vertical   => LayoutNode::VSplit(SplitNode::new(existing_tabs, new_node, ratio)),
                };
                true
            }
            LayoutNode::Leaf(_) => false,
            LayoutNode::HSplit(s) | LayoutNode::VSplit(s) => {
                Self::split_node(&mut s.first, target, new_node.clone(), dir, ratio)
                    || Self::split_node(&mut s.second, target, new_node, dir, ratio)
            }
            LayoutNode::Tabs(t) => {
                // Split the whole TabContainer out if target is inside it.
                if t.tabs.iter().any(|l| l.view_id == target) {
                    let existing = std::mem::replace(node, LayoutNode::Leaf(LeafView::new(
                        ViewId::new(0), ViewType::Spreadsheet, "",
                    )));
                    *node = match dir {
                        SplitDir::Horizontal => LayoutNode::HSplit(SplitNode::new(existing, new_node, ratio)),
                        SplitDir::Vertical   => LayoutNode::VSplit(SplitNode::new(existing, new_node, ratio)),
                    };
                    return true;
                }
                false
            }
        }
    }

    fn add_tab_node(node: &mut LayoutNode, target: ViewId, new_leaf: LeafView) -> bool {
        match node {
            LayoutNode::Leaf(leaf) if leaf.view_id == target => {
                let existing_leaf = if let LayoutNode::Leaf(l) = std::mem::replace(
                    node,
                    LayoutNode::Leaf(LeafView::new(ViewId::new(0), ViewType::Spreadsheet, "")),
                ) {
                    l
                } else {
                    unreachable!()
                };
                let mut tabs = TabContainer::new(existing_leaf);
                tabs.push(new_leaf);
                *node = LayoutNode::Tabs(tabs);
                true
            }
            LayoutNode::Leaf(_) => false,
            LayoutNode::HSplit(s) | LayoutNode::VSplit(s) => {
                Self::add_tab_node(&mut s.first, target, new_leaf.clone())
                    || Self::add_tab_node(&mut s.second, target, new_leaf)
            }
            LayoutNode::Tabs(t) => {
                // Target is inside this container — just append.
                if t.tabs.iter().any(|l| l.view_id == target) {
                    t.push(new_leaf);
                    return true;
                }
                false
            }
        }
    }

    fn remove_tab_node(node: &mut LayoutNode, view_id: ViewId, index: usize) -> bool {
        match node {
            LayoutNode::Leaf(_) => false,
            LayoutNode::HSplit(s) | LayoutNode::VSplit(s) => {
                Self::remove_tab_node(&mut s.first, view_id, index)
                    || Self::remove_tab_node(&mut s.second, view_id, index)
            }
            LayoutNode::Tabs(t) => {
                if t.tabs.iter().any(|l| l.view_id == view_id) {
                    t.close(index).is_some()
                } else {
                    false
                }
            }
        }
    }

    fn remove_view_node(node: &mut LayoutNode, target: ViewId) -> bool {
        match node {
            LayoutNode::Leaf(l) => l.view_id == target,
            LayoutNode::HSplit(s) | LayoutNode::VSplit(s) => {
                let first_has = s.first.all_view_ids().contains(&target);
                let second_has = s.second.all_view_ids().contains(&target);

                if first_has {
                    // If the first child only contains this one view, collapse the split.
                    if Self::is_sole_occupant(&s.first, target) {
                        let survivor = std::mem::replace(
                            &mut *s.second,
                            LayoutNode::Leaf(LeafView::new(ViewId::new(0), ViewType::Spreadsheet, "")),
                        );
                        *node = survivor;
                        return true;
                    }
                    return Self::remove_view_node(&mut s.first, target);
                }
                if second_has {
                    if Self::is_sole_occupant(&s.second, target) {
                        let survivor = std::mem::replace(
                            &mut *s.first,
                            LayoutNode::Leaf(LeafView::new(ViewId::new(0), ViewType::Spreadsheet, "")),
                        );
                        *node = survivor;
                        return true;
                    }
                    return Self::remove_view_node(&mut s.second, target);
                }
                false
            }
            LayoutNode::Tabs(t) => {
                if let Some(pos) = t.tabs.iter().position(|l| l.view_id == target) {
                    t.close(pos).is_some()
                } else {
                    false
                }
            }
        }
    }

    /// True when `node` is a container whose only leaf is `target`.
    fn is_sole_occupant(node: &LayoutNode, target: ViewId) -> bool {
        let ids = node.all_view_ids();
        ids.len() == 1 && ids[0] == target
    }

    // ── Tab group helpers ────────────────────────────────────────────────

    fn find_group_mut(node: &mut LayoutNode, group_id: ViewId) -> Option<&mut TabContainer> {
        match node {
            LayoutNode::Leaf(_) => None,
            LayoutNode::HSplit(s) | LayoutNode::VSplit(s) => {
                Self::find_group_mut(&mut s.first, group_id)
                    .or_else(|| Self::find_group_mut(&mut s.second, group_id))
            }
            LayoutNode::Tabs(t) => {
                if t.tabs.first().map(|l| l.view_id) == Some(group_id) {
                    Some(t)
                } else {
                    None
                }
            }
        }
    }

    fn remove_tab_from_group(node: &mut LayoutNode, group_id: ViewId, tab_index: usize) -> bool {
        if let Some(tabs) = Self::find_group_mut(node, group_id) {
            tabs.close(tab_index).is_some()
        } else {
            false
        }
    }

    fn push_tab_to_group(node: &mut LayoutNode, group_id: ViewId, leaf: LeafView) {
        if let Some(tabs) = Self::find_group_mut(node, group_id) {
            tabs.push(leaf);
        }
    }

    fn set_tab_type_in_group(node: &mut LayoutNode, group_id: ViewId, tab_index: usize, vt: ViewType) -> bool {
        if let Some(tabs) = Self::find_group_mut(node, group_id) {
            tabs.set_view_type(tab_index, vt);
            true
        } else {
            false
        }
    }

    fn reorder_tabs_in_group(node: &mut LayoutNode, group_id: ViewId, from: usize, to: usize) -> bool {
        if let Some(tabs) = Self::find_group_mut(node, group_id) {
            tabs.reorder(from, to);
            true
        } else {
            false
        }
    }

    fn set_ratio_node(node: &mut LayoutNode, view_id: ViewId, ratio: f32) {
        match node {
            LayoutNode::Leaf(_) | LayoutNode::Tabs(_) => {}
            LayoutNode::HSplit(s) | LayoutNode::VSplit(s) => {
                if s.first.all_view_ids().contains(&view_id) {
                    s.set_ratio(ratio);
                } else {
                    Self::set_ratio_node(&mut s.first, view_id, ratio);
                    Self::set_ratio_node(&mut s.second, view_id, ratio);
                }
            }
        }
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Default human-readable tab name for a view type.
pub fn default_tab_name(vt: ViewType) -> &'static str {
    match vt {
        ViewType::Spreadsheet => "Sheet",
        ViewType::Document    => "Doc",
    }
}

#[derive(Clone, Copy)]
enum SplitDir {
    Horizontal,
    Vertical,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_layout_has_one_spreadsheet() {
        let mgr = LayoutManager::default_layout();
        let ids = mgr.root.all_view_ids();
        assert_eq!(ids.len(), 1);
        // Root is always a TabContainer (VS Code-style).
        if let LayoutNode::Tabs(t) = &mgr.root {
            assert_eq!(t.active_leaf().view_type, ViewType::Spreadsheet);
        } else {
            panic!("expected Tabs");
        }
    }

    #[test]
    fn test_split_vertical_adds_view() {
        let mut mgr = LayoutManager::default_layout();
        let new_id = mgr.split_vertical("Doc", None, 0.5).unwrap();
        let ids = mgr.root.all_view_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&new_id));
        assert!(matches!(mgr.root, LayoutNode::VSplit(_)));
    }

    #[test]
    fn test_split_horizontal_adds_view() {
        let mut mgr = LayoutManager::default_layout();
        mgr.split_horizontal("Doc", None, 0.5).unwrap();
        assert!(matches!(mgr.root, LayoutNode::HSplit(_)));
        assert_eq!(mgr.root.all_view_ids().len(), 2);
    }

    #[test]
    fn test_close_view_collapses_split() {
        let mut mgr = LayoutManager::default_layout();
        let orig_id = mgr.focus.focused.unwrap();
        let new_id = mgr.split_vertical("Doc", None, 0.5).unwrap();

        // Close the newly-added view.
        let ok = mgr.close_view(new_id);
        assert!(ok);
        assert_eq!(mgr.root.all_view_ids(), vec![orig_id]);
        // After collapse, root is a single TabContainer (VS Code-style).
        assert!(matches!(mgr.root, LayoutNode::Tabs(_)));
    }

    #[test]
    fn test_cannot_close_last_view() {
        let mut mgr = LayoutManager::default_layout();
        let only_id = mgr.focus.focused.unwrap();
        let ok = mgr.close_view(only_id);
        assert!(!ok);
        assert_eq!(mgr.root.all_view_ids().len(), 1);
    }

    #[test]
    fn test_open_tab_wraps_leaf_in_tabcontainer() {
        let mut mgr = LayoutManager::default_layout();
        let tab_id = mgr.open_tab("Doc B", None).unwrap();
        assert!(matches!(mgr.root, LayoutNode::Tabs(_)));
        let ids = mgr.root.all_view_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&tab_id));
        assert_eq!(mgr.focus.focused, Some(tab_id));
    }

    #[test]
    fn test_focus_next_advances() {
        let mut mgr = LayoutManager::default_layout();
        let id1 = mgr.focus.focused.unwrap();
        let id2 = mgr.split_vertical("Doc", None, 0.5).unwrap();
        mgr.set_focus(id1);
        mgr.focus_next();
        assert_eq!(mgr.focus.focused, Some(id2));
        mgr.focus_next();
        assert_eq!(mgr.focus.focused, Some(id1)); // wraps
    }

    #[test]
    fn test_normalize_wraps_bare_leaves_in_tabs() {
        // Simulate an old persisted layout with bare Leaf nodes.
        let leaf1 = LeafView::new(ViewId::new(1), ViewType::Spreadsheet, "A");
        let leaf2 = LeafView::new(ViewId::new(2), ViewType::Document, "B");
        let old_root = LayoutNode::VSplit(SplitNode::new(
            LayoutNode::Leaf(leaf1),
            LayoutNode::Leaf(leaf2),
            0.5,
        ));

        let mgr = LayoutManager::restored(
            old_root,
            FocusManager::new(),
            ViewIdGen::new(),
        );

        // After normalization, both children of the split should be Tabs.
        if let LayoutNode::VSplit(s) = &mgr.root {
            assert!(matches!(*s.first, LayoutNode::Tabs(_)), "first child should be Tabs");
            assert!(matches!(*s.second, LayoutNode::Tabs(_)), "second child should be Tabs");
        } else {
            panic!("expected VSplit");
        }
        assert_eq!(mgr.root.all_view_ids().len(), 2);
    }
}
