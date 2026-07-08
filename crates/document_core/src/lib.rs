//! Document model — structured document built on the CDM.
//!
//! # Spec: Document View Phase 1
//!
//! Block-based editor model (Notion/Obsidian style).
//! Documents are composed of Sections containing Blocks.

use cdm::{CdmObject, ObjectId, ObjectType};
use serde::{Deserialize, Serialize};

// ── Document ─────────────────────────────────────────────────────────────────

/// A structured narrative container. (Spec §Document)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: ObjectId,
    pub name: String,
    pub sections: Vec<Section>,
}

impl CdmObject for Document {
    fn id(&self) -> ObjectId {
        self.id
    }
    fn object_type(&self) -> ObjectType {
        ObjectType::Document
    }
}

impl Document {
    pub fn new(id: ObjectId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            sections: vec![Section::new(id, "Content")],
        }
    }
}

// ── Section ──────────────────────────────────────────────────────────────────

/// Composable document unit. (Spec §Section)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub id: ObjectId,
    pub title: String,
    pub blocks: Vec<Block>,
}

impl Section {
    pub fn new(parent_id: ObjectId, title: impl Into<String>) -> Self {
        Self {
            id: ObjectId::new(parent_id.0.wrapping_add(1)),
            title: title.into(),
            blocks: Vec::new(),
        }
    }

    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
    }
}

// ── Block ────────────────────────────────────────────────────────────────────

/// A single content block within a section. (Spec §CDM Objects: Paragraph, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Block {
    Heading(HeadingBlock),
    Paragraph(ParagraphBlock),
    Reference(ReferenceBlock),
    Table(TableBlock),
}

impl Block {
    pub fn block_type(&self) -> &'static str {
        match self {
            Block::Heading(_) => "Heading",
            Block::Paragraph(_) => "Paragraph",
            Block::Reference(_) => "Reference",
            Block::Table(_) => "Table",
        }
    }
}

// ── Heading ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadingBlock {
    pub id: ObjectId,
    pub level: u8, // 1, 2, 3
    pub text: String,
}

impl HeadingBlock {
    pub fn new(id: ObjectId, level: u8, text: impl Into<String>) -> Self {
        Self {
            id,
            level: level.clamp(1, 3),
            text: text.into(),
        }
    }
}

// ── Paragraph ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphBlock {
    pub id: ObjectId,
    pub text: String,
}

impl ParagraphBlock {
    pub fn new(id: ObjectId, text: impl Into<String>) -> Self {
        Self {
            id,
            text: text.into(),
        }
    }
}

// ── Reference Block ──────────────────────────────────────────────────────────

/// A live reference to a CDM object (dataset, metric, chart).
/// Spec §References: "User can insert Dataset Reference, Workflow Reference, etc."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceBlock {
    pub id: ObjectId,
    pub target: ObjectId,
    pub label: String,
}

// ── Table Block ──────────────────────────────────────────────────────────────

/// An embedded table sourced from a dataset or workflow output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableBlock {
    pub id: ObjectId,
    pub source: ObjectId,
    pub caption: String,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::new(ObjectId::new(1), "My Report");
        assert_eq!(doc.name, "My Report");
        assert_eq!(doc.sections.len(), 1);
    }

    #[test]
    fn test_section_add_block() {
        let mut section = Section::new(ObjectId::new(10), "Analysis");
        let heading = HeadingBlock::new(ObjectId::new(11), 1, "Overview");
        section.add_block(Block::Heading(heading));
        section.add_block(Block::Paragraph(ParagraphBlock::new(
            ObjectId::new(12),
            "This is the analysis section.",
        )));
        assert_eq!(section.blocks.len(), 2);
    }

    #[test]
    fn test_block_types() {
        let p = Block::Paragraph(ParagraphBlock::new(ObjectId::new(1), "text"));
        assert_eq!(p.block_type(), "Paragraph");

        let h = Block::Heading(HeadingBlock::new(ObjectId::new(2), 2, "Title"));
        assert_eq!(h.block_type(), "Heading");
    }
}
