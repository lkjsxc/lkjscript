use std::collections::HashSet;
use std::fmt;

use crate::plan::{
    BlockId, FunctionDeclaration, FunctionId, FunctionPlan, LocalId, Operation, RuntimeCallSlot,
    Signature, Terminator, ValueDefinition, ValueId, ValueType,
};
use crate::BackendLimits;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationError {
    EmptyPlan,
    LimitExceeded(&'static str),
    MissingFunctionBody(FunctionId),
    DuplicateSourceFunction,
    UnsupportedSignature(FunctionId),
    MissingEntry(FunctionId),
    EmptyFunction(FunctionId),
    MissingTerminator(BlockId),
    InvalidTarget(BlockId),
    UnreachableBlock(BlockId),
    InvalidValue(ValueId),
    ValueNotAvailable(ValueId),
    InvalidLocal(LocalId),
    LocalNotInitialized(LocalId),
    TypeMismatch(&'static str),
    InvalidCall(FunctionId),
    InvalidReturn(FunctionId),
}

impl fmt::Display for VerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPlan => formatter.write_str("machine plan has no functions"),
            Self::LimitExceeded(limit) => write!(formatter, "machine plan exceeds {limit} limit"),
            Self::MissingFunctionBody(function) => {
                write!(formatter, "function {function:?} has no body")
            }
            Self::DuplicateSourceFunction => {
                formatter.write_str("machine plan has duplicate source-function identities")
            }
            Self::UnsupportedSignature(function) => {
                write!(
                    formatter,
                    "function {function:?} has an unsupported native ABI signature"
                )
            }
            Self::MissingEntry(function) => {
                write!(formatter, "function {function:?} has no entry block")
            }
            Self::EmptyFunction(function) => {
                write!(formatter, "function {function:?} has no blocks")
            }
            Self::MissingTerminator(block) => {
                write!(formatter, "block {block:?} has no terminator")
            }
            Self::InvalidTarget(block) => {
                write!(formatter, "block {block:?} has an invalid branch target")
            }
            Self::UnreachableBlock(block) => write!(formatter, "block {block:?} is unreachable"),
            Self::InvalidValue(value) => write!(formatter, "value {value:?} is invalid"),
            Self::ValueNotAvailable(value) => {
                write!(formatter, "value {value:?} is not definitely available")
            }
            Self::InvalidLocal(local) => write!(formatter, "local {local:?} is invalid"),
            Self::LocalNotInitialized(local) => {
                write!(formatter, "local {local:?} is not definitely initialized")
            }
            Self::TypeMismatch(context) => write!(formatter, "type mismatch in {context}"),
            Self::InvalidCall(function) => write!(formatter, "invalid call to {function:?}"),
            Self::InvalidReturn(function) => write!(formatter, "invalid return in {function:?}"),
        }
    }
}

impl std::error::Error for VerificationError {}

#[derive(Clone, Debug)]
pub struct VerifiedMachinePlan {
    pub(crate) functions: Vec<FunctionPlan>,
    pub(crate) limits: BackendLimits,
    pub(crate) work_units: u64,
}

impl VerifiedMachinePlan {
    #[must_use]
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    #[must_use]
    pub fn work_units(&self) -> u64 {
        self.work_units
    }
}

pub(crate) fn verify_plan(
    plan: u64,
    declarations: Vec<FunctionDeclaration>,
    limits: BackendLimits,
) -> Result<VerifiedMachinePlan, crate::NativeError> {
    verify_plan_inner(plan, declarations, limits).map_err(crate::NativeError::Verification)
}

