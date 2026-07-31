fn borrow(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let view = representation_operand(chunk, proto, instruction)?;
    if view.category != StructuralValueCategory::View {
        return fail(
            proto,
            instruction,
            "structural borrow requires a view representation",
        );
    }
    let source = pop(state, proto, instruction)?;
    let Kind::StructuralOwnerRef {
        representation,
        owner,
        ..
    } = source
    else {
        return fail(
            proto,
            instruction,
            "structural borrow expects an owner reference",
        );
    };
    require_same_type(chunk, representation, view.id, proto, instruction)?;
    let mutable = instruction.op() == Op::StructuralBorrowMut;
    if state.locals.iter().flatten().any(|kind| {
        matches!(
            kind,
            Kind::StructuralView {
                owner: active,
                mutable: active_mutable,
                ..
            } if *active == owner && (mutable || *active_mutable)
        )
    }) {
        return fail(
            proto,
            instruction,
            "structural borrow conflicts with a live loan",
        );
    }
    state.stack.push(Kind::StructuralView {
        representation: view.id,
        owner,
        mutable,
        used: false,
    });
    Ok(())
}

fn publish(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let representation = representation_operand(chunk, proto, instruction)?;
    if representation.category != StructuralValueCategory::Owner {
        return fail(
            proto,
            instruction,
            "structural publish requires an owner representation",
        );
    }
    let input = pop(state, proto, instruction)?;
    if let Kind::StructuralOwner {
        representation: actual,
        owner,
        active_variant,
    } = input
    {
        require_same_type(chunk, actual, representation.id, proto, instruction)?;
        state.stack.push(Kind::StructuralOwner {
            representation: representation.id,
            owner,
            active_variant,
        });
        return Ok(());
    }
    let ty = chunk
        .structural_types
        .get(representation.type_id.index())
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "structural type is missing",
            )
        })?;
    let active_variant = match (ty.kind, input) {
        (crate::StructuralTypeKind::String, Kind::Str)
        | (crate::StructuralTypeKind::Path, Kind::Path) => Some(None),
        (crate::StructuralTypeKind::Product(expected), Kind::Product(actual))
            if expected == actual =>
        {
            Some(None)
        }
        (crate::StructuralTypeKind::Enum(expected), Kind::Enum(actual, variant))
            if expected == actual =>
        {
            Some(variant)
        }
        (crate::StructuralTypeKind::String, _)
        | (crate::StructuralTypeKind::Path, _)
        | (crate::StructuralTypeKind::Product(_), _)
        | (crate::StructuralTypeKind::Enum(_), _) => None,
    };
    let Some(active_variant) = active_variant else {
        return fail(
            proto,
            instruction,
            "structural publish input does not match exact type metadata",
        );
    };
    let owner = fresh_identity(proto, instruction, 1)?;
    state.stack.push(Kind::StructuralOwner {
        representation: representation.id,
        owner,
        active_variant,
    });
    Ok(())
}
