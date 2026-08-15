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
    BlockArgument,
}

impl NodeKind {
    pub const ALL: [Self; 9] = [
        Self::WorkspaceRoot,
        Self::Package,
        Self::Module,
        Self::Function,
        Self::Parameter,
        Self::Region,
        Self::Block,
        Self::Operation,
        Self::BlockArgument,
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
            Self::BlockArgument => "block_argument",
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
            Self::BlockArgument => 9,
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
            9 => Some(Self::BlockArgument),
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
pub enum RegionRole {
    IfThen,
    IfElse,
    ForBody,
}

impl RegionRole {
    pub const ALL: [Self; 3] = [Self::IfThen, Self::IfElse, Self::ForBody];

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::IfThen => "then",
            Self::IfElse => "else",
            Self::ForBody => "body",
        }
    }

    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::IfThen => 1,
            Self::IfElse => 2,
            Self::ForBody => 3,
        }
    }

    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::IfThen),
            2 => Some(Self::IfElse),
            3 => Some(Self::ForBody),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockArgumentRole {
    LoopIndex,
    LoopCarried,
}

impl BlockArgumentRole {
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::LoopIndex => "loop_index",
            Self::LoopCarried => "loop_carried",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationCode {
    ConstI64,
    ConstBool,
    AddI64,
    Hole,
    Return,
    ConstUnit,
    LtI64,
    Call,
    If,
    ForI64,
    Yield,
}

impl OperationCode {
    pub const ALL: [Self; 11] = [
        Self::ConstUnit,
        Self::ConstI64,
        Self::ConstBool,
        Self::AddI64,
        Self::LtI64,
        Self::Call,
        Self::Hole,
        Self::If,
        Self::ForI64,
        Self::Return,
        Self::Yield,
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
            6 => Some(Self::ConstUnit),
            7 => Some(Self::LtI64),
            8 => Some(Self::Call),
            9 => Some(Self::If),
            10 => Some(Self::ForI64),
            11 => Some(Self::Yield),
            _ => None,
        }
    }

