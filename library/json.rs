//! Owned JSON with exact number text. Grammar: https://www.rfc-editor.org/rfc/rfc8259
//! Object member order is retained. Duplicate decoded keys and unpaired UTF-16
//! surrogate escapes are rejected, preserving unambiguous Unicode-scalar data.
//! There are no library length or nesting limits; ordinary allocation/stack rules apply.
use crate::{BTreeSet, CheckedArithmetic, Option, Result, String};
use crate::option::Option::{None, Some};
use crate::result::Result::{Err, Ok};

pub enum Value {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}
#[derive(Clone, PartialEq, Eq)]
pub struct Error {
    // Parsing: an input byte offset, including input.len() for unexpected EOF.
    // Writing: the prospective output byte where an invalid value was found.
    pub offset: usize,
    pub message: String,
}
impl Error {
    fn new(offset: usize, message: &str) -> Error {
        Error { offset, message: message.to_string() }
    }
}
impl crate::fmt::Display for Error {
    fn fmt(&self, output: &mut crate::fmt::Formatter) -> crate::fmt::Result {
        let text = format!("JSON at byte {}: {}", self.offset, self.message);
        output.write_str(text.as_str())
    }
}
impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        if let Value::Object(fields) = self {
            let mut index = 0usize;
            while index < fields.len() {
                if fields[index].0.as_str() == key { return Some(&fields[index].1); }
                index += 1usize;
            }
        }
        None
    }
    pub fn as_str(&self) -> Option<&str> {
        match self { Value::String(text) => Some(text.as_str()), _ => None }
    }
    pub fn as_number(&self) -> Option<&str> {
        match self { Value::Number(text) => Some(text.as_str()), _ => None }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self { Value::Bool(value) => Some(*value), _ => None }
    }
    pub fn as_array(&self) -> Option<&[Value]> {
        match self { Value::Array(values) => Some(values.as_slice()), _ => None }
    }
    pub fn as_object(&self) -> Option<&[(String, Value)]> {
        match self { Value::Object(fields) => Some(fields.as_slice()), _ => None }
    }
    // Only an unsigned decimal integer spelling is accepted, without exponent,
    // fraction, sign, leading zeroes or overflow. No conversion uses floating point.
    pub fn as_u64(&self) -> Option<u64> {
        let text = match self { Value::Number(text) => text.as_bytes(), _ => return None };
        if text.is_empty() || (text.len() > 1usize && text[0usize] == b'0') { return None; }
        let mut value = 0u64;
        for byte in text {
            if !digit(*byte) { return None; }
            value = value.checked_mul(10u64)?.checked_add((*byte - b'0') as u64)?;
        }
        Some(value)
    }
    pub fn as_usize(&self) -> Option<usize> {
        let value = self.as_u64()?;
        if value > (!0usize) as u64 { None } else { Some(value as usize) }
    }
    pub fn from_u64(value: u64) -> Value { Value::Number(format!("{value}")) }
    pub fn from_usize(value: usize) -> Value { Value::Number(format!("{value}")) }
}
fn digit(byte: u8) -> bool { byte >= b'0' && byte <= b'9' }

