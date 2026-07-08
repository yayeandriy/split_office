use polars::prelude::*;

/// Outlier detection using the IQR method.
pub struct OutlierAnalysis {
    /// Count of values below Q1 - 1.5*IQR.
    pub low_outliers: usize,
    /// Count of values above Q3 + 1.5*IQR.
    pub high_outliers: usize,
    /// Lower fence: Q1 - 1.5*IQR.
    pub lower_fence: f64,
    /// Upper fence: Q3 + 1.5*IQR.
    pub upper_fence: f64,
    /// First quartile.
    pub q1: f64,
    /// Third quartile.
    pub q3: f64,
}

impl OutlierAnalysis {
    /// Detect outliers using IQR method. Returns None for non-numeric columns.
    pub fn detect(series: &Series) -> Option<Self> {
        if !series.dtype().is_primitive_numeric() {
            return None;
        }

        let arr = series.f64().ok()?;

        // Collect non-null values.
        let values: Vec<f64> = arr.into_iter().flatten().collect();
        if values.len() < 4 {
            return None;
        }

        let mut sorted = values.clone();
        sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let _n = sorted.len();
        let q1 = percentile(&sorted, 0.25);
        let q3 = percentile(&sorted, 0.75);
        let iqr = q3 - q1;

        let lower_fence = q1 - 1.5 * iqr;
        let upper_fence = q3 + 1.5 * iqr;

        let low_outliers = values.iter().filter(|&&v| v < lower_fence).count();
        let high_outliers = values.iter().filter(|&&v| v > upper_fence).count();

        Some(Self {
            low_outliers,
            high_outliers,
            lower_fence,
            upper_fence,
            q1,
            q3,
        })
    }

    pub fn total_outliers(&self) -> usize {
        self.low_outliers + self.high_outliers
    }

    pub fn outlier_pct(&self, total: usize) -> f64 {
        if total == 0 {
            0.0
        } else {
            self.total_outliers() as f64 / total as f64
        }
    }
}

/// Compute the p-th percentile from a sorted slice.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = p * (sorted.len() - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    if lo == hi {
        return sorted[lo];
    }
    let frac = idx - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}
