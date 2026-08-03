fn validate_operation_references(chunk: &Chunk, mut bytes: usize) -> Result<usize> {
    for reference in &chunk.structural_destination_fields {
        let destination = lookup_destination(chunk, reference.destination)?;
        if usize::from(reference.field) >= destination.fields.len() {
            return Err(Error::msg(
                "bytecode structural destination-field reference is out of range",
            ));
        }
        bytes = add(bytes, 4, "structural metadata byte size")?;
    }
    for reference in &chunk.structural_aggregate_fields {
        let representation = lookup_representation(chunk, reference.representation)?;
        if representation.category != StructuralValueCategory::View {
            return Err(Error::msg(
                "bytecode structural aggregate field requires a view representation",
            ));
        }
        let expected = layout_fields(chunk, representation.layout, reference.active_variant)?;
        if expected.get(usize::from(reference.field)) != Some(&reference.result) {
            return Err(Error::msg(
                "bytecode structural aggregate-field result metadata is stale",
            ));
        }
        validate_field(chunk, &reference.result)?;
        validate_result_representation(
            chunk,
            &reference.result,
            reference.result_representation,
            None,
        )?;
        bytes = add(bytes, 68, "structural metadata byte size")?;
    }
    for proto in std::iter::once(&chunk.main).chain(&chunk.protos) {
        if proto.memory_plan != chunk.memory_plan {
            return Err(Error::msg(
                "bytecode function MemoryPlanId does not match its chunk",
            ));
        }
        for action in proto.failure_cleanups.iter().flat_map(|plan| &plan.actions) {
            match action {
                crate::FailureCleanupAction::EndStructuralBorrow { representation, .. }
                | crate::FailureCleanupAction::DropStructural { representation, .. } => {
                    let _ = lookup_representation(chunk, *representation)?;
                }
                crate::FailureCleanupAction::AbortStructuralDestination {
                    destination: id, ..
                } => {
                    let _ = lookup_destination(chunk, *id)?;
                }
                crate::FailureCleanupAction::EndBorrow { .. }
                | crate::FailureCleanupAction::DropUnique { .. }
                | crate::FailureCleanupAction::DropResource { .. } => {}
            }
        }
        for representation_id in proto
            .parameter_structurals
            .iter()
            .copied()
            .flatten()
            .chain(proto.return_structural)
        {
            let item = lookup_representation(chunk, representation_id)?;
            if item.category != StructuralValueCategory::Owner {
                return Err(Error::msg(
                    "bytecode structural signature requires owner representations",
                ));
            }
        }
    }
    for reference in &chunk.structural_payloads {
        let representation = lookup_representation(chunk, reference.representation)?;
        if representation.category != StructuralValueCategory::Owner {
            return Err(Error::msg(
                "bytecode structural payload consume requires an owner representation",
            ));
        }
        let fields = layout_fields(chunk, representation.layout, Some(reference.variant))?;
        if fields.as_slice() != [reference.result] {
            return Err(Error::msg(
                "bytecode structural payload consume requires one exact active field",
            ));
        }
        validate_field(chunk, &reference.result)?;
        validate_result_representation(
            chunk,
            &reference.result,
            reference.result_representation,
            Some(StructuralValueCategory::Owner),
        )?;
        bytes = add(bytes, 99, "structural metadata byte size")?;
    }
    Ok(bytes)
}

fn validate_result_representation(
    chunk: &Chunk,
    field: &StructuralFieldMetadata,
    result: Option<crate::StructuralRepresentationId>,
    category: Option<StructuralValueCategory>,
) -> Result<()> {
    match (field.route, result) {
        (StructuralFieldRoute::Structural(type_id), Some(result)) => {
            let representation = lookup_representation(chunk, result)?;
            if representation.type_id != type_id
                || category.is_some_and(|expected| representation.category != expected)
            {
                return Err(Error::msg(
                    "bytecode structural result representation is not exact",
                ));
            }
            Ok(())
        }
        (StructuralFieldRoute::Structural(_), None) => Err(Error::msg(
            "bytecode structural result representation is missing",
        )),
        (_, None) => Ok(()),
        (_, Some(_)) => Err(Error::msg(
            "bytecode non-structural result carries a representation",
        )),
    }
}
