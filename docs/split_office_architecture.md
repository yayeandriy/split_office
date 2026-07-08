# Split Office Architecture

## Workspace-Centric Office Platform

Version: 1.0

### Vision

Split Office is not a document editor, spreadsheet, or office suite.

It is a Workspace-Centric Knowledge System: a VS Code for Office.

The workspace is the primary object. Documents, datasets, analyses, visualizations, workflows, presentations, and agents are different projections of the same underlying knowledge space.

### Core Thesis

Traditional office software is file-centric.

Word, Excel, and PowerPoint create isolated files connected only through copy-paste and exports.

Split Office is workspace-centric.

Workspace
├── Documents
├── Datasets
├── Analyses
├── Visualizations
├── Workflows
├── References
├── Agents
└── Presentations

### Universal Knowledge Layer

Canonical model:

Workspace
↓
Knowledge Graph
↓
Views

Views:
- Document View
- Spreadsheet View
- Notebook View
- Report View
- Presentation View
- Dashboard View
- Agent View

### Fundamental Principle

A document is not text.
A spreadsheet is not cells.
A presentation is not slides.

They are all representations of underlying knowledge.

### Workspace Objects

Everything becomes a first-class object:

- Dataset
- Document
- Section
- Table
- Chart
- Metric
- Workflow
- Agent Session
- Analysis
- Citation
- Reference

### Domains

Dataset Domain
- transformations
- profiling
- statistics
- workflows

Document Domain
- structured content
- references
- reports

Analysis Domain
- forecasting
- metrics
- correlation
- modeling

Visualization Domain
- charts
- plots
- dashboards

Agent Domain
- planning
- orchestration
- generation

### Bidirectional Knowledge Flow

Dataset ↔ Analysis ↔ Report ↔ Presentation

Relationships are preserved.

### Dynamic Reports

Report statements should reference metrics and analyses rather than storing static values.

Report Statement
↓
Metric
↓
Analysis
↓
Workflow
↓
Dataset

### Multi-View Editing

Examples:

Document | Spreadsheet
Document | Notebook
Spreadsheet | Dashboard
Document | Agent

All synchronized within the same workspace.

### Object Explorer

Workspace
├── Documents
├── Datasets
├── Workflows
├── Analyses
├── Charts
└── Agents

### Agent Architecture

Agents operate on workspace objects, not files.

Examples:
- Build workflow
- Generate report
- Create visualization
- Create analysis

### Graph Architecture

Workspace Graph
Workflow Graph
Lineage Graph
Reference Graph

All interconnected.

### Product Evolution

Phase 1: Analytical Workbench
Phase 2: Transformation Engine
Phase 3: Workspace Foundation
Phase 4: Structured Documents
Phase 5: Bidirectional Report ↔ Dataset
Phase 6: Agent Workspace
Phase 7: Unified Knowledge Platform

### Technical North Star

VS Code unified files, tools, extensions, terminals, and language services.

Split Office should unify documents, datasets, analyses, visualizations, workflows, and agents into a single workspace.

### Definition

Split Office is a Workspace-Centric Knowledge System where structured data, narrative content, analytical workflows, visualizations, and agents coexist as interconnected objects while preserving lineage, provenance, reproducibility, and bidirectional transformation between representations.
