use std::fmt;

use lkjscript_ir::{
    Block, CallTarget, Constant, Function, FunctionId, Instruction, InstructionKind, RuntimeOp,
    SsaType, StructuredOutcome, Terminator, ValueId, VerifiedProgram,
};
use lkjscript_native::{
    BackendLimits, BoolComparison, F64Comparison, FunctionBuilder, I64Comparison, InstallableImage,
    LocalId, MachinePlanBuilder, NativeError, RuntimeCallSlot, RuntimeOutcome, Signature,
    SourceFunctionId, SourceOrigin, TrapCode, ValueType,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoweringFailureCode {
    UnsupportedType,
    UnsupportedOperation,
    UnsupportedSignature,
    IndirectCall,
    RecursiveCallGraph,
    InvalidFunction,
    Backend,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweringError {
    code: LoweringFailureCode,
    function: Option<FunctionId>,
    detail: String,
}

impl LoweringError {
    fn new(
        code: LoweringFailureCode,
        function: Option<FunctionId>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code,
            function,
            detail: detail.into(),
        }
    }

    fn backend(error: impl fmt::Display) -> Self {
        Self::new(LoweringFailureCode::Backend, None, error.to_string())
    }

    pub const fn code(&self) -> LoweringFailureCode {
        self.code
    }

    pub const fn function(&self) -> Option<FunctionId> {
        self.function
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for LoweringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(function) = self.function {
            write!(formatter, "function {}: {}", function.raw(), self.detail)
        } else {
            formatter.write_str(&self.detail)
        }
    }
}

impl std::error::Error for LoweringError {}

pub(crate) struct LoweredGroup {
    pub(crate) image: InstallableImage,
    pub(crate) functions: Vec<FunctionId>,
    pub(crate) native_functions: Vec<(FunctionId, lkjscript_native::FunctionId)>,
    pub(crate) explicit_traps: Vec<String>,
}

pub(crate) fn reachable_group(
    verified: &VerifiedProgram,
    root: FunctionId,
) -> Result<Vec<FunctionId>, LoweringError> {
    let program = verified.program();
    let mut marks = vec![0_u8; program.functions.len()];
    let mut reached = Vec::new();
    visit(program, root, &mut marks, &mut reached)?;
    reached.sort_by_key(|function| function.raw());
    Ok(reached)
}

fn visit(
    program: &lkjscript_ir::Program,
    function: FunctionId,
    marks: &mut [u8],
    reached: &mut Vec<FunctionId>,
) -> Result<(), LoweringError> {
    let index = function.index().ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            Some(function),
            "function ID is outside the verified program",
        )
    })?;
    let mark = marks.get(index).copied().ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            Some(function),
            "function ID is outside the verified program",
        )
    })?;
    if mark == 2 {
        return Ok(());
    }
    if mark == 1 {
        return Err(LoweringError::new(
            LoweringFailureCode::RecursiveCallGraph,
            Some(function),
            "recursive native call graph is unsupported by the baseline JIT MVP",
        ));
    }
    marks[index] = 1;
    let item = program
        .functions
        .get(index)
        .filter(|item| item.id == function)
        .ok_or_else(|| {
            LoweringError::new(
                LoweringFailureCode::InvalidFunction,
                Some(function),
                "verified function storage is inconsistent",
            )
        })?;
    for block in &item.blocks {
        for instruction in &block.instructions {
            if let InstructionKind::Call { target, .. } = &instruction.kind {
                match target {
                    CallTarget::Direct(callee) => visit(program, *callee, marks, reached)?,
                    CallTarget::Indirect(_) => {
                        return Err(LoweringError::new(
                            LoweringFailureCode::IndirectCall,
                            Some(function),
                            "indirect native calls are unsupported",
                        ));
                    }
                }
            }
        }
    }
    marks[index] = 2;
    reached.push(function);
    Ok(())
}

