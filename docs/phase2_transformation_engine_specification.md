# Phase 2 — Transformation Engine
## Detailed Implementation Specification

Version: 1.0

## Mission

Transform the application from a Dataset Microscope into a Dataset Workshop.

Users must be able to create reproducible analytical workflows using:

- filtering
- sorting
- joins
- grouping
- aggregations
- derived columns
- workflow persistence
- lineage tracking
- undo / redo

while maintaining million-row scalability and lazy execution.

---

# Architecture

New crates:

workspace/
├── transform/
├── workflow/
├── expression/
├── lineage/
├── project/

Responsibilities:

transform:
- transformation definitions
- validation
- execution abstraction

workflow:
- workflow graph
- execution ordering
- persistence

expression:
- parsing
- validation
- expression trees

lineage:
- dependency tracking
- provenance
- impact analysis

project:
- AWB project format
- serialization
- loading/saving

---

# Core Concepts

Dataset = immutable source

Transformation = operation on a dataset

Workflow = ordered transformation pipeline

View = workflow result

Example:

sales.parquet
↓
Filter(country == "US")
↓
GroupBy(product)
↓
Aggregate(sum(revenue))
↓
Sort(desc(revenue))

---

# User Stories

## Filtering

- numeric filters
- categorical filters
- compound filters
- filter removal

Tests:

- correctness
- latency
- memory
- responsiveness

Datasets:

100k
1M
10M rows

---

## Sorting

- ascending
- descending
- multi-column sorting

Tests:

- execution latency
- viewport responsiveness
- memory stability

---

## Derived Columns

Examples:

Profit = Revenue - Cost

Margin = Profit / Revenue

Year = created_at.year()

CountryCode = country.upper()

Requirements:

- validation
- lineage generation
- lazy execution

---

## Aggregation

Group by:

- single column
- multiple columns

Aggregates:

- sum
- count
- avg
- min
- max
- median

---

## Joins

Support:

- inner join
- left join
- multi-column joins

Validate:

- missing keys
- schema mismatch
- type mismatch

Benchmark:

1M + 1M row datasets

---

# Workflow Management

Users can:

- add transformations
- remove transformations
- reorder transformations
- enable/disable transformations

Workflow updates must be deterministic.

---

# Undo / Redo

Requirements:

- 100+ sequential operations
- state consistency
- no corruption

---

# Project Format (.awb)

Store:

- dataset references
- workflow graph
- profiles
- UI state
- preferences

Never store full datasets.

---

# Lineage System

Every derived artifact tracks origin.

Example:

Profit
↓
Revenue
Cost

Required:

- column lineage
- workflow lineage
- impact analysis

Example:

Deleting Revenue should show all dependent columns.

---

# Workflow Sidebar

Visual structure:

Dataset
├─ Filter
├─ Derived Column
├─ Aggregate
├─ Sort

States:

- enabled
- disabled
- warning
- error

---

# Transformation Editor

Every transformation displays:

- inputs
- outputs
- preview
- validation state
- execution cost

---

# Preview System

Show:

- rows before
- rows after
- added columns
- removed columns
- changed types

---

# Performance Targets

Perceived UI response:

< 100 ms

Operations:

- lazy whenever possible
- avoid full recomputation
- preserve viewport smoothness

---

# Benchmark Suite

Datasets:

A:
100k rows
20 columns

B:
1M rows
50 columns

C:
10M rows
100 columns

Scenarios:

- filter
- sort
- aggregate
- join
- derived column
- workflow load
- project load
- undo
- redo

Metrics:

- FPS
- frame time
- memory
- CPU
- query latency
- workflow execution time

---

# Failure Scenarios

Must handle:

- missing datasets
- renamed datasets
- schema changes
- broken joins
- invalid expressions
- corrupted projects

Requirements:

- no crashes
- actionable errors
- workflow remains inspectable

---

# Quality Gates

Before completion:

✓ filtering

✓ sorting

✓ joins

✓ aggregations

✓ derived columns

✓ workflow persistence

✓ undo / redo

✓ lineage

✓ benchmark suite

✓ performance targets

✓ stable million-row datasets

---

# End State

The application becomes a reproducible analytical environment.

Users can build analytical workflows without code while retaining:

- transparency
- reproducibility
- performance
- scalability

This execution model becomes the foundation for agents, workflow graphs, notebooks, forecasting, and advanced analytics.
