//! Native evaluation metadata and framing, shared by the IR driver and reader.
//! Descriptor v1 is "LKJRD1\n", a scalar tag and width byte, then a u64 trap
//! count and length-prefixed UTF-8 messages. All integers are little-endian.
//! The control stream is the native 24-byte packet followed by "\nSTATUS\n"
//! from its parent. STATUS is the observed process status, not target metadata.
//! f64 results retain their exact bits as f64:0x followed by 16 hex digits.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScalarKind {
    Unit,
    Bool,
    Char,
    Int { bits: u8, signed: bool },
    F64,
}

pub struct Descriptor {
    pub kind: ScalarKind,
    pub traps: Vec<String>,
}

pub struct Outcome {
    pub stdout: String,
    pub stderr: String,
    pub status: u8,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Result<u8, String> {
        if self.at == self.bytes.len() {
            return Err(format!("missing byte at offset {}", self.at));
        }
        let value = self.bytes[self.at];
        self.at += 1;
        Ok(value)
    }

    fn word(&mut self) -> Result<u64, String> {
        if self.bytes.len() - self.at < 8 {
            return Err(format!("incomplete word at offset {}", self.at));
        }
        let mut value = 0u64;
        let mut index = 0usize;
        while index < 8 {
            value |= (self.bytes[self.at + index] as u64) << ((index * 8) as u64);
            index += 1;
        }
        self.at += 8;
        Ok(value)
    }

    fn length(&mut self) -> Result<usize, String> {
        let value = self.word()?;
        let length = value as usize;
        if length as u64 != value {
            return Err("length does not fit the address space".to_string());
        }
        Ok(length)
    }

    fn text(&mut self) -> Result<String, String> {
        let length = self.length()?;
        if length > self.bytes.len() - self.at {
            return Err(format!(
                "string exceeds remaining bytes at offset {}",
                self.at
            ));
        }
        let start = self.at;
        self.at += length;
        match String::from_utf8(self.bytes[start..self.at].to_vec()) {
            Ok(text) => Ok(text),
            Err(_) => Err(format!("invalid UTF-8 at offset {start}")),
        }
    }
}

fn scalar_tag(kind: ScalarKind) -> Result<(u8, u8), String> {
    match kind {
        ScalarKind::Unit => Ok((0u8, 0u8)),
        ScalarKind::Bool => Ok((1u8, 0u8)),
        ScalarKind::Char => Ok((2u8, 0u8)),
        ScalarKind::Int { bits, signed } => {
            if bits != 8u8 && bits != 16u8 && bits != 32u8 && bits != 64u8 {
                return Err("invalid integer width in result descriptor".to_string());
            }
            Ok((if signed { 3u8 } else { 4u8 }, bits))
        }
        ScalarKind::F64 => Ok((5u8, 0u8)),
    }
}

fn scalar_kind(tag: u8, bits: u8) -> Result<ScalarKind, String> {
    let kind = match tag {
        0u8 => ScalarKind::Unit,
        1u8 => ScalarKind::Bool,
        2u8 => ScalarKind::Char,
        3u8 => ScalarKind::Int { bits, signed: true },
        4u8 => ScalarKind::Int {
            bits,
            signed: false,
        },
        5u8 => ScalarKind::F64,
        _ => return Err("invalid scalar tag in result descriptor".to_string()),
    };
    let expected = scalar_tag(kind)?;
    if expected.1 != bits {
        return Err("unexpected scalar width in result descriptor".to_string());
    }
    Ok(kind)
}

fn append_word(bytes: &mut Vec<u8>, mut word: u64) {
    let mut index = 0usize;
    while index < 8 {
        bytes.push(word as u8);
        word >>= 8;
        index += 1;
    }
}

pub fn encode(descriptor: &Descriptor) -> Result<Vec<u8>, String> {
    let tag = scalar_tag(descriptor.kind)?;
    let mut bytes = "LKJRD1\n".as_bytes().to_vec();
    bytes.push(tag.0);
    bytes.push(tag.1);
    append_word(&mut bytes, descriptor.traps.len() as u64);
    for message in &descriptor.traps {
        append_word(&mut bytes, message.len() as u64);
        let source = message.as_bytes();
        let mut index = 0usize;
        while index < source.len() {
            bytes.push(source[index]);
            index += 1;
        }
    }
    Ok(bytes)
}

