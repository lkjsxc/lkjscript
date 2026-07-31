pub(super) fn lower_structural_instruction(
    function: &Function,
    instruction: &Instruction,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    layouts: &LayoutInterner,
    builder: &mut FunctionBuilder,
) -> NativeResult {
    let catalog = layouts.structural();
    let operation = match &instruction.kind {
        InstructionKind::StructuralPublish { value, .. } => {
            let value_type = structural_type(catalog, &instruction.ty)?;
            let input = read_value(builder, block, locals, *value, function.id)?;
            let operation = publish_operation(function, *value, value_type, value_types)?;
            return structural_call(builder, block, operation, vec![input]);
        }
        InstructionKind::StructuralCopy { value, .. } => {
            let value_type = structural_type(catalog, &instruction.ty)?;
            let input = observe_value(function, *value, block, locals, builder)?;
            return structural_call(
                builder,
                block,
                lkjscript_native::StructuralOperation::Copy(value_type),
                vec![input],
            );
        }
        InstructionKind::DestinationCreate { .. } => {
            let (aggregate, initialized) = catalog.destination(function, instruction.id)?;
            if initialized != 0 {
                return Err(invalid_structural("new destination is already initialized"));
            }
            lkjscript_native::StructuralOperation::DestinationCreate(aggregate)
        }
        InstructionKind::DestinationFieldInit {
            destination,
            field,
            value,
        } => {
            let (aggregate, initialized) = catalog.destination(function, *destination)?;
            if initialized != *field {
                return Err(invalid_structural(
                    "destination field initialization is out of order",
                ));
            }
            let arguments =
                read_values(builder, block, locals, &[*destination, *value], function.id)?;
            return structural_call(
                builder,
                block,
                lkjscript_native::StructuralOperation::DestinationInitialize {
                    aggregate,
                    field: *field,
                },
                arguments,
            );
        }
        InstructionKind::DestinationFinish { destination } => {
            let (aggregate, initialized) = catalog.destination(function, *destination)?;
            if usize::from(initialized) != aggregate.fields().len() {
                return Err(invalid_structural("destination finish is incomplete"));
            }
            let input = read_value(builder, block, locals, *destination, function.id)?;
            return structural_call(
                builder,
                block,
                lkjscript_native::StructuralOperation::DestinationFinish(aggregate),
                vec![input],
            );
        }
        InstructionKind::DestinationAbort { destination } => {
            let (aggregate, initialized) = catalog.destination(function, *destination)?;
            let input = read_value(builder, block, locals, *destination, function.id)?;
            return structural_call(
                builder,
                block,
                lkjscript_native::StructuralOperation::DestinationAbort {
                    aggregate,
                    initialized,
                },
                vec![input],
            );
        }
        InstructionKind::AggregateFieldBorrow {
            representation,
            field,
            value,
            ..
        } => {
            let (_, root_ty) = catalog
                .representation(*representation, lkjscript_ir::StructuralValueCategory::View)?;
            let root = structural_type(catalog, &root_ty)?;
            let projected = structural_type(catalog, &instruction.ty)?;
            let projection = catalog.view(
                root,
                projected,
                vec![*field],
                lkjscript_native::StructuralProjectionKind::Field,
                false,
            );
            let input = observe_value(function, *value, block, locals, builder)?;
            let view = structural_call(
                builder,
                block,
                lkjscript_native::StructuralOperation::Borrow {
                    projection: projection.clone(),
                },
                vec![input],
            )?;
            if value_type(value_types, instruction.id)?
                == ValueType::StructuralView(projection.view_type())
            {
                return Ok(view);
            }
            let copied = structural_call(
                builder,
                block,
                lkjscript_native::StructuralOperation::CopyView(projection.view_type()),
                vec![view],
            )?;
            let _ = structural_call(
                builder,
                block,
                lkjscript_native::StructuralOperation::EndView(projection.view_type()),
                vec![view],
            )?;
            return Ok(copied);
        }
        InstructionKind::AggregateTag { value, .. } => {
            let root = structural_type(catalog, source_type(function, *value)?)?;
            let input = observe_value(function, *value, block, locals, builder)?;
            return structural_call(
                builder,
                block,
                lkjscript_native::StructuralOperation::ObserveOwnedTag(root),
                vec![input],
            );
        }
        InstructionKind::AggregateConsumePayload {
            representation,
            variant,
            value,
            ..
        } => {
            let (type_id, _) = catalog.representation(
                *representation,
                lkjscript_ir::StructuralValueCategory::Owner,
            )?;
            let aggregate = catalog.aggregate(type_id, Some(*variant))?;
            let input = read_value(builder, block, locals, *value, function.id)?;
            return structural_call(
                builder,
                block,
                lkjscript_native::StructuralOperation::ConsumePayload(aggregate),
                vec![input],
            );
        }
        InstructionKind::StringUtf8View { value, .. } => {
            let root = structural_type(catalog, source_type(function, *value)?)?;
            let projection = catalog.view(
                root,
                root,
                Vec::new(),
                lkjscript_native::StructuralProjectionKind::Utf8,
                false,
            );
            let input = observe_value(function, *value, block, locals, builder)?;
            return structural_call(
                builder,
                block,
                lkjscript_native::StructuralOperation::StringUtf8View { projection },
                vec![input],
            );
        }
        _ => {
            return Err(invalid_structural(
                "instruction is not an explicit structural call",
            ))
        }
    };
    structural_call(builder, block, operation, Vec::new())
}