    pub const fn machine_name(self) -> &'static str {
        self.descriptor().machine_name
    }

    pub const fn descriptor(self) -> &'static OperationDescriptor {
        match self {
            Self::ConstUnit => &CONST_UNIT_DESCRIPTOR,
            Self::ConstI64 => &CONST_I64_DESCRIPTOR,
            Self::ConstBool => &CONST_BOOL_DESCRIPTOR,
            Self::AddI64 => &ADD_I64_DESCRIPTOR,
            Self::LtI64 => &LT_I64_DESCRIPTOR,
            Self::Call => &CALL_DESCRIPTOR,
            Self::Hole => &HOLE_DESCRIPTOR,
            Self::If => &IF_DESCRIPTOR,
            Self::ForI64 => &FOR_I64_DESCRIPTOR,
            Self::Return => &RETURN_DESCRIPTOR,
            Self::Yield => &YIELD_DESCRIPTOR,
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
    PayloadResult,
    PayloadCarried,
    CallTargetParameter,
    CallTargetResult,
    OwningRegionYield,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiteralField {
    I64Value,
    BoolValue,
    ExpectedType,
    ResultType,
    CarriedType,
    PositiveStep,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OperandArity {
    Fixed(u8),
    CallTargetParameters,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperandDescriptor {
    pub ty: TypeRule,
    pub use_mode: OperandUse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockArgumentDescriptor {
    pub role: BlockArgumentRole,
    pub ty: TypeRule,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionDescriptor {
    pub role: RegionRole,
    pub block_arguments: &'static [BlockArgumentDescriptor],
    pub terminator: OperationCode,
    pub yield_type: TypeRule,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationDescriptor {
    pub code: OperationCode,
    pub machine_name: &'static str,
    pub stable_tag: u8,
    pub operand_arity: OperandArity,
    /// Fixed operands, or the repeated per-argument prototype for dynamic calls.
    pub operands: &'static [OperandDescriptor],
    pub results: &'static [TypeRule],
    pub literal_fields: &'static [LiteralField],
    pub regions: &'static [RegionDescriptor],
    pub terminator: bool,
    pub complete: bool,
}

const NO_OPERANDS: &[OperandDescriptor] = &[];
const NO_RESULTS: &[TypeRule] = &[];
const NO_LITERALS: &[LiteralField] = &[];
const NO_REGIONS: &[RegionDescriptor] = &[];
const UNIT_RESULT: &[TypeRule] = &[TypeRule::Fixed(SemanticType::Unit)];
const I64_RESULT: &[TypeRule] = &[TypeRule::Fixed(SemanticType::I64)];
const BOOL_RESULT: &[TypeRule] = &[TypeRule::Fixed(SemanticType::Bool)];
const PAYLOAD_RESULT: &[TypeRule] = &[TypeRule::PayloadExpected];
const STRUCTURED_RESULT: &[TypeRule] = &[TypeRule::PayloadResult];
const CARRIED_RESULT: &[TypeRule] = &[TypeRule::PayloadCarried];
const CALL_RESULT: &[TypeRule] = &[TypeRule::CallTargetResult];
const I64_BINARY_OPERANDS: &[OperandDescriptor] = &[
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::I64),
        use_mode: OperandUse::Copy,
    },
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::I64),
        use_mode: OperandUse::Copy,
    },
];
const CALL_OPERANDS: &[OperandDescriptor] = &[OperandDescriptor {
    ty: TypeRule::CallTargetParameter,
    use_mode: OperandUse::Copy,
}];
const IF_OPERANDS: &[OperandDescriptor] = &[OperandDescriptor {
    ty: TypeRule::Fixed(SemanticType::Bool),
    use_mode: OperandUse::Copy,
}];
const FOR_OPERANDS: &[OperandDescriptor] = &[
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::I64),
        use_mode: OperandUse::Copy,
    },
    OperandDescriptor {
        ty: TypeRule::Fixed(SemanticType::I64),
        use_mode: OperandUse::Copy,
    },
    OperandDescriptor {
        ty: TypeRule::PayloadCarried,
        use_mode: OperandUse::Copy,
    },
];
const RETURN_OPERANDS: &[OperandDescriptor] = &[OperandDescriptor {
    ty: TypeRule::OwnerFunctionResult,
    use_mode: OperandUse::Copy,
}];
const YIELD_OPERANDS: &[OperandDescriptor] = &[OperandDescriptor {
    ty: TypeRule::OwningRegionYield,
    use_mode: OperandUse::Copy,
}];
const I64_LITERAL: &[LiteralField] = &[LiteralField::I64Value];
const BOOL_LITERAL: &[LiteralField] = &[LiteralField::BoolValue];
const EXPECTED_LITERAL: &[LiteralField] = &[LiteralField::ExpectedType];
const IF_LITERALS: &[LiteralField] = &[LiteralField::ResultType];
const FOR_LITERALS: &[LiteralField] = &[LiteralField::PositiveStep, LiteralField::CarriedType];
const NO_BLOCK_ARGUMENTS: &[BlockArgumentDescriptor] = &[];
const FOR_BLOCK_ARGUMENTS: &[BlockArgumentDescriptor] = &[
    BlockArgumentDescriptor {
        role: BlockArgumentRole::LoopIndex,
        ty: TypeRule::Fixed(SemanticType::I64),
    },
    BlockArgumentDescriptor {
        role: BlockArgumentRole::LoopCarried,
        ty: TypeRule::PayloadCarried,
    },
];
const IF_REGIONS: &[RegionDescriptor] = &[
    RegionDescriptor {
        role: RegionRole::IfThen,
        block_arguments: NO_BLOCK_ARGUMENTS,
        terminator: OperationCode::Yield,
        yield_type: TypeRule::PayloadResult,
    },
    RegionDescriptor {
        role: RegionRole::IfElse,
        block_arguments: NO_BLOCK_ARGUMENTS,
        terminator: OperationCode::Yield,
        yield_type: TypeRule::PayloadResult,
    },
];
const FOR_REGIONS: &[RegionDescriptor] = &[RegionDescriptor {
    role: RegionRole::ForBody,
    block_arguments: FOR_BLOCK_ARGUMENTS,
    terminator: OperationCode::Yield,
    yield_type: TypeRule::PayloadCarried,
}];

