use crate::{
    Error, InlineStructuralValue, LayoutIdentity, Result, SemanticTypeIdentity,
    StaticStructuralLeaf,
};

use super::StructuralSnapshotMetrics;

mod sealed;
pub use sealed::{
    SealedSemanticDagBorrow, SealedSemanticDagBorrowFailure, SealedSemanticDagError,
    SealedSemanticDagFailure, SealedSemanticDagMetrics, SealedSemanticDagOwner,
    SealedSemanticDagReleaseFailure, SealedSemanticDagReleaseReport, SealedSemanticDagRuntime,
};

/// One table-local semantic DAG node identity. It is not a runtime key.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticDagNodeId(u32);

impl SemanticDagNodeId {
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Closed wire shape for a key-free semantic DAG node.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticDagKind {
    Unit,
    Bool,
    I64,
    F64,
    Static,
    String,
    Path,
    Bytes,
    Product,
    Enum,
    EmptyList,
    List,
}

/// Exact semantic and runtime-layout identity required to interpret one node.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticDagType {
    pub layout: LayoutIdentity,
    pub semantic_type: SemanticTypeIdentity,
    pub kind: SemanticDagKind,
}

impl SemanticDagType {
    pub const fn new(
        layout: LayoutIdentity,
        semantic_type: SemanticTypeIdentity,
        kind: SemanticDagKind,
    ) -> Self {
        Self {
            layout,
            semantic_type,
            kind,
        }
    }
}

/// Payloads contain only semantic data and table-local node IDs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticDagPayload {
    Inline(InlineStructuralValue),
    Static(StaticStructuralLeaf),
    String(Vec<u8>),
    Path(Vec<u8>),
    Bytes(Vec<u8>),
    Product(Vec<SemanticDagNodeId>),
    Enum {
        tag: u64,
        fields: Vec<SemanticDagNodeId>,
    },
    EmptyList,
    List {
        head: SemanticDagNodeId,
        tail: SemanticDagNodeId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDagNode {
    pub value_type: SemanticDagType,
    pub payload: SemanticDagPayload,
}

impl SemanticDagNode {
    pub const fn new(value_type: SemanticDagType, payload: SemanticDagPayload) -> Self {
        Self {
            value_type,
            payload,
        }
    }
}

/// A validated, reverse-topological, key-free semantic DAG.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDagSnapshot {
    nodes: Vec<SemanticDagNode>,
    root: SemanticDagNodeId,
    metrics: StructuralSnapshotMetrics,
}

impl SemanticDagSnapshot {
    /// Validates privately owned candidate storage and publishes it only on
    /// complete success. Candidate vectors and payloads drop normally on error.
    pub fn new(nodes: Vec<SemanticDagNode>, root: SemanticDagNodeId) -> Result<Self> {
        let metrics = validate_semantic_dag(&nodes, root)?;
        Ok(Self {
            nodes,
            root,
            metrics,
        })
    }

    pub fn nodes(&self) -> &[SemanticDagNode] {
        &self.nodes
    }

    pub(super) fn nodes_mut(&mut self) -> &mut [SemanticDagNode] {
        &mut self.nodes
    }

    pub const fn root(&self) -> SemanticDagNodeId {
        self.root
    }

    pub fn root_node(&self) -> &SemanticDagNode {
        &self.nodes[self.root.0 as usize]
    }

    pub const fn metrics(&self) -> StructuralSnapshotMetrics {
        self.metrics
    }

    pub fn require_root_type(&self, expected: SemanticDagType) -> Result<()> {
        if self.root_node().value_type == expected {
            Ok(())
        } else {
            Err(Error::msg(
                "semantic DAG root type/layout identity mismatch",
            ))
        }
    }

    pub(super) fn validate_encode(&self) -> Result<StructuralSnapshotMetrics> {
        let metrics = validate_semantic_dag(&self.nodes, self.root)?;
        if metrics == self.metrics {
            Ok(metrics)
        } else {
            Err(Error::msg("semantic DAG snapshot metrics disagree"))
        }
    }

    pub(super) fn from_decoded(
        nodes: Vec<SemanticDagNode>,
        root: SemanticDagNodeId,
        measured: StructuralSnapshotMetrics,
    ) -> Result<Self> {
        let metrics = validate_semantic_dag(&nodes, root)?;
        if metrics != measured {
            return Err(Error::msg("semantic DAG decode accounting disagrees"));
        }
        Ok(Self {
            nodes,
            root,
            metrics,
        })
    }
}

include!("semantic_dag/validation_support.rs");
include!("semantic_dag/validation.rs");
