use crate::ssa::*;

impl FunctionBuilder<'_> {
    pub(in crate::ssa) fn value_type(&self, value: ValueId) -> Result<SsaType> {
        self.value_types
            .get(value.index().unwrap_or(usize::MAX))
            .cloned()
            .ok_or_else(|| Error::msg(format!("missing SSA value type {}", value.raw())))
    }

    pub(in crate::ssa) fn finish(self) -> Result<Function> {
        if !self.cleanup.loan_ends.is_empty() || !self.active_loans.is_empty() {
            return Err(Error::msg(
                "SSA lowering did not discharge every authoritative HIR loan",
            ));
        }
        let blocks = self
            .blocks
            .into_iter()
            .map(|block| {
                Ok(Block {
                    id: block.id,
                    parameters: block.parameters,
                    instructions: block.instructions,
                    terminator: block.terminator.ok_or_else(|| {
                        Error::msg(format!("SSA block {} has no terminator", block.id.raw()))
                    })?,
                    metadata: block.metadata,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Function {
            id: self.id,
            name: self.name,
            signature: self.signature,
            places: self.places,
            effects: self.function_effect,
            entry: self.entry,
            blocks,
            origin: self.function_origin,
        })
    }
}
