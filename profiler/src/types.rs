use serde::{Deserialize, Serialize};

/// Semantic classification of a column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticType {
    /// Primary/foreign key, unique row identifier (e.g. `id`, `customer_id`).
    Identifier,
    /// Quantitative value suitable for aggregation (e.g. `revenue`, `age`).
    Measure,
    /// Date or timestamp (e.g. `created_at`, `order_date`).
    Temporal,
    /// Low-cardinality categorical value (e.g. `country`, `status`).
    Category,
    /// Geographic location (e.g. `city`, `region`, `lat`/`lon`).
    Geographic,
    /// True/false binary flag.
    Boolean,
    /// Free-text (e.g. `description`, `notes`).
    Text,
    /// Could not be classified.
    Unknown,
}

impl SemanticType {
    pub fn label(&self) -> &'static str {
        match self {
            SemanticType::Identifier => "Identifier",
            SemanticType::Measure => "Measure",
            SemanticType::Temporal => "Temporal",
            SemanticType::Category => "Category",
            SemanticType::Geographic => "Geographic",
            SemanticType::Boolean => "Boolean",
            SemanticType::Text => "Text",
            SemanticType::Unknown => "Unknown",
        }
    }

    /// Short ASCII-safe icon label for this semantic type.
    /// The UI layer renders this with appropriate icon fonts.
    pub fn icon_label(&self) -> &'static str {
        match self {
            SemanticType::Identifier => "◈",
            SemanticType::Measure => "▤",
            SemanticType::Temporal => "◷",
            SemanticType::Category => "▥",
            SemanticType::Geographic => "◎",
            SemanticType::Boolean => "✓",
            SemanticType::Text => "≡",
            SemanticType::Unknown => "?",
        }
    }
}

/// Compact histogram summary for a numeric column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Distribution {
    /// Number of bins.
    pub bin_count: usize,
    /// Normalized bin heights (0.0–1.0).
    pub bins: Vec<f64>,
    /// Bin edge values (len = bin_count + 1).
    pub edges: Vec<f64>,
}

impl Distribution {
    /// Render as a sparkline string (▁▂▃▄▅▆▇█).
    pub fn sparkline(&self) -> String {
        const CHARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        if self.bins.is_empty() {
            return String::new();
        }
        let max = self.bins.iter().cloned().fold(0.0_f64, f64::max);
        if max == 0.0 {
            return "▁".repeat(self.bins.len());
        }
        self.bins
            .iter()
            .map(|&h| {
                let idx = ((h / max) * 7.0).round() as usize;
                CHARS[idx.min(7)]
            })
            .collect()
    }
}

/// Per-column statistical profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnProfile {
    pub name: String,
    pub physical_type: String,
    pub semantic_type: SemanticType,

    // Counts
    pub count: usize,
    pub null_count: usize,
    pub null_pct: f64,
    pub unique_count: usize,
    pub unique_pct: f64,

    // Numeric stats (None for non-numeric)
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub mean: Option<f64>,
    pub median: Option<f64>,
    pub variance: Option<f64>,
    pub stddev: Option<f64>,

    // Quantiles
    pub p1: Option<f64>,
    pub p5: Option<f64>,
    pub p25: Option<f64>,
    pub p50: Option<f64>,
    pub p75: Option<f64>,
    pub p95: Option<f64>,
    pub p99: Option<f64>,

    // Distribution (numeric only)
    pub distribution: Option<Distribution>,

    // Outliers
    pub outlier_count: Option<usize>,
    pub outlier_lower: Option<f64>,
    pub outlier_upper: Option<f64>,

    // Correlations (column_name → coefficient)
    pub top_correlations: Vec<(String, f64)>,
}

/// Pairwise correlation coefficient.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationPair {
    pub col_a: String,
    pub col_b: String,
    pub coefficient: f64,
}

/// Full correlation matrix for numeric columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationMatrix {
    pub columns: Vec<String>,
    /// Flattened upper-triangular matrix: (row * n + col) for row < col.
    pub coefficients: Vec<f64>,
}

/// Detected relationship between columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from_column: String,
    pub to_column: String,
    pub kind: RelationshipKind,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipKind {
    /// from_column values all appear in to_column (foreign-key-like).
    ForeignKey,
    /// from_column is a parent category of to_column.
    Hierarchy,
    /// Columns share significant value overlap.
    Repeated,
}

/// Overall dataset quality score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetQuality {
    /// 0.0–1.0 overall quality score.
    pub score: f64,
    /// Columns with >90% nulls.
    pub high_null_columns: Vec<String>,
    /// Columns with zero variance (constant).
    pub constant_columns: Vec<String>,
    /// Duplicate row estimate.
    pub duplicate_pct: f64,
    /// Issues found.
    pub issues: Vec<String>,
}

/// Complete profile for a dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetProfile {
    pub dataset_name: String,
    pub row_count: usize,
    pub column_count: usize,
    pub columns: Vec<ColumnProfile>,
    pub correlation_matrix: Option<CorrelationMatrix>,
    pub relationships: Vec<Relationship>,
    pub quality: DatasetQuality,
}

impl DatasetProfile {
    /// Find a column profile by name.
    pub fn column(&self, name: &str) -> Option<&ColumnProfile> {
        self.columns.iter().find(|c| c.name == name)
    }
}

impl ColumnProfile {
    /// Null indicator bar: "███████░"
    pub fn null_bar(&self) -> String {
        let filled = ((1.0 - self.null_pct) * 10.0).round() as usize;
        let empty = 10 - filled;
        "█".repeat(filled) + &"░".repeat(empty)
    }
}
