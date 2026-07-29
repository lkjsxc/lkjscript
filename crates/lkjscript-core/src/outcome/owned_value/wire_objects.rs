fn encode_object(out: &mut Encoder, object: &HeapObj) -> Result<()> {
    match object {
        HeapObj::Str(text) => {
            out.u8(0)?;
            out.text(text)
        }
        HeapObj::Pair { car, cdr } => {
            out.u8(1)?;
            encode_value(out, *car)?;
            encode_value(out, *cdr)
        }
        HeapObj::Buf(bytes) => {
            out.u8(2)?;
            out.bytes(bytes)
        }
        HeapObj::Path(bytes) => {
            out.u8(3)?;
            out.bytes(bytes)
        }
        HeapObj::Product { product, fields } => {
            out.u8(4)?;
            out.u16(product.raw())?;
            encode_values(out, fields)
        }
        HeapObj::Enum {
            layout,
            physical_tag,
            active_payload,
        } => {
            out.u8(5)?;
            out.fixed(&layout.bytes())?;
            out.u16(*physical_tag)?;
            encode_values(out, active_payload)
        }
    }
}

fn decode_object(input: &mut Decoder<'_>) -> Result<HeapObj> {
    Ok(match input.u8()? {
        0 => HeapObj::Str(input.text()?),
        1 => HeapObj::Pair {
            car: decode_value(input)?,
            cdr: decode_value(input)?,
        },
        2 => HeapObj::Buf(input.bytes()?.to_vec()),
        3 => HeapObj::Path(input.bytes()?.to_vec()),
        4 => HeapObj::Product {
            product: ProductId::new(input.u16()?),
            fields: decode_values(input)?,
        },
        5 => {
            let layout: [u8; 32] = input
                .fixed(32)?
                .try_into()
                .map_err(|_| Error::msg("runtime layout identity length"))?;
            HeapObj::Enum {
                layout: RuntimeLayoutId::new(layout),
                physical_tag: input.u16()?,
                active_payload: decode_values(input)?,
            }
        }
        _ => return Err(Error::msg("unknown owned heap object tag")),
    })
}

fn encode_values(out: &mut Encoder, values: &[Value]) -> Result<()> {
    out.usize(values.len())?;
    for value in values {
        encode_value(out, *value)?;
    }
    Ok(())
}

fn decode_values(input: &mut Decoder<'_>) -> Result<Vec<Value>> {
    let length = count(input.usize()?)?;
    let mut values = reserve(length)?;
    for _ in 0..length {
        values.push(decode_value(input)?);
    }
    Ok(values)
}

fn count(value: usize) -> Result<usize> {
    if value <= MAX_WIRE_ITEMS {
        Ok(value)
    } else {
        Err(Error::msg("owned value item count exceeds bound"))
    }
}

fn reserve<T>(length: usize) -> Result<Vec<T>> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| Error::msg("owned value wire allocation failed"))?;
    Ok(values)
}

fn validate_symbol(value: Value, symbols: &[Option<String>]) -> Result<()> {
    if let Some(index) = value.as_symbol() {
        let index = usize::try_from(index).map_err(|_| Error::msg("symbol index overflow"))?;
        if symbols.get(index).and_then(Option::as_ref).is_none() {
            return Err(Error::msg("owned value references a missing symbol"));
        }
    }
    Ok(())
}
