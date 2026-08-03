use super::*;

impl StructuralCatalog {
    pub(in crate::lower) fn owner_storage(
        &self,
        function: &Function,
        value: ValueId,
    ) -> Result<lkjscript_native::StructuralStorageRoute, LoweringError> {
        let instruction = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find(|instruction| instruction.id == value)
            .ok_or_else(|| invalid_structural("structural owner definition is absent"))?;
        match &instruction.kind {
            InstructionKind::StructuralPublish { representation, .. }
            | InstructionKind::StructuralCopy { representation, .. } => self
                .representation_storage(
                    *representation,
                    lkjscript_ir::StructuralValueCategory::Owner,
                ),
            InstructionKind::DestinationFinish { destination } => self
                .destination(function, *destination)
                .map(|(_, storage, _)| storage),
            InstructionKind::Move { value, .. } => self.owner_storage(function, *value),
            InstructionKind::Call {
                arguments,
                instantiation: Some(instantiation),
                ..
            } => {
                let mut storages = arguments
                    .iter()
                    .filter(|argument| {
                        aggregate_value_type(function, **argument)
                            .is_some_and(|ty| ty == &instruction.ty)
                    })
                    .map(|argument| self.owner_storage(function, *argument));
                let storage = storages.next().ok_or_else(|| {
                    invalid_structural("dynamic call owner route has no source argument")
                })??;
                if storages.any(|candidate| candidate.is_err() || candidate.ok() != Some(storage))
                    || !instantiation
                        .substitutions
                        .iter()
                        .any(|substitution| substitution.ty == instruction.ty)
                {
                    return Err(invalid_structural("dynamic call owner route is ambiguous"));
                }
                Ok(storage)
            }
            _ => Err(invalid_structural(
                "structural owner definition has no exact storage route",
            )),
        }
    }
}

fn aggregate_value_type(function: &Function, value: ValueId) -> Option<&SsaType> {
    function.blocks.iter().find_map(|block| {
        block
            .parameters
            .iter()
            .find(|parameter| parameter.id == value)
            .map(|parameter| &parameter.ty)
            .or_else(|| {
                block
                    .instructions
                    .iter()
                    .find(|instruction| instruction.id == value)
                    .map(|instruction| &instruction.ty)
            })
    })
}
