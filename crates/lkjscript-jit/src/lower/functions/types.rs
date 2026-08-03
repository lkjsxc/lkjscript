use super::*;

pub(super) fn collect_value_types(
    program: &lkjscript_ir::Program,
    function: &Function,
    layouts: &LayoutInterner,
    modes: &BytesModes,
) -> Result<Vec<ValueType>, LoweringError> {
    let mut types: Vec<Option<ValueType>> = Vec::new();
    for block in &function.blocks {
        for parameter in &block.parameters {
            set_value_type(
                &mut types,
                parameter.id,
                lower_value_type(function.id, parameter.id, &parameter.ty, modes, layouts)?,
            )?;
        }
        for instruction in &block.instructions {
            let value_type =
                if let Some(view) = structural_instruction_type(function, instruction, layouts)? {
                    view
                } else if static_trap_message(function, instruction.id).is_some() {
                    ValueType::Unit
                } else if matches!(
                    instruction.kind,
                    InstructionKind::Call {
                        target: CallTarget::Direct(callee),
                        instantiation: Some(ref instantiation),
                        ..
                    } if !instantiation.memory_witnesses.is_empty()
                        && program.functions
                            .get(callee.index().unwrap_or(usize::MAX))
                            .filter(|function| function.id == callee)
                            .is_some_and(|function| matches!(
                                function.signature.result.as_ref(),
                                SsaType::TypeParameter(_)
                            ))
                ) {
                    ValueType::StructuralKey
                } else if matches!(
                    instruction.kind,
                    InstructionKind::DestinationCreate { .. }
                        | InstructionKind::DestinationFieldInit { .. }
                ) {
                    let (aggregate, storage, initialized) =
                        layouts.structural().destination(function, instruction.id)?;
                    ValueType::StructuralDestination(aggregate.destination(storage, initialized))
                } else {
                    lower_value_type(function.id, instruction.id, &instruction.ty, modes, layouts)?
                };
            set_value_type(&mut types, instruction.id, value_type)?;
        }
    }
    types
        .into_iter()
        .map(|ty| {
            ty.ok_or_else(|| {
                LoweringError::new(
                    LoweringFailureCode::InvalidFunction,
                    Some(function.id),
                    "SSA value IDs are not dense",
                )
            })
        })
        .collect()
}

fn structural_instruction_type(
    function: &Function,
    instruction: &Instruction,
    layouts: &LayoutInterner,
) -> Result<Option<ValueType>, LoweringError> {
    if matches!(
        instruction.kind,
        InstructionKind::Runtime {
            operation: RuntimeOp::EmptyStr,
            ..
        }
    ) || (matches!(
        instruction.kind,
        InstructionKind::Constant(Constant::Str(_))
    ) && static_trap_message(function, instruction.id).is_none())
    {
        let value_type = layouts
            .structural()
            .value_type(&instruction.ty)
            .ok_or_else(|| invalid_structural("static structural string type is missing"))?;
        return Ok(Some(ValueType::StaticString(value_type)));
    }
    let (value, kind, path, mutable) = match instruction.kind {
        InstructionKind::Borrow { kind, value, .. } => (
            value,
            lkjscript_native::StructuralProjectionKind::Field,
            Vec::new(),
            kind == lkjscript_ir::BorrowKind::Mutable,
        ),
        InstructionKind::AggregateFieldBorrow { field, value, .. }
            if layouts.structural().selected(&instruction.ty) =>
        {
            (
                value,
                lkjscript_native::StructuralProjectionKind::Field,
                vec![field],
                false,
            )
        }
        InstructionKind::StringUtf8View { value, .. } => (
            value,
            lkjscript_native::StructuralProjectionKind::Utf8,
            Vec::new(),
            false,
        ),
        _ => return Ok(None),
    };
    let root_ty = function
        .blocks
        .iter()
        .find_map(|block| {
            block
                .parameters
                .iter()
                .find(|parameter| parameter.id == value)
                .map(|parameter| &parameter.ty)
                .or_else(|| {
                    block
                        .instructions
                        .iter()
                        .find(|candidate| candidate.id == value)
                        .map(|candidate| &candidate.ty)
                })
        })
        .ok_or_else(|| invalid_structural("structural view source type is missing"))?;
    let Some(root) = layouts.structural().value_type(root_ty) else {
        return Ok(None);
    };
    let projected = if kind == lkjscript_native::StructuralProjectionKind::Utf8 {
        root
    } else {
        layouts
            .structural()
            .value_type(&instruction.ty)
            .ok_or_else(|| invalid_structural("structural projected type is missing"))?
    };
    let projection = layouts
        .structural()
        .view(root, projected, path, kind, mutable);
    Ok(Some(ValueType::StructuralView(projection.view_type())))
}

pub(super) fn set_value_type(
    types: &mut Vec<Option<ValueType>>,
    value: ValueId,
    ty: ValueType,
) -> Result<(), LoweringError> {
    let index = value.index().ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            None,
            "SSA value ID cannot index native locals",
        )
    })?;
    if types.len() <= index {
        types.resize(index + 1, None);
    }
    if types[index].replace(ty).is_some() {
        return Err(LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            None,
            "duplicate SSA value during native lowering",
        ));
    }
    Ok(())
}
