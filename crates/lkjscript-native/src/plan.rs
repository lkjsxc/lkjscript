use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::verify::{verify_plan, VerifiedMachinePlan};
use crate::{BackendLimits, NativeError};

static NEXT_PLAN_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ValueType {
    I64,
    F64,
    Bool,
    Unit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signature {
    parameters: Vec<ValueType>,
    result: ValueType,
}

impl Signature {
    pub fn new(parameters: Vec<ValueType>, result: ValueType) -> Result<Self, PlanError> {
        if parameters.len() > 16 {
            return Err(PlanError::TooManyParameters {
                count: parameters.len(),
                maximum: 16,
            });
        }
        Ok(Self { parameters, result })
    }

    #[must_use]
    pub fn parameters(&self) -> &[ValueType] {
        &self.parameters
    }

    #[must_use]
    pub const fn result(&self) -> ValueType {
        self.result
    }

    pub(crate) fn machine_parameter_count(&self) -> usize {
        self.parameters
            .iter()
            .filter(|parameter| **parameter != ValueType::Unit)
            .count()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceFunctionId(u32);

impl SourceFunctionId {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceOrigin(u32);

impl SourceOrigin {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FunctionId {
    pub(crate) plan: u64,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BlockId {
    pub(crate) function: FunctionId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ValueId {
    pub(crate) function: FunctionId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LocalId {
    pub(crate) function: FunctionId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum I64Comparison {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum F64Comparison {
    OrderedEqual,
    OrderedNotEqual,
    OrderedLessThan,
    OrderedLessThanOrEqual,
    OrderedGreaterThan,
    OrderedGreaterThanOrEqual,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BoolComparison {
    Equal,
    NotEqual,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u32)]
pub enum TrapCode {
    I64Overflow = 1,
    DivisionByZero = 2,
    Explicit = 3,
}

impl TrapCode {
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeCallSlot {
    IdentityI64V1,
    /// Cooperative deadline and native fuel poll. The execution context is the
    /// implicit first ABI argument; no language value is boxed for this call.
    PollV1,
    /// Records entry to a source function for exact native-tier accounting.
    EnterFunctionV1,
}

impl RuntimeCallSlot {
    #[must_use]
    pub fn signature(self) -> Signature {
        match self {
            Self::IdentityI64V1 => Signature {
                parameters: vec![ValueType::I64],
                result: ValueType::I64,
            },
            Self::PollV1 => Signature {
                parameters: Vec::new(),
                result: ValueType::Unit,
            },
            Self::EnterFunctionV1 => Signature {
                parameters: vec![ValueType::I64],
                result: ValueType::Unit,
            },
        }
    }

    #[must_use]
    pub const fn version(self) -> u16 {
        match self {
            Self::IdentityI64V1 | Self::PollV1 | Self::EnterFunctionV1 => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeOutcome {
    DeadlineExceeded,
    ResourceLimitExceeded,
    HostFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanError {
    TooManyParameters { count: usize, maximum: usize },
    TooManyItems,
    ForeignId(&'static str),
    UnknownFunction,
    FunctionAlreadyDefined,
    UnknownBlock,
    UnknownValue,
    UnknownLocal,
    BlockAlreadyTerminated,
}

impl fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyParameters { count, maximum } => {
                write!(
                    formatter,
                    "signature has {count} parameters; maximum is {maximum}"
                )
            }
            Self::TooManyItems => formatter.write_str("machine plan exceeds its ID space"),
            Self::ForeignId(kind) => {
                write!(formatter, "{kind} belongs to a different plan or function")
            }
            Self::UnknownFunction => formatter.write_str("unknown machine-plan function"),
            Self::FunctionAlreadyDefined => {
                formatter.write_str("machine-plan function is already defined")
            }
            Self::UnknownBlock => formatter.write_str("unknown machine-plan block"),
            Self::UnknownValue => formatter.write_str("unknown machine-plan value"),
            Self::UnknownLocal => formatter.write_str("unknown machine-plan local"),
            Self::BlockAlreadyTerminated => {
                formatter.write_str("machine-plan block is already terminated")
            }
        }
    }
}

impl std::error::Error for PlanError {}

#[derive(Clone, Debug)]
pub(crate) enum Operation {
    I64Const(i64),
    F64Const(u64),
    BoolConst(bool),
    Unit,
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
    WriteLocal(LocalId, ValueId),
    Call(FunctionId, Vec<ValueId>),
    RuntimeCall(RuntimeCallSlot, Vec<ValueId>),
}

impl Operation {
    pub(crate) fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::I64Const(_)
            | Self::F64Const(_)
            | Self::BoolConst(_)
            | Self::Unit
            | Self::ReadLocal(_) => Vec::new(),
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
            Self::Call(_, arguments) | Self::RuntimeCall(_, arguments) => arguments.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Instruction {
    pub(crate) output: ValueId,
    pub(crate) output_type: ValueType,
    pub(crate) operation: Operation,
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
    Trap(TrapCode),
    Exit(ValueId),
    Outcome(RuntimeOutcome),
}

impl Terminator {
    pub(crate) fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::Branch(_) | Self::Trap(_) | Self::Outcome(_) => Vec::new(),
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

#[derive(Debug)]
pub struct MachinePlanBuilder {
    plan: u64,
    functions: Vec<FunctionDeclaration>,
}

impl MachinePlanBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            plan: NEXT_PLAN_ID.fetch_add(1, Ordering::Relaxed),
            functions: Vec::new(),
        }
    }

    pub fn declare_function(
        &mut self,
        source_function: SourceFunctionId,
        signature: Signature,
    ) -> Result<FunctionId, PlanError> {
        let index = u32::try_from(self.functions.len()).map_err(|_| PlanError::TooManyItems)?;
        let id = FunctionId {
            plan: self.plan,
            index,
        };
        self.functions.push(FunctionDeclaration {
            id,
            signature,
            source_function,
            body: None,
        });
        Ok(id)
    }

    pub fn function_builder(&self, function: FunctionId) -> Result<FunctionBuilder, PlanError> {
        let declaration = self.declaration(function)?;
        if declaration.body.is_some() {
            return Err(PlanError::FunctionAlreadyDefined);
        }
        Ok(FunctionBuilder::new(
            declaration.id,
            declaration.signature.clone(),
            declaration.source_function,
            self.functions
                .iter()
                .map(|item| (item.id, item.signature.clone()))
                .collect(),
        ))
    }

    pub fn define_function(&mut self, function: FunctionPlan) -> Result<(), PlanError> {
        let declaration = self.declaration_mut(function.id)?;
        if declaration.body.is_some() {
            return Err(PlanError::FunctionAlreadyDefined);
        }
        if declaration.signature != function.signature
            || declaration.source_function != function.source_function
        {
            return Err(PlanError::ForeignId("function definition"));
        }
        declaration.body = Some(function);
        Ok(())
    }

    pub fn verify(self, limits: BackendLimits) -> Result<VerifiedMachinePlan, NativeError> {
        verify_plan(self.plan, self.functions, limits)
    }

    fn declaration(&self, function: FunctionId) -> Result<&FunctionDeclaration, PlanError> {
        if function.plan != self.plan {
            return Err(PlanError::ForeignId("function ID"));
        }
        self.functions
            .get(function.index as usize)
            .filter(|item| item.id == function)
            .ok_or(PlanError::UnknownFunction)
    }

    fn declaration_mut(
        &mut self,
        function: FunctionId,
    ) -> Result<&mut FunctionDeclaration, PlanError> {
        if function.plan != self.plan {
            return Err(PlanError::ForeignId("function ID"));
        }
        self.functions
            .get_mut(function.index as usize)
            .filter(|item| item.id == function)
            .ok_or(PlanError::UnknownFunction)
    }
}

impl Default for MachinePlanBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct FunctionBuilder {
    function: FunctionId,
    signature: Signature,
    source_function: SourceFunctionId,
    signatures: Vec<(FunctionId, Signature)>,
    blocks: Vec<Block>,
    entry: Option<BlockId>,
    values: Vec<ValueFact>,
    locals: Vec<LocalFact>,
}

impl FunctionBuilder {
    fn new(
        function: FunctionId,
        signature: Signature,
        source_function: SourceFunctionId,
        signatures: Vec<(FunctionId, Signature)>,
    ) -> Self {
        let values = signature
            .parameters()
            .iter()
            .copied()
            .enumerate()
            .map(|(index, value_type)| ValueFact {
                id: ValueId {
                    function,
                    index: u32::try_from(index).unwrap_or(u32::MAX),
                },
                value_type,
                definition: ValueDefinition::Parameter(index),
            })
            .collect();
        Self {
            function,
            signature,
            source_function,
            signatures,
            blocks: Vec::new(),
            entry: None,
            values,
            locals: Vec::new(),
        }
    }

    #[must_use]
    pub fn function_id(&self) -> FunctionId {
        self.function
    }

    pub fn parameter(&self, index: usize) -> Result<ValueId, PlanError> {
        if index >= self.signature.parameters().len() {
            return Err(PlanError::UnknownValue);
        }
        self.values
            .get(index)
            .map(|fact| fact.id)
            .ok_or(PlanError::UnknownValue)
    }

    pub fn create_block(&mut self) -> Result<BlockId, PlanError> {
        let index = u32::try_from(self.blocks.len()).map_err(|_| PlanError::TooManyItems)?;
        let id = BlockId {
            function: self.function,
            index,
        };
        self.blocks.push(Block {
            id,
            instructions: Vec::new(),
            terminator: None,
        });
        Ok(id)
    }

    pub fn set_entry(&mut self, block: BlockId) -> Result<(), PlanError> {
        self.check_block(block)?;
        self.entry = Some(block);
        Ok(())
    }

    pub fn create_local(&mut self, value_type: ValueType) -> Result<LocalId, PlanError> {
        let index = u32::try_from(self.locals.len()).map_err(|_| PlanError::TooManyItems)?;
        let id = LocalId {
            function: self.function,
            index,
        };
        self.locals.push(LocalFact { id, value_type });
        Ok(id)
    }

    pub fn i64_const(&mut self, block: BlockId, value: i64) -> Result<ValueId, PlanError> {
        self.append(block, ValueType::I64, Operation::I64Const(value), None)
    }

    pub fn f64_const_bits(&mut self, block: BlockId, bits: u64) -> Result<ValueId, PlanError> {
        self.append(block, ValueType::F64, Operation::F64Const(bits), None)
    }

    pub fn bool_const(&mut self, block: BlockId, value: bool) -> Result<ValueId, PlanError> {
        self.append(block, ValueType::Bool, Operation::BoolConst(value), None)
    }

    pub fn unit(&mut self, block: BlockId) -> Result<ValueId, PlanError> {
        self.append(block, ValueType::Unit, Operation::Unit, None)
    }

    pub fn i64_add(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64Add(left, right),
            left,
            right,
        )
    }

    pub fn i64_sub(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64Sub(left, right),
            left,
            right,
        )
    }

    pub fn i64_mul(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64Mul(left, right),
            left,
            right,
        )
    }

    pub fn i64_div(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64Div(left, right),
            left,
            right,
        )
    }

    pub fn i64_bit_and(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64BitAnd(left, right),
            left,
            right,
        )
    }

    pub fn i64_bit_or(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64BitOr(left, right),
            left,
            right,
        )
    }

    pub fn i64_bit_xor(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64BitXor(left, right),
            left,
            right,
        )
    }

    pub fn i64_to_f64(&mut self, block: BlockId, value: ValueId) -> Result<ValueId, PlanError> {
        self.check_value(value)?;
        self.append(block, ValueType::F64, Operation::I64ToF64(value), None)
    }

    pub fn f64_add(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::F64,
            Operation::F64Add(left, right),
            left,
            right,
        )
    }

    pub fn f64_sub(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::F64,
            Operation::F64Sub(left, right),
            left,
            right,
        )
    }

    pub fn f64_mul(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::F64,
            Operation::F64Mul(left, right),
            left,
            right,
        )
    }

    pub fn f64_div(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::F64,
            Operation::F64Div(left, right),
            left,
            right,
        )
    }

    pub fn i64_compare(
        &mut self,
        block: BlockId,
        comparison: I64Comparison,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::Bool,
            Operation::I64Compare(comparison, left, right),
            left,
            right,
        )
    }

    pub fn f64_compare(
        &mut self,
        block: BlockId,
        comparison: F64Comparison,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::Bool,
            Operation::F64Compare(comparison, left, right),
            left,
            right,
        )
    }

    pub fn f64_bits_equal(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::Bool,
            Operation::F64BitsEqual(left, right),
            left,
            right,
        )
    }

    pub fn bool_compare(
        &mut self,
        block: BlockId,
        comparison: BoolComparison,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::Bool,
            Operation::BoolCompare(comparison, left, right),
            left,
            right,
        )
    }

    pub fn bool_not(&mut self, block: BlockId, value: ValueId) -> Result<ValueId, PlanError> {
        self.check_value(value)?;
        self.append(block, ValueType::Bool, Operation::BoolNot(value), None)
    }

    pub fn read_local(&mut self, block: BlockId, local: LocalId) -> Result<ValueId, PlanError> {
        let value_type = self.local_type(local)?;
        self.append(block, value_type, Operation::ReadLocal(local), None)
    }

    pub fn write_local(
        &mut self,
        block: BlockId,
        local: LocalId,
        value: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.local_type(local)?;
        self.check_value(value)?;
        self.append(
            block,
            ValueType::Unit,
            Operation::WriteLocal(local, value),
            None,
        )
    }

    pub fn call(
        &mut self,
        block: BlockId,
        callee: FunctionId,
        arguments: Vec<ValueId>,
    ) -> Result<ValueId, PlanError> {
        if callee.plan != self.function.plan {
            return Err(PlanError::ForeignId("callee"));
        }
        for argument in &arguments {
            self.check_value(*argument)?;
        }
        let signature = self
            .signatures
            .iter()
            .find(|(id, _)| *id == callee)
            .map(|(_, signature)| signature)
            .ok_or(PlanError::UnknownFunction)?;
        self.append(
            block,
            signature.result(),
            Operation::Call(callee, arguments),
            None,
        )
    }

    pub fn runtime_call(
        &mut self,
        block: BlockId,
        slot: RuntimeCallSlot,
        arguments: Vec<ValueId>,
    ) -> Result<ValueId, PlanError> {
        for argument in &arguments {
            self.check_value(*argument)?;
        }
        self.append(
            block,
            slot.signature().result(),
            Operation::RuntimeCall(slot, arguments),
            None,
        )
    }

    pub fn set_instruction_source(
        &mut self,
        value: ValueId,
        source: SourceOrigin,
    ) -> Result<(), PlanError> {
        self.check_value(value)?;
        let block_id = match self
            .values
            .get(value.index as usize)
            .map(|fact| &fact.definition)
        {
            Some(ValueDefinition::Instruction(block)) => *block,
            _ => return Err(PlanError::UnknownValue),
        };
        let block = self.block_mut(block_id)?;
        let instruction = block
            .instructions
            .iter_mut()
            .find(|instruction| instruction.output == value)
            .ok_or(PlanError::UnknownValue)?;
        instruction.source = Some(source);
        Ok(())
    }

    pub fn branch(&mut self, block: BlockId, target: BlockId) -> Result<(), PlanError> {
        self.check_block(target)?;
        self.terminate(block, Terminator::Branch(target))
    }

    pub fn branch_if(
        &mut self,
        block: BlockId,
        condition: ValueId,
        when_true: BlockId,
        when_false: BlockId,
    ) -> Result<(), PlanError> {
        self.check_value(condition)?;
        self.check_block(when_true)?;
        self.check_block(when_false)?;
        self.terminate(
            block,
            Terminator::BranchIf {
                condition,
                when_true,
                when_false,
            },
        )
    }

    pub fn return_value(&mut self, block: BlockId, value: ValueId) -> Result<(), PlanError> {
        self.check_value(value)?;
        self.terminate(block, Terminator::Return(value))
    }

    pub fn trap(&mut self, block: BlockId, trap: TrapCode) -> Result<(), PlanError> {
        self.terminate(block, Terminator::Trap(trap))
    }

    pub fn exit(&mut self, block: BlockId, code: ValueId) -> Result<(), PlanError> {
        self.check_value(code)?;
        self.terminate(block, Terminator::Exit(code))
    }

    pub fn outcome(&mut self, block: BlockId, outcome: RuntimeOutcome) -> Result<(), PlanError> {
        self.terminate(block, Terminator::Outcome(outcome))
    }

    #[must_use]
    pub fn finish(self) -> FunctionPlan {
        FunctionPlan {
            id: self.function,
            signature: self.signature,
            source_function: self.source_function,
            blocks: self.blocks,
            entry: self.entry,
            values: self.values,
            locals: self.locals,
        }
    }

    fn append_binary(
        &mut self,
        block: BlockId,
        output_type: ValueType,
        operation: Operation,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.check_value(left)?;
        self.check_value(right)?;
        self.append(block, output_type, operation, None)
    }

    fn append(
        &mut self,
        block: BlockId,
        output_type: ValueType,
        operation: Operation,
        source: Option<SourceOrigin>,
    ) -> Result<ValueId, PlanError> {
        self.check_block(block)?;
        if self.block(block)?.terminator.is_some() {
            return Err(PlanError::BlockAlreadyTerminated);
        }
        let index = u32::try_from(self.values.len()).map_err(|_| PlanError::TooManyItems)?;
        let output = ValueId {
            function: self.function,
            index,
        };
        self.values.push(ValueFact {
            id: output,
            value_type: output_type,
            definition: ValueDefinition::Instruction(block),
        });
        self.block_mut(block)?.instructions.push(Instruction {
            output,
            output_type,
            operation,
            source,
        });
        Ok(output)
    }

    fn terminate(&mut self, block: BlockId, terminator: Terminator) -> Result<(), PlanError> {
        let block = self.block_mut(block)?;
        if block.terminator.is_some() {
            return Err(PlanError::BlockAlreadyTerminated);
        }
        block.terminator = Some(terminator);
        Ok(())
    }

    fn check_block(&self, block: BlockId) -> Result<(), PlanError> {
        self.block(block).map(|_| ())
    }

    fn block(&self, block: BlockId) -> Result<&Block, PlanError> {
        if block.function != self.function {
            return Err(PlanError::ForeignId("block ID"));
        }
        self.blocks
            .get(block.index as usize)
            .filter(|item| item.id == block)
            .ok_or(PlanError::UnknownBlock)
    }

    fn block_mut(&mut self, block: BlockId) -> Result<&mut Block, PlanError> {
        if block.function != self.function {
            return Err(PlanError::ForeignId("block ID"));
        }
        self.blocks
            .get_mut(block.index as usize)
            .filter(|item| item.id == block)
            .ok_or(PlanError::UnknownBlock)
    }

    fn check_value(&self, value: ValueId) -> Result<(), PlanError> {
        if value.function != self.function {
            return Err(PlanError::ForeignId("value ID"));
        }
        self.values
            .get(value.index as usize)
            .filter(|fact| fact.id == value)
            .map(|_| ())
            .ok_or(PlanError::UnknownValue)
    }

    fn local_type(&self, local: LocalId) -> Result<ValueType, PlanError> {
        if local.function != self.function {
            return Err(PlanError::ForeignId("local ID"));
        }
        self.locals
            .get(local.index as usize)
            .filter(|fact| fact.id == local)
            .map(|fact| fact.value_type)
            .ok_or(PlanError::UnknownLocal)
    }
}
