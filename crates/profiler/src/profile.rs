use anyhow::Result;
use polars::prelude::*;
use tracing::instrument;

use crate::correlation;
use crate::distribution;
use crate::outliers;
use crate::relationships;
use crate::stats;
use crate::types::*;

/// Profile a dataset from its Polars DataFrame.
#[instrument(skip(df))]
pub fn profile_dataframe(df: &DataFrame, dataset_name: &str) -> Result<DatasetProfile> {
    let row_count = df.height();
    let columns: Vec<&Series> = df.get_columns().iter().filter_map(|c| c.as_series()).collect();

    let mut column_profiles = Vec::with_capacity(columns.len());

    for col in &columns {
        let name = col.name().to_string();
        let is_numeric = col.dtype().is_primitive_numeric();

        let mut profile = stats::compute_column_stats(col, &name);

        // Quantiles (numeric only).
        if is_numeric {
            if let Ok(quantiles) = stats::compute_quantiles(col) {
                stats::apply_quantiles(&mut profile, &quantiles);
            }

            // Distribution.
            if let Ok(dist) = distribution::build_distribution(col) {
                profile.distribution = Some(dist);
            }

            // Outliers.
            if let Some(oa) = outliers::OutlierAnalysis::detect(col) {
                profile.outlier_count = Some(oa.total_outliers());
                profile.outlier_lower = Some(oa.lower_fence);
                profile.outlier_upper = Some(oa.upper_fence);
            }
        }

        column_profiles.push(profile);
    }

    // Correlation matrix.
    let correlation_matrix = correlation::compute_correlation_matrix(df);

    // Fill in top correlations per column.
    if let Some(ref matrix) = correlation_matrix {
        for profile in &mut column_profiles {
            let pairs = correlation::top_correlations(matrix, &profile.name, 3);
            profile.top_correlations = pairs
                .into_iter()
                .map(|p| (p.col_b, p.coefficient))
                .collect();
        }
    }

    // Relationships.
    let relationships = relationships::discover_relationships(df);

    // Quality assessment.
    let quality = assess_quality(&column_profiles, row_count);

    Ok(DatasetProfile {
        dataset_name: dataset_name.to_string(),
        row_count,
        column_count: column_profiles.len(),
        columns: column_profiles,
        correlation_matrix,
        relationships,
        quality,
    })
}

/// Assess overall dataset quality from column profiles.
fn assess_quality(columns: &[ColumnProfile], row_count: usize) -> DatasetQuality {
    let mut issues = Vec::new();
    let mut high_null_columns = Vec::new();
    let mut constant_columns = Vec::new();

    for col in columns {
        if col.null_pct > 0.9 {
            high_null_columns.push(col.name.clone());
            issues.push(format!("Column '{}' has {:.0}% nulls", col.name, col.null_pct * 100.0));
        }
        if col.unique_count <= 1 && col.count > 1 {
            constant_columns.push(col.name.clone());
            issues.push(format!("Column '{}' is constant", col.name));
        }
        if let Some(outlier_count) = col.outlier_count {
            if outlier_count > 0 {
                let pct = outlier_count as f64 / col.count as f64 * 100.0;
                if pct > 5.0 {
                    issues.push(format!(
                        "Column '{}' has {:.1}% outliers",
                        col.name, pct
                    ));
                }
            }
        }
    }

    // Score: start at 1.0, deduct for issues.
    let mut score = 1.0;
    score -= high_null_columns.len() as f64 * 0.1;
    score -= constant_columns.len() as f64 * 0.05;
    score = score.max(0.0);

    let duplicate_pct = if row_count > 0 && !columns.is_empty() {
        // Estimate: if avg unique ratio is low, likely duplicates.
        let avg_unique: f64 = columns
            .iter()
            .map(|c| c.unique_pct)
            .sum::<f64>() / columns.len() as f64;
        (1.0 - avg_unique).max(0.0)
    } else {
        0.0
    };

    DatasetQuality {
        score,
        high_null_columns,
        constant_columns,
        duplicate_pct,
        issues,
    }
}