macro_rules! descriptor {
    ($name:ident, $code:ident, $machine:literal, $tag:literal, $arity:expr, $operands:expr, $results:expr, $literals:expr, $regions:expr, $terminator:expr, $complete:expr) => {
        static $name: OperationDescriptor = OperationDescriptor {
            code: OperationCode::$code,
            machine_name: $machine,
            stable_tag: $tag,
            operand_arity: $arity,
            operands: $operands,
            results: $results,
            literal_fields: $literals,
            regions: $regions,
            terminator: $terminator,
            complete: $complete,
        };
    };
}

descriptor!(
    CONST_I64_DESCRIPTOR,
    ConstI64,
    "const_i64",
    1,
    OperandArity::Fixed(0),
    NO_OPERANDS,
    I64_RESULT,
    I64_LITERAL,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    CONST_BOOL_DESCRIPTOR,
    ConstBool,
    "const_bool",
    2,
    OperandArity::Fixed(0),
    NO_OPERANDS,
    BOOL_RESULT,
    BOOL_LITERAL,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    ADD_I64_DESCRIPTOR,
    AddI64,
    "add_i64",
    3,
    OperandArity::Fixed(2),
    I64_BINARY_OPERANDS,
    I64_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    HOLE_DESCRIPTOR,
    Hole,
    "hole",
    4,
    OperandArity::Fixed(0),
    NO_OPERANDS,
    PAYLOAD_RESULT,
    EXPECTED_LITERAL,
    NO_REGIONS,
    false,
    false
);
descriptor!(
    RETURN_DESCRIPTOR,
    Return,
    "return",
    5,
    OperandArity::Fixed(1),
    RETURN_OPERANDS,
    NO_RESULTS,
    NO_LITERALS,
    NO_REGIONS,
    true,
    true
);
descriptor!(
    CONST_UNIT_DESCRIPTOR,
    ConstUnit,
    "const_unit",
    6,
    OperandArity::Fixed(0),
    NO_OPERANDS,
    UNIT_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    LT_I64_DESCRIPTOR,
    LtI64,
    "lt_i64",
    7,
    OperandArity::Fixed(2),
    I64_BINARY_OPERANDS,
    BOOL_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    CALL_DESCRIPTOR,
    Call,
    "call",
    8,
    OperandArity::CallTargetParameters,
    CALL_OPERANDS,
    CALL_RESULT,
    NO_LITERALS,
    NO_REGIONS,
    false,
    true
);
descriptor!(
    IF_DESCRIPTOR,
    If,
    "if",
    9,
    OperandArity::Fixed(1),
    IF_OPERANDS,
    STRUCTURED_RESULT,
    IF_LITERALS,
    IF_REGIONS,
    false,
    true
);
descriptor!(
    FOR_I64_DESCRIPTOR,
    ForI64,
    "for_i64",
    10,
    OperandArity::Fixed(3),
    FOR_OPERANDS,
    CARRIED_RESULT,
    FOR_LITERALS,
    FOR_REGIONS,
    false,
    true
);
descriptor!(
    YIELD_DESCRIPTOR,
    Yield,
    "yield",
    11,
    OperandArity::Fixed(1),
    YIELD_OPERANDS,
    NO_RESULTS,
    NO_LITERALS,
    NO_REGIONS,
    true,
    true
);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ValueRef {
    FunctionParameter(NodeId),
    BlockArgument(NodeId),
    OperationResult { operation: NodeId, output: u8 },
}