pub(crate) fn lower_group(
    verified: &VerifiedProgram,
    root: FunctionId,
    limits: BackendLimits,
) -> Result<LoweredGroup, LoweringError> {
    let functions = reachable_group(verified, root)?;
    let program = verified.program();
    for function in &functions {
        let item = source_function(program, *function)?;
        preflight_function(item)?;
    }

    let mut plan = MachinePlanBuilder::new();
    let mut native_functions = Vec::with_capacity(functions.len());
    for function in &functions {
        let item = source_function(program, *function)?;
        let signature = lower_signature(*function, &item.signature)?;
        let native = plan
            .declare_function(SourceFunctionId::new(function.raw()), signature)
            .map_err(LoweringError::backend)?;
        native_functions.push((*function, native));
    }

    let mut explicit_traps = Vec::new();
    for function in &functions {
        let item = source_function(program, *function)?;
        let native = native_function(&native_functions, *function)?;
        let mut builder = plan
            .function_builder(native)
            .map_err(LoweringError::backend)?;
        lower_function(item, &native_functions, &mut builder, &mut explicit_traps)?;
        plan.define_function(builder.finish())
            .map_err(LoweringError::backend)?;
    }

    let verified_plan = plan.verify(limits).map_err(LoweringError::backend)?;
    let image =
        lkjscript_native::encode(verified_plan, lkjscript_native::EncodingConfig::default())
            .map_err(LoweringError::backend)?;
    Ok(LoweredGroup {
        image,
        functions,
        native_functions,
        explicit_traps,
    })
}

fn source_function(
    program: &lkjscript_ir::Program,
    function: FunctionId,
) -> Result<&Function, LoweringError> {
    function
        .index()
        .and_then(|index| program.functions.get(index))
        .filter(|item| item.id == function)
        .ok_or_else(|| {
            LoweringError::new(
                LoweringFailureCode::InvalidFunction,
                Some(function),
                "function is absent from the verified program",
            )
        })
}

fn preflight_function(function: &Function) -> Result<(), LoweringError> {
    lower_signature(function.id, &function.signature)?;
    if function.id.raw() >= 64 {
        return Err(LoweringError::new(
            LoweringFailureCode::UnsupportedSignature,
            Some(function.id),
            "native entry accounting supports at most 64 dense source functions",
        ));
    }
    for block in &function.blocks {
        for parameter in &block.parameters {
            lower_type(function.id, &parameter.ty)?;
        }
        for instruction in &block.instructions {
            lower_type(function.id, &instruction.ty)?;
            match &instruction.kind {
                InstructionKind::Constant(constant) => match constant {
                    Constant::Unit | Constant::Bool(_) | Constant::I64(_) | Constant::F64(_) => {}
                    _ => return unsupported_operation(function.id, "reference constant"),
                },
                InstructionKind::Copy(_) => {}
                InstructionKind::PlaceInit { .. }
                | InstructionKind::PlaceEnd { .. }
                | InstructionKind::Move { .. }
                | InstructionKind::Borrow { .. } => {
                    return unsupported_operation(
                        function.id,
                        "ownership/reference operation in initial Owned Buf slice",
                    );
                }
                InstructionKind::Runtime { operation, .. } if supported_runtime(*operation) => {}
                InstructionKind::Call {
                    target: CallTarget::Direct(_),
                    signature,
                    ..
                } => {
                    lower_signature(function.id, signature)?;
                }
                InstructionKind::Call {
                    target: CallTarget::Indirect(_),
                    ..
                } => {
                    return Err(LoweringError::new(
                        LoweringFailureCode::IndirectCall,
                        Some(function.id),
                        "indirect native calls are unsupported",
                    ));
                }
                InstructionKind::FunctionRef(_) => {
                    return unsupported_operation(function.id, "first-class function reference");
                }
                InstructionKind::Runtime { operation, .. } => {
                    return unsupported_operation(
                        function.id,
                        &format!("runtime operation {operation:?}"),
                    );
                }
                InstructionKind::ProductValue { .. }
                | InstructionKind::ProductField { .. }
                | InstructionKind::WithProductField { .. } => {
                    return unsupported_operation(function.id, "product/reference operation");
                }
            }
        }
        if let Terminator::Outcome {
            detail: Some(_), ..
        } = block.terminator
        {
            return unsupported_operation(function.id, "structured outcome reference detail");
        }
    }
    Ok(())
}

