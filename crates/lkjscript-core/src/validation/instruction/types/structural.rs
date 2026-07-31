pub(super) fn pop_structural_leaf(
    chunk: &Chunk,
    state: &mut State,
    expected: crate::StructuralKind,
    legacy: Kind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let actual = pop(state, proto, instruction)?;
    if actual == legacy
        && (chunk.memory_plan.is_none()
            || (expected == crate::StructuralKind::String && actual == Kind::Str))
    {
        return Ok(());
    }
    let expected_name = match expected {
        crate::StructuralKind::String => "string",
        crate::StructuralKind::Path => "path",
        _ => "structural leaf",
    };
    let representation = match actual {
        Kind::StructuralOwner { representation, .. }
        | Kind::StructuralOwnerRef { representation, .. } => representation,
        _ => {
            return Err(instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                &format!("operation category mismatch: expected {expected_name}, got {actual}"),
            ))
        }
    };
    let kind = chunk
        .structural_representations
        .get(representation.index())
        .and_then(|item| chunk.structural_types.get(item.type_id.index()))
        .map(|item| item.runtime_type.kind);
    if kind == Some(expected) {
        Ok(())
    } else {
        Err(crate::Error::msg(
            "structural leaf operation has the wrong exact type",
        ))
    }
}

pub(super) fn structural_leaf_owner(
    chunk: &Chunk,
    kind: crate::StructuralKind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    if chunk.memory_plan.is_none() {
        return Ok(match kind {
            crate::StructuralKind::String => Kind::Str,
            crate::StructuralKind::Path => Kind::Path,
            _ => {
                return Err(crate::Error::msg(
                    "unsupported legacy structural leaf result",
                ))
            }
        });
    }
    let representation = chunk
        .structural_representations
        .iter()
        .find(|representation| {
            representation.category == crate::StructuralValueCategory::Owner
                && chunk
                    .structural_types
                    .get(representation.type_id.index())
                    .is_some_and(|ty| ty.runtime_type.kind == kind)
        });
    let representation = representation.ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "operation result lacks exact structural leaf metadata",
        )
    })?;
    Ok(Kind::StructuralOwner {
        representation: representation.id,
        owner: super::bytes::new_owner(instruction)?,
        active_variant: None,
    })
}
