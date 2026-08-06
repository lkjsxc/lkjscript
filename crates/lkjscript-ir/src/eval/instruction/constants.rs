impl Evaluator<'_> {
    pub(crate) fn constant(&mut self, constant: &Constant) -> std::result::Result<EvalValue, Flow> {
        match constant {
            Constant::Unit => Ok(EvalValue::Unit),
            Constant::Bool(value) => Ok(EvalValue::Bool(*value)),
            Constant::I64(value) => Ok(EvalValue::I64(*value)),
            Constant::F64(value) => Ok(EvalValue::F64(*value)),
            Constant::Str(value) => self
                .structural
                .static_string_identity(value)
                .map(EvalValue::StaticString)
                .map_err(Flow::Trap),
            Constant::StaticBytes(value) => self
                .static_bytes
                .iter()
                .position(|bytes| bytes.as_ref() == value)
                .and_then(|index| u64::try_from(index).ok())
                .map(EvalValue::StaticBytes)
                .ok_or_else(|| Flow::Trap("evaluator static bytes table mismatch".into())),
            Constant::Symbol(value) => self
                .structural
                .static_symbol_identity(value)
                .map(EvalValue::StaticSymbol)
                .map_err(Flow::Trap),
            Constant::EmptyList => Ok(EvalValue::SegmentedList(self.lists.empty())),
        }
    }
}
