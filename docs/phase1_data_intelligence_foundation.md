# Phase 1 — Data Intelligence Foundation

## Mission

Transform the application from a high-performance dataset viewer into a true analytical explorer.

Phase 0 proved that the architecture works:

- Rust
- egui
- wgpu
- DuckDB
- Polars
- Arrow

The objective of Phase 1 is different.

The system must begin understanding data rather than merely rendering it.

The application should automatically discover:

- column types
- distributions
- null patterns
- cardinality
- correlations
- outliers
- dataset quality indicators

without requiring user-defined analysis.

---

# Primary Goal

Move from:

Dataset Viewer

to:

Analytical Explorer

A user should be able to open a dataset and immediately understand:

- what the data contains
- which columns are important
- which columns are problematic
- where anomalies exist
- how variables relate to one another

---

# Architecture Changes

Add a new workspace crate:

workspace/
├── profiler/

Responsibilities:

- dataset profiling
- type inference
- statistics
- correlation analysis
- metadata generation
- semantic classification

---

# Core Types

DatasetProfile
ColumnProfile
SemanticType
Distribution
CorrelationMatrix
DatasetQuality

---

# Module 1 — Column Type Inference

Goals:

- infer semantic types
- do not trust imported schemas
- distinguish identifiers, measures, dates and categories

Examples:

customer_id -> Identifier
revenue -> Measure
created_at -> Temporal
country -> Category

---

# Module 2 — Statistical Profiling

For numeric columns compute:

- count
- null count
- unique count
- mean
- median
- variance
- stddev
- min
- max

Quantiles:

- p1
- p5
- p25
- p50
- p75
- p95
- p99

---

# Module 3 — Distribution Engine

Generate compact histogram summaries.

Examples:

▁▂▃▄▅▆▇█

Every numeric column receives:

- histogram bins
- density approximation
- quantile summary

---

# Module 4 — Null Analysis

Compute:

- null count
- null percentage

Visual summaries:

███████░

---

# Module 5 — Cardinality Analysis

Compute:

- unique count
- unique ratio

Distinguish:

- country-like columns
- identifier-like columns

---

# Module 6 — Semantic Classification

Categories:

- Identifier
- Measure
- Temporal
- Category
- Geographic
- Boolean

---

# Module 7 — Outlier Detection

Implement:

- IQR method
- optional z-score method

Store outlier counts and thresholds.

---

# Module 8 — Correlation Engine

Compute correlations for numeric columns.

Examples:

Revenue ↔ Orders
Revenue ↔ Visits
Revenue ↔ Discounts

Generate correlation matrix metadata.

---

# Module 9 — Relationship Discovery

Detect:

- foreign-key-like relationships
- hierarchies
- repeated structures

Examples:

country -> city
customer_id -> order.customer_id

---

# Module 10 — Inline Visualization Library

Reusable widgets:

- histogram sparkline
- missing-value indicator
- trend indicator

These become the visual language of the application.

---

# Module 11 — Dataset Overview Panel

Display:

- row count
- column count
- type breakdown
- missing values
- detected issues
- detected relationships

---

# Module 12 — Column Inspector

Display:

- name
- physical type
- semantic type
- nulls
- unique values
- statistics
- distribution
- outliers
- correlations

---

# Background Execution Model

Requirements:

- background workers
- progress reporting
- cancellation
- incremental updates

Flow:

Dataset Load
↓
Grid Available
↓
Profiler Starts
↓
Progressive Results

---

# Performance Targets

100k rows: <1 second

1M rows: <5 seconds

10M rows: progressive computation

UI must remain responsive at all times.

---

# Non-Goals

Do not build yet:

- formulas
- dashboards
- charting system
- AI chat
- agents
- node graphs
- collaboration

---

# Acceptance Criteria

- automatic profiling
- semantic classification
- statistical summaries
- distributions
- outlier detection
- correlation analysis
- dataset overview
- column inspector
- non-blocking execution
- responsive 1M+ datasets

---

# End State

The application should behave like a dataset microscope.

Users open a dataset and immediately see:

- structure
- quality
- distributions
- anomalies
- relationships
- statistical context

This semantic layer becomes the foundation for AI agents, transformation graphs, forecasting, statistical modeling, and future analytical workflows.
