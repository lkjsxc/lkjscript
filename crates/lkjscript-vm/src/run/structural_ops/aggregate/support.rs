fn require_owner_representation(
    chunk: &ValidatedChunk,
    owner: OwnerRecord,
    expected: StructuralRepresentationId,
) -> Result<()> {
    if owner.value_type
        != representation(chunk, expected).and_then(|item| {
            chunk
                .structural_types()
                .get_structural(item.type_id)
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

fn require_active_variant(
    vm: &Vm<'_>,
    owner: StructuralValueKey,
    value_type: StructuralType,
    variant: Option<VariantId>,
) -> Result<()> {
    let Some(variant) = variant else {
        return Ok(());
    };
    let expected_tag = physical_tag(vm.chunk, value_type, variant)?;
    let node = invocation(vm)?
        .runtime
        .value_node(owner, value_type)
        .map_err(map_value_error)?;
    match node.payload() {
        StructuralNodeView::Enum { tag, .. } if tag == expected_tag => Ok(()),
        StructuralNodeView::Enum { .. } => Err(Error::msg(
            "structural operation selected an inactive enum variant",
        )),
        _ => Err(Error::msg(
            "structural enum operation reached non-enum payload",
        )),
    }
}

fn preflight_payload(
    vm: &Vm<'_>,
    owner: StructuralValueKey,
    value_type: StructuralType,
    reference: StructuralPayloadRef,
) -> Result<()> {
    let expected_tag = physical_tag(vm.chunk, value_type, reference.variant)?;
    let expected_type = reference
        .result
        .runtime_type
        .ok_or_else(|| Error::msg("structural payload result lacks exact runtime type"))?;
    let node = invocation(vm)?
        .runtime
        .value_node(owner, value_type)
        .map_err(map_value_error)?;
    match node.payload() {
        StructuralNodeView::Enum { tag, fields }
            if tag == expected_tag
                && fields.len() == 1
                && node.child(0).is_some_and(|field| field.value_type() == expected_type) =>
        {
            Ok(())
        }
        StructuralNodeView::Enum { .. } => Err(Error::msg(
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
) -> Result<u64> {
    let ty = chunk
        .structural_types()
        .iter()
        .find(|item| item.runtime_type == value_type)
        .ok_or_else(|| Error::msg("structural enum type metadata is missing"))?;
    let layout = chunk
        .structural_layouts()
        .get_structural(ty.layout)
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

fn structural_node_to_value(
    chunk: &ValidatedChunk,
    node: lkjscript_core::StructuralNode<'_>,
) -> Result<Value> {
    match node.payload() {
        StructuralNodeView::Inline(InlineStructuralValue::Unit) => Ok(Value::UNIT),
        StructuralNodeView::Inline(InlineStructuralValue::Bool(value)) => {
            Ok(Value::from_bool(value))
        }
        StructuralNodeView::Inline(InlineStructuralValue::I64(value)) => Ok(Value::from_i64(value)),
        StructuralNodeView::Inline(InlineStructuralValue::F64Bits(value)) => {
            Ok(Value::from_f64_bits(value))
        }
        StructuralNodeView::Static(StaticStructuralLeaf::Function(value)) => {
            chunk.function_value(value)
        }
        StructuralNodeView::Static(StaticStructuralLeaf::Symbol(value)) => chunk.symbol_value(value),
        StructuralNodeView::Static(StaticStructuralLeaf::Bytes(value)) => {
            Ok(Value::from_static_bytes(value))
        }
        StructuralNodeView::Bytes(_)
        | StructuralNodeView::Product(_)
        | StructuralNodeView::Enum { .. } => Err(Error::msg(
            "owned structural payload cannot be represented as a copied VM value",
        )),
    }
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
        _ => Err(Error::msg(
            "owned structural payload cannot be represented as a copied VM value",
        )),
    }
}

fn register_view_or_end(
    vm: &mut Vm<'_>,
    view: lkjscript_core::StructuralViewKey,
    representation: StructuralRepresentationId,
    expected: StructuralType,
    utf8: bool,
) -> Result<Value> {
    match invocation_mut(vm)?.register_view(view, representation, expected, utf8) {
        Ok(value) => Ok(value),
        Err(error) => {
            let _ = invocation_mut(vm)?.runtime.end_view(view);
            Err(error)
        }
    }
}
