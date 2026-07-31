use crate::outcome::codec::{Decoder, Encoder};

impl OwnedValue {
    pub(crate) fn encode_wire(&self, out: &mut Encoder) -> Result<()> {
        self.validate_wire_symbols()?;
        if let Some(structural) = &self.structural {
            out.u8(3)?;
            let limits = out.structural_limits();
            encode_structural_snapshot(out, structural, limits)?;
            return encode_symbol_table(out, &self.symbols);
        }
        if let Some(bytes) = &self.unique_byte_vector {
            out.u8(1)?;
            return out.bytes(bytes);
        }
        if let Some(bytes) = &self.unique_bytes {
            out.u8(2)?;
            return out.bytes(bytes);
        }
        out.u8(0)?;
        encode_value(out, self.root)?;
        if self.lists.len() > MAX_WIRE_ITEMS {
            return Err(Error::msg("owned list table exceeds wire item limit"));
        }
        out.usize(self.lists.len())?;
        for node in &self.lists {
            encode_value(out, node.head)?;
            encode_value(out, node.tail)?;
        }
        encode_symbol_table(out, &self.symbols)
    }

    pub(crate) fn decode_wire(input: &mut Decoder<'_>) -> Result<Self> {
        match input.u8()? {
            0 => {
                let root = decode_value(input)?;
                let list_count = count(input.usize()?)?;
                let mut lists = reserve(list_count)?;
                for _ in 0..list_count {
                    lists.push(OwnedListNode {
                        head: decode_value(input)?,
                        tail: decode_value(input)?,
                    });
                }
                let mut value = Self::from_materialized_snapshot(root, lists)?;
                value.symbols = decode_symbol_table(input)?;
                value.validate_wire_symbols()?;
                Ok(value)
            }
            1 => Self::from_unique_byte_vector(input.bytes()?.to_vec()),
            2 => Self::from_unique_bytes(input.bytes()?.to_vec()),
            3 => {
                let limits = input.structural_limits();
                let structural = decode_structural_snapshot(input, limits)?;
                let mut value = Self::from_owned_structural(structural);
                value.symbols = decode_symbol_table(input)?;
                value.validate_wire_symbols()?;
                Ok(value)
            }
            _ => Err(Error::msg("unknown owned value tag")),
        }
    }

    fn validate_wire_symbols(&self) -> Result<()> {
        validate_symbol(self.root, &self.symbols)?;
        for node in &self.lists {
            validate_symbol(node.head, &self.symbols)?;
            validate_symbol(node.tail, &self.symbols)?;
        }
        for symbol in self.structural_symbol_order()? {
            validate_symbol_index(symbol, &self.symbols)?;
        }
        Ok(())
    }
}

fn encode_symbol_table(out: &mut Encoder, symbols: &[Option<String>]) -> Result<()> {
    out.usize(symbols.len())?;
    for symbol in symbols {
        match symbol {
            Some(text) => {
                out.u8(1)?;
                out.text(text)?;
            }
            None => out.u8(0)?,
        }
    }
    Ok(())
}

fn decode_symbol_table(input: &mut Decoder<'_>) -> Result<Vec<Option<String>>> {
    let symbol_count = count(input.usize()?)?;
    let mut symbols = reserve(symbol_count)?;
    for _ in 0..symbol_count {
        symbols.push(match input.u8()? {
            0 => None,
            1 => Some(input.text()?),
            _ => return Err(Error::msg("unknown owned symbol slot tag")),
        });
    }
    Ok(symbols)
}

fn encode_value(out: &mut Encoder, value: Value) -> Result<()> {
    if value.is_unit() {
        out.u8(0)
    } else if let Some(value) = value.as_bool() {
        out.u8(if value { 2 } else { 1 })
    } else if let Some(value) = value.as_i64() {
        out.u8(3)?;
        out.u64(value as u64)
    } else if let Some(value) = value.as_f64_bits() {
        out.u8(4)?;
        out.u64(value)
    } else if value.is_empty_list() {
        out.u8(5)
    } else if let Some(value) = value.as_symbol() {
        out.u8(9)?;
        out.u32(value)
    } else if let Some(value) = value.as_owned_list() {
        out.u8(11)?;
        out.u32(value)
    } else {
        Err(Error::msg("owned value category is not process transportable"))
    }
}

fn decode_value(input: &mut Decoder<'_>) -> Result<Value> {
    Ok(match input.u8()? {
        0 => Value::UNIT,
        1 => Value::FALSE,
        2 => Value::TRUE,
        3 => Value::from_i64(input.u64()? as i64),
        4 => Value::from_f64_bits(input.u64()?),
        5 => Value::EMPTY_LIST,
        9 => Value::from_symbol(input.u32()?),
        11 => Value::from_owned_list(input.u32()?),
        _ => return Err(Error::msg("unknown value tag")),
    })
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
        validate_symbol_index(index, symbols)?;
    }
    Ok(())
}

fn validate_symbol_index(index: u32, symbols: &[Option<String>]) -> Result<()> {
    let index = usize::try_from(index).map_err(|_| Error::msg("symbol index overflow"))?;
    if symbols.get(index).and_then(Option::as_ref).is_none() {
        return Err(Error::msg("owned value references a missing symbol"));
    }
    Ok(())
}

include!("structural_wire.rs");
include!("structural_encode.rs");
include!("structural_decode_budget.rs");
include!("structural_decode.rs");
