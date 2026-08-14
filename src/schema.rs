use crate::ids::NodeId;
use crate::transaction::NodeTarget;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticType {
    Unit,
    Bool,
    I64,
}

impl SemanticType {
    pub const ALL: [Self; 3] = [Self::Unit, Self::Bool, Self::I64];

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Bool => "bool",
            Self::I64 => "i64",
        }
    }

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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
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
    pub const ALL: [Self; 8] = [
        Self::WorkspaceRoot,
        Self::Package,
        Self::Module,
        Self::Function,
        Self::Parameter,
        Self::Region,
        Self::Block,
        Self::Operation,
    ];

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::WorkspaceRoot => "workspace_root",
            Self::Package => "package",
            Self::Module => "module",
            Self::Function => "function",
            Self::Parameter => "parameter",
            Self::Region => "region",
            Self::Block => "block",
            Self::Operation => "operation",
        }
    }

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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperandUse {
    Copy,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationCode {
    ConstI64,
    ConstBool,
    AddI64,
    Hole,
    Return,
}

impl OperationCode {
    pub const ALL: [Self; 5] = [
        Self::ConstI64,
        Self::ConstBool,
        Self::AddI64,
        Self::Hole,
        Self::Return,
    ];

    pub const fn stable_tag(self) -> u8 {
        self.descriptor().stable_tag
    }

    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::ConstI64),
            2 => Some(Self::ConstBool),
            3 => Some(Self::AddI64),
            4 => Some(Self::Hole),
            5 => Some(Self::Return),
            _ => None,
        }
    }

    pub const fn machine_name(self) -> &'static str {
        self.descriptor().machine_name
    }

    pub const fn descriptor(self) -> &'static OperationDescriptor {
        match self {
            Self::ConstI64 => &CONST_I64_DESCRIPTOR,
            Self::ConstBool => &CONST_BOOL_DESCRIPTOR,
            Self::AddI64 => &ADD_I64_DESCRIPTOR,
            Self::Hole => &HOLE_DESCRIPTOR,
            Self::Return => &RETURN_DESCRIPTOR,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TypeRule {
    Fixed(SemanticType),
    PayloadExpected,
    OwnerFunctionResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiteralField {
    I64Value,
    BoolValue,
    ExpectedType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperandDescriptor {
    pub ty: TypeRule,
    pub use_mode: OperandUse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationDescriptor {
    pub code: OperationCode,
    pub machine_name: &'static str,
    pub stable_tag: u8,
    pub operands: &'static [OperandDescriptor],
    pub results: &'static [TypeRule],
    pub literal_fields: &'static [LiteralField],
    pub terminator: bool,
    pub complete: bool,
}

const NO_OPERANDS: &[OperandDescriptor] = &[];
const NO_RESULTS: &[TypeRule] = &[];
const NO_LITERALS: &[LiteralField] = &[];
const I64_RESULT: &[TypeRule] = &[TypeRule::Fixed(SemanticType::I64)];
const BOOL_RESULT: &[TypeRule] = &[TypeRule::Fixed(SemanticType::Bool)];
const PAYLOAD_RESULT: &[TypeRule] = &[TypeRule::PayloadExpected];
const ADD_I64_OPERANDS: &[OperandDescriptor] = &[
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::I64),
        use_mode: OperandUse::Copy,
    },
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::I64),
        use_mode: OperandUse::Copy,
    },
];
const RETURN_OPERANDS: &[OperandDescriptor] = &[OperandDescriptor {
    ty: TypeRule::OwnerFunctionResult,
    use_mode: OperandUse::Copy,
}];
const I64_LITERAL: &[LiteralField] = &[LiteralField::I64Value];
const BOOL_LITERAL: &[LiteralField] = &[LiteralField::BoolValue];
const EXPECTED_LITERAL: &[LiteralField] = &[LiteralField::ExpectedType];

