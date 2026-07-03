use std::collections::HashMap;

use core::{Selection, SortDirection, SortSpec};

/// Mutable UI state for the grid.
#[derive(Debug, Clone)]
pub struct GridState {
    /// Width of each column by index (pixels).
    pub column_widths: HashMap<usize, f32>,
    /// Vertical scroll offset in "virtual" pixels (row_index * row_height).
    pub scroll_y: f32,
    /// Horizontal scroll offset.
    pub scroll_x: f32,
    /// Currently selected rows.
    pub selection: Selection,
    /// Active sort specs (multi-column sort, ordered by priority).
    pub sort_specs: Vec<SortSpec>,
    /// Per-column filter text (column name → filter string).
    pub column_filters: HashMap<String, String>,
    /// Which column's filter input currently has keyboard focus.
    pub focused_filter_col: Option<String>,
    /// Whether a column is currently being resized (col_index, start_x).
    pub resizing: Option<(usize, f32)>,
}

impl GridState {
    pub const DEFAULT_COL_WIDTH: f32 = 120.0;
    pub const MIN_COL_WIDTH: f32 = 40.0;
    pub const MAX_COL_WIDTH: f32 = 600.0;
    pub const ROW_HEIGHT: f32 = 24.0;
    pub const HEADER_HEIGHT: f32 = 30.0;
    pub const FILTER_ROW_HEIGHT: f32 = 26.0;
    pub const RESIZE_HANDLE_WIDTH: f32 = 6.0;

    /// Total height of the header area (column headers + filter row).
    pub fn total_header_height() -> f32 {
        Self::HEADER_HEIGHT + Self::FILTER_ROW_HEIGHT
    }

    pub fn new() -> Self {
        Self {
            column_widths: HashMap::new(),
            scroll_y: 0.0,
            scroll_x: 0.0,
            selection: Selection::empty(),
            sort_specs: Vec::new(),
            column_filters: HashMap::new(),
            focused_filter_col: None,
            resizing: None,
        }
    }

    pub fn col_width(&self, idx: usize) -> f32 {
        *self
            .column_widths
            .get(&idx)
            .unwrap_or(&Self::DEFAULT_COL_WIDTH)
    }

    /// Toggle sort on a column. If `shift` is held, add to multi-sort; otherwise replace.
    /// If the column is already the *last* sort key, flip its direction.
    /// If the column is already in the sort list but not last, remove it.
    pub fn toggle_sort(&mut self, col_name: &str, shift_held: bool) {
        if shift_held {
            // Multi-sort: add/remove/toggle this column in the list.
            if let Some(pos) = self.sort_specs.iter().position(|s| s.column == col_name) {
                if pos == self.sort_specs.len() - 1 {
                    // Last in list → toggle direction.
                    let new_dir = self.sort_specs[pos].direction.toggle();
                    self.sort_specs[pos].direction = new_dir;
                } else {
                    // Not last → remove from list.
                    self.sort_specs.remove(pos);
                }
            } else {
                // Not in list → add at end with ascending.
                self.sort_specs.push(SortSpec::asc(col_name));
            }
        } else {
            // Single-sort: if already single-sorted by this column, toggle; else replace.
            if self.sort_specs.len() == 1 && self.sort_specs[0].column == col_name {
                let new_dir = self.sort_specs[0].direction.toggle();
                self.sort_specs[0].direction = new_dir;
            } else {
                self.sort_specs = vec![SortSpec::asc(col_name)];
            }
        }
    }

    /// Clear all sorts.
    pub fn clear_sorts(&mut self) {
        self.sort_specs.clear();
    }

    /// Set filter text for a column. Empty string clears the filter.
    pub fn set_column_filter(&mut self, column: String, text: String) {
        if text.is_empty() {
            self.column_filters.remove(&column);
        } else {
            self.column_filters.insert(column, text);
        }
    }

    /// Get the sort priority number (1-based) for a column, if it's sorted.
    pub fn sort_priority(&self, col_name: &str) -> Option<usize> {
        self.sort_specs.iter().position(|s| s.column == col_name).map(|i| i + 1)
    }

    /// Get sort direction for a column, if it's sorted.
    pub fn sort_direction(&self, col_name: &str) -> Option<SortDirection> {
        self.sort_specs.iter().find(|s| s.column == col_name).map(|s| s.direction)
    }

    /// Whether any column has an active filter.
    pub fn has_active_filters(&self) -> bool {
        !self.column_filters.is_empty()
    }

    /// Compute first visible row from scroll position.
    pub fn first_row(&self) -> usize {
        (self.scroll_y / Self::ROW_HEIGHT) as usize
    }

    /// How many rows fit in the given viewport height.
    pub fn rows_in_viewport(&self, viewport_height: f32) -> usize {
        ((viewport_height - Self::total_header_height()) / Self::ROW_HEIGHT).ceil() as usize + 1
    }
}

impl Default for GridState {
    fn default() -> Self {
        Self::new()
    }
}

/// Actions emitted by the grid renderer that the app must handle.
#[derive(Debug, Clone)]
pub enum GridAction {
    /// User clicked a column header — update sort and re-query.
    /// `shift_held` indicates multi-sort modifier.
    SortRequested { column: String, shift_held: bool },
    /// User clicked to clear all sorts.
    SortCleared,
    /// User scrolled — update viewport and re-query if needed.
    ScrollChanged { first_row: usize },
    /// User typed in a column filter input — re-query.
    FilterColumnChanged { column: String, text: String },
    /// User selected rows.
    SelectionChanged(Selection),
}