pub fn decode(bytes: &[u8]) -> Result<Descriptor, String> {
    let magic = "LKJRD1\n".as_bytes();
    if bytes.len() < magic.len() {
        return Err("incomplete result descriptor header".to_string());
    }
    let mut index = 0usize;
    while index < magic.len() {
        if bytes[index] != magic[index] {
            return Err("unsupported result descriptor header".to_string());
        }
        index += 1;
    }
    let mut reader = Reader {
        bytes,
        at: magic.len(),
    };
    let tag = reader.byte()?;
    let bits = reader.byte()?;
    let kind = scalar_kind(tag, bits)?;
    let count = reader.length()?;
    // Even an empty message consumes its eight-byte length.
    if count > (bytes.len() - reader.at) / 8 {
        return Err("trap count exceeds the remaining descriptor".to_string());
    }
    let mut traps: Vec<String> = Vec::new();
    let mut item = 0usize;
    while item < count {
        traps.push(reader.text()?);
        item += 1;
    }
    if reader.at != bytes.len() {
        return Err(format!("trailing bytes at offset {}", reader.at));
    }
    Ok(Descriptor { kind, traps })
}

fn render_scalar(kind: ScalarKind, value: u64) -> Result<String, String> {
    scalar_tag(kind)?;
    match kind {
        ScalarKind::Unit => Ok(String::new()),
        ScalarKind::Bool => {
            if value > 1u64 {
                return Err("invalid bool in native result packet".to_string());
            }
            Ok(format!("{}\n", value != 0u64))
        }
        ScalarKind::Char => {
            if value > 0xffff_ffffu64 {
                return Err("invalid char in native result packet".to_string());
            }
            match char::from_u32(value as u32) {
                Some(value) => Ok(format!("{value}\n")),
                None => Err("invalid char in native result packet".to_string()),
            }
        }
        ScalarKind::Int { bits, signed } => {
            if signed {
                let shift = 64u64 - bits as u64;
                let signed_value = ((value << shift) as i64) >> shift;
                Ok(format!("{signed_value}\n"))
            } else {
                if bits < 64u8 && value >> (bits as u64) != 0u64 {
                    return Err("invalid unsigned integer in native result packet".to_string());
                }
                Ok(format!("{value}\n"))
            }
        }
        ScalarKind::F64 => Ok(format!("f64:0x{value:016x}\n")),
    }
}

fn trap_message(descriptor: &Descriptor, code: u64) -> Result<String, String> {
    let message = match code {
        1u64 => "integer overflow",
        2u64 => "division by zero",
        3u64 => "index out of bounds",
        4u64 => "explicit trap",
        5u64 => "assertion failed",
        6u64 => "heap allocation failed",
        7u64 => "host output failure",
        8u64 => "stack exhausted",
        _ => {
            if code < 16u64 {
                return Err(format!("unknown native trap code {code}"));
            }
            let index = (code - 16u64) as usize;
            if index as u64 != code - 16u64 || index >= descriptor.traps.len() {
                return Err(format!("unknown native trap code {code}"));
            }
            return Ok(descriptor.traps[index].clone());
        }
    };
    Ok(message.to_string())
}

/// Validate the packet against the parent's observed status before rendering.
pub fn evaluate(descriptor: &Descriptor, label: &str, bytes: &[u8]) -> Result<Outcome, String> {
    if bytes.len() < 27 || bytes[24] != 10u8 || bytes[bytes.len() - 1] != 10u8 {
        return Err("expected native packet followed by newline, status and newline".to_string());
    }
    let mut status = 0u64;
    let mut at = 25usize;
    while at < bytes.len() - 1 {
        let digit = bytes[at];
        if digit < 48u8 || digit > 57u8 {
            return Err("invalid process status after native packet".to_string());
        }
        status = status * 10u64 + (digit - 48u8) as u64;
        if status > 255u64 {
            return Err("process status is outside 0..255".to_string());
        }
        at += 1;
    }
    let mut reader = Reader { bytes, at: 0 };
    let code = reader.word()?;
    let value = reader.word()?;
    let last = reader.word()?;
    if code == 0u64 {
        if last > 255u64 || status != last {
            return Err(format!(
                "native exit status {status} disagrees with packet status {last}"
            ));
        }
        Ok(Outcome {
            stdout: render_scalar(descriptor.kind, value)?,
            stderr: String::new(),
            status: status as u8,
        })
    } else {
        if status != 1u64 {
            return Err(format!(
                "native trap code {code} disagrees with exit status {status}"
            ));
        }
        let message = trap_message(descriptor, code)?;
        Ok(Outcome {
            stdout: String::new(),
            stderr: format!("{label}:{last}: R{code:04} {message}\n"),
            status: 1u8,
        })
    }
}
