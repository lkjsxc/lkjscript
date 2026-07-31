//! Host effect helpers for print and byte I/O.

use lkjscript_core::{Error, Result, Value};

pub fn write_output(
    provider: &dyn lkjscript_host::StdioProvider,
    bytes: &[u8],
    operation: &str,
) -> Result<()> {
    provider
        .write(bytes)
        .map_err(|error| Error::host(format!("{operation}: {error}")))
}

pub fn flush_out(provider: &dyn lkjscript_host::StdioProvider) -> Result<()> {
    provider
        .flush()
        .map_err(|error| Error::host(format!("flush: {error}")))
}

pub fn read_byte(provider: &dyn lkjscript_host::StdioProvider) -> Result<i64> {
    provider
        .read_byte()
        .map(|value| value.map_or(-1, i64::from))
        .map_err(|error| Error::host(format!("read-byte: {error}")))
}

pub fn write_byte(provider: &dyn lkjscript_host::StdioProvider, number: i64) -> Result<Value> {
    let byte = u8::try_from(number).map_err(|_| Error::msg("write-byte out of range"))?;
    provider
        .write(&[byte])
        .map_err(|error| Error::host(format!("write-byte: {error}")))?;
    Ok(Value::UNIT)
}

pub fn write_str(provider: &dyn lkjscript_host::StdioProvider, text: &str) -> Result<Value> {
    provider
        .write(text.as_bytes())
        .map_err(|error| Error::host(format!("write-string: {error}")))?;
    Ok(Value::UNIT)
}
