use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn to_sql(&self) -> &'static str {
        match self {
            SortDirection::Ascending => "ASC",
            SortDirection::Descending => "DESC",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }

    pub fn arrow_label(&self) -> &'static str {
        match self {
            SortDirection::Ascending => "↑",
            SortDirection::Descending => "↓",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortSpec {
    pub column: String,
    pub direction: SortDirection,
}

impl SortSpec {
    pub fn asc(column: impl Into<String>) -> Self {
        Self {
            column: column.into(),
            direction: SortDirection::Ascending,
        }
    }

    pub fn desc(column: impl Into<String>) -> Self {
        Self {
            column: column.into(),
            direction: SortDirection::Descending,
        }
    }

    pub fn to_sql(&self) -> String {
        format!(
            "\"{}\" {}",
            self.column.replace('"', "\"\""),
            self.direction.to_sql()
        )
    }
}
