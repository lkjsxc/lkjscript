use super::*;

fn definition<'a, J: RuntimeTier>(
    vm: &'a Vm<'_, J>,
    id: EnumId,
) -> Result<&'a lkjscript_core::EnumMetadata> {
    vm.chunk
        .enums()
        .iter()
        .find(|definition| definition.id == id)
        .ok_or_else(|| Error::msg("enum metadata identity is invalid"))
}

fn variant(
    definition: &lkjscript_core::EnumMetadata,
    id: VariantId,
) -> Result<&lkjscript_core::EnumVariantMetadata> {
    definition
        .variants
        .iter()
        .find(|variant| variant.id == id)
        .ok_or_else(|| Error::msg("enum variant identity is invalid"))
}

fn construction<J: RuntimeTier>(vm: &Vm<'_, J>, index: usize) -> Result<EnumConstructionRef> {
    vm.chunk
        .enum_constructions()
        .get(index)
        .copied()
        .ok_or_else(|| Error::msg("enum construction descriptor index out of range"))
}

fn variant_ref<J: RuntimeTier>(vm: &Vm<'_, J>, index: usize) -> Result<EnumVariantRef> {
    vm.chunk
        .enum_variants()
        .get(index)
        .copied()
        .ok_or_else(|| Error::msg("enum variant descriptor index out of range"))
}

fn field_ref<J: RuntimeTier>(vm: &Vm<'_, J>, index: usize) -> Result<EnumFieldRef> {
    vm.chunk
        .enum_fields()
        .get(index)
        .copied()
        .ok_or_else(|| Error::msg("enum field descriptor index out of range"))
}

fn charge_construction<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    if vm.logical_aggregate_constructions >= vm.config.max_logical_aggregate_constructions {
        return Err(Error::resource(
            ResourceLimitKind::LogicalAggregateConstructions,
            "logical aggregate construction limit exceeded before enum allocation",
        ));
    }
    vm.logical_aggregate_constructions += 1;
    Ok(())
}

fn make_enum<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let index = usize::from(vm.read_u16()?);
    let descriptor = construction(vm, index)?;
    let metadata = definition(vm, descriptor.enum_id)?;
    let selected = variant(metadata, descriptor.variant)?;
    if descriptor.layout != metadata.layout
        || usize::from(descriptor.substitution_arity) != usize::from(metadata.type_parameter_count)
    {
        return Err(Error::msg("enum construction layout/substitution mismatch"));
    }
    let field_count = selected.fields.len();
    let physical_tag = selected.physical_tag;
    charge_construction(vm)?;
    let mut active_payload = Vec::with_capacity(field_count);
    for _ in 0..field_count {
        active_payload.push(vm.pop()?);
    }
    active_payload.reverse();
    if active_payload
        .iter()
        .copied()
        .any(is_structural_runtime_value)
    {
        return Err(Error::msg(
            "legacy traced enum cannot contain a structural runtime value",
        ));
    }
    let definition = vm
        .chunk
        .enums()
        .iter()
        .find(|item| item.id == descriptor.enum_id)
        .ok_or_else(|| Error::msg("enum metadata identity is invalid"))?;
    let value = vm.arena.alloc_validated_enum(
        definition,
        descriptor.layout,
        physical_tag,
        active_payload,
    )?;
    vm.push(value);
    Ok(())
}

fn is_variant<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let index = usize::from(vm.read_u16()?);
    let descriptor = variant_ref(vm, index)?;
    let definition = definition(vm, descriptor.enum_id)?;
    let expected_tag = variant(definition, descriptor.variant)?.physical_tag;
    if descriptor.layout != definition.layout {
        return Err(Error::msg("enum variant test layout mismatch"));
    }
    let value = vm.pop()?;
    let matches = if let Some(matches) = structural_ops::adapter_is_variant(
        vm,
        value,
        descriptor.enum_id,
        descriptor.layout,
        descriptor.variant,
    )? {
        matches
    } else {
        match vm.arena.get(value)? {
            HeapObj::Enum {
                layout,
                physical_tag,
                ..
            } if *layout == descriptor.layout => *physical_tag == expected_tag,
            HeapObj::Enum { .. } => return Err(Error::msg("enum variant test layout mismatch")),
            _ => return Err(Error::msg("enum variant test expects enum")),
        }
    };
    vm.push(Value::from_bool(matches));
    Ok(())
}

fn load_field<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let descriptor_index = usize::from(vm.read_u16()?);
    let descriptor = field_ref(vm, descriptor_index)?;
    let definition = definition(vm, descriptor.enum_id)?;
    let selected = variant(definition, descriptor.variant)?;
    if descriptor.layout != definition.layout {
        return Err(Error::msg("enum projection layout mismatch"));
    }
    let index = selected
        .fields
        .iter()
        .position(|field| field.id == descriptor.field)
        .ok_or_else(|| Error::msg("enum projection field identity mismatch"))?;
    let expected_tag = selected.physical_tag;
    let value = vm.pop()?;
    let projected = if let Some(projected) = structural_ops::adapter_take_field(
        vm,
        value,
        descriptor.enum_id,
        descriptor.layout,
        descriptor.variant,
    )? {
        if index != 0 {
            return Err(Error::msg("aggregate adapter field identity mismatch"));
        }
        super::ext_ops::clear_resource_aliases(vm, value);
        projected
    } else {
        match vm.arena.get(value)? {
            HeapObj::Enum {
                layout,
                physical_tag,
                active_payload,
            } if *layout == descriptor.layout && *physical_tag == expected_tag => active_payload
                .get(index)
                .copied()
                .ok_or_else(|| Error::msg("enum active payload is malformed"))?,
            HeapObj::Enum { .. } => {
                return Err(Error::msg(
                    "inactive enum projection rejected before payload access",
                ));
            }
            _ => return Err(Error::msg("enum projection expects enum")),
        }
    };
    vm.push(projected);
    Ok(())
}

pub(super) fn handles(op: u8) -> bool {
    matches!(
        Op::from_byte(op),
        Some(Op::MakeEnum | Op::IsEnumVariant | Op::LoadEnumField)
    )
}

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<()> {
    match Op::from_byte(op) {
        Some(Op::MakeEnum) => make_enum(vm),
        Some(Op::IsEnumVariant) => is_variant(vm),
        Some(Op::LoadEnumField) => load_field(vm),
        _ => unreachable!("enum opcode family checked"),
    }
}