fn supported_runtime(operation: RuntimeOp) -> bool {
    matches!(
        operation,
        RuntimeOp::Add
            | RuntimeOp::Subtract
            | RuntimeOp::Multiply
            | RuntimeOp::Divide
            | RuntimeOp::EqualValue
            | RuntimeOp::F64BitsEqual
            | RuntimeOp::Less
            | RuntimeOp::LessEqual
            | RuntimeOp::Greater
            | RuntimeOp::GreaterEqual
            | RuntimeOp::Not
            | RuntimeOp::BitAnd
            | RuntimeOp::BitOr
            | RuntimeOp::BitXor
    )
}

fn unsupported_operation<T>(function: FunctionId, operation: &str) -> Result<T, LoweringError> {
    Err(LoweringError::new(
        LoweringFailureCode::UnsupportedOperation,
        Some(function),
        format!("{operation} is unsupported by allocation-free scalar native code"),
    ))
}

fn lower_signature(
    function: FunctionId,
    signature: &lkjscript_ir::Signature,
) -> Result<Signature, LoweringError> {
    if !signature.type_parameters.is_empty() {
        return Err(LoweringError::new(
            LoweringFailureCode::UnsupportedSignature,
            Some(function),
            "polymorphic native signatures are unsupported",
        ));
    }
    let parameters = signature
        .parameters
        .iter()
        .map(|ty| lower_type(function, ty))
        .collect::<Result<Vec<_>, _>>()?;
    let result = lower_type(function, &signature.result)?;
    Signature::new(parameters, result).map_err(|error| {
        LoweringError::new(
            LoweringFailureCode::UnsupportedSignature,
            Some(function),
            error.to_string(),
        )
    })
}

fn lower_type(function: FunctionId, ty: &SsaType) -> Result<ValueType, LoweringError> {
    match ty {
        SsaType::Unit => Ok(ValueType::Unit),
        SsaType::Bool => Ok(ValueType::Bool),
        SsaType::I64 => Ok(ValueType::I64),
        SsaType::F64 => Ok(ValueType::F64),
        _ => Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            format!("type {ty:?} contains a reference or unsupported native representation"),
        )),
    }
}

#[derive(Clone, Copy)]
struct EdgeBlocks {
    branch: Option<lkjscript_native::BlockId>,
    when_true: Option<lkjscript_native::BlockId>,
    when_false: Option<lkjscript_native::BlockId>,
}

fn lower_function(
    function: &Function,
    native_functions: &[(FunctionId, lkjscript_native::FunctionId)],
    builder: &mut FunctionBuilder,
    explicit_traps: &mut Vec<String>,
) -> Result<(), LoweringError> {
    let value_types = collect_value_types(function)?;
    let mut locals = Vec::with_capacity(value_types.len());
    for value_type in &value_types {
        locals.push(
            builder
                .create_local(*value_type)
                .map_err(LoweringError::backend)?,
        );
    }

    let mut blocks = Vec::with_capacity(function.blocks.len());
    for _ in &function.blocks {
        blocks.push(builder.create_block().map_err(LoweringError::backend)?);
    }
    let mut edges = Vec::with_capacity(function.blocks.len());
    for block in &function.blocks {
        let edge = match block.terminator {
            Terminator::Branch { .. } => EdgeBlocks {
                branch: Some(builder.create_block().map_err(LoweringError::backend)?),
                when_true: None,
                when_false: None,
            },
            Terminator::ConditionalBranch { .. } => EdgeBlocks {
                branch: None,
                when_true: Some(builder.create_block().map_err(LoweringError::backend)?),
                when_false: Some(builder.create_block().map_err(LoweringError::backend)?),
            },
            _ => EdgeBlocks {
                branch: None,
                when_true: None,
                when_false: None,
            },
        };
        edges.push(edge);
    }

    let entry_index = function.entry.index().ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            Some(function.id),
            "entry block ID is invalid",
        )
    })?;
    let entry = *blocks.get(entry_index).ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            Some(function.id),
            "entry block is absent",
        )
    })?;
    builder.set_entry(entry).map_err(LoweringError::backend)?;

    let source_entry = function.blocks.get(entry_index).ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            Some(function.id),
            "entry block is absent",
        )
    })?;
    if source_entry.parameters.len() != function.signature.parameters.len() {
        return Err(LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            Some(function.id),
            "entry block parameters do not match the function signature",
        ));
    }
    for (index, parameter) in source_entry.parameters.iter().enumerate() {
        let value = builder.parameter(index).map_err(LoweringError::backend)?;
        let local = value_local(&locals, parameter.id, function.id)?;
        builder
            .write_local(entry, local, value)
            .map_err(LoweringError::backend)?;
    }
    let source_id = builder
        .i64_const(entry, i64::from(function.id.raw()))
        .map_err(LoweringError::backend)?;
    builder
        .runtime_call(entry, RuntimeCallSlot::EnterFunctionV1, vec![source_id])
        .map_err(LoweringError::backend)?;
    builder
        .runtime_call(entry, RuntimeCallSlot::PollV1, Vec::new())
        .map_err(LoweringError::backend)?;

    for (index, block) in function.blocks.iter().enumerate() {
        let native_block = blocks[index];
        for instruction in &block.instructions {
            lower_instruction(
                function,
                instruction,
                native_block,
                &locals,
                &value_types,
                native_functions,
                builder,
            )?;
        }
        lower_terminator(
            function,
            block,
            TerminatorContext {
                native_block,
                edges: edges[index],
                blocks: &blocks,
                locals: &locals,
            },
            builder,
            explicit_traps,
        )?;
    }
    Ok(())
}

