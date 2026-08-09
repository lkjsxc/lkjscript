#[cfg(test)]
thread_local! {
    static NONOWNED_STRUCTURAL_COLLECTIONS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static NONOWNED_STRUCTURAL_CFG_EDGES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_nonowned_structural_work() {
    NONOWNED_STRUCTURAL_COLLECTIONS.with(|count| count.set(0));
    NONOWNED_STRUCTURAL_CFG_EDGES.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn nonowned_structural_work() -> (u64, u64) {
    (
        NONOWNED_STRUCTURAL_COLLECTIONS.with(std::cell::Cell::get),
        NONOWNED_STRUCTURAL_CFG_EDGES.with(std::cell::Cell::get),
    )
}

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
            LocalProducerKind::Parameter
                if self.nonowned_structural_values.contains(&value) =>
            {
                Some(StructuralLocalKind::OwnerRef)
            }
            LocalProducerKind::Parameter => Some(StructuralLocalKind::Owner),
            LocalProducerKind::Other => None,
        };
        Ok(kind)
    }
}

pub(in crate::codegen) fn collect_nonowned_structural_values(
    function: &Function,
) -> HashSet<ValueId> {
    #[cfg(test)]
    NONOWNED_STRUCTURAL_COLLECTIONS.with(|count| {
        count.set(count.get().saturating_add(1));
    });
    let mut values = HashSet::new();
    for block in &function.blocks {
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

    let parameters = function
        .blocks
        .iter()
        .map(|block| (block.id, block.parameters.as_slice()))
        .collect::<HashMap<_, _>>();
    let mut dependents: HashMap<ValueId, Vec<ValueId>> = HashMap::new();
    let mut rejected = Vec::new();
    let mut record_edge = |target: BlockId, arguments: &[ValueId]| {
        #[cfg(test)]
        NONOWNED_STRUCTURAL_CFG_EDGES.with(|count| {
            count.set(count.get().saturating_add(1));
        });
        if target == function.entry {
            return;
        }
        let Some(target_parameters) = parameters.get(&target) else {
            return;
        };
        for (parameter, argument) in target_parameters.iter().zip(arguments) {
            if parameter.owner_place.is_some() {
                continue;
            }
            dependents.entry(*argument).or_default().push(parameter.id);
            if !values.contains(argument) {
                rejected.push(parameter.id);
            }
        }
    };
    for block in &function.blocks {
        match &block.terminator {
            Terminator::Branch { target, arguments } => record_edge(*target, arguments),
            Terminator::ConditionalBranch {
                true_target,
                true_arguments,
                false_target,
                false_arguments,
                ..
            } => {
                record_edge(*true_target, true_arguments);
                record_edge(*false_target, false_arguments);
            }
            Terminator::Return(_)
            | Terminator::Trap { .. }
            | Terminator::Exit { .. }
            | Terminator::Outcome { .. } => {}
        }
    }
    while let Some(value) = rejected.pop() {
        if values.remove(&value) {
            rejected.extend(dependents.get(&value).into_iter().flatten().copied());
        }
    }
    values
}
