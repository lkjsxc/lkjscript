//! Host effect helpers for print and byte IO.

use std::io::{self, Read, Write};

use lkjscript_core::{Error, GcHeap as Arena, HeapObj, Result, Value};

pub fn display_value(arena: &Arena, v: Value) -> Result<String> {
    if v.is_invalid() {
        return Err(Error::msg("invalid VM value escaped initialized storage"));
    }
    if v.is_unit() {
        return Ok("unit".into());
    }
    if v.is_empty_list() {
        return Ok("empty-list".into());
    }
    if let Some(b) = v.as_bool() {
        return Ok(b.to_string());
    }
    if let Some(n) = v.as_small_i64() {
        return Ok(n.to_string());
    }
    if let Some(h) = v.as_handle() {
        return Ok(format!("handle#{h}"));
    }
    match arena.get(v)? {
        HeapObj::Int(number) => Ok(number.to_string()),
        HeapObj::Float(f) => Ok(format!("{f}")),
        HeapObj::Str(s) => Ok(s.clone()),
        HeapObj::Symbol(s) => Ok(s.clone()),
        HeapObj::Pair { car, cdr } => {
            let a = display_value(arena, *car)?;
            let d = display_value(arena, *cdr)?;
            Ok(format!("({a} . {d})"))
        }
        HeapObj::Closure { proto, .. } => Ok(format!("#<fn:{proto}>")),
        HeapObj::Builtin(id) => Ok(format!("#<builtin:{id}>")),
        HeapObj::Buf(b) => Ok(format!("#<buf:{}>", b.len())),
        HeapObj::Product { product, .. } => Ok(format!("#<product:{}>", product.raw())),
        HeapObj::Enum {
            layout,
            physical_tag,
            active_payload,
        } => display_enum(arena, layout.bytes(), *physical_tag, active_payload),
    }
}

fn display_enum(arena: &Arena, layout: [u8; 32], tag: u16, payload: &[Value]) -> Result<String> {
    if layout == lkjscript_core::OPTION_LAYOUT {
        return match (tag, payload) {
            (0, [value]) => Ok(format!("some({})", display_value(arena, *value)?)),
            (1, []) => Ok("none".into()),
            _ => Err(Error::msg("malformed Option value")),
        };
    }
    if layout == lkjscript_core::RESULT_LAYOUT {
        return match (tag, payload) {
            (0, [value]) => Ok(format!("Ok({})", display_value(arena, *value)?)),
            (1, [value]) => Ok(format!("Err({})", display_value(arena, *value)?)),
            _ => Err(Error::msg("malformed Result value")),
        };
    }
    if layout == lkjscript_core::NUMERIC_ERROR_LAYOUT && payload.is_empty() {
        let name = match tag {
            0 => "Fractional",
            1 => "NonFinite",
            2 => "Inexact",
            3 => "OutOfRange",
            _ => return Err(Error::msg("malformed NumericError value")),
        };
        return Ok(format!("NumericError.{name}"));
    }
    if layout == lkjscript_core::UTF8_ERROR_LAYOUT {
        let name = match tag {
            0 => "InvalidLeadingByte",
            1 => "UnexpectedContinuation",
            2 => "Surrogate",
            3 => "OutOfRange",
            4 => "MissingContinuation",
            5 => "OverlongEncoding",
            _ => return Err(Error::msg("malformed Utf8Error value")),
        };
        return match payload {
            [offset] => Ok(format!(
                "Utf8Error.{name}({})",
                display_value(arena, *offset)?
            )),
            _ => Err(Error::msg("malformed Utf8Error payload")),
        };
    }
    if layout == lkjscript_core::SYSTEM_ERROR_LAYOUT {
        let name = match tag {
            0 => "Io",
            1 => "Terminal",
            2 => "Sqlite",
            3 => "Time",
            4 => "Network",
            5 => "Utf8",
            6 => "Unsupported",
            7 => "Random",
            _ => return Err(Error::msg("malformed SystemError value")),
        };
        let mut fields = Vec::with_capacity(payload.len());
        for value in payload {
            fields.push(display_value(arena, *value)?);
        }
        return Ok(format!("SystemError.{name}({})", fields.join(", ")));
    }
    Ok(format!("#<enum:{tag}>"))
}

pub fn write_output(bytes: &[u8], operation: &str) -> Result<()> {
    io::stdout()
        .write_all(bytes)
        .map_err(|error| Error::host(format!("{operation}: {error}")))
}

pub fn flush_out() -> Result<()> {
    io::stdout()
        .flush()
        .map_err(|error| Error::host(format!("flush: {error}")))
}

pub fn read_byte() -> Result<i64> {
    let mut buf = [0u8; 1];
    match io::stdin().read(&mut buf) {
        Ok(0) => Ok(-1),
        Ok(_) => Ok(i64::from(buf[0])),
        Err(error) => Err(Error::host(format!("read-byte: {error}"))),
    }
}

pub fn write_byte(number: i64) -> Result<Value> {
    let byte = u8::try_from(number).map_err(|_| Error::msg("write-byte out of range"))?;
    io::stdout()
        .write_all(&[byte])
        .map_err(|error| Error::host(format!("write-byte: {error}")))?;
    Ok(Value::UNIT)
}

pub fn write_str(arena: &Arena, v: Value) -> Result<Value> {
    let s = crate::host_ext::as_str(arena, v)?;
    io::stdout()
        .write_all(s.as_bytes())
        .map_err(|error| Error::host(format!("write-str: {error}")))?;
    Ok(Value::UNIT)
}
