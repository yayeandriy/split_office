//! Canonical Data Model — the universal object model of the workspace.
//!
//! # Spec: Split Office CDM
//!
//! The CDM is the single source of truth. Every view (Spreadsheet, Document,
//! Notebook, etc.) is a projection of objects in the CDM.
//!
//! # Core Principle
//!
//! Everything is an Object. Everything has identity. Everything can be referenced.

use serde::{Deserialize, Serialize};
use std::fmt;

// ── Object Identity ──────────────────────────────────────────────────────────

/// Globally unique, immutable object identifier.
///
/// Never derived from names. Never reused. (Spec §Object Identity)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId(pub u64);

impl ObjectId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "obj:{}", self.0)
    }
}

/// Counter for generating unique ObjectIds.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IdGenerator {
    next: u64,
}

impl IdGenerator {
    pub fn new() -> Self {
        Self { next: 1 }
    }

    pub fn next(&mut self) -> ObjectId {
        let id = ObjectId(self.next);
        self.next += 1;
        id
    }
}

// ── Object Type ──────────────────────────────────────────────────────────────

/// The class of a CDM object. (Spec §Object Taxonomy)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    Workspace,
    Dataset,
    Document,
    Section,
    Paragraph,
    Heading,
    Table,
    Chart,
    Metric,
    Workflow,
    WorkflowNode,
    Visualization,
    Notebook,
    Presentation,
    Citation,
    Reference,
    AgentSession,
    Analysis,
    Attachment,
}

impl ObjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectType::Workspace => "Workspace",
            ObjectType::Dataset => "Dataset",
            ObjectType::Document => "Document",
            ObjectType::Section => "Section",
            ObjectType::Paragraph => "Paragraph",
            ObjectType::Heading => "Heading",
            ObjectType::Table => "Table",
            ObjectType::Chart => "Chart",
            ObjectType::Metric => "Metric",
            ObjectType::Workflow => "Workflow",
            ObjectType::WorkflowNode => "WorkflowNode",
            ObjectType::Visualization => "Visualization",
            ObjectType::Notebook => "Notebook",
            ObjectType::Presentation => "Presentation",
            ObjectType::Citation => "Citation",
            ObjectType::Reference => "Reference",
            ObjectType::AgentSession => "AgentSession",
            ObjectType::Analysis => "Analysis",
            ObjectType::Attachment => "Attachment",
        }
    }
}

// ── Base Object ──────────────────────────────────────────────────────────────

/// Every CDM object must satisfy this contract. (Spec §Universal Object Contract)
pub trait CdmObject {
    fn id(&self) -> ObjectId;
    fn object_type(&self) -> ObjectType;
}

// ── Relationship ─────────────────────────────────────────────────────────────

/// A directed relationship between two CDM objects. (Spec §Relationship Types)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relationship {
    Contains,
    References,
    DependsOn,
    DerivedFrom,
    GeneratedBy,
    Uses,
    Explains,
    Cites,
    Visualizes,
    Aggregates,
}

// ── Reference ────────────────────────────────────────────────────────────────

/// A typed reference to another CDM object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectRef {
    pub target: ObjectId,
    pub relationship: Relationship,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_generator_uniqueness() {
        let mut gen = IdGenerator::new();
        let a = gen.next();
        let b = gen.next();
        assert_ne!(a, b);
    }

    #[test]
    fn test_object_id_display() {
        let id = ObjectId::new(42);
        assert_eq!(id.to_string(), "obj:42");
    }
}
