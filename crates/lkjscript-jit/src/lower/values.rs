use super::*;

pub(super) fn read_values(
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

pub(super) fn read_value(
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

pub(super) fn value_local(
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

pub(super) fn native_function(
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

impl From<lkjscript_native::PlanError> for LoweringError {
    fn from(error: lkjscript_native::PlanError) -> Self {
        Self::backend(error)
    }
}

impl From<NativeError> for LoweringError {
    fn from(error: NativeError) -> Self {
        Self::backend(error)
    }
}
