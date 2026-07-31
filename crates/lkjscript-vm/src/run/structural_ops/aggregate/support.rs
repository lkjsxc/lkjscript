fn require_owner_representation(
    chunk: &ValidatedChunk,
    owner: OwnerRecord,
    expected: StructuralRepresentationId,
) -> Result<()> {
    if owner.value_type
        != representation(chunk, expected).and_then(|item| {
            chunk
                .structural_types()
                .get(item.type_id.index())
                .map(|ty| ty.runtime_type)
                .ok_or_else(|| Error::msg("structural operation type metadata is stale"))
        })?
        || !same_representation_type(chunk, owner.representation, expected)?
    {
        return Err(Error::msg(
            "structural operation representation does not match its owner",
        ));
    }
    Ok(())
}

fn require_active_variant<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    owner: StructuralValueKey,
    value_type: StructuralType,
    variant: Option<VariantId>,
) -> Result<()> {
    let Some(variant) = variant else {
        return Ok(());
    };
    let expected_tag = physical_tag(vm.chunk, value_type, variant)?;
    let value = invocation(vm)?
        .runtime
        .value(owner, value_type)
        .map_err(map_value_error)?;
    match value.payload {
        SemanticPayload::Enum { tag, .. } if tag == expected_tag => Ok(()),
        SemanticPayload::Enum { .. } => Err(Error::msg(
            "structural operation selected an inactive enum variant",
        )),
        _ => Err(Error::msg(
            "structural enum operation reached non-enum payload",
        )),
    }
}

fn preflight_payload<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    owner: StructuralValueKey,
    value_type: StructuralType,
    reference: StructuralPayloadRef,
) -> Result<()> {
    let expected_tag = physical_tag(vm.chunk, value_type, reference.variant)?;
    let expected_type = reference
        .result
        .runtime_type
        .ok_or_else(|| Error::msg("structural payload result lacks exact runtime type"))?;
    let value = invocation(vm)?
        .runtime
        .value(owner, value_type)
        .map_err(map_value_error)?;
    match &value.payload {
        SemanticPayload::Enum {
            tag,
            active_payload,
        } if *tag == expected_tag
            && active_payload.len() == 1
            && active_payload[0].value_type == expected_type =>
        {
            Ok(())
        }
        SemanticPayload::Enum { .. } => Err(Error::msg(
            "structural payload consume selected an inactive or malformed payload",
        )),
        _ => Err(Error::msg(
            "structural payload consume expects enum payload",
        )),
    }
}

fn physical_tag(
    chunk: &ValidatedChunk,
    value_type: StructuralType,
    variant: VariantId,
) -> Result<u16> {
    let ty = chunk
        .structural_types()
        .iter()
        .find(|item| item.runtime_type == value_type)
        .ok_or_else(|| Error::msg("structural enum type metadata is missing"))?;
    let layout = chunk
        .structural_layouts()
        .get(ty.layout.index())
        .filter(|layout| layout.id == ty.layout)
        .ok_or_else(|| Error::msg("structural enum layout metadata is missing"))?;
    let lkjscript_core::StructuralLayoutKind::Enum { variants, .. } = &layout.kind else {
        return Err(Error::msg("structural active variant requires enum layout"));
    };
    variants
        .iter()
        .find(|candidate| candidate.variant == variant)
        .map(|candidate| candidate.physical_tag)
        .ok_or_else(|| Error::msg("structural enum variant metadata is missing"))
}

fn semantic_to_value(chunk: &ValidatedChunk, value: &SemanticValue) -> Result<Value> {
    match value.payload {
        SemanticPayload::Inline(InlineStructuralValue::Unit) => Ok(Value::UNIT),
        SemanticPayload::Inline(InlineStructuralValue::Bool(value)) => Ok(Value::from_bool(value)),
        SemanticPayload::Inline(InlineStructuralValue::I64(value)) => Ok(Value::from_i64(value)),
        SemanticPayload::Inline(InlineStructuralValue::F64Bits(value)) => {
            Ok(Value::from_f64_bits(value))
        }
        SemanticPayload::Static(StaticStructuralLeaf::Function(value)) => {
            chunk.function_value(value)
        }
        SemanticPayload::Static(StaticStructuralLeaf::Symbol(value)) => chunk.symbol_value(value),
        SemanticPayload::Static(StaticStructuralLeaf::Bytes(value)) => {
            Ok(Value::from_static_bytes(value))
        }
        SemanticPayload::String(_)
        | SemanticPayload::Path(_)
        | SemanticPayload::Bytes(_)
        | SemanticPayload::ByteVector(_)
        | SemanticPayload::Product(_)
        | SemanticPayload::Enum { .. } => Err(Error::msg(
            "owned structural payload cannot be represented as a copied VM value",
        )),
    }
}

fn view_representation_for_type(
    chunk: &ValidatedChunk,
    type_id: lkjscript_core::StructuralTypeId,
) -> Result<StructuralRepresentationId> {
    representation_for_type(chunk, type_id, StructuralValueCategory::View)
}

fn owner_representation_for_type(
    chunk: &ValidatedChunk,
    type_id: lkjscript_core::StructuralTypeId,
) -> Result<StructuralRepresentationId> {
    representation_for_type(chunk, type_id, StructuralValueCategory::Owner)
}

fn representation_for_type(
    chunk: &ValidatedChunk,
    type_id: lkjscript_core::StructuralTypeId,
    category: StructuralValueCategory,
) -> Result<StructuralRepresentationId> {
    chunk
        .structural_representations()
        .iter()
        .find(|item| item.type_id == type_id && item.category == category)
        .map(|item| item.id)
        .ok_or_else(|| Error::msg("structural field representation metadata is missing"))
}
