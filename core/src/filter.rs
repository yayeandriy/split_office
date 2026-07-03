use serde::{Deserialize, Serialize};

/// Filter expression tree.
///
/// Intentionally kept simple for Phase 0. DuckDB translates these to SQL WHERE clauses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterExpr {
    /// Always true — no filtering.
    None,
    Eq { column: String, value: FilterValue },
    Gt { column: String, value: FilterValue },
    Lt { column: String, value: FilterValue },
    Gte { column: String, value: FilterValue },
    Lte { column: String, value: FilterValue },
    Contains { column: String, pattern: String },
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterValue {
    Integer(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
}

impl FilterExpr {
    /// Render the expression as a SQL WHERE fragment (no "WHERE" keyword).
    /// Returns empty string for `FilterExpr::None`.
    pub fn to_sql(&self) -> String {
        match self {
            FilterExpr::None => String::new(),
            FilterExpr::Eq { column, value } => {
                format!("{} = {}", quote_ident(column), value.to_sql())
            }
            FilterExpr::Gt { column, value } => {
                format!("{} > {}", quote_ident(column), value.to_sql())
            }
            FilterExpr::Lt { column, value } => {
                format!("{} < {}", quote_ident(column), value.to_sql())
            }
            FilterExpr::Gte { column, value } => {
                format!("{} >= {}", quote_ident(column), value.to_sql())
            }
            FilterExpr::Lte { column, value } => {
                format!("{} <= {}", quote_ident(column), value.to_sql())
            }
            FilterExpr::Contains { column, pattern } => {
                format!(
                    "{} ILIKE '%{}%'",
                    quote_ident(column),
                    pattern.replace('\'', "''")
                )
            }
            FilterExpr::And(a, b) => format!("({} AND {})", a.to_sql(), b.to_sql()),
            FilterExpr::Or(a, b) => format!("({} OR {})", a.to_sql(), b.to_sql()),
            FilterExpr::Not(inner) => format!("NOT ({})", inner.to_sql()),
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, FilterExpr::None)
    }
}

impl FilterValue {
    pub fn to_sql(&self) -> String {
        match self {
            FilterValue::Integer(n) => n.to_string(),
            FilterValue::Float(f) => f.to_string(),
            FilterValue::Text(s) => format!("'{}'", s.replace('\'', "''")),
            FilterValue::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        }
    }
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
