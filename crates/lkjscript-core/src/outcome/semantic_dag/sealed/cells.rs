use super::super::{SemanticDagNodeId, SemanticDagType};
use crate::{InlineStructuralValue, StaticStructuralLeaf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SealedDagNodePayload {
    Inline(InlineStructuralValue),
    Static(StaticStructuralLeaf),
    Bytes {
        first: u32,
        chunks: u32,
        length: u32,
    },
    Product {
        first: u32,
        fields: u32,
    },
    Enum {
        tag: u64,
        first: u32,
        fields: u32,
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
