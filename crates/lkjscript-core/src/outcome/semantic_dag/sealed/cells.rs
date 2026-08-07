use super::super::{SemanticDagNodeId, SemanticDagType};
use crate::{InlineStructuralValue, StaticStructuralLeaf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SealedDagNodePayload {
    Inline(InlineStructuralValue),
    Static(StaticStructuralLeaf),
    Bytes {
        first: u64,
        chunks: u64,
        length: u64,
    },
    Product {
        first: u64,
        fields: u64,
    },
    Enum {
        tag: u64,
        first: u64,
        fields: u64,
    },
    EmptyList,
    List {
        head: SemanticDagNodeId,
        tail: SemanticDagNodeId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SealedDagNodeCell {
    pub value_type: SemanticDagType,
    pub payload: SealedDagNodePayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SealedDagCell {
    Node(SealedDagNodeCell),
    Child(SemanticDagNodeId),
    Bytes {
        length: u8,
        bytes: [u8; super::SEALED_DAG_BYTE_CHUNK],
    },
}
