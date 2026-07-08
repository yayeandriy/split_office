# Split Office Transition Plan — Phase 1
## From Analytical Workbench to Workspace Foundation

Version: 1.0

# Objective

Current state:

- Dataset engine
- Profiling engine
- Workflow engine
- DAG architecture
- Modifier stack UI

Target state:

- Workspace-centric application
- Multiple synchronized views
- Canonical Data Model integration
- Document + Spreadsheet coexistence

# Success Criteria

User can:

- Open a Workspace
- Open Datasets
- Open Documents
- View both side-by-side
- Navigate workspace objects
- Create references between objects

# Scope

Included:

- Workspace shell
- Object explorer
- View management
- CDM integration
- Workspace persistence
- Split-view layout

Excluded:

- AI agents
- Collaboration
- Presentation view
- Notebook view

# Architecture Changes

New crates:

workspace_core/
workspace_graph/
workspace_events/
workspace_views/
workspace_storage/

# Deliverable 1 — Workspace Shell

Replace spreadsheet-centric startup.

Application opens:

Workspace

instead of

Dataset.

Requirements:

- Recent workspaces
- Open workspace
- Create workspace
- Recover workspace

# Deliverable 2 — Object Explorer

VS Code style explorer.

Workspace
├─ Documents
├─ Datasets
├─ Workflows
├─ Analyses
├─ Charts

Supports:

- search
- rename
- create
- delete
- move

# Deliverable 3 — Docking & Split Views

User can open:

Spreadsheet | Document

Spreadsheet | Spreadsheet

Document | Document

Requirements:

- tabs
- split horizontally
- split vertically
- persistent layout

# Deliverable 4 — CDM Integration

Every dataset becomes CDM object.

Every workflow becomes CDM object.

Object IDs replace direct references.

# Deliverable 5 — Event System

Events:

ObjectCreated
ObjectUpdated
ObjectDeleted
RelationshipAdded
RelationshipRemoved

# Deliverable 6 — Workspace Persistence

.workspace package:

- object store
- graph store
- settings
- layouts

# User Stories

- Create workspace
- Import dataset
- Save workspace
- Reopen workspace
- Open two views simultaneously
- Browse workspace graph

# Performance Targets

Workspace load < 500ms

View creation < 100ms

Explorer updates < 16ms

# Testing

Unit:
- object creation
- graph relations
- persistence

Integration:
- workspace load/save
- multi-view synchronization

Performance:
- 10k objects
- 100 datasets
- 100 documents

# Exit Criteria

Split Office foundation exists.

Application is workspace-centric.
