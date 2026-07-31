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

fn make_enum<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let _index = vm.read_u16()?;
    Err(Error::msg("legacy enum construction is removed"))
}

fn is_variant<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let index = usize::from(vm.read_u16()?);
    let descriptor = variant_ref(vm, index)?;
    let definition = definition(vm, descriptor.enum_id)?;
    let _selected = variant(definition, descriptor.variant)?;
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
        return Err(Error::msg("legacy enum variant test is removed"));
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
        return Err(Error::msg("legacy enum projection is removed"));
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
