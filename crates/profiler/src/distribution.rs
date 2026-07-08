use anyhow::{Context, Result};
use crate::types::Distribution;

/// Number of bins for histogram.
const DEFAULT_BINS: usize = 20;

/// Build a histogram distribution from a numeric series.
pub fn build_distribution(series: &polars::prelude::Series) -> Result<Distribution> {
    let min_val = series
        .min_reduce()
        .ok()
        .and_then(|s| crate::stats::scalar_to_f64(&s))
        .unwrap_or(0.0);
    let max_val = series
        .max_reduce()
        .ok()
        .and_then(|s| crate::stats::scalar_to_f64(&s))
        .unwrap_or(1.0);

    if (max_val - min_val).abs() < f64::EPSILON {
        return Ok(Distribution {
            bin_count: 1,
            bins: vec![1.0],
            edges: vec![min_val, min_val + 1.0],
        });
    }

    let range = max_val - min_val;
    let bin_width = range / DEFAULT_BINS as f64;

    let edges: Vec<f64> = (0..=DEFAULT_BINS)
        .map(|i| min_val + i as f64 * bin_width)
        .collect();

    // Collect non-null f64 values.
    let values: Vec<f64> = series
        .f64()
        .context("column is not f64")?
        .into_iter()
        .flatten()
        .collect();

    let mut bins = vec![0.0_f64; DEFAULT_BINS];
    for val in &values {
        let mut idx = ((val - min_val) / bin_width) as usize;
        if idx >= DEFAULT_BINS {
            idx = DEFAULT_BINS - 1;
        }
        bins[idx] += 1.0;
    }

    Ok(Distribution {
        bin_count: DEFAULT_BINS,
        bins,
        edges,
    })
}
