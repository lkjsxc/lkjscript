use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use lkjscript_core::{Error, Result};

use super::{EntityId, NodeId, RevisionId, WorkspaceNamespace};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProgramState {
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum EntityKind {
    Main,
    Parameter,
    ImmutableLocal,
    StaticBytesLocal,
    MutableLocal,
    Function,
    BuiltinOperation,
    Product,
    ProductField,
    Enum,
    EnumVariant,
    EnumField,
    Trait,
    Implementation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum NodeKind {
    Literal,
    Load,
    Move,
    Borrow,
    Call,
    Operation,
    Conversion,
    Sequence,
    Conditional,
    While,
    Loop,
    Return,
    Break,
    Continue,
    Trap,
    Exit,
    Let,
    MutableLocal,
    SetLocal,
    Product,
    Enum,
    MatchUnreachable,
    Symbol,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticOwner {
    Entity(EntityId),
    Node(NodeId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticChild {
    Entity(EntityId),
    Node(NodeId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityHeader {
    pub id: EntityId,
    pub kind: EntityKind,
    pub name: Arc<str>,
    pub owner: Option<EntityId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeHeader {
    pub id: NodeId,
    pub kind: NodeKind,
    pub owner: SemanticOwner,
    pub actual_type: Arc<str>,
    pub expected_type: Option<Arc<str>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContainmentEdge {
    pub owner: SemanticOwner,
    pub child: SemanticChild,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReferenceEdge {
    pub site: NodeId,
    pub target: EntityId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CallEdge {
    pub caller: EntityId,
    pub callee: EntityId,
    pub site: NodeId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DependencyEdge {
    pub dependent: EntityId,
    pub dependency: EntityId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticHeader {
    pub code: Arc<str>,
    pub severity: DiagnosticSeverity,
    pub subject: Option<SemanticChild>,
    pub message: Arc<str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceAttachment {
    path: PathBuf,
    exact_source_len: u64,
    exact_source_sha256: [u8; 32],
}

impl SourceAttachment {
    pub(super) fn new(path: PathBuf, exact_source_len: u64, exact_source_sha256: [u8; 32]) -> Self {
        Self {
            path,
            exact_source_len,
            exact_source_sha256,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub const fn exact_source_len(&self) -> u64 {
        self.exact_source_len
    }

    pub const fn exact_source_sha256(&self) -> [u8; 32] {
        self.exact_source_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresentationAttachments {
    files: Arc<[SourceAttachment]>,
}

impl PresentationAttachments {
    pub(super) fn new(files: Vec<SourceAttachment>) -> Self {
        Self {
            files: files.into(),
        }
    }

    pub fn files(&self) -> &[SourceAttachment] {
        &self.files
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ImportMetrics {
    pub source_loading: Duration,
    pub parsing: Duration,
    pub hir_analysis: Duration,
    pub effect_analysis: Duration,
    pub source_files: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SnapshotIndexes {
    pub entities: Vec<EntityHeader>,
    pub nodes: Vec<NodeHeader>,
    pub containment: Vec<ContainmentEdge>,
    pub references: Vec<ReferenceEdge>,
    pub calls: Vec<CallEdge>,
    pub dependencies: Vec<DependencyEdge>,
    pub diagnostics: Vec<DiagnosticHeader>,
}

impl SnapshotIndexes {
    fn id_index(id: u64, len: usize, kind: &str) -> Result<usize> {
        let index = usize::try_from(id)
            .map_err(|_| Error::msg(format!("{kind} identity is not host-addressable")))?;
        if index >= len {
            return Err(Error::msg(format!("{kind} identity is stale")));
        }
        Ok(index)
    }

    pub(super) fn entity(
        &self,
        namespace: WorkspaceNamespace,
        id: EntityId,
    ) -> Result<&EntityHeader> {
        require_entity_namespace(namespace, id)?;
        let index = Self::id_index(id.slot(), self.entities.len(), "workspace entity")?;
        let header = &self.entities[index];
        if header.id != id {
            return Err(Error::msg("workspace entity generation is stale"));
        }
        Ok(header)
    }

    pub(super) fn node(&self, namespace: WorkspaceNamespace, id: NodeId) -> Result<&NodeHeader> {
        require_node_namespace(namespace, id)?;
        let index = Self::id_index(id.slot(), self.nodes.len(), "workspace node")?;
        let header = &self.nodes[index];
        if header.id != id {
            return Err(Error::msg("workspace node generation is stale"));
        }
        Ok(header)
    }
}

pub(super) fn require_entity_namespace(namespace: WorkspaceNamespace, id: EntityId) -> Result<()> {
    if id.namespace() != namespace {
        return Err(Error::msg(
            "entity belongs to a different workspace namespace",
        ));
    }
    if id.generation() == 0 {
        return Err(Error::msg("entity generation is invalid"));
    }
    Ok(())
}

pub(super) fn require_node_namespace(namespace: WorkspaceNamespace, id: NodeId) -> Result<()> {
    if id.namespace() != namespace {
        return Err(Error::msg(
            "node belongs to a different workspace namespace",
        ));
    }
    if id.generation() == 0 {
        return Err(Error::msg("node generation is invalid"));
    }
    Ok(())
}

pub(super) fn require_revision(
    namespace: WorkspaceNamespace,
    current: RevisionId,
    requested: RevisionId,
) -> Result<()> {
    if requested.namespace() != namespace {
        return Err(Error::msg(
            "revision belongs to a different workspace namespace",
        ));
    }
    if requested != current {
        return Err(Error::msg("workspace revision is stale"));
    }
    Ok(())
}
