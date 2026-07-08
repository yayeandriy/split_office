# UI Design Constitution Addendum

This document extends the Analytical Workbench Constitution with mandatory UI and design-system rules.

---

# Design Philosophy

The interface should feel:

- calm
- dense
- analytical
- consistent
- predictable

Users should focus on data, not interface decoration.

---

# Typography Constitution

## Single Primary Typography System

90% of the interface should use a single typography style family.

Avoid introducing multiple competing font sizes, weights, and styles.

Preferred hierarchy:

- Display (rare)
- Heading
- Body
- Monospace (data/code)

Most screens should consist primarily of Body typography.

---

## Typography Is Not Decoration

Do not create visual hierarchy by randomly changing:

- font size
- weight
- color

Use spacing and structure first.

---

# Design System First

## Reusable Components Mandatory

Before introducing a new visual pattern, verify whether an existing component can be reused.

Preferred:

Button
Table
Panel
Inspector
Badge
Tooltip
Dialog

Avoid one-off components.

---

## Component Ownership

Every component should have:

- documented purpose
- API
- visual specification
- examples

---

# Spacing Constitution

## Single Spacing Scale

All paddings, margins, gaps, and layout spacing must come from a shared spacing system.

Example:

4
8
12
16
24
32
48

Do not invent arbitrary spacing values.

---

## Horizontal Rhythm

Horizontal spacing must remain consistent across:

- panels
- dialogs
- inspectors
- tables
- toolbars

---

## Vertical Rhythm

Vertical spacing must remain consistent across:

- forms
- inspectors
- menus
- property panels
- analysis views

Users should subconsciously feel alignment and order.

---

## Alignment Over Decoration

Prefer:

consistent spacing

over:

extra borders
extra colors
extra visual effects

---

# Layout Principles

## Predictability

Identical content types should appear in identical layouts.

Users should build spatial memory.

---

## Information Density

Maximize signal per pixel.

Avoid excessive whitespace.

Avoid dashboard-style empty areas.

---

## Progressive Disclosure

Show:

1. summary
2. details
3. advanced controls

Avoid overwhelming first views.

---

# Visual Consistency

## Limited Visual Vocabulary

Use a small set of:

- colors
- radii
- shadows
- borders

Avoid visual fragmentation.

---

## Consistent Interaction Patterns

Identical interactions should behave identically throughout the application.

---

# UI Testing Requirements

## Visual Regression Testing

Critical screens should be snapshot tested.

Detect:

- layout drift
- spacing regressions
- visual inconsistencies

---

## Component Testing

All reusable components should have:

- state coverage
- interaction coverage
- accessibility review

---

## Design System Verification

Automated checks should verify:

- spacing rules
- typography usage
- component reuse

where practical.

---

# Table and Grid Rules

The grid is the primary product surface.

Requirements:

- stable row heights
- predictable column behavior
- smooth scrolling
- consistent cell padding

No special-case visual styling unless justified by data semantics.

---

# Long-Term Goal

The interface should eventually feel closer to:

an IDE
+
scientific software
+
professional design tools

than a traditional business dashboard.

Consistency is a feature.

Every new screen should look as if it already existed before it was implemented.