fn collect_value_types(function: &Function) -> Result<Vec<ValueType>, LoweringError> {
    let mut types: Vec<Option<ValueType>> = Vec::new();
    for block in &function.blocks {
        for parameter in &block.parameters {
            set_value_type(
                &mut types,
                parameter.id,
                lower_type(function.id, &parameter.ty)?,
            )?;
        }
        for instruction in &block.instructions {
            set_value_type(
                &mut types,
                instruction.id,
                lower_type(function.id, &instruction.ty)?,
            )?;
        }
    }
    types
        .into_iter()
        .map(|ty| {
            ty.ok_or_else(|| {
                LoweringError::new(
                    LoweringFailureCode::InvalidFunction,
                    Some(function.id),
                    "SSA value IDs are not dense",
                )
            })
        })
        .collect()
}

fn set_value_type(
    types: &mut Vec<Option<ValueType>>,
    value: ValueId,
    ty: ValueType,
) -> Result<(), LoweringError> {
    let index = value.index().ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            None,
            "SSA value ID cannot index native locals",
        )
    })?;
    if types.len() <= index {
        types.resize(index + 1, None);
    }
    if types[index].replace(ty).is_some() {
        return Err(LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            None,
            "duplicate SSA value during native lowering",
        ));
    }
    Ok(())
}

fn lower_instruction(
    function: &Function,
    instruction: &Instruction,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    native_functions: &[(FunctionId, lkjscript_native::FunctionId)],
    builder: &mut FunctionBuilder,
) -> Result<(), LoweringError> {
    let output = match &instruction.kind {
        InstructionKind::Constant(constant) => match constant {
            Constant::Unit => builder.unit(block),
            Constant::Bool(value) => builder.bool_const(block, *value),
            Constant::I64(value) => builder.i64_const(block, *value),
            Constant::F64(value) => builder.f64_const_bits(block, value.to_bits()),
            _ => return unsupported_operation(function.id, "reference constant"),
        },
        InstructionKind::Copy(value) => {
            let value = read_value(builder, block, locals, *value, function.id)?;
            Ok(value)
        }
        InstructionKind::Runtime {
            operation,
            arguments,
            ..
        } => lower_runtime(
            function,
            *operation,
            arguments,
            block,
            locals,
            value_types,
            builder,
        ),
        InstructionKind::Call {
            target: CallTarget::Direct(callee),
            arguments,
            ..
        } => {
            builder
                .runtime_call(block, RuntimeCallSlot::PollV1, Vec::new())
                .map_err(LoweringError::backend)?;
            let arguments = read_values(builder, block, locals, arguments, function.id)?;
            let callee = native_function(native_functions, *callee)?;
            builder.call(block, callee, arguments)
        }
        InstructionKind::Call {
            target: CallTarget::Indirect(_),
            ..
        } => {
            return Err(LoweringError::new(
                LoweringFailureCode::IndirectCall,
                Some(function.id),
                "indirect native calls are unsupported",
            ));
        }
        _ => return unsupported_operation(function.id, "reference operation"),
    }
    .map_err(LoweringError::backend)?;
    builder
        .set_instruction_source(output, SourceOrigin::new(instruction.metadata.origin.node))
        .map_err(LoweringError::backend)?;
    let local = value_local(locals, instruction.id, function.id)?;
    builder
        .write_local(block, local, output)
        .map_err(LoweringError::backend)?;
    Ok(())
}

