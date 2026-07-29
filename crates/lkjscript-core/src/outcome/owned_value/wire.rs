use crate::outcome::codec::{Decoder, Encoder};
use crate::CapabilityKind;

const MAX_WIRE_ITEMS: usize = 262_144;

impl OwnedValue {
    pub(crate) fn encode_wire(&self, out: &mut Encoder) -> Result<()> {
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
        out.usize(self.heap.len())?;
        for object in &self.heap {
            match object {
                Some(object) => {
                    out.u8(1)?;
                    encode_object(out, object)?;
                }
                None => out.u8(0)?,
            }
        }
        out.usize(self.symbols.len())?;
        for symbol in &self.symbols {
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

    pub(crate) fn decode_wire(input: &mut Decoder<'_>) -> Result<Self> {
        match input.u8()? {
            0 => {
                let root = decode_value(input)?;
                let heap_count = count(input.usize()?)?;
                let mut heap = reserve(heap_count)?;
                for _ in 0..heap_count {
                    heap.push(match input.u8()? {
                        0 => None,
                        1 => Some(decode_object(input)?),
                        _ => return Err(Error::msg("unknown owned heap slot tag")),
                    });
                }
                let symbol_count = count(input.usize()?)?;
                let mut symbols = reserve(symbol_count)?;
                for _ in 0..symbol_count {
                    symbols.push(match input.u8()? {
                        0 => None,
                        1 => Some(input.text()?),
                        _ => return Err(Error::msg("unknown owned symbol slot tag")),
                    });
                }
                let mut value = Self::from_vm_snapshot(root, heap)?;
                value.symbols = symbols;
                value.validate_wire_symbols()?;
                Ok(value)
            }
            1 => Self::from_unique_byte_vector(input.bytes()?.to_vec()),
            2 => Self::from_unique_bytes(input.bytes()?.to_vec()),
            _ => Err(Error::msg("unknown owned value tag")),
        }
    }

    fn validate_wire_symbols(&self) -> Result<()> {
        validate_symbol(self.root, &self.symbols)?;
        for object in self.heap.iter().flatten() {
            let mut invalid = false;
            object.trace(&mut |value| {
                invalid |= validate_symbol(value, &self.symbols).is_err();
            });
            if invalid {
                return Err(Error::msg("owned value references a missing symbol"));
            }
        }
        Ok(())
    }
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
    } else if let Some(value) = value.as_capability() {
        out.u8(6)?;
        out.u8(value as u8)
    } else if let Some(value) = value.as_resource() {
        out.u8(7)?;
        out.u32(value)
    } else if let Some(value) = value.as_function() {
        out.u8(8)?;
        out.u32(value)
    } else if let Some(value) = value.as_symbol() {
        out.u8(9)?;
        out.u32(value)
    } else if let Some(value) = value.as_legacy_traced() {
        out.u8(10)?;
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
        6 => Value::from_capability(
            CapabilityKind::from_tag(input.u8()?)
                .ok_or_else(|| Error::msg("unknown capability value tag"))?,
        ),
        7 => Value::from_resource(input.u32()?),
        8 => Value::from_function(input.u32()?),
        9 => Value::from_symbol(input.u32()?),
        10 => Value::from_legacy_traced(input.u32()?),
        _ => return Err(Error::msg("unknown value tag")),
    })
}

include!("wire_objects.rs");
