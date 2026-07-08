# Split Office Split View Framework
## Agent-Ready Implementation Specification

Version: 1.0

### Purpose

Split View is a core platform subsystem, not a Spreadsheet|Document feature.

It must support:

- Spreadsheet | Document
- Spreadsheet | Spreadsheet
- Document | Document
- Document | Dashboard
- Document | Agent
- Any View | Any View

The framework becomes the workspace window manager for Split Office.

---

## Core Principles

Views are projections.

Workspace is the source of truth.

Split View composes projections.

No view receives special treatment.

Everything is a View.

---

## Phase 1 Scope

Required:

- horizontal splits
- vertical splits
- nested splits
- tabs
- layout persistence
- focus management
- workspace integration

Excluded:

- floating windows
- multi-monitor support
- collaborative cursors

---

## Core Concepts

Workspace

View

View Instance

Layout Tree

Split Container

Tab Container

Focus Manager

View Registry

---

## View Definition

A view is a projection of CDM objects.

Examples:

SpreadsheetView

DocumentView

DashboardView

NotebookView

AgentView

ObjectExplorerView

PropertiesView

---

## View Instance

Contains:

ViewId

ViewType

ObjectReference

ViewState

Examples:

SpreadsheetView
DatasetId

DocumentView
DocumentId

AgentView
AgentSessionId

---

## Layout Tree

Represent the UI as a tree.

Example:

HorizontalSplit
├─ SpreadsheetView
└─ DocumentView

Nested Example:

VerticalSplit
├─ SpreadsheetView
└─ HorizontalSplit
   ├─ DocumentView
   └─ AgentView

---

## Layout Nodes

LeafView

HorizontalSplit

VerticalSplit

TabContainer

Every layout configuration must be representable using these primitives.

---

## Layout Manager

Responsibilities:

- create split
- remove split
- move view
- open tab
- close tab
- restore layout
- persist layout

---

## View Registry

Responsibilities:

- register view types
- instantiate views
- destroy views
- restore views

Phase 1 View Types:

- SpreadsheetView
- DocumentView
- ObjectExplorerView
- InspectorView

---

## Focus Manager

Tracks:

Focused View

Active View

Keyboard Routing

Command Routing

Only one view receives keyboard commands at a time.

---

## Split Ratios

Store ratios explicitly.

Examples:

70/30

50/50

25/75

Persist across sessions.

---

## Tabs

Every split region may contain tabs.

Examples:

Spreadsheet A

Spreadsheet B

Document A

Document B

Agent

Dashboard

---

## Persistence

Persist:

Layout Tree

View States

Tab Order

Split Ratios

Focused View

Workspace References

Restore exactly after restart.

---

## Workspace Integration

Views reference workspace objects.

Examples:

SpreadsheetView → DatasetId

DocumentView → DocumentId

AgentView → AgentSessionId

Never reference files directly.

---

## Synchronization

Multiple views may observe the same object.

Example:

SpreadsheetView A
SpreadsheetView B

Both reference DatasetId.

Updates remain synchronized.

---

## Spreadsheet + Document Workflow

Primary implementation scenario.

Dataset
↓
Workflow
↓
Analysis
↓
Generate Report
↓
Open Document beside Spreadsheet

Result:

Spreadsheet | Document

---

## Commands

Split Horizontal

Split Vertical

Close View

Move View

Focus Next

Focus Previous

Open Tab

Close Tab

Duplicate View

---

## Rendering Rules

Only visible views render.

Hidden tabs remain suspended.

Background updates must not consume rendering resources.

---

## Memory Rules

Views never own datasets.

Views reference shared workspace objects.

Opening the same dataset in multiple views must not duplicate memory.

---

## User Stories

Story 1

Open Spreadsheet

Split Vertically

Open Document

Save Workspace

Restart

Layout restored.

---

Story 2

Generate Report

Automatically open:

Spreadsheet | Document

---

Story 3

Open multiple documents in tabs.

Switch instantly.

---

Story 4

Create nested layout:

Spreadsheet
|
Document + Agent

---

## Testing Requirements

Unit Tests

- Layout Tree
- View Registry
- Focus Manager
- Persistence

Integration Tests

- Split Creation
- Nested Splits
- Layout Restore
- View Synchronization

UI Tests

- Tab Switching
- Split Resize
- Keyboard Navigation
- Focus Routing

Stress Tests

- 50 Views
- 20 Splits
- 100 Tabs

---

## Performance Targets

Split Creation < 16 ms

Tab Switch < 16 ms

Layout Restore < 100 ms

No visible frame drops.

---

## Failure Scenarios

Missing Object

Missing View Type

Corrupt Layout

Workspace Migration

Requirements:

- graceful recovery
- fallback views
- layout repair

---

## Acceptance Criteria

Users can:

- split horizontally
- split vertically
- create nested layouts
- use tabs
- persist layouts
- restore layouts
- open Spreadsheet|Document
- maintain responsive UI

---

## Long-Term Vision

The Split View Framework becomes the universal composition engine for Split Office.

Every future projection integrates through this framework:

- Spreadsheet
- Document
- Notebook
- Dashboard
- Presentation
- Agent
- Knowledge Graph

The framework must never require special handling for any individual view type.