fn verify_plan_inner(
    plan: u64,
    declarations: Vec<FunctionDeclaration>,
    limits: BackendLimits,
) -> Result<VerifiedMachinePlan, VerificationError> {
    if declarations.is_empty() {
        return Err(VerificationError::EmptyPlan);
    }
    if declarations.len() > limits.max_functions() {
        return Err(VerificationError::LimitExceeded("function count"));
    }
    let mut source_functions = HashSet::new();
    let signatures: Vec<_> = declarations
        .iter()
        .map(|declaration| (declaration.id, declaration.signature.clone()))
        .collect();
    let mut functions = Vec::with_capacity(declarations.len());
    let mut total_blocks = 0_usize;
    let mut total_values = 0_usize;
    let mut total_work = 0_u64;

    for declaration in declarations {
        if !source_functions.insert(declaration.source_function) {
            return Err(VerificationError::DuplicateSourceFunction);
        }
        verify_signature(declaration.id, &declaration.signature)?;
        let function = declaration
            .body
            .ok_or(VerificationError::MissingFunctionBody(declaration.id))?;
        if function.id.plan != plan || function.id != declaration.id {
            return Err(VerificationError::MissingFunctionBody(declaration.id));
        }
        total_blocks = total_blocks
            .checked_add(function.blocks.len())
            .ok_or(VerificationError::LimitExceeded("block count"))?;
        total_values = total_values
            .checked_add(function.values.len())
            .ok_or(VerificationError::LimitExceeded("value count"))?;
        if total_blocks > limits.max_blocks() {
            return Err(VerificationError::LimitExceeded("block count"));
        }
        if total_values > limits.max_values() {
            return Err(VerificationError::LimitExceeded("value count"));
        }
        if function.locals.len() > limits.max_locals_per_function() {
            return Err(VerificationError::LimitExceeded("local count"));
        }
        let function_work = verify_function(&function, &signatures)?;
        total_work = total_work
            .checked_add(function_work)
            .ok_or(VerificationError::LimitExceeded("work units"))?;
        if total_work > limits.max_work_units() {
            return Err(VerificationError::LimitExceeded("work units"));
        }
        functions.push(function);
    }

    Ok(VerifiedMachinePlan {
        functions,
        limits,
        work_units: total_work,
    })
}

fn verify_signature(function: FunctionId, signature: &Signature) -> Result<(), VerificationError> {
    if signature.machine_parameter_count() > 2 {
        return Err(VerificationError::UnsupportedSignature(function));
    }
    Ok(())
}

