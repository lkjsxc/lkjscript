fn field_borrow<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let (reference, owner, root_type, expected) = field_projection_input(vm)?;
    let representation = match reference.result.route {
        StructuralFieldRoute::Copy => None,
        StructuralFieldRoute::Structural(type_id) => {
            Some(view_representation_for_type(vm.chunk, type_id)?)
        }
        StructuralFieldRoute::Unique
        | StructuralFieldRoute::Resource
        | StructuralFieldRoute::LegacyHeap => {
            return Err(Error::msg(
                "structural field borrow crosses an unsupported ownership route",
            ));
        }
    };
    let view = borrow_field(vm, owner, root_type, reference.field, expected)?;
    if let Some(representation) = representation {
        let value = register_view_or_end(vm, view, representation, expected, false)?;
        vm.push(value);
        return Ok(());
    }
    let result = invocation(vm)?
        .runtime
        .projected_node(view)
        .map_err(map_value_error)
        .and_then(|node| structural_node_to_value(vm.chunk, node));
    let ended = end_projected_view(vm, view);
    match (result, ended) {
        (Ok(value), Ok(())) => vm.push(value),
        (Err(primary), _) => return Err(primary),
        (Ok(_), Err(cleanup)) => return Err(cleanup),
    }
    Ok(())
}

fn field_copy<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let (reference, owner, root_type, expected) = field_projection_input(vm)?;
    match reference.result.route {
        StructuralFieldRoute::Copy => {
            let view = borrow_field(vm, owner, root_type, reference.field, expected)?;
            let result = invocation(vm)?
                .runtime
                .projected_node(view)
                .map_err(map_value_error)
                .and_then(|node| structural_node_to_value(vm.chunk, node));
            let ended = end_projected_view(vm, view);
            match (result, ended) {
                (Ok(value), Ok(())) => vm.push(value),
                (Err(primary), _) => return Err(primary),
                (Ok(_), Err(cleanup)) => return Err(cleanup),
            }
        }
        StructuralFieldRoute::Structural(type_id) => {
            let representation = owner_representation_for_type(vm.chunk, type_id)?;
            let view = borrow_field(vm, owner, root_type, reference.field, expected)?;
            let result = invocation(vm)?
                .runtime
                .projected(view)
                .map_err(map_value_error);
            let ended = end_projected_view(vm, view);
            let semantic = match (result, ended) {
                (Ok(value), Ok(())) => value,
                (Err(primary), _) => return Err(primary),
                (Ok(_), Err(cleanup)) => return Err(cleanup),
            };
            let key = invocation_mut(vm)?
                .runtime
                .publish_owned(semantic)
                .map_err(|failure| map_value_error(failure.error))?;
            let value = invocation_mut(vm)?.register_owner(key, representation, expected)?;
            vm.push(value);
        }
        StructuralFieldRoute::Unique
        | StructuralFieldRoute::Resource
        | StructuralFieldRoute::LegacyHeap => {
            return Err(Error::msg(
                "structural field copy crosses an unsupported ownership route",
            ));
        }
    }
    Ok(())
}

fn field_projection_input<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
) -> Result<(
    lkjscript_core::StructuralAggregateFieldRef,
    lkjscript_core::StructuralValueKey,
    lkjscript_core::StructuralType,
    lkjscript_core::StructuralType,
)> {
    let index = usize::from(vm.read_u16()?);
    let reference = *vm
        .chunk
        .structural_aggregate_fields()
        .get(index)
        .ok_or_else(|| Error::msg("structural aggregate-field reference is stale"))?;
    let source = vm.pop()?;
    let (owner, record) = invocation(vm)?.owner(source)?;
    require_owner_representation(vm.chunk, record, reference.representation)?;
    require_active_variant(vm, owner, record.value_type, reference.active_variant)?;
    let expected = reference
        .result
        .runtime_type
        .ok_or_else(|| Error::msg("structural field result lacks exact runtime type"))?;
    Ok((reference, owner, record.value_type, expected))
}

fn borrow_field<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    owner: lkjscript_core::StructuralValueKey,
    root_type: lkjscript_core::StructuralType,
    field: u16,
    expected: lkjscript_core::StructuralType,
) -> Result<lkjscript_core::StructuralViewKey> {
    invocation_mut(vm)?
        .runtime
        .borrow_projected(
            owner,
            root_type,
            StructuralProjection::Field {
                path: StructuralFieldPath::new(vec![field]),
                expected,
            },
            false,
        )
        .map_err(map_value_error)
}

fn end_projected_view<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    view: lkjscript_core::StructuralViewKey,
) -> Result<()> {
    invocation_mut(vm)?
        .runtime
        .end_view(view)
        .map_err(map_value_error)
}
