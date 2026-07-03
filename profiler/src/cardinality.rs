use polars::prelude::*;

/// Cardinality analysis for a column.
pub struct CardinalityAnalysis {
    pub unique_count: usize,
    pub unique_pct: f64,
    pub total_count: usize,
    /// Classification: "identifier", "category", "high-cardinality", etc.
    pub classification: CardinalityClass,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CardinalityClass {
    /// Every value is unique → likely an identifier.
    Identifier,
    /// < 5% unique → category.
    Category,
    /// < 50% unique → moderate cardinality.
    Moderate,
    /// ≥ 50% unique → high cardinality.
    HighCardinality,
    /// All null.
    Empty,
}

impl CardinalityAnalysis {
    pub fn analyze(series: &Series) -> Self {
        let total_count = series.len();
        let unique_count = series.n_unique().unwrap_or(0);
        let unique_pct = if total_count > 0 {
            unique_count as f64 / total_count as f64
        } else {
            0.0
        };

        let unique_ratio = unique_pct;
        let classification = if total_count == 0 || unique_count == 0 {
            CardinalityClass::Empty
        } else if unique_count == total_count {
            CardinalityClass::Identifier
        } else if unique_ratio < 0.05 {
            CardinalityClass::Category
        } else if unique_ratio < 0.50 {
            CardinalityClass::Moderate
        } else {
            CardinalityClass::HighCardinality
        };

        Self {
            unique_count,
            unique_pct,
            total_count,
            classification,
        }
    }

    pub fn classification_label(&self) -> &'static str {
        match self.classification {
            CardinalityClass::Identifier => "Identifier",
            CardinalityClass::Category => "Category",
            CardinalityClass::Moderate => "Moderate",
            CardinalityClass::HighCardinality => "High",
            CardinalityClass::Empty => "Empty",
        }
    }
}
