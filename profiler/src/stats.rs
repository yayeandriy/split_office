use anyhow::Result;
use polars::prelude::*;
use crate::types::ColumnProfile;

/// Extract an f64 value from a Polars Scalar.
pub fn scalar_to_f64(s: &polars::prelude::Scalar) -> Option<f64> {
    match s.value() {
        polars::prelude::AnyValue::Float64(v) => Some(*v),
        polars::prelude::AnyValue::Float32(v) => Some(*v as f64),
        polars::prelude::AnyValue::Int64(v) => Some(*v as f64),
        polars::prelude::AnyValue::Int32(v) => Some(*v as f64),
        polars::prelude::AnyValue::UInt32(v) => Some(*v as f64),
        polars::prelude::AnyValue::UInt64(v) => Some(*v as f64),
        polars::prelude::AnyValue::Null => None,
        _ => None,
    }
}

/// Compute full statistical profile for a column.
pub fn compute_column_stats(series: &Series, name: &str) -> ColumnProfile {
    let count = series.len();
    let null_count = series.null_count();
    let null_pct = if count > 0 {
        null_count as f64 / count as f64
    } else {
        0.0
    };

    let unique_count = series.n_unique().unwrap_or(count);
    let unique_pct = if count > 0 {
        unique_count as f64 / count as f64
    } else {
        0.0
    };

    let is_numeric = series.dtype().is_primitive_numeric();

    let (min, max, mean, median, variance, stddev) = if is_numeric {
        let min = series.min_reduce().ok().and_then(|s| scalar_to_f64(&s));
        let max = series.max_reduce().ok().and_then(|s| scalar_to_f64(&s));
        let mean = series.mean();
        let median_val = series.median();
        let var = series.var(1);
        let std = series.std(1);
        (min, max, mean, median_val, var, std)
    } else {
        (None, None, None, None, None, None)
    };

    ColumnProfile {
        name: name.to_string(),
        physical_type: format!("{}", series.dtype()),
        semantic_type: crate::inference::infer_semantic_type(
            name,
            &crate::arrow_to_core_dtype(series.dtype()),
        ),
        count,
        null_count,
        null_pct,
        unique_count,
        unique_pct,
        min,
        max,
        mean,
        median,
        variance,
        stddev,
        p1: None,
        p5: None,
        p25: None,
        p50: None,
        p75: None,
        p95: None,
        p99: None,
        distribution: None,
        outlier_count: None,
        outlier_lower: None,
        outlier_upper: None,
        top_correlations: Vec::new(),
    }
}

/// Compute quantiles for a numeric series using Polars' quantile_reduce.
pub fn compute_quantiles(series: &Series) -> Result<Vec<Option<f64>>> {
    use polars::prelude::QuantileMethod;

    let probs = [0.01, 0.05, 0.25, 0.50, 0.75, 0.95, 0.99];
    let mut out = Vec::with_capacity(probs.len());

    for &p in &probs {
        let q = series
            .quantile_reduce(p, QuantileMethod::Linear)
            .map_err(|e| anyhow::anyhow!("quantile error for p={p}: {e}"))?;
        out.push(scalar_to_f64(&q));
    }

    Ok(out)
}

/// Apply quantiles to a ColumnProfile.
pub fn apply_quantiles(profile: &mut ColumnProfile, quantiles: &[Option<f64>]) {
    if quantiles.len() >= 7 {
        profile.p1 = quantiles[0];
        profile.p5 = quantiles[1];
        profile.p25 = quantiles[2];
        profile.p50 = quantiles[3];
        profile.p75 = quantiles[4];
        profile.p95 = quantiles[5];
        profile.p99 = quantiles[6];
        // Use p50 as median if not already set.
        if profile.median.is_none() {
            profile.median = quantiles[3];
        }
    }
}
