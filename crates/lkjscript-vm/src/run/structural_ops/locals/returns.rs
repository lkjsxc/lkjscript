pub(in crate::run) fn prepare_return<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    value: Value,
    prototype: Option<usize>,
) -> Result<()> {
    cleanup_copy_roots(vm, value, prototype.is_none())?;
    let (expected, type_variable, variable_representation) = if let Some(prototype) = prototype {
        let proto = vm
            .chunk
            .protos()
            .get(prototype)
            .ok_or_else(|| Error::msg("return prototype metadata is missing"))?;
        let variable_representation = proto.return_type_variable.and_then(|variable| {
            vm.frames
                .last()
                .and_then(|frame| {
                    frame
                        .memory_witnesses
                        .iter()
                        .find(|binding| binding.parameter == variable)
                })
                .and_then(|binding| {
                    usize::try_from(binding.witness)
                        .ok()
                        .and_then(|index| vm.chunk.memory_witnesses().get(index))
                })
                .and_then(|witness| match witness.value_kind {
                    lkjscript_core::MemoryWitnessValueKind::Structural(representation) => {
                        Some(representation)
                    }
                    _ => None,
                })
        });
        (
            proto.return_structural,
            proto.return_type_variable,
            variable_representation,
        )
    } else {
        (
            vm.chunk.main().return_structural,
            vm.chunk.main().return_type_variable,
            None,
        )
    };
    match (
        expected,
        type_variable,
        variable_representation,
        value.as_structural_root(),
    ) {
        (Some(expected), None, None, Some(_)) => {
            let (_, record) = invocation(vm)?.owner(value)?;
            let expected_type =
                representation_type(vm.chunk, expected, StructuralValueCategory::Owner)?;
            if record.value_type != expected_type
                || !same_representation_type(vm.chunk, record.representation, expected)?
            {
                return Err(Error::msg("structural return representation mismatch"));
            }
            commit_handoff(vm, value)
        }
        (Some(_), None, None, None) => {
            Err(Error::msg("structural function returned a non-owner value"))
        }
        (None, Some(_), Some(expected), Some(_))
            if structural_value_matches_copy_representation(vm, value, expected)? =>
        {
            commit_handoff(vm, value)
        }
        (None, Some(_), Some(_), None) => Err(Error::msg(
            "copy-structural type variable returned a non-owner value",
        )),
        (None, Some(_), None, Some(_)) => Err(Error::msg(
            "structural owner escaped an unbound type-variable return",
        )),
        (None, None, None, Some(_)) if values::is_host_owner(invocation(vm)?, value) => Ok(()),
        (None, _, _, Some(_)) => Err(Error::msg(
            "structural owner escaped a function without exact bound metadata",
        )),
        (Some(_), Some(_), _, _) | (Some(_), None, Some(_), _) => Err(Error::msg(
            "structural return metadata overlaps a type variable",
        )),
        (None, _, _, None)
            if value.as_structural_view().is_some()
                || value.as_structural_destination().is_some() =>
        {
            Err(Error::msg("private structural value escaped a function"))
        }
        (None, _, _, None) => Ok(()),
    }
}

fn structural_value_matches_copy_representation<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    value: Value,
    expected: StructuralRepresentationId,
) -> Result<bool> {
    let (_, record) = invocation(vm)?.owner(value)?;
    let mode = vm.chunk.structural_representations()
        .get(record.representation.index())
        .and_then(|representation| {
            vm.chunk.structural_types().get(representation.type_id.index())
        })
        .map(|ty| ty.mode);
    let dynamic_owner = mode == Some(lkjscript_core::StructuralTypeMode::Immutable)
        && vm.frames.last().is_some_and(|frame| {
            frame.memory_witnesses.iter().any(|binding| {
                usize::try_from(binding.witness)
                    .ok()
                    .and_then(|index| vm.chunk.memory_witnesses().get(index))
                    .is_some_and(|witness| {
                        witness.facts.operations.contains(
                            &lkjscript_core::MemoryWitnessOperation::IndependentOwner,
                        ) && witness.facts.operations.contains(
                            &lkjscript_core::MemoryWitnessOperation::Dispose,
                        )
                    })
            })
        });
    Ok((mode == Some(lkjscript_core::StructuralTypeMode::Copy) || dynamic_owner)
        && same_representation_type(vm.chunk, record.representation, expected)?
        && record.value_type
            == representation_type(vm.chunk, expected, StructuralValueCategory::Owner)?)
}
