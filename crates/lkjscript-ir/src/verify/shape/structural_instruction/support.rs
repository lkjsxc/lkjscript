fn lookup_representation(
    program: &Program,
    id: crate::StructuralRepresentationId,
    category: StructuralValueCategory,
) -> crate::Result<(&SsaType, crate::StructuralTypeId)> {
    let item = program
        .memory
        .representations
        .get(id.index().unwrap_or(usize::MAX))
        .filter(|item| item.id == id && item.category == category)
        .ok_or_else(|| crate::IrError::new("SSA structural representation is missing or stale"))?;
    let ty = structural_type(program, item.type_id)?;
    Ok((ty, item.type_id))
}

fn structural_type(program: &Program, id: crate::StructuralTypeId) -> crate::Result<&SsaType> {
    program
        .memory
        .types
        .get(id.index().unwrap_or(usize::MAX))
        .filter(|item| item.id == id)
        .map(|item| &item.ty)
        .ok_or_else(|| crate::IrError::new("SSA structural type metadata is missing"))
}

fn require_structural_copy_mode(
    program: &Program,
    type_id: crate::StructuralTypeId,
) -> crate::Result<()> {
    let metadata = program
        .memory
        .types
        .get(type_id.index().unwrap_or(usize::MAX))
        .filter(|metadata| metadata.id == type_id)
        .ok_or_else(|| crate::IrError::new("SSA structural copy type is missing"))?;
    if metadata.mode == crate::StructuralTypeMode::Affine {
        fail("SSA structural copy cannot duplicate an affine owner")
    } else {
        Ok(())
    }
}

fn structural_layout(
    program: &Program,
    type_id: crate::StructuralTypeId,
) -> crate::Result<&crate::StructuralLayoutMetadata> {
    let item = program
        .memory
        .types
        .get(type_id.index().unwrap_or(usize::MAX))
        .ok_or_else(|| crate::IrError::new("SSA structural type metadata is missing"))?;
    program
        .memory
        .layouts
        .get(item.layout.index().unwrap_or(usize::MAX))
        .filter(|layout| layout.id == item.layout)
        .ok_or_else(|| crate::IrError::new("SSA structural layout metadata is missing"))
}

fn destination_type(
    types: &[SsaType],
    value: crate::ValueId,
) -> crate::Result<crate::StructuralTypeId> {
    match value_type(types, value)? {
        SsaType::StructuralDestination(id) => Ok(*id),
        _ => fail("SSA destination operation requires private destination value"),
    }
}


fn aggregate_field(
    program: &Program,
    type_id: crate::StructuralTypeId,
    field: u16,
) -> crate::Result<&SsaType> {
    let index = usize::from(field);
    match &structural_layout(program, type_id)?.kind {
        StructuralLayoutKind::Product { fields, .. } => fields
            .get(index)
            .ok_or_else(|| crate::IrError::new("SSA aggregate field is out of range")),
        StructuralLayoutKind::Enum { variants, .. } => {
            let mut fields = variants.iter().map(|variant| variant.fields.get(index));
            let expected = fields
                .next()
                .flatten()
                .ok_or_else(|| crate::IrError::new("SSA enum field schema is variant-dependent"))?;
            if fields.all(|field| field == Some(expected)) {
                Ok(expected)
            } else {
                fail("SSA enum field schema is variant-dependent")
            }
        }
        StructuralLayoutKind::String | StructuralLayoutKind::Path => {
            fail("SSA leaf has no aggregate fields")
        }
    }
}

fn destination_field<'program>(
    program: &'program Program,
    function: &Function,
    destination: crate::ValueId,
    type_id: crate::StructuralTypeId,
    field: u16,
) -> crate::Result<&'program SsaType> {
    let index = usize::from(field);
    match &structural_layout(program, type_id)?.kind {
        StructuralLayoutKind::Product { fields, .. } => fields
            .get(index)
            .ok_or_else(|| crate::IrError::new("SSA destination field is out of range")),
        StructuralLayoutKind::Enum { variants, .. } => {
            let active = destination_active_variant(function, destination)?
                .ok_or_else(|| crate::IrError::new("SSA enum destination lost its active variant"))?;
            variants
                .iter()
                .find(|variant| variant.variant == active)
                .and_then(|variant| variant.fields.get(index))
                .ok_or_else(|| crate::IrError::new("SSA active payload field is out of range"))
        }
        StructuralLayoutKind::String | StructuralLayoutKind::Path => {
            fail("SSA leaf destination has no aggregate fields")
        }
    }
}

fn destination_active_variant(
    function: &Function,
    destination: crate::ValueId,
) -> crate::Result<Option<crate::VariantId>> {
    let instruction_count = function
        .blocks
        .iter()
        .map(|block| block.instructions.len())
        .sum::<usize>();
    let mut current = destination;
    for _ in 0..=instruction_count {
        let instruction = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find(|instruction| instruction.id == current)
            .ok_or_else(|| crate::IrError::new("SSA destination definition is missing"))?;
        match &instruction.kind {
            InstructionKind::DestinationCreate { active_variant, .. } => {
                return Ok(*active_variant)
            }
            InstructionKind::DestinationFieldInit { destination, .. } => current = *destination,
            _ => return fail("SSA destination value has invalid provenance"),
        }
    }
    fail("SSA destination provenance is cyclic")
}

fn verify_active_variant(
    program: &Program,
    type_id: crate::StructuralTypeId,
    active: Option<crate::VariantId>,
) -> crate::Result<()> {
    match (&structural_layout(program, type_id)?.kind, active) {
        (StructuralLayoutKind::Enum { variants, .. }, Some(active))
            if variants.iter().any(|variant| variant.variant == active) =>
        {
            Ok(())
        }
        (StructuralLayoutKind::Enum { .. }, _) => {
            fail("SSA enum destination has missing or inactive payload metadata")
        }
        (
            StructuralLayoutKind::String
            | StructuralLayoutKind::Path
            | StructuralLayoutKind::Product { .. },
            None,
        ) => Ok(()),
        _ => fail("SSA non-enum destination carries active payload metadata"),
    }
}
