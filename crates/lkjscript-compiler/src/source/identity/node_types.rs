use std::fmt;

use crate::source::{SourceOrigin, SourceSpan};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RevisionId(pub(crate) [u8; 32]);

impl RevisionId {
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn to_hex(self) -> String {
        crate::source::identity::hex(&self.0)
    }
}

impl fmt::Display for RevisionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&crate::source::identity::hex(&self.0))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId {
    pub(crate) revision: RevisionId,
    pub(crate) index: u32,
}

impl NodeId {
    pub const fn revision(self) -> RevisionId {
        self.revision
    }

    pub const fn index(self) -> u32 {
        self.index
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum NodeKind {
    I64Literal,
    F64Literal,
    BoolLiteral,
    UnitLiteral,
    StringLiteral,
    BytesLiteral,
    Symbol,
    Call,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeSummary {
    pub(crate) id: NodeId,
    pub(crate) kind: NodeKind,
    pub(crate) label: Option<String>,
    pub(crate) origin: SourceOrigin,
    pub(crate) span: SourceSpan,
    pub(crate) parent: Option<NodeId>,
    pub(crate) children: Vec<NodeId>,
}

impl NodeSummary {
    pub const fn id(&self) -> NodeId {
        self.id
    }

    pub const fn kind(&self) -> NodeKind {
        self.kind
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn origin(&self) -> &SourceOrigin {
        &self.origin
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    pub const fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    pub fn children(&self) -> &[NodeId] {
        &self.children
    }
}
