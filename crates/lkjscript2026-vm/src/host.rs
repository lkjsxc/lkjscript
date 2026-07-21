//! Host effect helpers for print and byte IO.

use std::io::{self, Read, Write};

use lkjscript2026_core::{Error, HeapObj, Result, Value};

use crate::arena::Arena;

pub fn display_value(arena: &Arena, v: Value) -> Result<String> {
    if v.is_nil() {
        return Ok("nil".into());
    }
    if let Some(b) = v.as_bool() {
        return Ok(b.to_string());
    }
    if let Some(n) = v.as_int() {
        return Ok(n.to_string());
    }
    match arena.get(v)? {
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
    }
}

pub fn print_value(arena: &Arena, v: Value) -> Result<()> {
    let s = display_value(arena, v)?;
    print!("{s}");
    Ok(())
}

pub fn flush_out() -> Result<()> {
    io::stdout()
        .flush()
        .map_err(|e| Error::msg(format!("flush: {e}")))
}

pub fn read_byte() -> Result<Value> {
    let mut buf = [0u8; 1];
    match io::stdin().read(&mut buf) {
        Ok(0) => Ok(Value::from_int(-1)),
        Ok(_) => Ok(Value::from_int(buf[0] as i64)),
        Err(e) => Err(Error::msg(format!("read-byte: {e}"))),
    }
}

pub fn write_byte(v: Value) -> Result<Value> {
    let n = v
        .as_int()
        .ok_or_else(|| Error::msg("write-byte expects int"))?;
    let b = (n & 0xff) as u8;
    io::stdout()
        .write_all(&[b])
        .map_err(|e| Error::msg(format!("write-byte: {e}")))?;
    Ok(Value::NIL)
}
