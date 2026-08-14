use crate::ids::NodeId;
use crate::transaction::NodeTarget;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SemanticType {
    Unit,
    Bool,
    I64,
}

impl SemanticType {
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::Unit => 1,
            Self::Bool => 2,
            Self::I64 => 3,
        }
    }

    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Unit),
            2 => Some(Self::Bool),
            3 => Some(Self::I64),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NodeKind {
    WorkspaceRoot,
    Package,
    Module,
    Function,
    Parameter,
    Region,
    Block,
    Operation,
}

impl NodeKind {
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::WorkspaceRoot => 1,
            Self::Package => 2,
            Self::Module => 3,
            Self::Function => 4,
            Self::Parameter => 5,
            Self::Region => 6,
            Self::Block => 7,
            Self::Operation => 8,
        }
    }

    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::WorkspaceRoot),
            2 => Some(Self::Package),
            3 => Some(Self::Module),
            4 => Some(Self::Function),
            5 => Some(Self::Parameter),
            6 => Some(Self::Region),
            7 => Some(Self::Block),
            8 => Some(Self::Operation),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OperandUse {
    Copy,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ValueRef {
    FunctionParameter(NodeId),
    OperationResult { operation: NodeId, output: u8 },
}

impl ValueRef {
    pub const fn referenced_node(self) -> NodeId {
        match self {
            Self::FunctionParameter(parameter) => parameter,
            Self::OperationResult { operation, .. } => operation,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ValueDraft {
    FunctionParameter(NodeTarget),
    OperationResult { operation: NodeTarget, output: u8 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationDraft {
    ConstI64(i64),
    ConstBool(bool),
    AddI64 { lhs: ValueDraft, rhs: ValueDraft },
    Hole { expected: SemanticType },
    Return { value: ValueDraft },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationKind {
    ConstI64(i64),
    ConstBool(bool),
    AddI64 { lhs: ValueRef, rhs: ValueRef },
    Hole { expected: SemanticType },
    Return { value: ValueRef },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationContract {
    pub operand_types: Vec<SemanticType>,
    pub operand_uses: Vec<OperandUse>,
    pub result_types: Vec<SemanticType>,
    pub terminator: bool,
    pub complete: bool,
}

impl OperationKind {
    pub const fn stable_tag(&self) -> u8 {
        match self {
            Self::ConstI64(_) => 1,
            Self::ConstBool(_) => 2,
            Self::AddI64 { .. } => 3,
            Self::Hole { .. } => 4,
            Self::Return { .. } => 5,
        }
    }

    pub fn contract(&self) -> OperationContract {
        match self {
            Self::ConstI64(_) => OperationContract {
                operand_types: Vec::new(),
                operand_uses: Vec::new(),
                result_types: vec![SemanticType::I64],
                terminator: false,
                complete: true,
            },
            Self::ConstBool(_) => OperationContract {
                operand_types: Vec::new(),
                operand_uses: Vec::new(),
                result_types: vec![SemanticType::Bool],
                terminator: false,
                complete: true,
            },
            Self::AddI64 { .. } => OperationContract {
                operand_types: vec![SemanticType::I64, SemanticType::I64],
                operand_uses: vec![OperandUse::Copy, OperandUse::Copy],
                result_types: vec![SemanticType::I64],
                terminator: false,
                complete: true,
            },
            Self::Hole { expected } => OperationContract {
                operand_types: Vec::new(),
                operand_uses: Vec::new(),
                result_types: vec![*expected],
                terminator: false,
                complete: false,
            },
            Self::Return { .. } => OperationContract {
                operand_types: Vec::new(),
                operand_uses: vec![OperandUse::Copy],
                result_types: Vec::new(),
                terminator: true,
                complete: true,
            },
        }
    }

    pub fn operands(&self) -> Vec<ValueRef> {
        match self {
            Self::ConstI64(_) | Self::ConstBool(_) | Self::Hole { .. } => Vec::new(),
            Self::AddI64 { lhs, rhs } => vec![*lhs, *rhs],
            Self::Return { value } => vec![*value],
        }
    }

    pub fn replace_operand(&mut self, index: u8, replacement: ValueRef) -> bool {
        match (self, index) {
            (Self::AddI64 { lhs, .. }, 0) => {
                *lhs = replacement;
                true
            }
            (Self::AddI64 { rhs, .. }, 1) => {
                *rhs = replacement;
                true
            }
            (Self::Return { value }, 0) => {
                *value = replacement;
                true
            }
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Node {
    WorkspaceRoot {
        packages: Vec<NodeId>,
    },
    Package {
        owner: NodeId,
        name: String,
        modules: Vec<NodeId>,
        entry: Option<NodeId>,
    },
    Module {
        owner: NodeId,
        name: String,
        functions: Vec<NodeId>,
    },
    Function {
        owner: NodeId,
        name: String,
        parameters: Vec<NodeId>,
        result: SemanticType,
        body: Option<NodeId>,
    },
    Parameter {
        owner: NodeId,
        ordinal: u32,
        name: String,
        ty: SemanticType,
    },
    Region {
        owner: NodeId,
        blocks: Vec<NodeId>,
    },
    Block {
        owner: NodeId,
        operations: Vec<NodeId>,
        terminator: Option<NodeId>,
    },
    Operation {
        owner: NodeId,
        operation: OperationKind,
    },
}

impl Node {
    pub const fn kind(&self) -> NodeKind {
        match self {
            Self::WorkspaceRoot { .. } => NodeKind::WorkspaceRoot,
            Self::Package { .. } => NodeKind::Package,
            Self::Module { .. } => NodeKind::Module,
            Self::Function { .. } => NodeKind::Function,
            Self::Parameter { .. } => NodeKind::Parameter,
            Self::Region { .. } => NodeKind::Region,
            Self::Block { .. } => NodeKind::Block,
            Self::Operation { .. } => NodeKind::Operation,
        }
    }

    pub const fn owner(&self) -> Option<NodeId> {
        match self {
            Self::WorkspaceRoot { .. } => None,
            Self::Package { owner, .. }
            | Self::Module { owner, .. }
            | Self::Function { owner, .. }
            | Self::Parameter { owner, .. }
            | Self::Region { owner, .. }
            | Self::Block { owner, .. }
            | Self::Operation { owner, .. } => Some(*owner),
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Package { name, .. }
            | Self::Module { name, .. }
            | Self::Function { name, .. }
            | Self::Parameter { name, .. } => Some(name),
            _ => None,
        }
    }

    pub fn set_name(&mut self, replacement: String) -> bool {
        match self {
            Self::Package { name, .. }
            | Self::Module { name, .. }
            | Self::Function { name, .. }
            | Self::Parameter { name, .. } => {
                *name = replacement;
                true
            }
            _ => false,
        }
    }

    pub fn owned_children(&self) -> Vec<NodeId> {
        match self {
            Self::WorkspaceRoot { packages } => packages.clone(),
            Self::Package { modules, .. } => modules.clone(),
            Self::Module { functions, .. } => functions.clone(),
            Self::Function {
                parameters, body, ..
            } => {
                let mut children = parameters.clone();
                if let Some(body) = body {
                    children.push(*body);
                }
                children
            }
            Self::Parameter { .. } | Self::Operation { .. } => Vec::new(),
            Self::Region { blocks, .. } => blocks.clone(),
            Self::Block {
                operations,
                terminator,
                ..
            } => {
                let mut children = operations.clone();
                if let Some(terminator) = terminator {
                    children.push(*terminator);
                }
                children
            }
        }
    }

    pub fn direct_references(&self) -> Vec<NodeId> {
        match self {
            Self::Package { entry, .. } => entry.iter().copied().collect(),
            Self::Operation { operation, .. } => operation
                .operands()
                .into_iter()
                .map(ValueRef::referenced_node)
                .collect(),
            _ => Vec::new(),
        }
    }
}

pub const fn expected_owner_kind(kind: NodeKind) -> Option<NodeKind> {
    match kind {
        NodeKind::WorkspaceRoot => None,
        NodeKind::Package => Some(NodeKind::WorkspaceRoot),
        NodeKind::Module => Some(NodeKind::Package),
        NodeKind::Function => Some(NodeKind::Module),
        NodeKind::Parameter => Some(NodeKind::Function),
        NodeKind::Region => None,
        NodeKind::Block => Some(NodeKind::Region),
        NodeKind::Operation => Some(NodeKind::Block),
    }
}

pub fn owner_kind_is_valid(child: NodeKind, owner: NodeKind) -> bool {
    match child {
        NodeKind::Region => matches!(owner, NodeKind::Function | NodeKind::Operation),
        _ => expected_owner_kind(child) == Some(owner),
    }
}
