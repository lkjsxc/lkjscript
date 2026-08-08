use super::EntityId;

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
    Load(EntityId),
    Call {
        callee: EntityId,
        arguments: Vec<DraftNodeId>,
    },
    If {
        condition: DraftNodeId,
        then_branch: DraftNodeId,
        else_branch: DraftNodeId,
    },
}

impl DraftNode {
    pub(super) fn for_each_child(&self, mut visit: impl FnMut(DraftNodeId)) {
        match self {
            Self::Call { arguments, .. } => {
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
            _ => {}
        }
    }
}
