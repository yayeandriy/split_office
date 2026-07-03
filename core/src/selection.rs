use serde::{Deserialize, Serialize};

/// Which rows are currently selected in the grid.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Selection {
    /// Indices into the *filtered* row set (not the raw dataset).
    pub rows: Vec<usize>,
}

impl Selection {
    pub fn empty() -> Self {
        Self { rows: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn contains(&self, row: usize) -> bool {
        self.rows.contains(&row)
    }

    pub fn select_single(&mut self, row: usize) {
        self.rows.clear();
        self.rows.push(row);
    }

    pub fn toggle(&mut self, row: usize) {
        if let Some(pos) = self.rows.iter().position(|&r| r == row) {
            self.rows.remove(pos);
        } else {
            self.rows.push(row);
        }
    }

    pub fn clear(&mut self) {
        self.rows.clear();
    }
}
