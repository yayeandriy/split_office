# Split Office Canonical Data Model (CDM)
## The Universal Object Model of the Workspace

Version: 1.0

---

# Executive Summary

The Canonical Data Model (CDM) is the most important architectural artifact in Split Office.

It is the single source of truth from which every view is derived:

- Document View
- Spreadsheet View
- Notebook View
- Dashboard View
- Presentation View
- Agent View
- Knowledge Graph View

The CDM plays the same role that:

- DOM plays in browsers
- AST plays in compilers
- IR plays in database engines

The CDM is not a UI model.

The CDM is not a storage model.

The CDM is the semantic representation of knowledge inside a workspace.

---

# Core Principle

Everything is an Object.

Everything has identity.

Everything can be referenced.

Everything participates in lineage.

Everything participates in provenance.

---

# Root Entity

Workspace

The workspace is the highest-level object.

Suggested structure:

Workspace
├── Metadata
├── Object Store
├── Graph Store
├── Version Store
└── Settings

---

# Universal Object Contract

Every object must implement:

ObjectId

ObjectType

CreatedAt

UpdatedAt

Version

Metadata

Relationships

Lineage

Provenance

---

# Object Identity

Object IDs are immutable.

Never derived from names.

Never reused.

Example:

DatasetId

DocumentId

WorkflowId

MetricId

---

# Object Taxonomy

Primary object classes:

Dataset

Document

Section

Table

Chart

Metric

Workflow

WorkflowNode

Visualization

Notebook

Presentation

Citation

Reference

AgentSession

Analysis

Attachment

---

# Dataset

Represents structured tabular information.

Owns:

Schema

Statistics

Profiles

Lineage

Workflow References

Storage References

Datasets never own visual state.

---

# Document

Structured narrative container.

Owns:

Sections

References

Embedded Objects

Narrative Blocks

Metadata

Documents are not plain text files.

---

# Section

Composable document unit.

Examples:

Executive Summary

Methods

Results

Conclusions

Sections may reference:

Datasets

Charts

Metrics

Analyses

---

# Metric

Semantic numerical result.

Examples:

Revenue Growth

Average Order Value

Conversion Rate

Metrics should reference:

Analysis

Workflow

Dataset

Never store raw values only.

Store derivation.

---

# Workflow

Transformation graph.

Owns:

Nodes

Edges

Execution Metadata

Validation State

Lineage

---

# Workflow Node

Represents a transformation.

Examples:

Filter

Aggregate

Join

Forecast

Derived Column

---

# Analysis

Represents analytical outputs.

Examples:

Profiling

Correlation Matrix

Forecast

Outlier Detection

Cluster Analysis

---

# Visualization

Represents graphical interpretation.

Examples:

Histogram

Scatter Plot

Distribution View

Heat Map

---

# Presentation

Structured communication artifact.

Contains:

Slides

Visualizations

References

Metrics

Narrative

---

# Notebook

Mixed analytical environment.

Contains:

Narrative

Datasets

Analyses

Visualizations

Agent Interactions

---

# Agent Session

Persistent analytical conversation.

Contains:

Messages

Plans

Generated Objects

Actions

References

---

# Citation

Represents provenance reference.

Can target:

Dataset

Document

Section

External Source

Analysis

---

# Attachment

Imported artifacts.

Examples:

PDF

DOCX

CSV

Parquet

Image

Audio

---

# Graph Model

All objects participate in a workspace graph.

---

# Relationship Types

Contains

References

DependsOn

DerivedFrom

GeneratedBy

Uses

Explains

Cites

Visualizes

Aggregates

---

# Example Relationship Graph

Dataset

↓

Workflow

↓

Analysis

↓

Metric

↓

Chart

↓

Document Section

↓

Presentation Slide

---

# Lineage Model

Every object exposes lineage.

Questions answered:

Where did this come from?

What generated it?

What depends on it?

---

# Provenance Model

Track:

Creator

Agent

Workflow

Source File

Import Operation

Timestamp

Version

---

# Workspace DAG

Workspace graph is acyclic whenever possible.

Dependencies represented explicitly.

Supports:

Impact Analysis

Incremental Updates

Agent Planning

---

# Versioning

Every object versioned independently.

Object

↓

Version 1

↓

Version 2

↓

Version 3

Never overwrite history.

---

# References

References should be object-based.

Avoid file-path references internally.

Example:

Document references MetricId.

Not a string value.

---

# Dynamic Content

Documents should support live objects.

Example:

Revenue increased {Metric:RevenueGrowth}

Metric resolves dynamically.

---

# Persistence Model

Workspace File

Contains:

Object Store

Relationship Store

Settings

Metadata

Caches

External References

---

# Storage Rules

Store objects.

Do not store rendered views.

Views are reconstructed.

---

# View Architecture

Views are projections.

Examples:

Spreadsheet View

Projects Dataset.

Document View

Projects Document.

Dashboard View

Projects Metrics + Visualizations.

---

# Synchronization

Changes propagate through graph.

Dataset change

↓

Workflow update

↓

Analysis update

↓

Metric update

↓

Document update

↓

Presentation update

---

# Event Model

Every mutation emits events.

Examples:

ObjectCreated

ObjectUpdated

ObjectDeleted

RelationshipAdded

RelationshipRemoved

---

# Agent Integration

Agents create and manipulate objects.

Agents never manipulate rendered views directly.

---

# Security Boundary

Permissions should be object-level.

Future support:

Workspace Permissions

Object Permissions

Role Permissions

---

# Testing Requirements

Validate:

Identity

Relationships

Lineage

Versioning

Persistence

Synchronization

Graph Integrity

---

# Acceptance Criteria

The CDM is complete when:

Every workspace artifact maps to an object.

Every object has identity.

Every object participates in relationships.

Every object participates in lineage.

Every view can be generated from the CDM.

No view becomes the source of truth.

---

# Long-Term Vision

The CDM becomes the universal semantic layer powering:

Documents

Spreadsheets

Notebooks

Dashboards

Presentations

Agents

Research Workspaces

Knowledge Graphs

Future views should require no fundamental changes to the model.

Only new projections of the same canonical workspace knowledge structure.
