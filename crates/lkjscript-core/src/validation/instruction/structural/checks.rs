fn reject_live_view(
    state: &State,
    owner: OwnerIdentity,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    if state
        .locals
        .iter()
        .flatten()
        .chain(&state.stack)
        .any(|kind| {
            matches!(
                kind,
                Kind::StructuralOwnerRef { owner: active, .. }
                    | Kind::StructuralView { owner: active, .. }
                    | Kind::ByteSlice { owner: active, .. }
                    if *active == owner
            )
        })
    {
        fail(
            proto,
            instruction,
            "structural owner operation has a live loan",
        )
    } else {
        Ok(())
    }
}

fn require_same_type(
    chunk: &Chunk,
    left: StructuralRepresentationId,
    right: StructuralRepresentationId,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let left = chunk
        .structural_representations
        .get(left.index())
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "structural representation is missing",
            )
        })?;
    let right = chunk
        .structural_representations
        .get(right.index())
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "structural representation is missing",
            )
        })?;
    if left.type_id == right.type_id && left.layout == right.layout {
        Ok(())
    } else {
        fail(
            proto,
            instruction,
            &format!(
                "structural representation type/layout mismatch: {}:{}/{} != {}:{}/{}",
                left.id.raw(),
                left.type_id.raw(),
                left.layout.raw(),
                right.id.raw(),
                right.type_id.raw(),
                right.layout.raw(),
            ),
        )
    }
}

fn require_field_value(
    chunk: &Chunk,
    expected: StructuralFieldMetadata,
    actual: Kind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let valid = match (expected.route, expected.runtime_type, actual) {
        (StructuralFieldRoute::Copy, Some(expected), value) => {
            exact_copy_kind(expected.kind, value)
        }
        (
            StructuralFieldRoute::Structural(type_id),
            Some(expected),
            Kind::StructuralOwner { representation, .. },
        ) => chunk
            .structural_representations
            .get(representation.index())
            .and_then(|item| {
                chunk
                    .structural_types
                    .get(item.type_id.index())
                    .map(|ty| (item, ty))
            })
            .is_some_and(|(item, ty)| {
                item.type_id == type_id
                    && item.category == StructuralValueCategory::Owner
                    && ty.runtime_type == expected
            }),
        (
            StructuralFieldRoute::Unique,
            Some(crate::StructuralType {
                kind: crate::StructuralKind::Bytes,
                ..
            }),
            Kind::Bytes(_),
        )
        | (
            StructuralFieldRoute::Unique,
            Some(crate::StructuralType {
                kind: crate::StructuralKind::ByteVector,
                ..
            }),
            Kind::ByteVector(_),
        ) => true,
        (StructuralFieldRoute::Resource, None, Kind::Resource { .. }) => true,
        (StructuralFieldRoute::LegacyHeap, _, _) => false,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        fail(
            proto,
            instruction,
            "destination field value takes the wrong ownership route",
        )
    }
}

fn field_result_kind(
    _chunk: &Chunk,
    field: StructuralFieldMetadata,
    _owner: OwnerIdentity,
    _mutable: bool,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    match field.route {
        StructuralFieldRoute::Copy => field
            .runtime_type
            .and_then(exact_copy_result)
            .ok_or_else(|| {
                instruction_error(
                    proto,
                    instruction.op(),
                    instruction.offset(),
                    "structural copy field has invalid exact runtime type",
                )
            }),
        StructuralFieldRoute::Structural(_) => fail(
            proto,
            instruction,
            "structural field result requires an operation-bound representation",
        ),
        StructuralFieldRoute::Unique
        | StructuralFieldRoute::Resource
        | StructuralFieldRoute::LegacyHeap => fail(
            proto,
            instruction,
            "structural field operation would cross a non-structural ownership route",
        ),
    }
}


include!("checks/results.rs");
