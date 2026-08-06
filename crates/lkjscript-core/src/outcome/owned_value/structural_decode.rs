fn decode_structural_snapshot(
    input: &mut Decoder<'_>,
    limits: StructuralSnapshotLimits,
) -> Result<OwnedStructuralValue> {
    let mut budget = StructuralDecodeBudget::new(limits)?;
    let value = decode_structural_node(input, &mut budget, 1)?;
    let measured = budget.finish();
    let validated = validate_structural_snapshot(&value, limits, SnapshotWork::Decode)?;
    if measured != validated {
        return Err(Error::msg("structural snapshot decode accounting disagrees"));
    }
    Ok(OwnedStructuralValue {
        value,
        metrics: validated,
    })
}

fn decode_structural_node(
    input: &mut Decoder<'_>,
    budget: &mut StructuralDecodeBudget,
    depth: u16,
) -> Result<SemanticValue> {
    budget.node(depth)?;
    let value_type = decode_structural_type(input)?;
    let (payload, expected) = match input.u8()? {
        0 => {
            let (payload, kind) = decode_structural_inline(input)?;
            (SemanticPayload::Inline(payload), kind)
        }
        1 => (
            SemanticPayload::Static(decode_structural_static(input)?),
            StructuralKind::Static,
        ),
        2 => (
            SemanticPayload::String(decode_structural_bytes(
                input,
                budget,
                DecodeByteClass::String,
            )?),
            StructuralKind::String,
        ),
        3 => (
            SemanticPayload::Path(decode_structural_bytes(
                input,
                budget,
                DecodeByteClass::Path,
            )?),
            StructuralKind::Path,
        ),
        4 => (
            SemanticPayload::Bytes(decode_structural_bytes(
                input,
                budget,
                DecodeByteClass::Other,
            )?),
            StructuralKind::Bytes,
        ),
        5 => (
            SemanticPayload::ByteVector(decode_structural_bytes(
                input,
                budget,
                DecodeByteClass::Other,
            )?),
            StructuralKind::ByteVector,
        ),
        6 => (
            SemanticPayload::Product(decode_structural_fields(input, budget, depth)?.into()),
            StructuralKind::Product,
        ),
        7 => {
            if input.u8()? != 1 {
                return Err(Error::msg(
                    "structural enum must contain one active payload section",
                ));
            }
            let tag = input.u16()?;
            let active_payload = decode_structural_fields(input, budget, depth)?.into();
            (
                SemanticPayload::Enum {
                    tag,
                    active_payload,
                },
                StructuralKind::Enum,
            )
        }
        _ => return Err(Error::msg("unknown structural payload tag")),
    };
    require_snapshot_kind(value_type, expected)?;
    Ok(SemanticValue::new(value_type, payload))
}

fn decode_structural_inline(
    input: &mut Decoder<'_>,
) -> Result<(InlineStructuralValue, StructuralKind)> {
    Ok(match input.u8()? {
        0 => (InlineStructuralValue::Unit, StructuralKind::Unit),
        1 => (InlineStructuralValue::Bool(false), StructuralKind::Bool),
        2 => (InlineStructuralValue::Bool(true), StructuralKind::Bool),
        3 => (
            InlineStructuralValue::I64(input.u64()? as i64),
            StructuralKind::I64,
        ),
        4 => (
            InlineStructuralValue::F64Bits(input.u64()?),
            StructuralKind::F64,
        ),
        _ => return Err(Error::msg("unknown structural inline tag")),
    })
}

fn decode_structural_static(input: &mut Decoder<'_>) -> Result<crate::StaticStructuralLeaf> {
    Ok(match input.u8()? {
        0 => crate::StaticStructuralLeaf::Function(input.u64()?),
        1 => crate::StaticStructuralLeaf::Symbol(input.u64()?),
        2 => crate::StaticStructuralLeaf::Bytes(input.u64()?),
        _ => return Err(Error::msg("unknown structural static tag")),
    })
}

fn decode_structural_bytes(
    input: &mut Decoder<'_>,
    budget: &mut StructuralDecodeBudget,
    class: DecodeByteClass,
) -> Result<Vec<u8>> {
    let bytes = input.bytes()?;
    budget.bytes(bytes.len(), class)?;
    if matches!(class, DecodeByteClass::String) {
        std::str::from_utf8(bytes)
            .map_err(|_| Error::msg("structural snapshot string is not UTF-8"))?;
    }
    if matches!(class, DecodeByteClass::Path) {
        validate_snapshot_path(bytes)?;
    }
    copy_wire_bytes(bytes)
}

fn decode_structural_fields(
    input: &mut Decoder<'_>,
    budget: &mut StructuralDecodeBudget,
    parent_depth: u16,
) -> Result<Vec<SemanticValue>> {
    let count = input.u32()?;
    budget.fields(count)?;
    let mut fields = reserve_structural_fields(count)?;
    for _ in 0..count {
        fields.push(decode_structural_node(input, budget, parent_depth + 1)?);
    }
    Ok(fields)
}
