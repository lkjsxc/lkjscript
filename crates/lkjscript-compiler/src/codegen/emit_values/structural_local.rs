impl Emitter<'_> {
    pub(in crate::codegen) fn structural_local_kind(
        &self,
        value: ValueId,
    ) -> Result<Option<StructuralLocalKind>> {
        let metadata = self
            .local_metadata
            .get(&value)
            .ok_or_else(|| Error::msg("SSA structural local lost value metadata"))?;
        let ty = &metadata.ty;
        if matches!(ty, SsaType::StructuralDestination(_)) {
            return Ok(Some(StructuralLocalKind::Destination));
        }
        let dynamic_owner = match ty {
            SsaType::TypeParameter(parameter) => self
                .function
                .signature
                .memory_witness_parameters
                .iter()
                .any(|requirement| {
                    requirement.parameter == *parameter
                        && requirement.operations.contains(
                            &lkjscript_contracts::MemoryWitnessOperation::IndependentOwner,
                        )
                        && requirement.operations.contains(
                            &lkjscript_contracts::MemoryWitnessOperation::Dispose,
                        )
                }),
            _ => false,
        };
        if !dynamic_owner && structural_owner_representation(self.chunk, ty).is_none() {
            return Ok(None);
        }
        let kind = match metadata.producer {
            LocalProducerKind::Owner
            | LocalProducerKind::ProductField
            | LocalProducerKind::StructuralMove
            | LocalProducerKind::Call
            | LocalProducerKind::RuntimeOrConversion => Some(StructuralLocalKind::Owner),
            LocalProducerKind::View => Some(StructuralLocalKind::View),
            LocalProducerKind::Parameter if self.nonowned_structural_values().contains(&value) => {
                Some(StructuralLocalKind::OwnerRef)
            }
            LocalProducerKind::Parameter => Some(StructuralLocalKind::Owner),
            LocalProducerKind::Other => None,
        };
        Ok(kind)
    }

    fn nonowned_structural_values(&self) -> HashSet<ValueId> {
        let mut values = HashSet::new();
        for block in &self.function.blocks {
            values.extend(
                block
                    .parameters
                    .iter()
                    .filter(|parameter| parameter.owner_place.is_none())
                    .map(|parameter| parameter.id),
            );
            for instruction in &block.instructions {
                if matches!(
                    instruction.kind,
                    InstructionKind::Constant(Constant::Str(_))
                        | InstructionKind::Borrow { .. }
                        | InstructionKind::AggregateFieldBorrow { .. }
                ) {
                    values.insert(instruction.id);
                }
                if let InstructionKind::StructuralPublish { value, .. } = instruction.kind {
                    values.insert(value);
                }
            }
        }
        let mut dependents: HashMap<ValueId, Vec<ValueId>> = HashMap::new();
        let mut rejected = Vec::new();
        for block in &self.function.blocks {
            if block.id == self.function.entry {
                continue;
            }
            for (index, parameter) in block.parameters.iter().enumerate() {
                if parameter.owner_place.is_some() {
                    continue;
                }
                for predecessor in &self.function.blocks {
                    for argument in edge_arguments_to(&predecessor.terminator, block.id, index) {
                        dependents.entry(argument).or_default().push(parameter.id);
                        if !values.contains(&argument) {
                            rejected.push(parameter.id);
                        }
                    }
                }
            }
        }
        while let Some(value) = rejected.pop() {
            if values.remove(&value) {
                rejected.extend(dependents.get(&value).into_iter().flatten().copied());
            }
        }
        values
    }

}

fn edge_arguments_to(terminator: &Terminator, target: BlockId, index: usize) -> Vec<ValueId> {
    match terminator {
        Terminator::Branch {
            target: actual,
            arguments,
        } if *actual == target => arguments.get(index).copied().into_iter().collect(),
        Terminator::ConditionalBranch {
            true_target,
            true_arguments,
            false_target,
            false_arguments,
            ..
        } => {
            let mut values = Vec::with_capacity(2);
            if *true_target == target {
                values.extend(true_arguments.get(index).copied());
            }
            if *false_target == target {
                values.extend(false_arguments.get(index).copied());
            }
            values
        }
        _ => Vec::new(),
    }
}
