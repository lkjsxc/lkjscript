fn encode_structural_snapshot(
    out: &mut Encoder,
    structural: &OwnedStructuralValue,
) -> Result<()> {
    let metrics = validate_structural_snapshot(&structural.value)?;
    if metrics != structural.metrics {
        return Err(Error::msg("structural snapshot metrics disagree"));
    }
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| Error::msg("structural snapshot encode allocation failed"))?;
    pending.push(&structural.value);
    while let Some(value) = pending.pop() {
        encode_structural_type(out, value.value_type)?;
        match &value.payload {
            SemanticPayload::Inline(inline) => {
                out.u8(0)?;
                encode_structural_inline(out, *inline)?;
            }
            SemanticPayload::Static(leaf) => {
                out.u8(1)?;
                encode_structural_static(out, *leaf)?;
            }
            SemanticPayload::String(bytes) => {
                out.u8(2)?;
                out.bytes(bytes)?;
            }
            SemanticPayload::Path(bytes) => {
                out.u8(3)?;
                out.bytes(bytes)?;
            }
            SemanticPayload::Bytes(bytes) => {
                out.u8(4)?;
                out.bytes(bytes)?;
            }
            SemanticPayload::ByteVector(bytes) => {
                out.u8(5)?;
                out.bytes(bytes)?;
            }
            SemanticPayload::Product(fields) => {
                out.u8(6)?;
                schedule_structural_fields(out, &mut pending, fields)?;
            }
            SemanticPayload::Enum {
                tag,
                active_payload,
            } => {
                out.u8(7)?;
                out.u8(1)?;
                out.u64(*tag)?;
                schedule_structural_fields(out, &mut pending, active_payload)?;
            }
        }
    }
    Ok(())
}

fn schedule_structural_fields<'a>(
    out: &mut Encoder,
    pending: &mut Vec<&'a SemanticValue>,
    fields: &'a [SemanticValue],
) -> Result<()> {
    out.u64(
        u64::try_from(fields.len())
            .map_err(|_| Error::msg("structural snapshot field count exceeds u64"))?,
    )?;
    pending
        .try_reserve(fields.len())
        .map_err(|_| Error::msg("structural snapshot encode allocation failed"))?;
    pending.extend(fields.iter().rev());
    Ok(())
}

fn encode_structural_inline(out: &mut Encoder, value: InlineStructuralValue) -> Result<()> {
    match value {
        InlineStructuralValue::Unit => out.u8(0),
        InlineStructuralValue::Bool(false) => out.u8(1),
        InlineStructuralValue::Bool(true) => out.u8(2),
        InlineStructuralValue::I64(value) => {
            out.u8(3)?;
            out.u64(value as u64)
        }
        InlineStructuralValue::F64Bits(value) => {
            out.u8(4)?;
            out.u64(value)
        }
    }
}

fn encode_structural_static(out: &mut Encoder, value: crate::StaticStructuralLeaf) -> Result<()> {
    match value {
        crate::StaticStructuralLeaf::Function(value) => {
            out.u8(0)?;
            out.u64(value)
        }
        crate::StaticStructuralLeaf::Symbol(value) => {
            out.u8(1)?;
            out.u64(value)
        }
        crate::StaticStructuralLeaf::Bytes(value) => {
            out.u8(2)?;
            out.u64(value)
        }
    }
}
