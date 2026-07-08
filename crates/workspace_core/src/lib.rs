//! Workspace shell — the top-level container for all CDM objects.
//!
//! # Spec: Phase 1 Transition — Workspace Shell
//!
//! The workspace is the primary container. Users open workspaces, not files.
//! All datasets, documents, workflows, etc. live inside the workspace.

use cdm::{IdGenerator, ObjectId, ObjectType};
use serde::{Deserialize, Serialize};

// ── Workspace ────────────────────────────────────────────────────────────────

/// The root workspace entity. (Spec §Workspace Shell)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: ObjectId,
    pub name: String,
    pub objects: Vec<WorkspaceObject>,
    #[serde(skip)]
    id_gen: IdGenerator,
}

impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        let mut gen = IdGenerator::new();
        let id = gen.next();
        Self {
            id,
            name: name.into(),
            objects: Vec::new(),
            id_gen: gen,
        }
    }

    pub fn add_object(&mut self, obj: WorkspaceObject) -> ObjectId {
        let id = obj.id();
        self.objects.push(obj);
        id
    }

    pub fn remove_object(&mut self, id: ObjectId) -> Option<WorkspaceObject> {
        if let Some(pos) = self.objects.iter().position(|o| o.id() == id) {
            Some(self.objects.remove(pos))
        } else {
            None
        }
    }

    pub fn next_id(&mut self) -> ObjectId {
        self.id_gen.next()
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    pub fn objects_of_type(&self, ty: ObjectType) -> Vec<&WorkspaceObject> {
        self.objects.iter().filter(|o| o.object_type() == ty).collect()
    }

    /// Serialize to JSON for persistence. (Spec §Workspace Persistence)
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// ── Workspace Object ─────────────────────────────────────────────────────────

/// An object that lives in the workspace. (Spec §Object Taxonomy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkspaceObject {
    Dataset(DatasetEntry),
    Document(DocumentEntry),
    Workflow(WorkflowEntry),
}

impl WorkspaceObject {
    pub fn id(&self) -> ObjectId {
        match self {
            WorkspaceObject::Dataset(d) => d.id,
            WorkspaceObject::Document(d) => d.id,
            WorkspaceObject::Workflow(w) => w.id,
        }
    }

    pub fn object_type(&self) -> ObjectType {
        match self {
            WorkspaceObject::Dataset(_) => ObjectType::Dataset,
            WorkspaceObject::Document(_) => ObjectType::Document,
            WorkspaceObject::Workflow(_) => ObjectType::Workflow,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            WorkspaceObject::Dataset(d) => &d.name,
            WorkspaceObject::Document(d) => &d.name,
            WorkspaceObject::Workflow(w) => &w.name,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            WorkspaceObject::Dataset(_) => "📊",
            WorkspaceObject::Document(_) => "📄",
            WorkspaceObject::Workflow(_) => "⚙",
        }
    }
}

// ── Entry types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetEntry {
    pub id: ObjectId,
    pub name: String,
    pub path: Option<String>,
    pub row_count: usize,
    pub col_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentEntry {
    pub id: ObjectId,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEntry {
    pub id: ObjectId,
    pub name: String,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_add_object() {
        let mut ws = Workspace::new("My Workspace");
        let id = ws.next_id();
        ws.add_object(WorkspaceObject::Document(DocumentEntry {
            id,
            name: "Report".into(),
        }));
        assert_eq!(ws.object_count(), 1);
    }

    #[test]
    fn test_workspace_filter_by_type() {
        let mut ws = Workspace::new("Test");
        let d = ws.next_id();
        ws.add_object(WorkspaceObject::Document(DocumentEntry {
            id: d,
            name: "Doc".into(),
        }));
        let ds = ws.next_id();
        ws.add_object(WorkspaceObject::Dataset(DatasetEntry {
            id: ds,
            name: "Data".into(),
            path: None,
            row_count: 0,
            col_count: 0,
        }));
        assert_eq!(ws.objects_of_type(ObjectType::Document).len(), 1);
        assert_eq!(ws.objects_of_type(ObjectType::Dataset).len(), 1);
    }

    #[test]
    fn test_workspace_roundtrip() {
        let mut ws = Workspace::new("Project");
        let id = ws.next_id();
        ws.add_object(WorkspaceObject::Document(DocumentEntry {
            id,
            name: "Notes".into(),
        }));
        let json = ws.to_json().unwrap();
        let restored = Workspace::from_json(&json).unwrap();
        assert_eq!(restored.name, "Project");
        assert_eq!(restored.object_count(), 1);
    }
}
