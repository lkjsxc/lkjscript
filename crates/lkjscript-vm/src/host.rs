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
    if v.is_none() {
        return Ok("none".into());
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
        HeapObj::ResultOk(x) => Ok(format!("Ok({})", display_value(arena, *x)?)),
        HeapObj::ResultErr(x) => Ok(format!("Err({})", display_value(arena, *x)?)),
        HeapObj::OptionSome(x) => Ok(format!("some({})", display_value(arena, *x)?)),
        HeapObj::Product { product, .. } => Ok(format!("#<product:{}>", product.raw())),
        HeapObj::Enum { physical_tag, .. } => Ok(format!("#<enum:{physical_tag}>")),
    }
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
