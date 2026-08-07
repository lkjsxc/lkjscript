fn registered_owner_type(
    structural: &StructuralInvocation,
    key: StructuralValueKey,
) -> Option<StructuralType> {
    structural
        .owners
        .get(&key.get())
        .map(|record| record.value_type)
        .or_else(|| {
            structural
                .list_owners
                .get(&key.get())
                .copied()
                .map(ListOwnerRecord::value_type)
        })
        .or_else(|| structural.host_owners.get(&key.get()).copied())
}

pub(in crate::run) fn copy_into_list(
    vm: &mut Vm<'_>,
    value: Value,
) -> Result<Value> {
    reject_affine_list_value(value)?;
    if let Some(key) = value.as_structural_root() {
        let record = if let Some(record) = invocation(vm)?.owners.get(&key.get()).copied() {
            ListOwnerRecord::Typed(record)
        } else if let Some(value_type) = invocation(vm)?.host_owners.get(&key.get()).copied() {
            ListOwnerRecord::Host(value_type)
        } else {
            return Err(Error::msg(
                "segmented list received a stale or unregistered structural owner",
            ));
        };
        let copy = invocation_mut(vm)?
            .runtime
            .clone_owned(key, record.value_type())
            .map_err(map_value_error)?;
        register_list_owner(vm, copy, record)?;
        return Ok(Value::from_structural_root(copy));
    }
    if value.as_structural_view().is_some() {
        let (view, record) = invocation(vm)?.view(value)?;
        if record.utf8 {
            return Err(Error::msg("segmented list cannot retain a UTF-8 view"));
        }
        let view_metadata = representation(vm.chunk, record.representation)?;
        let value_type = representation_type(
            vm.chunk,
            record.representation,
            StructuralValueCategory::View,
        )?;
        let mut candidates = vm
            .chunk
            .structural_representations()
            .iter()
            .filter(|item| {
                item.type_id == view_metadata.type_id
                    && item.category == StructuralValueCategory::Owner
                    && item.storage == lkjscript_core::StructuralStorage::UniqueStructural
            });
        let owner_representation = candidates
            .next()
            .map(|item| item.id)
            .ok_or_else(|| Error::msg("segmented list owner representation is missing"))?;
        if candidates.next().is_some() {
            return Err(Error::msg(
                "segmented list owner representation is ambiguous",
            ));
        }
        let semantic = invocation(vm)?
            .runtime
            .projected(view)
            .map_err(map_value_error)?;
        let copy = invocation_mut(vm)?
            .runtime
            .publish_owned(semantic)
            .map_err(|failure| map_value_error(failure.error))?;
        register_list_owner(
            vm,
            copy,
            ListOwnerRecord::Typed(OwnerRecord {
                representation: owner_representation,
                value_type,
                taken_from: None,
            }),
        )?;
        return Ok(Value::from_structural_root(copy));
    }
    Ok(value)
}

pub(in crate::run) fn copy_from_list(
    vm: &mut Vm<'_>,
    value: Value,
    expected: Option<StructuralRepresentationId>,
) -> Result<Value> {
    let Some(key) = value.as_structural_root() else {
        if expected.is_some() {
            return Err(Error::msg(
                "list-first structural witness produced a non-structural value",
            ));
        }
        return Ok(value);
    };
    let expected = expected.ok_or_else(|| {
        Error::msg("list-first structural value lacks exact representation metadata")
    })?;
    let expected_type = representation_type(
        vm.chunk,
        expected,
        StructuralValueCategory::Owner,
    )?;
    let record = invocation(vm)?
        .list_owners
        .get(&key.get())
        .copied()
        .ok_or_else(|| Error::msg("segmented list structural owner is stale"))?;
    match record {
        ListOwnerRecord::Typed(record) => {
            if record.value_type != expected_type
                || !same_representation_type(vm.chunk, record.representation, expected)?
            {
                return Err(Error::msg("list-first structural element type mismatch"));
            }
            let copy = invocation_mut(vm)?
                .runtime
                .clone_owned(key, expected_type)
                .map_err(map_value_error)?;
            invocation_mut(vm)?.register_owner(copy, expected, expected_type)
        }
        ListOwnerRecord::Host(host_type) => {
            if host_type.kind != expected_type.kind {
                return Err(Error::msg("list-first host structural element type mismatch"));
            }
            let semantic = invocation(vm)?
                .runtime
                .value(key, host_type)
                .map_err(map_value_error)?;
            let copy = invocation_mut(vm)?
                .runtime
                .publish_owned(lkjscript_core::SemanticValue::new(
                    expected_type,
                    semantic.payload,
                ))
                .map_err(|failure| map_value_error(failure.error))?;
            invocation_mut(vm)?.register_owner(copy, expected, expected_type)
        }
    }
}

fn register_list_owner(
    vm: &mut Vm<'_>,
    key: StructuralValueKey,
    record: ListOwnerRecord,
) -> Result<()> {
    if invocation_mut(vm)?
        .list_owners
        .insert(key.get(), record)
        .is_some()
    {
        let _ = invocation_mut(vm)?
            .runtime
            .drop_owned(key, record.value_type());
        return Err(Error::msg("duplicate segmented-list owner key"));
    }
    Ok(())
}

fn reject_affine_list_value(value: Value) -> Result<()> {
    if value.as_structural_destination().is_some()
        || value.as_bytes_key().is_some()
        || value.as_byte_vector_key().is_some()
        || value.as_bytes_borrow().is_some()
        || value.as_byte_slice().is_some()
        || value.as_resource().is_some()
    {
        Err(Error::msg(
            "segmented list cannot contain an affine, borrowed, or resource value",
        ))
    } else {
        Ok(())
    }
}
