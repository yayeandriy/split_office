
# Analytical Workbench Constitution

## Purpose

This document defines the architectural, engineering, UX, and product principles of the project.

The goal is to prevent architectural drift while the codebase is still small.

Every major technical decision should be evaluated against this constitution.

---

# Core Vision

The system is not a spreadsheet.

The system is a local-first analytical workbench.

Primary concepts:

- Dataset
- Column
- Transformation
- Analysis
- Visualization
- Agent

The grid is only one representation of data.

---

# Foundational Principles

## Dataset First

Prefer:

Dataset → Column → Transformation → View

Avoid:

Workbook → Sheet → Cell

---

## Local First

Execute locally whenever practical.

Benefits:

- privacy
- responsiveness
- offline support
- predictable costs

---

## Semantic First

The application should understand data.

Every dataset should expose:

- semantic types
- distributions
- quality metrics
- relationships
- correlations

---

## Scientific Over Decorative

Prioritize:

- distributions
- uncertainty
- significance
- correlations

Avoid dashboard-centric thinking.

---

## Reproducibility

Every operation must be inspectable and reproducible.

No hidden transformations.

No irreversible magic.

---

# Domain Driven Design

## Core Domain

Competitive advantage:

- profiling
- semantic inference
- statistics
- transformations
- analytical workflows

---

## Supporting Domains

- storage
- rendering
- import/export
- synchronization

---

## Generic Domains

Prefer third-party solutions for:

- updates
- telemetry
- authentication
- crash reporting

---

# Bounded Contexts

## Dataset Context

Owns:

- datasets
- schemas
- metadata

---

## Query Context

Owns:

- DuckDB
- filtering
- sorting
- aggregation

---

## Profiling Context

Owns:

- distributions
- correlations
- statistics
- quality metrics

---

## Visualization Context

Owns:

- rendering
- sparklines
- plots
- visual summaries

Must not execute queries.

---

## Agent Context

Owns:

- planning
- tool orchestration
- explanations

Must not access storage directly.

---

# Dependency Rules

Allowed:

UI
→ Application
→ Domain
→ Infrastructure

Forbidden:

Infrastructure
→ UI

Domain code must not depend on egui, wgpu, file dialogs, or networking.

---

# Rust Engineering Principles

## Strong Types

Prefer:

DatasetId
ColumnId
ViewId

Avoid passing raw strings and integers through the system.

---

## Ownership Clarity

Prefer ownership and immutable data.

Avoid shared mutable state.

---

## Error Handling

Never panic in production code paths.

Use structured errors.

Errors must be actionable.

---

## Async Discipline

Use async for:

- IO
- background work
- long-running computation

Avoid unnecessary async propagation.

---

# Data Architecture

## Canonical Memory Format

Apache Arrow.

---

## Storage Format

Parquet by default.

CSV is import/export only.

---

## Query Engine

DuckDB owns:

- joins
- sorting
- filtering
- aggregations

---

## Analytics Engine

Polars owns:

- profiling
- statistics
- feature engineering
- transformations

---

# UI / UX Constitution

## Information Density

Prefer compact summaries.

Every screen should maximize signal.

---

## Progressive Disclosure

Show:

1. Summary
2. Details
3. Advanced analysis

---

## Immediate Feedback

Target perceived response under 100 ms.

---

## Keyboard First

All critical workflows must support keyboard navigation.

---

## Discoverability

Users should not require documentation for common tasks.

---

## Explainability

Agent actions must always show:

- what happened
- why it happened
- resulting effects

---

## Non-Blocking UI

Never freeze the interface.

Long-running tasks require:

- background execution
- progress reporting
- cancellation

---

# Visualization Principles

## Column-Centric Analytics

Every column should expose:

- type
- quality
- distribution
- relationships

---

## Small Multiples

Prefer many compact views.

Avoid giant dashboards.

---

## Inline Visualizations

Prefer:

▁▂▃▄▅▆▇█

inside tables and inspectors.

---

## Distribution First

Before showing averages, show distributions.

---

# Agent Principles

## Agent As Analyst

Agents suggest actions.

Agents do not silently mutate data.

---

## Tool-Based Execution

Agents operate through tools.

No unrestricted access.

---

## Auditability

Every action should be logged and inspectable.

---

## Deterministic Workflows

Agent plans should compile into reproducible workflows.

---

# Performance Constitution

## Measure Everything

Track:

- FPS
- memory
- query latency
- profiling latency

---

## Benchmark First

Every optimization should have a benchmark.

---

## Virtualization Everywhere

Never render invisible data.

---

## Streaming Preferred

Prefer streaming over full materialization.

---

# Testing Strategy

Required:

- domain tests
- integration tests
- performance tests

Cover:

- profiling
- transformations
- semantic inference
- rendering boundaries

---

# Code Review Checklist

Before merge:

- bounded contexts respected
- tests included
- performance impact considered
- errors handled
- UI responsiveness preserved
- architecture updated if needed

---

# Feature Acceptance Checklist

Before shipping:

- solves a real analytical problem
- scales to million-row datasets
- works offline
- measurable
- explainable
- reproducible
- testable

---

# Anti-Patterns

Avoid:

- cell-centric architecture
- business logic in UI
- hidden agent actions
- global mutable state
- giant dashboard screens
- premature microservices
- framework-driven design

---

# Long-Term North Star

Database
+
Spreadsheet
+
Statistical Package
+
Notebook
+
Visualization System
+
Agent Runtime

inside a single local-first analytical environment.

Every architectural decision should move the system toward that vision.
