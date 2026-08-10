use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use lkjscript_core::{Error, Result};

use super::{EntityId, NodeId, RevisionId, WorkspaceNamespace};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProgramState {
    Complete,
    Incomplete,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum EntityKind {
    Main,
    Parameter,
    ImmutableLocal,
    StaticBytesLocal,
    MutableLocal,
    Function,
    TypeParameter,
    BuiltinOperation,
    Product,
    ProductField,
    Enum,
    EnumVariant,
    EnumField,
    Trait,
    Implementation,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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
    Match,
    MatchUnreachable,
    Symbol,
    Hole,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticOwner {
    Entity(EntityId),
    Node(NodeId),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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

/// Stable identity of an incomplete expression goal.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HoleId(pub(crate) NodeId);

impl HoleId {
    pub const fn node(self) -> NodeId {
        self.0
    }
}

/// Public, typed context for one editable hole. Its backing HIR address remains private.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoleState {
    pub id: HoleId,
    pub kind: HoleKind,
    pub expected_type: super::SemanticType,
    pub goal: Arc<str>,
    pub owner: EntityId,
    pub context: NodeId,
    pub visible_entities: Arc<[EntityId]>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum HoleKind {
    MissingBody,
    TypedExpression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompletenessBlocker {
    MissingEntryPoint,
    MissingBody {
        declaration: EntityId,
        hole: HoleId,
        expected_type: super::SemanticType,
    },
    TypedHole {
        hole: HoleId,
        expected_type: super::SemanticType,
        owner: EntityId,
        context: NodeId,
    },
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) enum EntityAddress {
    Main,
    Binding(u64),
    FunctionTypeParameter {
        function: u64,
        ordinal: u64,
    },
    Product(u64),
    ProductField {
        product: u64,
        field: u64,
    },
    Enum(u64),
    EnumTypeParameter {
        enumeration: u64,
        ordinal: u64,
    },
    EnumVariant {
        enumeration: u64,
        variant: u64,
    },
    EnumField {
        enumeration: u64,
        variant: u64,
        field: u64,
    },
    Trait(u64),
    Implementation(u64),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct NodeAddress {
    pub root: EntityAddress,
    pub preorder: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct NodeKey {
    pub owner: SemanticOwner,
    pub ordinal: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct HoleRecord {
    pub state: HoleState,
    pub expected_internal: crate::Type,
    pub address: NodeAddress,
    pub key: NodeKey,
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
    pub package_validation: Duration,
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
    pub declaration_dependencies: Vec<DependencyEdge>,
    pub diagnostics: Vec<DiagnosticHeader>,
    pub entity_addresses: Vec<EntityAddress>,
    pub node_addresses: Vec<NodeAddress>,
    pub node_keys: Vec<NodeKey>,
    pub node_match_plans: Vec<Option<crate::hir::MatchPlanId>>,
    pub node_enclosing_entities: Vec<EntityId>,
    pub node_actual_types: Vec<crate::Type>,
    pub node_expected_types: Vec<Option<crate::Type>>,
    pub entity_types: Vec<Option<crate::Type>>,
    pub entity_lookup: HashMap<EntityId, usize>,
    pub node_lookup: HashMap<NodeId, usize>,
    pub node_children: HashMap<NodeId, Vec<NodeId>>,
    pub product_name_indices: HashMap<String, usize>,
    pub enum_identity_indices: HashMap<crate::hir::EnumId, usize>,
    pub variant_identity_indices:
        HashMap<(crate::hir::EnumId, crate::hir::VariantId), (usize, usize)>,
    pub address_entities: HashMap<EntityAddress, EntityId>,
    pub address_nodes: HashMap<NodeAddress, NodeId>,
    pub type_parameter_entities: HashMap<EntityId, HashMap<Arc<str>, EntityId>>,
}

impl SnapshotIndexes {
    pub(super) fn rebuild_maps(&mut self) -> Result<()> {
        self.entity_lookup.clear();
        self.node_lookup.clear();
        self.node_children.clear();
        self.address_entities.clear();
        self.address_nodes.clear();
        self.type_parameter_entities.clear();
        self.entity_lookup
            .try_reserve(self.entities.len())
            .map_err(|_| Error::host("workspace entity lookup allocation failed"))?;
        self.node_lookup
            .try_reserve(self.nodes.len())
            .map_err(|_| Error::host("workspace node lookup allocation failed"))?;
        self.node_children
            .try_reserve(self.nodes.len())
            .map_err(|_| Error::host("workspace node-child allocation failed"))?;
        self.address_entities
            .try_reserve(self.entities.len())
            .map_err(|_| Error::host("workspace entity address allocation failed"))?;
        self.address_nodes
            .try_reserve(self.nodes.len())
            .map_err(|_| Error::host("workspace node address allocation failed"))?;
        self.type_parameter_entities
            .try_reserve(self.entities.len())
            .map_err(|_| Error::host("workspace type-parameter lookup allocation failed"))?;
        for (index, (header, address)) in
            self.entities.iter().zip(&self.entity_addresses).enumerate()
        {
            self.entity_lookup.insert(header.id, index);
            self.address_entities.insert(*address, header.id);
            if header.kind == EntityKind::TypeParameter {
                let owner = header
                    .owner
                    .ok_or_else(|| Error::msg("workspace type parameter is missing its owner"))?;
                let parameters = self.type_parameter_entities.entry(owner).or_default();
                parameters.try_reserve(1).map_err(|_| {
                    Error::host("workspace type-parameter lookup allocation failed")
                })?;
                if parameters
                    .insert(Arc::clone(&header.name), header.id)
                    .is_some()
                {
                    return Err(Error::msg(
                        "workspace type parameter name is duplicated for its owner",
                    ));
                }
            }
        }
        for (index, (header, address)) in self.nodes.iter().zip(&self.node_addresses).enumerate() {
            self.node_lookup.insert(header.id, index);
            self.address_nodes.insert(*address, header.id);
            if let SemanticOwner::Node(owner) = header.owner {
                self.node_children.entry(owner).or_default().push(header.id);
            }
        }
        Ok(())
    }

    pub(super) fn entity(
        &self,
        namespace: WorkspaceNamespace,
        id: EntityId,
    ) -> Result<&EntityHeader> {
        require_entity_namespace(namespace, id)?;
        let Some(index) = self.entity_lookup.get(&id).copied() else {
            return Err(Error::msg("workspace entity identity is stale"));
        };
        self.entities
            .get(index)
            .filter(|header| header.id == id)
            .ok_or_else(|| Error::msg("workspace entity generation is stale"))
    }

    pub(super) fn node(&self, namespace: WorkspaceNamespace, id: NodeId) -> Result<&NodeHeader> {
        require_node_namespace(namespace, id)?;
        let Some(index) = self.node_lookup.get(&id).copied() else {
            return Err(Error::msg("workspace node identity is stale"));
        };
        self.nodes
            .get(index)
            .filter(|header| header.id == id)
            .ok_or_else(|| Error::msg("workspace node generation is stale"))
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
