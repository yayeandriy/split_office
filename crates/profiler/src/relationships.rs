use polars::prelude::*;
use crate::types::{Relationship, RelationshipKind};

/// Discover relationships between columns.
///
/// Currently detects:
/// - Foreign-key-like relationships (all values of col A appear in col B).
/// - Hierarchies (col A cardinality < col B cardinality, high overlap).
pub fn discover_relationships(df: &DataFrame) -> Vec<Relationship> {
    let raw_columns = df.get_columns();
    let columns: Vec<&Series> = raw_columns.iter().filter_map(|c| c.as_series()).collect();
    let mut relationships = Vec::new();

    for i in 0..columns.len() {
        for j in (i + 1)..columns.len() {
            let col_a = &columns[i];
            let col_b = &columns[j];

            let a_unique = col_a.n_unique().unwrap_or(0) as f64;
            let b_unique = col_b.n_unique().unwrap_or(0) as f64;
            let a_total = col_a.len() as f64;

            if a_total == 0.0 || b_unique == 0.0 {
                continue;
            }

            let a_ratio = a_unique / a_total;

            // Foreign key detection: A has many unique values that are a subset of B.
            if a_unique > 1.0 && a_unique < a_total && a_unique <= b_unique {
                // Quick overlap check: sample-based.
                let overlap = estimate_overlap(col_a, col_b);
                if overlap > 0.85 && a_ratio > 0.1 {
                    relationships.push(Relationship {
                        from_column: col_a.name().to_string(),
                        to_column: col_b.name().to_string(),
                        kind: RelationshipKind::ForeignKey,
                        confidence: overlap,
                    });
                }
            }

            // Hierarchy: A cardinality < B, names suggest parent-child.
            if a_unique < b_unique
                && a_ratio < 0.3
                && a_unique > 1.0
            {
                let overlap = estimate_overlap(col_a, col_b);
                if overlap > 0.7 {
                    relationships.push(Relationship {
                        from_column: col_a.name().to_string(),
                        to_column: col_b.name().to_string(),
                        kind: RelationshipKind::Hierarchy,
                        confidence: overlap,
                    });
                }
            }

            // Repeated: very high overlap (nearly identical columns).
            if (a_unique - b_unique).abs() / a_total.max(1.0) < 0.02 && a_unique > 1.0 {
                let overlap = estimate_overlap(col_a, col_b);
                if overlap > 0.95 {
                    relationships.push(Relationship {
                        from_column: col_a.name().to_string(),
                        to_column: col_b.name().to_string(),
                        kind: RelationshipKind::Repeated,
                        confidence: overlap,
                    });
                }
            }
        }
    }

    relationships
}

/// Estimate overlapping values between two columns using sampling.
fn estimate_overlap(a: &Series, b: &Series) -> f64 {
    use std::collections::HashSet;

    let a_strs: Vec<String> = a
        .iter()
        .take(500)
        .map(|v| format!("{}", v))
        .collect();

    let b_set: HashSet<String> = b
        .iter()
        .take(1000)
        .map(|v| format!("{}", v))
        .collect();

    if a_strs.is_empty() {
        return 0.0;
    }

    let overlap = a_strs.iter().filter(|s| b_set.contains(*s)).count();
    overlap as f64 / a_strs.len() as f64
}
