enum StructuralDecodeFrame {
    Product {
        value_type: StructuralType,
        remaining: usize,
        fields: Vec<SemanticValue>,
    },
    Enum {
        value_type: StructuralType,
        tag: u64,
        remaining: usize,
        fields: Vec<SemanticValue>,
    },
}

impl StructuralDecodeFrame {
    fn push(&mut self, value: SemanticValue) -> Result<bool> {
        let (remaining, fields) = match self {
            Self::Product {
                remaining, fields, ..
            }
            | Self::Enum {
                remaining, fields, ..
            } => (remaining, fields),
        };
        if *remaining == 0 {
            return Err(Error::msg("structural snapshot decode frame overflow"));
        }
        fields.push(value);
        *remaining -= 1;
        Ok(*remaining == 0)
    }

    fn finish(self) -> SemanticValue {
        match self {
            Self::Product {
                value_type,
                fields,
                ..
            } => SemanticValue::new(value_type, SemanticPayload::Product(fields.into())),
            Self::Enum {
                value_type,
                tag,
                fields,
                ..
            } => SemanticValue::new(
                value_type,
                SemanticPayload::Enum {
                    tag,
                    active_payload: fields.into(),
                },
            ),
        }
    }
}

enum DecodedStructuralItem {
    Complete(SemanticValue),
    Aggregate(StructuralDecodeFrame),
}

fn decode_structural_snapshot(input: &mut Decoder<'_>) -> Result<OwnedStructuralValue> {
    let mut budget = StructuralDecodeBudget::new();
    let mut frames = Vec::new();
    frames
        .try_reserve(1)
        .map_err(|_| Error::msg("structural snapshot decode allocation failed"))?;
    let value = 'decode: loop {
        let mut item = decode_structural_item(input, &mut budget)?;
        loop {
            match item {
                DecodedStructuralItem::Aggregate(frame) => {
                    let empty = match &frame {
                        StructuralDecodeFrame::Product { remaining, .. }
                        | StructuralDecodeFrame::Enum { remaining, .. } => *remaining == 0,
                    };
                    if empty {
                        item = DecodedStructuralItem::Complete(frame.finish());
                        continue;
                    }
                    frames
                        .try_reserve(1)
                        .map_err(|_| Error::msg("structural snapshot decode allocation failed"))?;
                    frames.push(frame);
                    break;
                }
                DecodedStructuralItem::Complete(value) => {
                    let Some(frame) = frames.last_mut() else {
                        break 'decode value;
                    };
                    if !frame.push(value)? {
                        break;
                    }
                    let frame = frames
                        .pop()
                        .ok_or_else(|| Error::msg("structural snapshot decode frame missing"))?;
                    item = DecodedStructuralItem::Complete(frame.finish());
                }
            }
        }
    };

    let measured = budget.finish();
    let validated = validate_structural_snapshot(&value)?;
    if measured != validated {
        return Err(Error::msg("structural snapshot decode accounting disagrees"));
    }
    Ok(OwnedStructuralValue {
        value,
        metrics: validated,
    })
}

fn decode_structural_item(
    input: &mut Decoder<'_>,
    budget: &mut StructuralDecodeBudget,
) -> Result<DecodedStructuralItem> {
    budget.node()?;
    let value_type = decode_structural_type(input)?;
    let item = match input.u8()? {
        0 => {
            let (payload, kind) = decode_structural_inline(input)?;
            require_snapshot_kind(value_type, kind)?;
            DecodedStructuralItem::Complete(SemanticValue::new(
                value_type,
                SemanticPayload::Inline(payload),
            ))
        }
        1 => {
            require_snapshot_kind(value_type, StructuralKind::Static)?;
            DecodedStructuralItem::Complete(SemanticValue::new(
                value_type,
                SemanticPayload::Static(decode_structural_static(input)?),
            ))
        }
        2 => decode_structural_byte_item(
            input,
            budget,
            value_type,
            StructuralKind::String,
            DecodeByteClass::String,
        )?,
        3 => decode_structural_byte_item(
            input,
            budget,
            value_type,
            StructuralKind::Path,
            DecodeByteClass::Path,
        )?,
        4 => decode_structural_byte_item(
            input,
            budget,
            value_type,
            StructuralKind::Bytes,
            DecodeByteClass::Other,
        )?,
        5 => decode_structural_byte_item(
            input,
            budget,
            value_type,
            StructuralKind::ByteVector,
            DecodeByteClass::Other,
        )?,
        6 => {
            require_snapshot_kind(value_type, StructuralKind::Product)?;
            let (remaining, fields) = decode_structural_field_storage(input, budget)?;
            DecodedStructuralItem::Aggregate(StructuralDecodeFrame::Product {
                value_type,
                remaining,
                fields,
            })
        }
        7 => {
            require_snapshot_kind(value_type, StructuralKind::Enum)?;
            if input.u8()? != 1 {
                return Err(Error::msg(
                    "structural enum must contain one active payload section",
                ));
            }
            let tag = input.u64()?;
            let (remaining, fields) = decode_structural_field_storage(input, budget)?;
            DecodedStructuralItem::Aggregate(StructuralDecodeFrame::Enum {
                value_type,
                tag,
                remaining,
                fields,
            })
        }
        _ => return Err(Error::msg("unknown structural payload tag")),
    };
    Ok(item)
}

fn decode_structural_byte_item(
    input: &mut Decoder<'_>,
    budget: &mut StructuralDecodeBudget,
    value_type: StructuralType,
    kind: StructuralKind,
    class: DecodeByteClass,
) -> Result<DecodedStructuralItem> {
    require_snapshot_kind(value_type, kind)?;
    let bytes = decode_structural_bytes(input, budget, class)?;
    let payload = match kind {
        StructuralKind::String => SemanticPayload::String(bytes),
        StructuralKind::Path => SemanticPayload::Path(bytes),
        StructuralKind::Bytes => SemanticPayload::Bytes(bytes),
        StructuralKind::ByteVector => SemanticPayload::ByteVector(bytes),
        _ => return Err(Error::msg("structural byte payload kind mismatch")),
    };
    Ok(DecodedStructuralItem::Complete(SemanticValue::new(
        value_type, payload,
    )))
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

fn decode_structural_field_storage(
    input: &mut Decoder<'_>,
    budget: &mut StructuralDecodeBudget,
) -> Result<(usize, Vec<SemanticValue>)> {
    let count = input.u64()?;
    budget.fields(count)?;
    let fields = reserve_structural_fields(count)?;
    let remaining = usize::try_from(count)
        .map_err(|_| Error::msg("structural snapshot field count exceeds platform"))?;
    Ok((remaining, fields))
}
