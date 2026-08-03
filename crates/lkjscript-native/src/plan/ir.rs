use super::*;

#[derive(Clone, Debug)]
pub(crate) enum Operation {
    I64Const(i64),
    F64Const(u64),
    BoolConst(bool),
    Unit,
    MemoryWitnessLocator(u16),
    StaticBytesConst(StaticBytesIdentity),
    StaticStringConst(StaticBytesIdentity, StructuralTypeIdentity),
    I64Add(ValueId, ValueId),
    I64Sub(ValueId, ValueId),
    I64Mul(ValueId, ValueId),
    I64Div(ValueId, ValueId),
    I64BitAnd(ValueId, ValueId),
    I64BitOr(ValueId, ValueId),
    I64BitXor(ValueId, ValueId),
    I64ToF64(ValueId),
    F64Add(ValueId, ValueId),
    F64Sub(ValueId, ValueId),
    F64Mul(ValueId, ValueId),
    F64Div(ValueId, ValueId),
    I64Compare(I64Comparison, ValueId, ValueId),
    F64Compare(F64Comparison, ValueId, ValueId),
    F64BitsEqual(ValueId, ValueId),
    BoolCompare(BoolComparison, ValueId, ValueId),
    BoolNot(ValueId),
    ReadLocal(LocalId),
    ObserveLocal(LocalId),
    WriteLocal(LocalId, ValueId),
    Call(FunctionId, Vec<ValueId>),
    RuntimeCall(RuntimeCallSlot, Vec<ValueId>),
    StructuralCall(Box<StructuralCallDescriptor>, Vec<ValueId>),
    HeapCall(Box<HeapCallDescriptor>, Vec<ValueId>),
}

impl Operation {
    pub(crate) fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::I64Const(_)
            | Self::F64Const(_)
            | Self::BoolConst(_)
            | Self::Unit
            | Self::MemoryWitnessLocator(_)
            | Self::StaticBytesConst(_)
            | Self::StaticStringConst(_, _)
            | Self::ReadLocal(_)
            | Self::ObserveLocal(_) => Vec::new(),
            Self::BoolNot(value) | Self::I64ToF64(value) | Self::WriteLocal(_, value) => {
                vec![*value]
            }
            Self::I64Add(left, right)
            | Self::I64Sub(left, right)
            | Self::I64Mul(left, right)
            | Self::I64Div(left, right)
            | Self::I64BitAnd(left, right)
            | Self::I64BitOr(left, right)
            | Self::I64BitXor(left, right)
            | Self::F64Add(left, right)
            | Self::F64Sub(left, right)
            | Self::F64Mul(left, right)
            | Self::F64Div(left, right)
            | Self::I64Compare(_, left, right)
            | Self::F64Compare(_, left, right)
            | Self::F64BitsEqual(left, right)
            | Self::BoolCompare(_, left, right) => vec![*left, *right],
            Self::Call(_, arguments)
            | Self::RuntimeCall(_, arguments)
            | Self::StructuralCall(_, arguments)
            | Self::HeapCall(_, arguments) => arguments.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FailureCleanupOperation {
    Runtime(RuntimeCallSlot),
    Structural(Box<StructuralCallDescriptor>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureCleanupCall {
    pub(crate) operation: FailureCleanupOperation,
    pub(crate) local: LocalId,
}

impl FailureCleanupCall {
    #[must_use]
    pub const fn new(slot: RuntimeCallSlot, local: LocalId) -> Self {
        Self {
            operation: FailureCleanupOperation::Runtime(slot),
            local,
        }
    }

    #[must_use]
    pub fn structural(descriptor: StructuralCallDescriptor, local: LocalId) -> Self {
        Self {
            operation: FailureCleanupOperation::Structural(Box::new(descriptor)),
            local,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Instruction {
    pub(crate) output: ValueId,
    pub(crate) output_type: ValueType,
    pub(crate) operation: Operation,
    pub(crate) failure_cleanup: Vec<FailureCleanupCall>,
    pub(crate) unentered_cleanup: Vec<FailureCleanupCall>,
    pub(crate) source: Option<SourceOrigin>,
}

#[derive(Clone, Debug)]
pub(crate) enum Terminator {
    Branch(BlockId),
    BranchIf {
        condition: ValueId,
        when_true: BlockId,
        when_false: BlockId,
    },
    Return(ValueId),
    Trap {
        trap: TrapCode,
        site: Option<u32>,
    },
    Exit(ValueId),
    Outcome(RuntimeOutcome),
}

impl Terminator {
    pub(crate) fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::Branch(_) | Self::Trap { .. } | Self::Outcome(_) => Vec::new(),
            Self::BranchIf { condition, .. } | Self::Return(condition) | Self::Exit(condition) => {
                vec![*condition]
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Block {
    pub(crate) id: BlockId,
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) terminator: Option<Terminator>,
}

#[derive(Clone, Debug)]
pub(crate) enum ValueDefinition {
    Parameter(usize),
    Instruction(BlockId),
}

#[derive(Clone, Debug)]
pub(crate) struct ValueFact {
    pub(crate) id: ValueId,
    pub(crate) value_type: ValueType,
    pub(crate) definition: ValueDefinition,
}

#[derive(Clone, Debug)]
pub(crate) struct LocalFact {
    pub(crate) id: LocalId,
    pub(crate) value_type: ValueType,
}

#[derive(Clone, Debug)]
pub struct FunctionPlan {
    pub(crate) id: FunctionId,
    pub(crate) signature: Signature,
    pub(crate) source_function: SourceFunctionId,
    pub(crate) blocks: Vec<Block>,
    pub(crate) entry: Option<BlockId>,
    pub(crate) values: Vec<ValueFact>,
    pub(crate) locals: Vec<LocalFact>,
}

#[derive(Clone, Debug)]
pub(crate) struct FunctionDeclaration {
    pub(crate) id: FunctionId,
    pub(crate) signature: Signature,
    pub(crate) source_function: SourceFunctionId,
    pub(crate) body: Option<FunctionPlan>,
}
