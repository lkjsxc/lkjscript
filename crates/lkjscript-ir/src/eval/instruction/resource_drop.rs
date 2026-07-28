impl Evaluator<'_> {
    fn drop_unique(
        &mut self,
        values: &mut [Option<EvalValue>],
        source: ValueId,
    ) -> std::result::Result<EvalValue, Flow> {
        let owner = take_value(values, source)?;
        self.unique.drop_owner(owner)?;
        Ok(EvalValue::Unit)
    }

    fn drop_resource(
        &mut self,
        values: &mut [Option<EvalValue>],
        source: ValueId,
        kind: lkjscript_contracts::ResourceKind,
    ) -> std::result::Result<EvalValue, Flow> {
        let EvalValue::Resource(resource) = take_value(values, source)? else {
            return Err(Flow::Trap(
                "evaluator resource Drop received a non-resource owner".into(),
            ));
        };
        self.resources
            .drop_owned(resource, kind)
            .map_err(Flow::HostFailure)?;
        Ok(EvalValue::Unit)
    }

    fn finish_explicit_resource_close(
        values: &mut [Option<EvalValue>],
        source: ValueId,
    ) -> std::result::Result<EvalValue, Flow> {
        let EvalValue::Resource(_) = take_value(values, source)? else {
            return Err(Flow::Trap(
                "evaluator explicit close fact received a non-resource owner".into(),
            ));
        };
        Ok(EvalValue::Unit)
    }
}
