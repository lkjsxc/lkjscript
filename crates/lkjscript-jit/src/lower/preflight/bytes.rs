use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::lower) enum BytesMode {
    Static,
    Owner,
    Loan,
}

pub(in crate::lower) struct BytesModes {
    modes: HashMap<(FunctionId, ValueId), BytesMode>,
    results: HashMap<FunctionId, BytesMode>,
}

impl BytesModes {
    pub(in crate::lower) fn analyze(
        program: &lkjscript_ir::Program,
        functions: &[FunctionId],
    ) -> Result<Self, LoweringError> {
        analyze_bytes_modes(program, functions)
    }

    pub(in crate::lower) fn value(
        &self,
        function: FunctionId,
        value: ValueId,
    ) -> Result<BytesMode, LoweringError> {
        self.modes
            .get(&(function, value))
            .copied()
            .ok_or_else(|| bytes_mode_error(function))
    }

    pub(in crate::lower) fn result(
        &self,
        function: FunctionId,
    ) -> Result<BytesMode, LoweringError> {
        self.results
            .get(&function)
            .copied()
            .ok_or_else(|| bytes_mode_error(function))
    }

    pub(super) fn new(
        modes: HashMap<(FunctionId, ValueId), BytesMode>,
        results: HashMap<FunctionId, BytesMode>,
    ) -> Self {
        Self { modes, results }
    }
}

pub(super) fn preflight_bytes_runtime(
    function: &Function,
    instruction: &Instruction,
    operation: RuntimeOp,
    arguments: &[ValueId],
    modes: &BytesModes,
) -> Result<(), LoweringError> {
    let observes_bytes = matches!(
        operation,
        RuntimeOp::BytesLength
            | RuntimeOp::BytesByteAt
            | RuntimeOp::CopyBytesSlice
            | RuntimeOp::CloneBytes
            | RuntimeOp::ThawBytes
    );
    let input = arguments
        .first()
        .filter(|_| observes_bytes)
        .map(|value| modes.value(function.id, *value))
        .transpose()?;
    let valid = match operation {
        RuntimeOp::BytesLength
        | RuntimeOp::BytesByteAt
        | RuntimeOp::CopyBytesSlice
        | RuntimeOp::CloneBytes => matches!(input, Some(BytesMode::Static | BytesMode::Loan)),
        RuntimeOp::ThawBytes => matches!(input, Some(BytesMode::Static | BytesMode::Owner)),
        RuntimeOp::FreezeByteVector => {
            instruction.ty == SsaType::Bytes
                && modes.value(function.id, instruction.id)? == BytesMode::Owner
        }
        _ => true,
    };
    if valid {
        Ok(())
    } else {
        Err(LoweringError::new(
            LoweringFailureCode::UnsupportedOperation,
            Some(function.id),
            format!("runtime operation {operation:?} has an unsupported native bytes mode"),
        ))
    }
}

pub(super) fn bytes_mode_error(function: FunctionId) -> LoweringError {
    LoweringError::new(
        LoweringFailureCode::UnsupportedType,
        Some(function),
        "native bytes static/dynamic modes conflict",
    )
}