fn lower_runtime(
    function: &Function,
    operation: RuntimeOp,
    arguments: &[ValueId],
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    builder: &mut FunctionBuilder,
) -> Result<lkjscript_native::ValueId, lkjscript_native::PlanError> {
    let values = read_values(builder, block, locals, arguments, function.id)
        .map_err(|_| lkjscript_native::PlanError::UnknownValue)?;
    match operation {
        RuntimeOp::Add | RuntimeOp::Subtract | RuntimeOp::Multiply | RuntimeOp::Divide => {
            let [left, right] = two_values(&values)?;
            match value_type(value_types, arguments[0])? {
                ValueType::I64 if value_type(value_types, arguments[1])? == ValueType::I64 => {
                    match operation {
                        RuntimeOp::Add => builder.i64_add(block, left, right),
                        RuntimeOp::Subtract => builder.i64_sub(block, left, right),
                        RuntimeOp::Multiply => builder.i64_mul(block, left, right),
                        RuntimeOp::Divide => builder.i64_div(block, left, right),
                        _ => Err(lkjscript_native::PlanError::UnknownValue),
                    }
                }
                _ => {
                    let left = convert_to_f64(
                        builder,
                        block,
                        left,
                        value_type(value_types, arguments[0])?,
                    )?;
                    let right = convert_to_f64(
                        builder,
                        block,
                        right,
                        value_type(value_types, arguments[1])?,
                    )?;
                    match operation {
                        RuntimeOp::Add => builder.f64_add(block, left, right),
                        RuntimeOp::Subtract => builder.f64_sub(block, left, right),
                        RuntimeOp::Multiply => builder.f64_mul(block, left, right),
                        RuntimeOp::Divide => builder.f64_div(block, left, right),
                        _ => Err(lkjscript_native::PlanError::UnknownValue),
                    }
                }
            }
        }
        RuntimeOp::Less | RuntimeOp::LessEqual | RuntimeOp::Greater | RuntimeOp::GreaterEqual => {
            let [left, right] = two_values(&values)?;
            let comparison_i64 = match operation {
                RuntimeOp::Less => I64Comparison::LessThan,
                RuntimeOp::LessEqual => I64Comparison::LessThanOrEqual,
                RuntimeOp::Greater => I64Comparison::GreaterThan,
                RuntimeOp::GreaterEqual => I64Comparison::GreaterThanOrEqual,
                _ => return Err(lkjscript_native::PlanError::UnknownValue),
            };
            if value_type(value_types, arguments[0])? == ValueType::I64
                && value_type(value_types, arguments[1])? == ValueType::I64
            {
                builder.i64_compare(block, comparison_i64, left, right)
            } else {
                let left =
                    convert_to_f64(builder, block, left, value_type(value_types, arguments[0])?)?;
                let right = convert_to_f64(
                    builder,
                    block,
                    right,
                    value_type(value_types, arguments[1])?,
                )?;
                let comparison = match operation {
                    RuntimeOp::Less => F64Comparison::OrderedLessThan,
                    RuntimeOp::LessEqual => F64Comparison::OrderedLessThanOrEqual,
                    RuntimeOp::Greater => F64Comparison::OrderedGreaterThan,
                    RuntimeOp::GreaterEqual => F64Comparison::OrderedGreaterThanOrEqual,
                    _ => return Err(lkjscript_native::PlanError::UnknownValue),
                };
                builder.f64_compare(block, comparison, left, right)
            }
        }
        RuntimeOp::EqualValue => {
            let [left, right] = two_values(&values)?;
            match value_type(value_types, arguments[0])? {
                ValueType::Unit => builder.bool_const(block, true),
                ValueType::Bool => builder.bool_compare(block, BoolComparison::Equal, left, right),
                ValueType::I64 => builder.i64_compare(block, I64Comparison::Equal, left, right),
                ValueType::F64 => {
                    builder.f64_compare(block, F64Comparison::OrderedEqual, left, right)
                }
                ValueType::Reference(_) => Err(lkjscript_native::PlanError::UnknownValue),
            }
        }
        RuntimeOp::F64BitsEqual => {
            let [left, right] = two_values(&values)?;
            builder.f64_bits_equal(block, left, right)
        }
        RuntimeOp::Not => builder.bool_not(block, one_value(&values)?),
        RuntimeOp::BitAnd => {
            let [left, right] = two_values(&values)?;
            builder.i64_bit_and(block, left, right)
        }
        RuntimeOp::BitOr => {
            let [left, right] = two_values(&values)?;
            builder.i64_bit_or(block, left, right)
        }
        RuntimeOp::BitXor => {
            let [left, right] = two_values(&values)?;
            builder.i64_bit_xor(block, left, right)
        }
        _ => Err(lkjscript_native::PlanError::UnknownValue),
    }
}

