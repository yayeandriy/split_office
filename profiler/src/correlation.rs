use polars::prelude::*;
use crate::types::{CorrelationMatrix, CorrelationPair};

/// Compute Pearson correlation matrix for all numeric columns in a DataFrame.
pub fn compute_correlation_matrix(df: &DataFrame) -> Option<CorrelationMatrix> {
    let numeric_cols: Vec<String> = df
        .get_columns()
        .iter()
        .filter(|s| s.dtype().is_primitive_numeric())
        .map(|s| s.name().to_string())
        .collect();

    if numeric_cols.len() < 2 {
        return None;
    }

    let n = numeric_cols.len();
    let mut coefficients = Vec::with_capacity(n * n);

    // Compute pairwise Pearson correlation.
    for i in 0..n {
        for j in 0..n {
            if i == j {
                coefficients.push(1.0);
            } else if j < i {
                // Mirror the upper triangle.
                let mirror_idx = j * n + i;
                coefficients.push(coefficients[mirror_idx]);
            } else {
                let col_a = &numeric_cols[i];
                let col_b = &numeric_cols[j];
                let corr = pearson_correlation(
                    df.column(col_a).ok().and_then(|c| c.as_series()),
                    df.column(col_b).ok().and_then(|c| c.as_series()),
                );
                coefficients.push(corr);
            }
        }
    }

    Some(CorrelationMatrix {
        columns: numeric_cols,
        coefficients,
    })
}

/// Compute Pearson correlation between two numeric series.
fn pearson_correlation(a: Option<&Series>, b: Option<&Series>) -> f64 {
    let (Some(a), Some(b)) = (a, b) else {
        return 0.0;
    };

    let a_vals: Vec<f64> = a.f64().ok().into_iter().flatten().flatten().collect();
    let b_vals: Vec<f64> = b.f64().ok().into_iter().flatten().flatten().collect();

    // Align by index (only pairs where both are present).
    let pairs: Vec<(f64, f64)> = a_vals
        .iter()
        .zip(b_vals.iter())
        .take(a_vals.len().min(b_vals.len()))
        .map(|(&a, &b)| (a, b))
        .collect();

    if pairs.len() < 3 {
        return 0.0;
    }

    let n = pairs.len() as f64;
    let sum_x: f64 = pairs.iter().map(|(x, _)| x).sum();
    let sum_y: f64 = pairs.iter().map(|(_, y)| y).sum();
    let sum_xy: f64 = pairs.iter().map(|(x, y)| x * y).sum();
    let sum_x2: f64 = pairs.iter().map(|(x, _)| x * x).sum();
    let sum_y2: f64 = pairs.iter().map(|(_, y)| y * y).sum();

    let numerator = n * sum_xy - sum_x * sum_y;
    let denom_a = (n * sum_x2 - sum_x * sum_x).sqrt();
    let denom_b = (n * sum_y2 - sum_y * sum_y).sqrt();
    let denominator = denom_a * denom_b;

    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

/// Get top correlations for a specific column from the matrix.
pub fn top_correlations(
    matrix: &CorrelationMatrix,
    column: &str,
    top_n: usize,
) -> Vec<CorrelationPair> {
    let col_idx = matrix.columns.iter().position(|c| c == column);
    let Some(ci) = col_idx else {
        return Vec::new();
    };

    let n = matrix.columns.len();
    let mut pairs: Vec<CorrelationPair> = (0..n)
        .filter(|&i| i != ci)
        .map(|i| CorrelationPair {
            col_a: column.to_string(),
            col_b: matrix.columns[i].clone(),
            coefficient: matrix.coefficients[ci * n + i],
        })
        .collect();

    pairs.sort_by(|a, b| b.coefficient.abs().partial_cmp(&a.coefficient.abs()).unwrap_or(std::cmp::Ordering::Equal));
    pairs.truncate(top_n);
    pairs
}
