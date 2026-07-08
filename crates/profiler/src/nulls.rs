use polars::prelude::*;

/// Analyze null patterns in a series.
pub struct NullAnalysis {
    pub null_count: usize,
    pub null_pct: f64,
    /// Number of non-null values.
    pub non_null_count: usize,
}

impl NullAnalysis {
    pub fn analyze(series: &Series) -> Self {
        let len = series.len();
        let null_count = series.null_count();
        let null_pct = if len > 0 {
            null_count as f64 / len as f64
        } else {
            0.0
        };
        Self {
            null_count,
            null_pct,
            non_null_count: len - null_count,
        }
    }

    /// Render as a 10-char bar: "███████░░░"
    pub fn bar(&self) -> String {
        let filled = ((1.0 - self.null_pct) * 10.0).round() as usize;
        let empty = 10 - filled;
        "█".repeat(filled) + &"░".repeat(empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_bar() {
        let s = Series::new("test".into(), &[Some(1), None, Some(3), Some(4)]);
        let na = NullAnalysis::analyze(&s);
        assert_eq!(na.null_count, 1);
        assert!((na.null_pct - 0.25).abs() < 0.01);
    }
}