// Locate malformed UTF-8 before parsing, including bytes outside a string.
// String::from_utf8 still enforces its own checked invariant on decoded strings.
fn invalid_utf8(bytes: &[u8]) -> Option<usize> {
    let mut at = 0usize;
    while at < bytes.len() {
        let first = bytes[at];
        if first < 0x80u8 { at += 1usize; continue; }
        let mut low = 0x80u8;
        let mut high = 0xbfu8;
        let width = if first >= 0xc2u8 && first <= 0xdfu8 {
            2usize
        } else if first >= 0xe0u8 && first <= 0xefu8 {
            if first == 0xe0u8 { low = 0xa0u8; }
            if first == 0xedu8 { high = 0x9fu8; }
            3usize
        } else if first >= 0xf0u8 && first <= 0xf4u8 {
            if first == 0xf0u8 { low = 0x90u8; }
            if first == 0xf4u8 { high = 0x8fu8; }
            4usize
        } else { return Some(at); };
        if bytes.len() - at < width { return Some(at); }
        if bytes[at + 1usize] < low || bytes[at + 1usize] > high {
            return Some(at + 1usize);
        }
        let mut part = 2usize;
        while part < width {
            let byte = bytes[at + part];
            if byte < 0x80u8 || byte > 0xbfu8 { return Some(at + part); }
            part += 1usize;
        }
        at += width;
    }
    None
}
fn number_end(bytes: &[u8], start: usize) -> Result<usize, Error> {
    let mut at = start;
    if at < bytes.len() && bytes[at] == b'-' { at += 1usize; }
    if at == bytes.len() { return Err(Error::new(at, "expected a digit in number")); }
    if bytes[at] == b'0' {
        at += 1usize;
        if at < bytes.len() && digit(bytes[at]) {
            return Err(Error::new(at, "leading zero in number"));
        }
    } else if bytes[at] >= b'1' && bytes[at] <= b'9' {
        while at < bytes.len() && digit(bytes[at]) { at += 1usize; }
    } else { return Err(Error::new(at, "expected a digit in number")); }
    if at < bytes.len() && bytes[at] == b'.' {
        at += 1usize;
        if at == bytes.len() || !digit(bytes[at]) {
            return Err(Error::new(at, "expected a digit after decimal point"));
        }
        while at < bytes.len() && digit(bytes[at]) { at += 1usize; }
    }
    if at < bytes.len() && (bytes[at] == b'e' || bytes[at] == b'E') {
        at += 1usize;
        if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') { at += 1usize; }
        if at == bytes.len() || !digit(bytes[at]) {
            return Err(Error::new(at, "expected a digit in exponent"));
        }
        while at < bytes.len() && digit(bytes[at]) { at += 1usize; }
    }
    Ok(at)
}
fn push_scalar(output: &mut Vec<u8>, code: u32) {
    if code < 0x80u32 {
        output.push(code as u8);
    } else if code < 0x800u32 {
        output.push(((code >> 6u32) | 0xc0u32) as u8);
        output.push(((code & 0x3fu32) | 0x80u32) as u8);
    } else if code < 0x10000u32 {
        output.push(((code >> 12u32) | 0xe0u32) as u8);
        output.push((((code >> 6u32) & 0x3fu32) | 0x80u32) as u8);
        output.push(((code & 0x3fu32) | 0x80u32) as u8);
    } else {
        output.push(((code >> 18u32) | 0xf0u32) as u8);
        output.push((((code >> 12u32) & 0x3fu32) | 0x80u32) as u8);
        output.push((((code >> 6u32) & 0x3fu32) | 0x80u32) as u8);
        output.push(((code & 0x3fu32) | 0x80u32) as u8);
    }
}
struct Parser<'a> { bytes: &'a [u8], at: usize }
impl<'a> Parser<'a> {
    fn whitespace(&mut self) {
        while self.at < self.bytes.len() {
            match self.bytes[self.at] {
                b' ' | b'\t' | b'\r' | b'\n' => self.at += 1usize,
                _ => break,
            }
        }
    }
    fn take(&mut self, expected: u8) -> bool {
        if self.at < self.bytes.len() && self.bytes[self.at] == expected {
            self.at += 1usize;
            true
        } else { false }
    }
    fn literal(&mut self, text: &[u8]) -> Result<(), Error> {
        for byte in text {
            if !self.take(*byte) { return Err(Error::new(self.at, "invalid or unfinished literal")); }
        }
        Ok(())
    }
    fn hex4(&mut self) -> Result<u32, Error> {
        let mut value = 0u32;
        for _part in 0usize..4usize {
            if self.at == self.bytes.len() { return Err(Error::new(self.at, "unfinished Unicode escape")); }
            let byte = self.bytes[self.at];
            let digit = if byte >= b'0' && byte <= b'9' {
                byte - b'0'
            } else if byte >= b'a' && byte <= b'f' {
                byte - b'a' + 10u8
            } else if byte >= b'A' && byte <= b'F' {
                byte - b'A' + 10u8
            } else { return Err(Error::new(self.at, "expected hexadecimal digit in Unicode escape")); };
            self.at += 1usize;
            value = value * 16u32 + digit as u32;
        }
        Ok(value)
    }
    fn string(&mut self) -> Result<String, Error> {
        if !self.take(b'"') { return Err(Error::new(self.at, "expected a string")); }
        let mut output = Vec::new();
        while self.at < self.bytes.len() {
            let at = self.at;
            let byte = self.bytes[self.at];
            self.at += 1usize;
            if byte == b'"' { return Ok(String::from_utf8(output).unwrap()); }
            if byte < 32u8 { return Err(Error::new(at, "unescaped control character in string")); }
            if byte != b'\\' { output.push(byte); continue; }
            if self.at == self.bytes.len() { return Err(Error::new(self.at, "unfinished escape")); }
            let escape = self.bytes[self.at];
            self.at += 1usize;
            match escape {
                b'"' | b'\\' | b'/' => output.push(escape),
                b'b' => output.push(8u8),
                b'f' => output.push(12u8),
                b'n' => output.push(b'\n'),
                b'r' => output.push(b'\r'),
                b't' => output.push(b'\t'),
                b'u' => {
                    let first = self.hex4()?;
                    let code = if first >= 0xd800u32 && first <= 0xdbffu32 {
                        let pair = self.at;
                        if !self.take(b'\\') || !self.take(b'u') {
                            return Err(Error::new(pair, "high surrogate requires a low-surrogate escape"));
                        }
                        let low_at = self.at;
                        let second = self.hex4()?;
                        if second < 0xdc00u32 || second > 0xdfffu32 {
                            return Err(Error::new(low_at, "expected a low surrogate"));
                        }
                        0x10000u32 + (first - 0xd800u32) * 1024u32 + second - 0xdc00u32
                    } else if first >= 0xdc00u32 && first <= 0xdfffu32 {
                        return Err(Error::new(at, "unpaired low surrogate"));
                    } else { first };
                    push_scalar(&mut output, code);
                }
                _ => return Err(Error::new(self.at - 1usize, "invalid string escape")),
            }
        }
        Err(Error::new(self.at, "unterminated string"))
    }
    fn array(&mut self) -> Result<Value, Error> {
        self.at += 1usize;
        self.whitespace();
        let mut values = Vec::new();
        if self.take(b']') { return Ok(Value::Array(values)); }
        loop {
            let value = self.value()?;
            values.push(value);
            self.whitespace();
            if self.take(b']') { return Ok(Value::Array(values)); }
            if !self.take(b',') { return Err(Error::new(self.at, "expected ',' or ']' after array value")); }
            self.whitespace();
        }
    }
    fn object(&mut self) -> Result<Value, Error> {
        self.at += 1usize;
        self.whitespace();
        let mut fields = Vec::new();
        let mut names: BTreeSet<String> = BTreeSet::new();
        if self.take(b'}') { return Ok(Value::Object(fields)); }
        loop {
            let key_at = self.at;
            let key = self.string()?;
            if !names.insert(key.clone()) { return Err(Error::new(key_at, "duplicate object key")); }
            self.whitespace();
            if !self.take(b':') { return Err(Error::new(self.at, "expected ':' after object key")); }
            let value = self.value()?;
            fields.push((key, value));
            self.whitespace();
            if self.take(b'}') { return Ok(Value::Object(fields)); }
            if !self.take(b',') { return Err(Error::new(self.at, "expected ',' or '}' after object value")); }
            self.whitespace();
        }
    }
    fn value(&mut self) -> Result<Value, Error> {
        self.whitespace();
        if self.at == self.bytes.len() { return Err(Error::new(self.at, "expected a JSON value")); }
        match self.bytes[self.at] {
            b'n' => { self.literal(b"null")?; Ok(Value::Null) }
            b't' => { self.literal(b"true")?; Ok(Value::Bool(true)) }
            b'f' => { self.literal(b"false")?; Ok(Value::Bool(false)) }
            b'"' => { let text = self.string()?; Ok(Value::String(text)) }
            b'[' => self.array(),
            b'{' => self.object(),
            byte => {
                if byte != b'-' && !digit(byte) { return Err(Error::new(self.at, "expected a JSON value")); }
                let start = self.at;
                let end = number_end(self.bytes, start)?;
                let text = String::from_utf8(self.bytes[start..end].to_vec()).unwrap();
                self.at = end;
                Ok(Value::Number(text))
            }
        }
    }
}
/// Parse exactly one UTF-8 JSON value. Only JSON's four ASCII whitespace bytes
/// may surround it. Numbers retain their spelling; no range or precision is lost.
pub fn parse(bytes: &[u8]) -> Result<Value, Error> {
    if let Some(at) = invalid_utf8(bytes) { return Err(Error::new(at, "invalid UTF-8")); }
    let mut parser = Parser { bytes, at: 0usize };
    let value = parser.value()?;
    parser.whitespace();
    if parser.at != bytes.len() { return Err(Error::new(parser.at, "trailing input after JSON value")); }
    Ok(value)
}
fn write_string(text: &str, output: &mut Vec<u8>) {
    output.push(b'"');
    for byte in text.as_bytes() {
        match *byte {
            b'"' => output.extend_from_slice(b"\\\""),
            b'\\' => output.extend_from_slice(b"\\\\"),
            8u8 => output.extend_from_slice(b"\\b"),
            12u8 => output.extend_from_slice(b"\\f"),
            b'\n' => output.extend_from_slice(b"\\n"),
            b'\r' => output.extend_from_slice(b"\\r"),
            b'\t' => output.extend_from_slice(b"\\t"),
            value => {
                if value < 32u8 {
                    output.extend_from_slice(b"\\u00");
                    output.push(b"0123456789abcdef"[(value >> 4u8) as usize]);
                    output.push(b"0123456789abcdef"[(value & 15u8) as usize]);
                } else { output.push(value); }
            }
        }
    }
    output.push(b'"');
}
fn write_value(value: &Value, output: &mut Vec<u8>) -> Result<(), Error> {
    match value {
        Value::Null => output.extend_from_slice(b"null"),
        Value::Bool(value) => {
            if *value { output.extend_from_slice(b"true"); }
            else { output.extend_from_slice(b"false"); }
        },
        Value::Number(text) => {
            let start = output.len();
            let end = match number_end(text.as_bytes(), 0usize) {
                Ok(end) => end,
                Err(problem) => return Err(Error {
                    offset: start.checked_add(problem.offset).unwrap_or(!0usize),
                    message: problem.message,
                }),
            };
            if end != text.len() { return Err(Error::new(start.checked_add(end).unwrap_or(!0usize), "invalid character after number")); }
            output.extend_from_slice(text.as_bytes());
        }
        Value::String(text) => write_string(text.as_str(), output),
        Value::Array(values) => {
            output.push(b'[');
            let mut first = true;
            for value in values {
                if !first { output.push(b','); }
                first = false;
                write_value(value, output)?;
            }
            output.push(b']');
        }
        Value::Object(fields) => {
            output.push(b'{');
            let mut names: BTreeSet<String> = BTreeSet::new();
            let mut first = true;
            for field in fields {
                if !names.insert(field.0.clone()) { return Err(Error::new(output.len(), "duplicate object key")); }
                if !first { output.push(b','); }
                first = false;
                write_string(field.0.as_str(), output);
                output.push(b':');
                write_value(&field.1, output)?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}
/// Serialize compact JSON in object insertion order. Invalid direct Number or
/// Object construction returns an error; partial output is never returned.
pub fn stringify(value: &Value) -> Result<String, Error> {
    let mut output = Vec::new();
    write_value(value, &mut output)?;
    Ok(String::from_utf8(output).unwrap())
}
