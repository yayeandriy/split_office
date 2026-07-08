# Document View (Word) Implementation Guide
## Phase 1 Structured Document System

Version: 1.0

# Objective

Introduce first-class document editing into Split Office.

Document View must coexist with Spreadsheet View.

# Philosophy

Not a DOCX clone.

Not rich text first.

Structured document first.

# Initial Scope

Supported:

- headings
- paragraphs
- lists
- tables
- references
- metrics
- embedded charts

Excluded:

- track changes
- collaboration
- page-perfect publishing

# Architecture

New crates:

document_core/
document_model/
document_view/
document_commands/

# CDM Objects

Document
Section
Paragraph
TableBlock
MetricBlock
ChartBlock
ReferenceBlock

# Document Structure

Document
├─ Section
│  ├─ Paragraph
│  ├─ MetricBlock
│  └─ ChartBlock

# Editor Model

Use block-based editor.

Similar to:

- Notion
- Obsidian
- modern editors

Not traditional word processor internals.

# View Layout

Document View

Toolbar
Document Outline
Editor Surface
Inspector

# Required Features

Create document

Rename document

Delete document

Duplicate document

# Document Outline

Shows:

- headings
- sections
- references

# References

User can insert:

Dataset Reference

Workflow Reference

Metric Reference

Chart Reference

# Dynamic Metrics

Example:

Revenue Growth

Inserted as live object.

Document updates automatically.

# Embedded Tables

Source:

Dataset

Workflow Output

Analysis Output

Requirements:

- live updates
- refresh indicators

# Embedded Charts

Source:

Dataset

Workflow

Analysis

# Split View Scenarios

Spreadsheet | Document

User selects workflow.

Clicks:

Generate Report

Document opens beside spreadsheet.

# Report Generation API

Input:

Dataset
Workflow
Analysis

Output:

Structured Document

# Synchronization

Dataset changes

→ workflow changes

→ metrics update

→ document refreshes

# Commands

Insert Heading

Insert Paragraph

Insert Metric

Insert Chart

Insert Table

Insert Reference

# Testing

Unit:

- document model
- block insertion
- references

Integration:

- dataset → document links
- metric updates
- chart embedding

UI:

- editing
- selection
- outline navigation

Performance:

100-page document

1000 blocks

50 embedded objects

# User Stories

- Create report
- Insert metric
- Insert chart
- Reference dataset
- Open document beside spreadsheet
- Generate report from workflow

# Exit Criteria

Split Office supports:

Spreadsheet View

and

Document View

inside same workspace with live references.