static CONST_I64_DESCRIPTOR: OperationDescriptor = OperationDescriptor {
    code: OperationCode::ConstI64,
    machine_name: "const_i64",
    stable_tag: 1,
    operands: NO_OPERANDS,
    results: I64_RESULT,
    literal_fields: I64_LITERAL,
    terminator: false,
    complete: true,
};
static CONST_BOOL_DESCRIPTOR: OperationDescriptor = OperationDescriptor {
    code: OperationCode::ConstBool,
    machine_name: "const_bool",
    stable_tag: 2,
    operands: NO_OPERANDS,
    results: BOOL_RESULT,
    literal_fields: BOOL_LITERAL,
    terminator: false,
    complete: true,
};
static ADD_I64_DESCRIPTOR: OperationDescriptor = OperationDescriptor {
    code: OperationCode::AddI64,
    machine_name: "add_i64",
    stable_tag: 3,
    operands: ADD_I64_OPERANDS,
    results: I64_RESULT,
    literal_fields: NO_LITERALS,
    terminator: false,
    complete: true,
};
static HOLE_DESCRIPTOR: OperationDescriptor = OperationDescriptor {
    code: OperationCode::Hole,
    machine_name: "hole",
    stable_tag: 4,
    operands: NO_OPERANDS,
    results: PAYLOAD_RESULT,
    literal_fields: EXPECTED_LITERAL,
    terminator: false,
    complete: false,
};
static RETURN_DESCRIPTOR: OperationDescriptor = OperationDescriptor {
    code: OperationCode::Return,
    machine_name: "return",
    stable_tag: 5,
    operands: RETURN_OPERANDS,
    results: NO_RESULTS,
    literal_fields: NO_LITERALS,
    terminator: true,
    complete: true,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ValueDraft {
    FunctionParameter(NodeTarget),
    OperationResult { operation: NodeTarget, output: u8 },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OperationDraft {
    ConstI64(i64),
    ConstBool(bool),
    AddI64 { lhs: ValueDraft, rhs: ValueDraft },
    Hole { expected: SemanticType },
    Return { value: ValueDraft },
}

impl OperationDraft {
    pub const fn code(&self) -> OperationCode {
        match self {
            Self::ConstI64(_) => OperationCode::ConstI64,
            Self::ConstBool(_) => OperationCode::ConstBool,
            Self::AddI64 { .. } => OperationCode::AddI64,
            Self::Hole { .. } => OperationCode::Hole,
            Self::Return { .. } => OperationCode::Return,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OperationKind {
    ConstI64(i64),
    ConstBool(bool),
    AddI64 { lhs: ValueRef, rhs: ValueRef },
    Hole { expected: SemanticType },
    Return { value: ValueRef },
}

impl OperationKind {
    pub const fn code(&self) -> OperationCode {
        match self {
            Self::ConstI64(_) => OperationCode::ConstI64,
            Self::ConstBool(_) => OperationCode::ConstBool,
            Self::AddI64 { .. } => OperationCode::AddI64,
            Self::Hole { .. } => OperationCode::Hole,
            Self::Return { .. } => OperationCode::Return,
        }
    }

    pub const fn stable_tag(&self) -> u8 {
        self.code().stable_tag()
    }

    pub const fn descriptor(&self) -> &'static OperationDescriptor {
        self.code().descriptor()
    }

    pub fn operand_count(&self) -> usize {
        self.descriptor().operands.len()
    }

    pub const fn operand(&self, index: usize) -> Option<ValueRef> {
        match (self, index) {
            (Self::AddI64 { lhs, .. }, 0) => Some(*lhs),
            (Self::AddI64 { rhs, .. }, 1) => Some(*rhs),
            (Self::Return { value }, 0) => Some(*value),
            _ => None,
        }
    }

    pub fn operand_type(
        &self,
        index: usize,
        owner_function_result: Option<SemanticType>,
    ) -> Option<SemanticType> {
        let rule = self.descriptor().operands.get(index)?.ty;
        self.resolve_type_rule(rule, owner_function_result)
    }

    pub fn operand_use(&self, index: usize) -> Option<OperandUse> {
        self.descriptor()
            .operands
            .get(index)
            .map(|operand| operand.use_mode)
    }

    pub fn result_count(&self) -> usize {
        self.descriptor().results.len()
    }

    pub fn result_type(
        &self,
        index: usize,
        owner_function_result: Option<SemanticType>,
    ) -> Option<SemanticType> {
        let rule = *self.descriptor().results.get(index)?;
        self.resolve_type_rule(rule, owner_function_result)
    }

    pub const fn is_terminator(&self) -> bool {
        self.descriptor().terminator
    }

    pub const fn is_complete(&self) -> bool {
        self.descriptor().complete
    }

    pub fn same_result_contract(&self, other: &Self) -> bool {
        self.result_count() == other.result_count()
            && (0..self.result_count())
                .all(|index| self.result_type(index, None) == other.result_type(index, None))
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

    const fn resolve_type_rule(
        &self,
        rule: TypeRule,
        owner_function_result: Option<SemanticType>,
    ) -> Option<SemanticType> {
        match rule {
            TypeRule::Fixed(ty) => Some(ty),
            TypeRule::PayloadExpected => match self {
                Self::Hole { expected } => Some(*expected),
                _ => None,
            },
            TypeRule::OwnerFunctionResult => owner_function_result,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DirectReference {
    Definition { target: NodeId },
    ValueOperand { index: u8, value: ValueRef },
}

impl DirectReference {
    pub const fn target(self) -> NodeId {
        match self {
            Self::Definition { target } => target,
            Self::ValueOperand { value, .. } => value.referenced_node(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
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

    pub fn owned_child_count(&self) -> usize {
        match self {
            Self::WorkspaceRoot { packages } => packages.len(),
            Self::Package { modules, .. } => modules.len(),
            Self::Module { functions, .. } => functions.len(),
            Self::Function {
                parameters, body, ..
            } => parameters.len() + usize::from(body.is_some()),
            Self::Parameter { .. } | Self::Operation { .. } => 0,
            Self::Region { blocks, .. } => blocks.len(),
            Self::Block {
                operations,
                terminator,
                ..
            } => operations.len() + usize::from(terminator.is_some()),
        }
    }

    pub fn owned_child(&self, index: usize) -> Option<NodeId> {
        match self {
            Self::WorkspaceRoot { packages } => packages.get(index).copied(),
            Self::Package { modules, .. } => modules.get(index).copied(),
            Self::Module { functions, .. } => functions.get(index).copied(),
            Self::Function {
                parameters, body, ..
            } => parameters
                .get(index)
                .copied()
                .or_else(|| (index == parameters.len()).then_some(*body).flatten()),
            Self::Parameter { .. } | Self::Operation { .. } => None,
            Self::Region { blocks, .. } => blocks.get(index).copied(),
            Self::Block {
                operations,
                terminator,
                ..
            } => operations
                .get(index)
                .copied()
                .or_else(|| (index == operations.len()).then_some(*terminator).flatten()),
        }
    }

    pub fn direct_reference_count(&self) -> usize {
        match self {
            Self::Package { entry, .. } => usize::from(entry.is_some()),
            Self::Operation { operation, .. } => operation.operand_count(),
            _ => 0,
        }
    }

    pub fn direct_reference(&self, index: usize) -> Option<DirectReference> {
        match self {
            Self::Package { entry, .. } if index == 0 => {
                entry.map(|target| DirectReference::Definition { target })
            }
            Self::Operation { operation, .. } => {
                let operand_index = u8::try_from(index).ok()?;
                operation
                    .operand(index)
                    .map(|value| DirectReference::ValueOperand {
                        index: operand_index,
                        value,
                    })
            }
            _ => None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn value(serial: u64) -> ValueRef {
        let workspace = crate::ids::WorkspaceId::from_bytes([0x51; 16]);
        ValueRef::OperationResult {
            operation: NodeId::new(workspace, serial).expect("node identity"),
            output: 0,
        }
    }

    #[test]
    fn operation_descriptors_are_unique_complete_and_context_accurate() {
        let mut tags = BTreeSet::new();
        let mut names = BTreeSet::new();
        for code in OperationCode::ALL {
            let descriptor = code.descriptor();
            assert_eq!(descriptor.code, code);
            assert_eq!(descriptor.stable_tag, code.stable_tag());
            assert_eq!(descriptor.machine_name, code.machine_name());
            assert_ne!(descriptor.stable_tag, 0);
            assert!(!descriptor.machine_name.is_empty());
            assert!(tags.insert(descriptor.stable_tag));
            assert!(names.insert(descriptor.machine_name));
            assert_eq!(
                OperationCode::from_stable_tag(code.stable_tag()),
                Some(code)
            );
        }

        assert_eq!(
            OperationCode::AddI64.descriptor().operands,
            ADD_I64_OPERANDS
        );
        assert_eq!(
            OperationCode::Hole.descriptor().results,
            &[TypeRule::PayloadExpected]
        );
        assert_eq!(OperationCode::Return.descriptor().operands, RETURN_OPERANDS);
        assert!(!OperationCode::Hole.descriptor().complete);
        assert!(OperationCode::Return.descriptor().terminator);
        let complete_expression_codes: Vec<OperationCode> = OperationCode::ALL
            .into_iter()
            .filter(|code| {
                let descriptor = code.descriptor();
                descriptor.complete && !descriptor.terminator && descriptor.results.len() == 1
            })
            .collect();
        assert_eq!(
            complete_expression_codes,
            [
                OperationCode::ConstI64,
                OperationCode::ConstBool,
                OperationCode::AddI64,
            ]
        );
    }

    #[test]
    fn operation_values_match_descriptor_arity_types_and_flags() {
        let lhs = value(2);
        let rhs = value(3);
        let operations = [
            OperationKind::ConstI64(1),
            OperationKind::ConstBool(true),
            OperationKind::AddI64 { lhs, rhs },
            OperationKind::Hole {
                expected: SemanticType::Bool,
            },
            OperationKind::Return { value: lhs },
        ];
        assert_eq!(
            operations
                .iter()
                .map(OperationKind::code)
                .collect::<Vec<_>>(),
            OperationCode::ALL
        );
        for operation in &operations {
            assert_eq!(
                operation.operand_count(),
                operation.descriptor().operands.len()
            );
            assert_eq!(
                operation.result_count(),
                operation.descriptor().results.len()
            );
            assert_eq!(operation.is_terminator(), operation.descriptor().terminator);
            assert_eq!(operation.is_complete(), operation.descriptor().complete);
            for index in 0..operation.operand_count() {
                assert!(operation.operand(index).is_some());
                assert!(operation.operand_use(index).is_some());
            }
            assert!(operation.operand(operation.operand_count()).is_none());
        }
        assert_eq!(operations[2].operand_type(0, None), Some(SemanticType::I64));
        assert_eq!(operations[3].result_type(0, None), Some(SemanticType::Bool));
        assert_eq!(operations[4].operand_type(0, None), None);
        assert_eq!(
            operations[4].operand_type(0, Some(SemanticType::I64)),
            Some(SemanticType::I64)
        );
    }

    #[test]
    fn indexed_node_access_preserves_semantic_order_and_reference_detail() {
        let workspace = crate::ids::WorkspaceId::from_bytes([0x52; 16]);
        let block = NodeId::new(workspace, 2).expect("block");
        let first = NodeId::new(workspace, 3).expect("first");
        let second = NodeId::new(workspace, 4).expect("second");
        let terminator = NodeId::new(workspace, 5).expect("terminator");
        let node = Node::Block {
            owner: block,
            operations: vec![first, second],
            terminator: Some(terminator),
        };
        assert_eq!(node.owned_child_count(), 3);
        assert_eq!(node.owned_child(0), Some(first));
        assert_eq!(node.owned_child(1), Some(second));
        assert_eq!(node.owned_child(2), Some(terminator));
        assert_eq!(node.owned_child(3), None);

        let add = Node::Operation {
            owner: block,
            operation: OperationKind::AddI64 {
                lhs: ValueRef::OperationResult {
                    operation: first,
                    output: 0,
                },
                rhs: ValueRef::OperationResult {
                    operation: second,
                    output: 0,
                },
            },
        };
        assert_eq!(add.direct_reference_count(), 2);
        assert_eq!(
            add.direct_reference(1),
            Some(DirectReference::ValueOperand {
                index: 1,
                value: ValueRef::OperationResult {
                    operation: second,
                    output: 0,
                },
            })
        );
    }
}
