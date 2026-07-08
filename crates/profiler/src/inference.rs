use core::DataType;
use crate::types::SemanticType;

/// Infer a semantic type from column name and physical type.
pub fn infer_semantic_type(name: &str, dtype: &DataType) -> SemanticType {
    let name_lower = name.to_lowercase();

    // Boolean physical type → Boolean.
    if matches!(dtype, DataType::Boolean) {
        return SemanticType::Boolean;
    }

    // Date/timestamp physical types → Temporal.
    if matches!(
        dtype,
        DataType::Date32 | DataType::Date64 | DataType::TimestampMillis | DataType::TimestampMicros
    ) {
        return SemanticType::Temporal;
    }

    // Name-based heuristics (ordered by priority).
    let name_patterns: &[(&[&str], SemanticType)] = &[
        // Identifier patterns
        (
            &["id", "_id", "uuid", "guid", "key", "pk"],
            SemanticType::Identifier,
        ),
        // Temporal patterns
        (
            &[
                "date", "time", "timestamp", "datetime", "created", "updated",
                "modified", "deleted_at", "at", "_at", "year", "month", "day",
                "quarter", "dob", "birth",
            ],
            SemanticType::Temporal,
        ),
        // Geographic patterns
        (
            &[
                "country", "city", "state", "region", "province", "zip", "postal",
                "latitude", "longitude", "lat", "lon", "lng", "location",
                "address", "geo",
            ],
            SemanticType::Geographic,
        ),
        // Boolean name patterns
        (
            &[
                "is_", "has_", "flag", "active", "enabled", "bool", "deleted",
                "verified", "valid",
            ],
            SemanticType::Boolean,
        ),
        // Measure patterns
        (
            &[
                "amount", "price", "cost", "revenue", "sales", "total", "sum",
                "count", "qty", "quantity", "rate", "pct", "percent", "ratio",
                "score", "value", "weight", "height", "width", "length", "size",
                "age", "duration", "distance", "volume", "margin",
            ],
            SemanticType::Measure,
        ),
        // Category patterns
        (
            &[
                "type", "category", "status", "group", "class", "tier", "level",
                "grade", "rank", "segment", "channel", "source", "color",
                "gender", "role", "priority", "stage",
            ],
            SemanticType::Category,
        ),
    ];

    for (patterns, st) in name_patterns {
        for pat in *patterns {
            if name_lower == *pat || name_lower.starts_with(pat) || name_lower.ends_with(pat) {
                return st.clone();
            }
        }
    }

    // Fallback: use physical type heuristics.
    if dtype.is_numeric() {
        SemanticType::Measure
    } else {
        match dtype {
            DataType::Utf8 | DataType::LargeUtf8 => SemanticType::Category,
            _ => SemanticType::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identifier_patterns() {
        assert_eq!(
            infer_semantic_type("customer_id", &DataType::Int64),
            SemanticType::Identifier
        );
        assert_eq!(
            infer_semantic_type("id", &DataType::Utf8),
            SemanticType::Identifier
        );
    }

    #[test]
    fn test_temporal_patterns() {
        assert_eq!(
            infer_semantic_type("created_at", &DataType::Utf8),
            SemanticType::Temporal
        );
        assert_eq!(
            infer_semantic_type("order_date", &DataType::Utf8),
            SemanticType::Temporal
        );
        assert_eq!(
            infer_semantic_type("created_at", &DataType::Date32),
            SemanticType::Temporal
        );
    }

    #[test]
    fn test_measure_fallback() {
        assert_eq!(
            infer_semantic_type("revenue", &DataType::Float64),
            SemanticType::Measure
        );
    }

    #[test]
    fn test_geographic() {
        assert_eq!(
            infer_semantic_type("country", &DataType::Utf8),
            SemanticType::Geographic
        );
    }
}
