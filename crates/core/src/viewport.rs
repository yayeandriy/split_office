use serde::{Deserialize, Serialize};

/// Describes the window of rows currently visible in the grid.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Viewport {
    /// Index of the first visible row in the filtered result set.
    pub first_row: usize,
    /// How many rows are visible in the viewport.
    pub visible_rows: usize,
}

impl Viewport {
    pub fn new(first_row: usize, visible_rows: usize) -> Self {
        Self {
            first_row,
            visible_rows,
        }
    }

    pub fn last_row(&self) -> usize {
        self.first_row + self.visible_rows
    }

    /// Clamp first_row so we never scroll past the end of the dataset.
    pub fn clamped(mut self, total_rows: usize) -> Self {
        let max_first = total_rows.saturating_sub(self.visible_rows);
        self.first_row = self.first_row.min(max_first);
        self
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            first_row: 0,
            visible_rows: 50,
        }
    }
}
