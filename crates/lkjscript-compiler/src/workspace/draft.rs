use super::EntityId;
use crate::operation::Operation;

/// Dense identity into one flat expression draft. It is never a workspace identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DraftNodeId(u64);

impl DraftNodeId {
    pub const fn new(index: u64) -> Self {
        Self(index)
    }

    pub(super) fn index(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}

/// Dense identity into one flat match-pattern draft. It is never a workspace identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DraftPatternNodeId(u64);

impl DraftPatternNodeId {
    pub const fn new(index: u64) -> Self {
        Self(index)
    }

    pub(super) fn index(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}

/// Identity of one immutable local inside a single [`ExpressionDraft`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DraftBindingId(u64);

impl DraftBindingId {
    pub const fn new(index: u64) -> Self {
        Self(index)
    }

    pub(super) const fn raw(self) -> u64 {
        self.0
    }
}

/// A binding reference whose identity domain is explicit.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DraftBindingRef {
    /// A stable parameter or local already published in this workspace.
    Entity(EntityId),
    /// A transaction-local immutable binding in this draft.
    Local(DraftBindingId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalDraft {
    pub binding: DraftBindingId,
    pub name: String,
    pub value: DraftNodeId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DraftFieldValue {
    pub field: EntityId,
    pub value: DraftNodeId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DraftPatternField {
    pub field: EntityId,
    pub pattern: DraftPatternNodeId,
}

/// One flat non-recursive pattern tree owned by a match arm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatternDraft {
    pub nodes: Vec<DraftPatternNode>,
    pub root: DraftPatternNodeId,
}

impl PatternDraft {
    pub fn new(nodes: Vec<DraftPatternNode>, root: DraftPatternNodeId) -> Self {
        Self { nodes, root }
    }

    pub fn wildcard() -> Self {
        Self::new(vec![DraftPatternNode::Wildcard], DraftPatternNodeId::new(0))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DraftPatternNode {
    Wildcard,
    Binding {
        binding: DraftBindingId,
        name: String,
    },
    EnumVariant {
        variant: EntityId,
        fields: Vec<DraftPatternField>,
    },
}

impl DraftPatternNode {
    pub(super) fn child_count(&self) -> usize {
        match self {
            Self::EnumVariant { fields, .. } => fields.len(),
            Self::Wildcard | Self::Binding { .. } => 0,
        }
    }

    pub(super) fn for_each_child(&self, mut visit: impl FnMut(DraftPatternNodeId)) {
        if let Self::EnumVariant { fields, .. } = self {
            for field in fields {
                visit(field.pattern);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchArmDraft {
    pub pattern: PatternDraft,
    pub body: DraftNodeId,
}

/// A non-recursive proposed expression graph. Child IDs refer to entries in `nodes`.
#[derive(Clone, Debug, PartialEq)]
pub struct ExpressionDraft {
    pub nodes: Vec<DraftNode>,
    pub root: DraftNodeId,
}

impl ExpressionDraft {
    pub fn new(nodes: Vec<DraftNode>, root: DraftNodeId) -> Self {
        Self { nodes, root }
    }

    pub fn scalar_i64(value: i64) -> Self {
        Self::new(vec![DraftNode::I64(value)], DraftNodeId::new(0))
    }

    pub fn scalar_f64(value: f64) -> Self {
        Self::new(vec![DraftNode::F64(value)], DraftNodeId::new(0))
    }

    pub fn scalar_bool(value: bool) -> Self {
        Self::new(vec![DraftNode::Bool(value)], DraftNodeId::new(0))
    }

    pub fn unit() -> Self {
        Self::new(vec![DraftNode::Unit], DraftNodeId::new(0))
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DraftNode {
    I64(i64),
    F64(f64),
    Bool(bool),
    Unit,
    Bytes(Vec<u8>),
    Load(DraftBindingRef),
    Move(DraftBindingRef),
    BorrowShared(DraftBindingRef),
    Call {
        callee: EntityId,
        arguments: Vec<DraftNodeId>,
    },
    Operation {
        operation: Operation,
        arguments: Vec<DraftNodeId>,
    },
    If {
        condition: DraftNodeId,
        then_branch: DraftNodeId,
        else_branch: DraftNodeId,
    },
    Let {
        bindings: Vec<LocalDraft>,
        body: DraftNodeId,
    },
    ProductValue {
        product: EntityId,
        fields: Vec<DraftFieldValue>,
    },
    ProductField {
        field: EntityId,
        value: DraftNodeId,
    },
    EnumValue {
        variant: EntityId,
        fields: Vec<DraftFieldValue>,
    },
    EnumIsVariant {
        variant: EntityId,
        value: DraftNodeId,
    },
    Match {
        scrutinee: DraftNodeId,
        arms: Vec<MatchArmDraft>,
    },
}

impl DraftNode {
    pub(super) fn child_count(&self) -> Option<usize> {
        match self {
            Self::Call { arguments, .. } | Self::Operation { arguments, .. } => {
                Some(arguments.len())
            }
            Self::If { .. } => Some(3),
            Self::Let { bindings, .. } => bindings.len().checked_add(1),
            Self::ProductValue { fields, .. } | Self::EnumValue { fields, .. } => {
                Some(fields.len())
            }
            Self::ProductField { .. } | Self::EnumIsVariant { .. } => Some(1),
            Self::Match { arms, .. } => arms.len().checked_add(1),
            _ => Some(0),
        }
    }

    pub(super) fn for_each_child(&self, mut visit: impl FnMut(DraftNodeId)) {
        match self {
            Self::Call { arguments, .. } | Self::Operation { arguments, .. } => {
                for child in arguments {
                    visit(*child);
                }
            }
            Self::If {
                condition,
                then_branch,
                else_branch,
            } => {
                visit(*condition);
                visit(*then_branch);
                visit(*else_branch);
            }
            Self::Let { bindings, body } => {
                for binding in bindings {
                    visit(binding.value);
                }
                visit(*body);
            }
            Self::ProductValue { fields, .. } | Self::EnumValue { fields, .. } => {
                for field in fields {
                    visit(field.value);
                }
            }
            Self::ProductField { value, .. } | Self::EnumIsVariant { value, .. } => visit(*value),
            Self::Match { scrutinee, arms } => {
                visit(*scrutinee);
                for arm in arms {
                    visit(arm.body);
                }
            }
            _ => {}
        }
    }
}
