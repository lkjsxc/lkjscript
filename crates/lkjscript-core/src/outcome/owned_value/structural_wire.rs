use std::num::NonZeroU64;

fn encode_structural_type(out: &mut Encoder, value: StructuralType) -> Result<()> {
    // These two nonzero values are semantic codec descriptors. Runtime/domain,
    // root, slot, generation, destination, view, loan, and owner identities are
    // not part of this record.
    out.u64(value.layout.get())?;
    out.u64(value.semantic_type.get())?;
    out.u8(structural_kind_tag(value.kind))
}

fn decode_structural_type(input: &mut Decoder<'_>) -> Result<StructuralType> {
    let layout = NonZeroU64::new(input.u64()?)
        .ok_or_else(|| Error::msg("zero structural layout descriptor"))?;
    let semantic_type = NonZeroU64::new(input.u64()?)
        .ok_or_else(|| Error::msg("zero structural semantic type descriptor"))?;
    Ok(StructuralType::new(
        crate::LayoutIdentity::new(layout),
        crate::SemanticTypeIdentity::new(semantic_type),
        decode_structural_kind(input.u8()?)?,
    ))
}

fn structural_kind_tag(value: StructuralKind) -> u8 {
    match value {
        StructuralKind::Unit => 0,
        StructuralKind::Bool => 1,
        StructuralKind::I64 => 2,
        StructuralKind::F64 => 3,
        StructuralKind::String => 4,
        StructuralKind::Path => 5,
        StructuralKind::Bytes => 6,
        StructuralKind::ByteVector => 7,
        StructuralKind::Product => 8,
        StructuralKind::Enum => 9,
        StructuralKind::Static => 10,
    }
}

fn decode_structural_kind(tag: u8) -> Result<StructuralKind> {
    Ok(match tag {
        0 => StructuralKind::Unit,
        1 => StructuralKind::Bool,
        2 => StructuralKind::I64,
        3 => StructuralKind::F64,
        4 => StructuralKind::String,
        5 => StructuralKind::Path,
        6 => StructuralKind::Bytes,
        7 => StructuralKind::ByteVector,
        8 => StructuralKind::Product,
        9 => StructuralKind::Enum,
        10 => StructuralKind::Static,
        _ => return Err(Error::msg("unknown structural type kind")),
    })
}

fn copy_wire_bytes(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len())
        .map_err(|_| Error::msg("structural snapshot allocation failed"))?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}

fn reserve_structural_fields(length: u64) -> Result<Vec<SemanticValue>> {
    let length = usize::try_from(length)
        .map_err(|_| Error::msg("structural field count exceeds platform"))?;
    let mut fields = Vec::new();
    fields
        .try_reserve_exact(length)
        .map_err(|_| Error::msg("structural field allocation failed"))?;
    Ok(fields)
}