fn verify_function(
    function: &FunctionPlan,
    signatures: &[(FunctionId, Signature)],
) -> Result<u64, VerificationError> {
    if function.blocks.is_empty() {
        return Err(VerificationError::EmptyFunction(function.id));
    }
    let entry = function
        .entry
        .ok_or(VerificationError::MissingEntry(function.id))?;
    block_index(function, entry)?;

    for (index, fact) in function.values.iter().enumerate() {
        if fact.id.function != function.id || fact.id.index as usize != index {
            return Err(VerificationError::InvalidValue(fact.id));
        }
        if let ValueDefinition::Parameter(parameter) = fact.definition {
            if function.signature.parameters().get(parameter).copied() != Some(fact.value_type) {
                return Err(VerificationError::InvalidValue(fact.id));
            }
        }
    }
    for (index, local) in function.locals.iter().enumerate() {
        if local.id.function != function.id || local.id.index as usize != index {
            return Err(VerificationError::InvalidLocal(local.id));
        }
    }

    let mut successors = vec![Vec::new(); function.blocks.len()];
    let mut predecessors = vec![Vec::new(); function.blocks.len()];
    for block in &function.blocks {
        let block_index_value = block_index(function, block.id)?;
        let terminator = block
            .terminator
            .as_ref()
            .ok_or(VerificationError::MissingTerminator(block.id))?;
        let targets: Vec<BlockId> = match terminator {
            Terminator::Branch(target) => vec![*target],
            Terminator::BranchIf {
                when_true,
                when_false,
                ..
            } => vec![*when_true, *when_false],
            Terminator::Return(_)
            | Terminator::Trap(_)
            | Terminator::Exit(_)
            | Terminator::Outcome(_) => Vec::new(),
        };
        for target in targets {
            let target_index = block_index(function, target)?;
            successors[block_index_value].push(target_index);
            predecessors[target_index].push(block_index_value);
        }
    }

    let entry_index = block_index(function, entry)?;
    let mut reachable = vec![false; function.blocks.len()];
    let mut pending = vec![entry_index];
    while let Some(index) = pending.pop() {
        if reachable[index] {
            continue;
        }
        reachable[index] = true;
        pending.extend(successors[index].iter().copied());
    }
    for (index, is_reachable) in reachable.iter().enumerate() {
        if !is_reachable {
            return Err(VerificationError::UnreachableBlock(
                function.blocks[index].id,
            ));
        }
    }

    let value_count = function.values.len();
    let local_count = function.locals.len();
    let mut in_values = vec![vec![true; value_count]; function.blocks.len()];
    let mut in_locals = vec![vec![true; local_count]; function.blocks.len()];
    let parameter_count = function.signature.parameters().len();
    for value in &mut in_values[entry_index] {
        *value = false;
    }
    for index in 0..parameter_count {
        if let Some(value) = in_values[entry_index].get_mut(index) {
            *value = true;
        }
    }
    for local in &mut in_locals[entry_index] {
        *local = false;
    }

    loop {
        let mut changed = false;
        for block_index_value in 0..function.blocks.len() {
            if block_index_value == entry_index {
                continue;
            }
            let mut next_values = vec![true; value_count];
            let mut next_locals = vec![true; local_count];
            for predecessor in &predecessors[block_index_value] {
                let (out_values, out_locals) = transfer_sets(
                    function,
                    *predecessor,
                    &in_values[*predecessor],
                    &in_locals[*predecessor],
                );
                intersect(&mut next_values, &out_values);
                intersect(&mut next_locals, &out_locals);
            }
            if next_values != in_values[block_index_value] {
                in_values[block_index_value] = next_values;
                changed = true;
            }
            if next_locals != in_locals[block_index_value] {
                in_locals[block_index_value] = next_locals;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut work = u64::try_from(function.blocks.len())
        .map_err(|_| VerificationError::LimitExceeded("work units"))?;
    for (block_index_value, block) in function.blocks.iter().enumerate() {
        let mut available_values = in_values[block_index_value].clone();
        let mut initialized_locals = in_locals[block_index_value].clone();
        for instruction in &block.instructions {
            verify_instruction(
                function,
                instruction,
                signatures,
                &available_values,
                &initialized_locals,
            )?;
            let output_index = value_index(function, instruction.output)?;
            let fact = &function.values[output_index];
            if fact.value_type != instruction.output_type
                || !matches!(fact.definition, ValueDefinition::Instruction(id) if id == block.id)
            {
                return Err(VerificationError::InvalidValue(instruction.output));
            }
            available_values[output_index] = true;
            if let Operation::WriteLocal(local, _) = instruction.operation {
                initialized_locals[local_index(function, local)?] = true;
            }
            work = work
                .checked_add(1)
                .ok_or(VerificationError::LimitExceeded("work units"))?;
        }
        let terminator = block
            .terminator
            .as_ref()
            .ok_or(VerificationError::MissingTerminator(block.id))?;
        for operand in terminator.operands() {
            require_available(function, operand, &available_values)?;
        }
        verify_terminator(function, terminator)?;
        work = work
            .checked_add(1)
            .ok_or(VerificationError::LimitExceeded("work units"))?;
    }
    Ok(work)
}

fn transfer_sets(
    function: &FunctionPlan,
    block_index_value: usize,
    in_values: &[bool],
    in_locals: &[bool],
) -> (Vec<bool>, Vec<bool>) {
    let mut values = in_values.to_vec();
    let mut locals = in_locals.to_vec();
    for instruction in &function.blocks[block_index_value].instructions {
        if let Some(value) = values.get_mut(instruction.output.index as usize) {
            *value = true;
        }
        if let Operation::WriteLocal(local, _) = instruction.operation {
            if let Some(value) = locals.get_mut(local.index as usize) {
                *value = true;
            }
        }
    }
    (values, locals)
}

fn intersect(target: &mut [bool], source: &[bool]) {
    for (target_item, source_item) in target.iter_mut().zip(source) {
        *target_item &= *source_item;
    }
}

fn verify_instruction(
    function: &FunctionPlan,
    instruction: &crate::plan::Instruction,
    signatures: &[(FunctionId, Signature)],
    available_values: &[bool],
    initialized_locals: &[bool],
) -> Result<(), VerificationError> {
    for operand in instruction.operation.operands() {
        require_available(function, operand, available_values)?;
    }
    match &instruction.operation {
        Operation::I64Const(_) => require_output(instruction, ValueType::I64, "I64 constant"),
        Operation::F64Const(_) => require_output(instruction, ValueType::F64, "F64 constant"),
        Operation::BoolConst(_) => require_output(instruction, ValueType::Bool, "Bool constant"),
        Operation::Unit => require_output(instruction, ValueType::Unit, "Unit constant"),
        Operation::I64Add(left, right)
        | Operation::I64Sub(left, right)
        | Operation::I64Mul(left, right)
        | Operation::I64Div(left, right)
        | Operation::I64BitAnd(left, right)
        | Operation::I64BitOr(left, right)
        | Operation::I64BitXor(left, right) => {
            require_types(function, [*left, *right], ValueType::I64, "I64 arithmetic")?;
            require_output(instruction, ValueType::I64, "I64 arithmetic")
        }
        Operation::I64ToF64(value) => {
            require_types(function, [*value], ValueType::I64, "I64 to F64 conversion")?;
            require_output(instruction, ValueType::F64, "I64 to F64 conversion")
        }
        Operation::F64Add(left, right)
        | Operation::F64Sub(left, right)
        | Operation::F64Mul(left, right)
        | Operation::F64Div(left, right) => {
            require_types(function, [*left, *right], ValueType::F64, "F64 arithmetic")?;
            require_output(instruction, ValueType::F64, "F64 arithmetic")
        }
        Operation::I64Compare(_, left, right) => {
            require_types(function, [*left, *right], ValueType::I64, "I64 comparison")?;
            require_output(instruction, ValueType::Bool, "I64 comparison")
        }
        Operation::F64Compare(_, left, right) | Operation::F64BitsEqual(left, right) => {
            require_types(function, [*left, *right], ValueType::F64, "F64 comparison")?;
            require_output(instruction, ValueType::Bool, "F64 comparison")
        }
        Operation::BoolCompare(_, left, right) => {
            require_types(
                function,
                [*left, *right],
                ValueType::Bool,
                "Bool comparison",
            )?;
            require_output(instruction, ValueType::Bool, "Bool comparison")
        }
        Operation::BoolNot(value) => {
            require_types(function, [*value], ValueType::Bool, "Bool not")?;
            require_output(instruction, ValueType::Bool, "Bool not")
        }
        Operation::ReadLocal(local) => {
            let index = local_index(function, *local)?;
            if !initialized_locals.get(index).copied().unwrap_or(false) {
                return Err(VerificationError::LocalNotInitialized(*local));
            }
            require_output(instruction, function.locals[index].value_type, "local read")
        }
        Operation::WriteLocal(local, value) => {
            let local_type = function.locals[local_index(function, *local)?].value_type;
            if value_type(function, *value)? != local_type {
                return Err(VerificationError::TypeMismatch("local write"));
            }
            require_output(instruction, ValueType::Unit, "local write")
        }
        Operation::Call(callee, arguments) => {
            let signature = signatures
                .iter()
                .find(|(function_id, _)| function_id == callee)
                .map(|(_, signature)| signature)
                .ok_or(VerificationError::InvalidCall(*callee))?;
            verify_arguments(function, arguments, signature, "compiled call")?;
            require_output(instruction, signature.result(), "compiled call")
        }
        Operation::RuntimeCall(slot, arguments) => {
            verify_runtime_slot(*slot)?;
            let signature = slot.signature();
            verify_arguments(function, arguments, &signature, "runtime call")?;
            require_output(instruction, signature.result(), "runtime call")
        }
    }
}

fn verify_runtime_slot(slot: RuntimeCallSlot) -> Result<(), VerificationError> {
    if slot.version() != 1 {
        return Err(VerificationError::TypeMismatch("runtime-call version"));
    }
    if !slot.plan_callable() {
        return Err(VerificationError::TypeMismatch(
            "encoder-owned runtime call",
        ));
    }
    let signature = slot.signature();
    match slot {
        RuntimeCallSlot::CollectReferenceV1
            if signature.parameters() == [ValueType::Reference(crate::ReferenceType::Buf)]
                && signature.result() == ValueType::Reference(crate::ReferenceType::Buf) => {}
        RuntimeCallSlot::CollectReferenceV1 => {
            return Err(VerificationError::TypeMismatch(
                "collecting runtime-call signature",
            ));
        }
        RuntimeCallSlot::IdentityI64V1
        | RuntimeCallSlot::PollV1
        | RuntimeCallSlot::EnterFunctionV1 => {}
        RuntimeCallSlot::RegisterFrameV1
        | RuntimeCallSlot::PublishSafepointV1
        | RuntimeCallSlot::UnregisterFrameV1 => {
            return Err(VerificationError::TypeMismatch(
                "encoder-owned runtime call",
            ));
        }
    }
    Ok(())
}

fn verify_arguments(
    function: &FunctionPlan,
    arguments: &[ValueId],
    signature: &Signature,
    context: &'static str,
) -> Result<(), VerificationError> {
    if arguments.len() != signature.parameters().len() {
        return Err(VerificationError::TypeMismatch(context));
    }
    for (argument, expected) in arguments.iter().zip(signature.parameters()) {
        if value_type(function, *argument)? != *expected {
            return Err(VerificationError::TypeMismatch(context));
        }
    }
    Ok(())
}

fn verify_terminator(
    function: &FunctionPlan,
    terminator: &Terminator,
) -> Result<(), VerificationError> {
    match terminator {
        Terminator::Branch(target) => {
            block_index(function, *target)?;
        }
        Terminator::BranchIf {
            condition,
            when_true,
            when_false,
        } => {
            block_index(function, *when_true)?;
            block_index(function, *when_false)?;
            if value_type(function, *condition)? != ValueType::Bool {
                return Err(VerificationError::TypeMismatch("conditional branch"));
            }
        }
        Terminator::Return(value) => {
            if value_type(function, *value)? != function.signature.result() {
                return Err(VerificationError::InvalidReturn(function.id));
            }
        }
        Terminator::Trap(_) | Terminator::Outcome(_) => {}
        Terminator::Exit(code) => {
            if value_type(function, *code)? != ValueType::I64 {
                return Err(VerificationError::TypeMismatch("exit status"));
            }
        }
    }
    Ok(())
}

fn require_output(
    instruction: &crate::plan::Instruction,
    expected: ValueType,
    context: &'static str,
) -> Result<(), VerificationError> {
    if instruction.output_type != expected {
        return Err(VerificationError::TypeMismatch(context));
    }
    Ok(())
}

fn require_types<const N: usize>(
    function: &FunctionPlan,
    values: [ValueId; N],
    expected: ValueType,
    context: &'static str,
) -> Result<(), VerificationError> {
    for value in values {
        if value_type(function, value)? != expected {
            return Err(VerificationError::TypeMismatch(context));
        }
    }
    Ok(())
}

fn require_available(
    function: &FunctionPlan,
    value: ValueId,
    available: &[bool],
) -> Result<(), VerificationError> {
    let index = value_index(function, value)?;
    if !available.get(index).copied().unwrap_or(false) {
        return Err(VerificationError::ValueNotAvailable(value));
    }
    Ok(())
}

fn value_type(function: &FunctionPlan, value: ValueId) -> Result<ValueType, VerificationError> {
    let index = value_index(function, value)?;
    function
        .values
        .get(index)
        .map(|fact| fact.value_type)
        .ok_or(VerificationError::InvalidValue(value))
}

fn value_index(function: &FunctionPlan, value: ValueId) -> Result<usize, VerificationError> {
    if value.function != function.id {
        return Err(VerificationError::InvalidValue(value));
    }
    let index = value.index as usize;
    if function.values.get(index).map(|fact| fact.id) != Some(value) {
        return Err(VerificationError::InvalidValue(value));
    }
    Ok(index)
}

fn local_index(function: &FunctionPlan, local: LocalId) -> Result<usize, VerificationError> {
    if local.function != function.id {
        return Err(VerificationError::InvalidLocal(local));
    }
    let index = local.index as usize;
    if function.locals.get(index).map(|fact| fact.id) != Some(local) {
        return Err(VerificationError::InvalidLocal(local));
    }
    Ok(index)
}

fn block_index(function: &FunctionPlan, block: BlockId) -> Result<usize, VerificationError> {
    if block.function != function.id {
        return Err(VerificationError::InvalidTarget(block));
    }
    let index = block.index as usize;
    if function.blocks.get(index).map(|item| item.id) != Some(block) {
        return Err(VerificationError::InvalidTarget(block));
    }
    Ok(index)
}
