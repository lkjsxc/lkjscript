fn encode_structural_snapshot(
    out: &mut Encoder,
    structural: &OwnedStructuralValue,
    limits: StructuralSnapshotLimits,
) -> Result<()> {
    let metrics = validate_structural_snapshot(&structural.value, limits, SnapshotWork::Encode)?;
    if metrics != structural.metrics {
        return Err(Error::msg("structural snapshot metrics disagree"));
    }
    encode_structural_node(out, &structural.value)
}

fn encode_structural_node(out: &mut Encoder, value: &SemanticValue) -> Result<()> {
    encode_structural_type(out, value.value_type)?;
    match &value.payload {
        SemanticPayload::Inline(inline) => {
            out.u8(0)?;
            encode_structural_inline(out, *inline)
        }
        SemanticPayload::Static(leaf) => {
            out.u8(1)?;
            encode_structural_static(out, *leaf)
        }
        SemanticPayload::String(bytes) => {
            out.u8(2)?;
            out.bytes(bytes)
        }
        SemanticPayload::Path(bytes) => {
            out.u8(3)?;
            out.bytes(bytes)
        }
        SemanticPayload::Bytes(bytes) => {
            out.u8(4)?;
            out.bytes(bytes)
        }
        SemanticPayload::ByteVector(bytes) => {
            out.u8(5)?;
            out.bytes(bytes)
        }
        SemanticPayload::Product(fields) => {
            out.u8(6)?;
            encode_structural_fields(out, fields)
        }
        SemanticPayload::Enum {
            tag,
            active_payload,
        } => {
            out.u8(7)?;
            out.u8(1)?;
            out.u16(*tag)?;
            encode_structural_fields(out, active_payload)
        }
    }
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

fn encode_structural_fields(out: &mut Encoder, fields: &[SemanticValue]) -> Result<()> {
    out.u32(
        u32::try_from(fields.len())
            .map_err(|_| Error::msg("structural snapshot field count exceeds u32"))?,
    )?;
    for field in fields {
        encode_structural_node(out, field)?;
    }
    Ok(())
}