fn convert_to_f64(
    builder: &mut FunctionBuilder,
    block: lkjscript_native::BlockId,
    value: lkjscript_native::ValueId,
    ty: ValueType,
) -> Result<lkjscript_native::ValueId, lkjscript_native::PlanError> {
    match ty {
        ValueType::F64 => Ok(value),
        ValueType::I64 => builder.i64_to_f64(block, value),
        _ => Err(lkjscript_native::PlanError::UnknownValue),
    }
}

fn two_values(
    values: &[lkjscript_native::ValueId],
) -> Result<[lkjscript_native::ValueId; 2], lkjscript_native::PlanError> {
    match values {
        [left, right] => Ok([*left, *right]),
        _ => Err(lkjscript_native::PlanError::UnknownValue),
    }
}

fn one_value(
    values: &[lkjscript_native::ValueId],
) -> Result<lkjscript_native::ValueId, lkjscript_native::PlanError> {
    match values {
        [value] => Ok(*value),
        _ => Err(lkjscript_native::PlanError::UnknownValue),
    }
}

fn value_type(
    value_types: &[ValueType],
    value: ValueId,
) -> Result<ValueType, lkjscript_native::PlanError> {
    value
        .index()
        .and_then(|index| value_types.get(index))
        .copied()
        .ok_or(lkjscript_native::PlanError::UnknownValue)
}

struct TerminatorContext<'a> {
    native_block: lkjscript_native::BlockId,
    edges: EdgeBlocks,
    blocks: &'a [lkjscript_native::BlockId],
    locals: &'a [LocalId],
}

fn lower_terminator(
    function: &Function,
    block: &Block,
    context: TerminatorContext<'_>,
    builder: &mut FunctionBuilder,
    explicit_traps: &mut Vec<String>,
) -> Result<(), LoweringError> {
    let native_block = context.native_block;
    let edges = context.edges;
    let blocks = context.blocks;
    let locals = context.locals;
    match &block.terminator {
        Terminator::Branch { target, arguments } => {
            let edge = edges.branch.ok_or_else(|| invalid_edges(function.id))?;
            builder
                .branch(native_block, edge)
                .map_err(LoweringError::backend)?;
            lower_edge(function, edge, *target, arguments, blocks, locals, builder)?;
        }
        Terminator::ConditionalBranch {
            condition,
            true_target,
            true_arguments,
            false_target,
            false_arguments,
        } => {
            let condition = read_value(builder, native_block, locals, *condition, function.id)?;
            let when_true = edges.when_true.ok_or_else(|| invalid_edges(function.id))?;
            let when_false = edges.when_false.ok_or_else(|| invalid_edges(function.id))?;
            builder
                .branch_if(native_block, condition, when_true, when_false)
                .map_err(LoweringError::backend)?;
            lower_edge(
                function,
                when_true,
                *true_target,
                true_arguments,
                blocks,
                locals,
                builder,
            )?;
            lower_edge(
                function,
                when_false,
                *false_target,
                false_arguments,
                blocks,
                locals,
                builder,
            )?;
        }
        Terminator::Return(value) => {
            let value = read_value(builder, native_block, locals, *value, function.id)?;
            builder
                .return_value(native_block, value)
                .map_err(LoweringError::backend)?;
        }
        Terminator::Trap { message } => {
            explicit_traps.push(message.clone());
            builder
                .trap(native_block, TrapCode::Explicit)
                .map_err(LoweringError::backend)?;
        }
        Terminator::Exit { code } => {
            let code = read_value(builder, native_block, locals, *code, function.id)?;
            builder
                .exit(native_block, code)
                .map_err(LoweringError::backend)?;
        }
        Terminator::Outcome { outcome, detail } => {
            if detail.is_some() {
                return unsupported_operation(function.id, "structured outcome detail");
            }
            let outcome = match outcome {
                StructuredOutcome::DeadlineExceeded => RuntimeOutcome::DeadlineExceeded,
                StructuredOutcome::ResourceLimitExceeded => RuntimeOutcome::ResourceLimitExceeded,
                StructuredOutcome::HostFailure => RuntimeOutcome::HostFailure,
            };
            builder
                .outcome(native_block, outcome)
                .map_err(LoweringError::backend)?;
        }
    }
    Ok(())
}

