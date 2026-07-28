impl Evaluator<'_> {
    pub(crate) fn constant(&mut self, constant: &Constant) -> std::result::Result<EvalValue, Flow> {
        match constant {
            Constant::Unit => Ok(EvalValue::Unit),
            Constant::Bool(value) => Ok(EvalValue::Bool(*value)),
            Constant::I64(value) => Ok(EvalValue::I64(*value)),
            Constant::F64(value) => Ok(EvalValue::F64(*value)),
            Constant::Str(value) => {
                self.allocate()?;
                Ok(EvalValue::Str(value.clone()))
            }
            Constant::Symbol(value) => {
                self.allocate()?;
                Ok(EvalValue::Symbol(value.clone()))
            }
            Constant::EmptyList => Ok(EvalValue::List(Vec::new())),
        }
    }
}