impl ValueRef {
    pub const fn referenced_node(self) -> NodeId {
        match self {
            Self::FunctionParameter(parameter) | Self::BlockArgument(parameter) => parameter,
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
    BlockArgument(NodeTarget),
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
    ConstUnit,
    ConstI64(i64),
    ConstBool(bool),
    AddI64 {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    LtI64 {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    Call {
        function: NodeTarget,
        arguments: Vec<ValueDraft>,
    },
    Hole {
        expected: SemanticType,
    },
    If {
        condition: ValueDraft,
        result: SemanticType,
        then_region: NodeTarget,
        else_region: NodeTarget,
    },
    ForI64 {
        start: ValueDraft,
        end_exclusive: ValueDraft,
        step: i64,
        initial: ValueDraft,
        carried: SemanticType,
        body_region: NodeTarget,
    },
    Return {
        value: ValueDraft,
    },
    Yield {
        value: ValueDraft,
    },
}

impl OperationDraft {
    pub const fn code(&self) -> OperationCode {
        match self {
            Self::ConstUnit => OperationCode::ConstUnit,
            Self::ConstI64(_) => OperationCode::ConstI64,
            Self::ConstBool(_) => OperationCode::ConstBool,
            Self::AddI64 { .. } => OperationCode::AddI64,
            Self::LtI64 { .. } => OperationCode::LtI64,
            Self::Call { .. } => OperationCode::Call,
            Self::Hole { .. } => OperationCode::Hole,
            Self::If { .. } => OperationCode::If,
            Self::ForI64 { .. } => OperationCode::ForI64,
            Self::Return { .. } => OperationCode::Return,
            Self::Yield { .. } => OperationCode::Yield,
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
    ConstUnit,
    ConstI64(i64),
    ConstBool(bool),
    AddI64 {
        lhs: ValueRef,
        rhs: ValueRef,
    },
    LtI64 {
        lhs: ValueRef,
        rhs: ValueRef,
    },
    Call {
        function: NodeId,
        arguments: Vec<ValueRef>,
    },
    Hole {
        expected: SemanticType,
    },
    If {
        condition: ValueRef,
        result: SemanticType,
        then_region: NodeId,
        else_region: NodeId,
    },
    ForI64 {
        start: ValueRef,
        end_exclusive: ValueRef,
        step: i64,
        initial: ValueRef,
        carried: SemanticType,
        body_region: NodeId,
    },
    Return {
        value: ValueRef,
    },
    Yield {
        value: ValueRef,
    },
}

impl OperationKind {
    pub const fn code(&self) -> OperationCode {
        match self {
            Self::ConstUnit => OperationCode::ConstUnit,
            Self::ConstI64(_) => OperationCode::ConstI64,
            Self::ConstBool(_) => OperationCode::ConstBool,
            Self::AddI64 { .. } => OperationCode::AddI64,
            Self::LtI64 { .. } => OperationCode::LtI64,
            Self::Call { .. } => OperationCode::Call,
            Self::Hole { .. } => OperationCode::Hole,
            Self::If { .. } => OperationCode::If,
            Self::ForI64 { .. } => OperationCode::ForI64,
            Self::Return { .. } => OperationCode::Return,
            Self::Yield { .. } => OperationCode::Yield,
        }
    }

    pub const fn stable_tag(&self) -> u8 {
        self.code().stable_tag()
    }
    pub const fn descriptor(&self) -> &'static OperationDescriptor {
        self.code().descriptor()
    }

    pub fn operand_count(&self) -> usize {
        match self {
            Self::Call { arguments, .. } => arguments.len(),
            _ => match self.descriptor().operand_arity {
                OperandArity::Fixed(count) => usize::from(count),
                OperandArity::CallTargetParameters => 0,
            },
        }
    }

    pub fn operand(&self, index: usize) -> Option<ValueRef> {
        match (self, index) {
            (Self::AddI64 { lhs, .. } | Self::LtI64 { lhs, .. }, 0) => Some(*lhs),
            (Self::AddI64 { rhs, .. } | Self::LtI64 { rhs, .. }, 1) => Some(*rhs),
            (Self::Call { arguments, .. }, index) => arguments.get(index).copied(),
            (Self::If { condition, .. }, 0) => Some(*condition),
            (Self::ForI64 { start, .. }, 0) => Some(*start),
            (Self::ForI64 { end_exclusive, .. }, 1) => Some(*end_exclusive),
            (Self::ForI64 { initial, .. }, 2) => Some(*initial),
            (Self::Return { value } | Self::Yield { value }, 0) => Some(*value),
            _ => None,
        }
    }

    pub fn operand_type(
        &self,
        index: usize,
        owner_function_result: Option<SemanticType>,
    ) -> Option<SemanticType> {
        let rule = if matches!(self, Self::Call { .. }) {
            self.descriptor().operands.first()?.ty
        } else {
            self.descriptor().operands.get(index)?.ty
        };
        self.resolve_type_rule(rule, owner_function_result)
    }

    pub fn operand_use(&self, index: usize) -> Option<OperandUse> {
        if index >= self.operand_count() {
            return None;
        }
        if matches!(self, Self::Call { .. }) {
            self.descriptor()
                .operands
                .first()
                .map(|operand| operand.use_mode)
        } else {
            self.descriptor()
                .operands
                .get(index)
                .map(|operand| operand.use_mode)
        }
    }

    pub fn result_count(&self) -> usize {
        self.descriptor().results.len()
    }

    /// Resolves node-local result rules. Call result types require a snapshot-aware helper.
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

    pub fn replace_operand(&mut self, index: u64, replacement: ValueRef) -> bool {
        let Ok(index) = usize::try_from(index) else {
            return false;
        };
        match (self, index) {
            (Self::AddI64 { lhs, .. } | Self::LtI64 { lhs, .. }, 0) => *lhs = replacement,
            (Self::AddI64 { rhs, .. } | Self::LtI64 { rhs, .. }, 1) => *rhs = replacement,
            (Self::Call { arguments, .. }, index) if index < arguments.len() => {
                arguments[index] = replacement
            }
            (Self::If { condition, .. }, 0) => *condition = replacement,
            (Self::ForI64 { start, .. }, 0) => *start = replacement,
            (Self::ForI64 { end_exclusive, .. }, 1) => *end_exclusive = replacement,
            (Self::ForI64 { initial, .. }, 2) => *initial = replacement,
            (Self::Return { value } | Self::Yield { value }, 0) => *value = replacement,
            _ => return false,
        }
        true
    }

    pub const fn definition_target(&self) -> Option<NodeId> {
        match self {
            Self::Call { function, .. } => Some(*function),
            _ => None,
        }
    }

    pub const fn owned_region_count(&self) -> usize {
        match self {
            Self::If { .. } => 2,
            Self::ForI64 { .. } => 1,
            _ => 0,
        }
    }

    pub const fn owned_region(&self, index: usize) -> Option<NodeId> {
        match (self, index) {
            (Self::If { then_region, .. }, 0) => Some(*then_region),
            (Self::If { else_region, .. }, 1) => Some(*else_region),
            (Self::ForI64 { body_region, .. }, 0) => Some(*body_region),
            _ => None,
        }
    }

    pub fn region_role(&self, region: NodeId) -> Option<RegionRole> {
        (0..self.owned_region_count()).find_map(|index| {
            (self.owned_region(index) == Some(region))
                .then_some(self.descriptor().regions[index].role)
        })
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
            TypeRule::PayloadResult => match self {
                Self::If { result, .. } => Some(*result),
                _ => None,
            },
            TypeRule::PayloadCarried => match self {
                Self::ForI64 { carried, .. } => Some(*carried),
                _ => None,
            },
            TypeRule::CallTargetParameter
            | TypeRule::CallTargetResult
            | TypeRule::OwningRegionYield => None,
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
    ValueOperand { index: u64, value: ValueRef },
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
        arguments: Vec<NodeId>,
        operations: Vec<NodeId>,
        terminator: Option<NodeId>,
    },
    BlockArgument {
        owner: NodeId,
        ordinal: u32,
        ty: SemanticType,
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
            Self::BlockArgument { .. } => NodeKind::BlockArgument,
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
            | Self::BlockArgument { owner, .. }
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
            Self::Parameter { .. } | Self::BlockArgument { .. } => 0,
            Self::Region { blocks, .. } => blocks.len(),
            Self::Block {
                arguments,
                operations,
                terminator,
                ..
            } => arguments.len() + operations.len() + usize::from(terminator.is_some()),
            Self::Operation { operation, .. } => operation.owned_region_count(),
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
            Self::Parameter { .. } | Self::BlockArgument { .. } => None,
            Self::Region { blocks, .. } => blocks.get(index).copied(),
            Self::Block {
                arguments,
                operations,
                terminator,
                ..
            } => arguments
                .get(index)
                .copied()
                .or_else(|| {
                    operations
                        .get(index.saturating_sub(arguments.len()))
                        .copied()
                })
                .or_else(|| {
                    (index == arguments.len() + operations.len())
                        .then_some(*terminator)
                        .flatten()
                }),
            Self::Operation { operation, .. } => operation.owned_region(index),
        }
    }

    pub fn direct_reference_count(&self) -> usize {
        match self {
            Self::Package { entry, .. } => usize::from(entry.is_some()),
            Self::Operation { operation, .. } => {
                operation.operand_count() + usize::from(operation.definition_target().is_some())
            }
            _ => 0,
        }
    }

    pub fn direct_reference(&self, index: usize) -> Option<DirectReference> {
        match self {
            Self::Package { entry, .. } if index == 0 => {
                entry.map(|target| DirectReference::Definition { target })
            }
            Self::Operation { operation, .. } => {
                if let Some(target) = operation.definition_target() {
                    if index == 0 {
                        return Some(DirectReference::Definition { target });
                    }
                    let operand_index = index - 1;
                    return operation.operand(operand_index).and_then(|value| {
                        u64::try_from(operand_index)
                            .ok()
                            .map(|index| DirectReference::ValueOperand { index, value })
                    });
                }
                operation.operand(index).and_then(|value| {
                    u64::try_from(index)
                        .ok()
                        .map(|index| DirectReference::ValueOperand { index, value })
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
        NodeKind::BlockArgument => Some(NodeKind::Block),
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

    #[test]
    fn operation_descriptors_are_unique_and_structured_contracts_are_exact() {
        let mut tags = BTreeSet::new();
        let mut names = BTreeSet::new();
        for code in OperationCode::ALL {
            let descriptor = code.descriptor();
            assert_eq!(descriptor.code, code);
            assert!(tags.insert(descriptor.stable_tag));
            assert!(names.insert(descriptor.machine_name));
            assert_eq!(
                OperationCode::from_stable_tag(code.stable_tag()),
                Some(code)
            );
        }
        assert_eq!(
            OperationCode::Call.descriptor().operand_arity,
            OperandArity::CallTargetParameters
        );
        assert_eq!(
            OperationCode::If
                .descriptor()
                .regions
                .iter()
                .map(|r| r.role)
                .collect::<Vec<_>>(),
            [RegionRole::IfThen, RegionRole::IfElse]
        );
        assert_eq!(
            OperationCode::ForI64.descriptor().regions[0].block_arguments,
            FOR_BLOCK_ARGUMENTS
        );
        assert_eq!(
            OperationCode::ForI64.descriptor().regions[0].terminator,
            OperationCode::Yield
        );
        assert!(!OperationCode::Hole.descriptor().complete);
        assert!(OperationCode::Return.descriptor().terminator);
        assert!(OperationCode::Yield.descriptor().terminator);
    }
}