fn lower_edge(
    function: &Function,
    edge: lkjscript_native::BlockId,
    target: lkjscript_ir::BlockId,
    arguments: &[ValueId],
    blocks: &[lkjscript_native::BlockId],
    locals: &[LocalId],
    builder: &mut FunctionBuilder,
) -> Result<(), LoweringError> {
    let target_index = target.index().ok_or_else(|| invalid_edges(function.id))?;
    let target_block = function
        .blocks
        .get(target_index)
        .filter(|block| block.id == target)
        .ok_or_else(|| invalid_edges(function.id))?;
    let native_target = *blocks
        .get(target_index)
        .ok_or_else(|| invalid_edges(function.id))?;
    if target_block.parameters.len() != arguments.len() {
        return Err(invalid_edges(function.id));
    }
    // Read every source before writing any target. This gives block arguments
    // parallel-copy semantics even when a loop rotates parameter locals.
    let values = read_values(builder, edge, locals, arguments, function.id)?;
    for (parameter, value) in target_block.parameters.iter().zip(values) {
        let local = value_local(locals, parameter.id, function.id)?;
        builder
            .write_local(edge, local, value)
            .map_err(LoweringError::backend)?;
    }
    if target_block.metadata.loop_header {
        builder
            .runtime_call(edge, RuntimeCallSlot::PollV1, Vec::new())
            .map_err(LoweringError::backend)?;
    }
    builder
        .branch(edge, native_target)
        .map_err(LoweringError::backend)
}

fn invalid_edges(function: FunctionId) -> LoweringError {
    LoweringError::new(
        LoweringFailureCode::InvalidFunction,
        Some(function),
        "SSA edge metadata is inconsistent",
    )
}

fn read_values(
    builder: &mut FunctionBuilder,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    values: &[ValueId],
    function: FunctionId,
) -> Result<Vec<lkjscript_native::ValueId>, LoweringError> {
    values
        .iter()
        .map(|value| read_value(builder, block, locals, *value, function))
        .collect()
}

fn read_value(
    builder: &mut FunctionBuilder,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value: ValueId,
    function: FunctionId,
) -> Result<lkjscript_native::ValueId, LoweringError> {
    let local = value_local(locals, value, function)?;
    builder
        .read_local(block, local)
        .map_err(LoweringError::backend)
}

fn value_local(
    locals: &[LocalId],
    value: ValueId,
    function: FunctionId,
) -> Result<LocalId, LoweringError> {
    value
        .index()
        .and_then(|index| locals.get(index))
        .copied()
        .ok_or_else(|| {
            LoweringError::new(
                LoweringFailureCode::InvalidFunction,
                Some(function),
                format!("SSA value {} has no native local", value.raw()),
            )
        })
}

fn native_function(
    functions: &[(FunctionId, lkjscript_native::FunctionId)],
    function: FunctionId,
) -> Result<lkjscript_native::FunctionId, LoweringError> {
    functions
        .iter()
        .find(|(source, _)| *source == function)
        .map(|(_, native)| *native)
        .ok_or_else(|| {
            LoweringError::new(
                LoweringFailureCode::InvalidFunction,
                Some(function),
                "direct callee is outside the native compilation group",
            )
        })
}

impl From<NativeError> for LoweringError {
    fn from(error: NativeError) -> Self {
        Self::backend(error)
    }
}
