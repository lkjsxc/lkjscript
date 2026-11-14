//! Framing for an explicit source bundle; source.rs owns module interpretation.
//! Version 1 is "LKJSB1\n", a root string, a one-byte optional-library tag
//! (0 = absent, 1 = followed by a string), a u64 file count, and path/text pairs.
//! Each string has a little-endian u64 UTF-8 byte length followed by its bytes.
//! No source-size policy belongs here: only framing, UTF-8 and full consumption.
use crate::source::{SourceBundle, SourceInput};

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

    fn length(&mut self) -> Result<usize, String> {
        if self.bytes.len() - self.at < 8 {
            return Err(format!("incomplete length at offset {}", self.at));
        }
        let mut value = 0_u64;
        let mut index = 0usize;
        while index < 8 {
            value |= (self.bytes[self.at + index] as u64) << ((index * 8) as u64);
            index += 1;
        }
        self.at += 8;
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
        let bytes = self.bytes[start..self.at].to_vec();
        match String::from_utf8(bytes) {
            Ok(text) => Ok(text),
            Err(_) => Err(format!("invalid UTF-8 at offset {start}")),
        }
    }
}

pub fn decode(bytes: &[u8]) -> Result<SourceBundle, String> {
    let magic = "LKJSB1\n".as_bytes();
    if bytes.len() < magic.len() {
        return Err("incomplete source bundle header".to_string());
    }
    let mut index = 0;
    while index < magic.len() {
        if bytes[index] != magic[index] {
            return Err("unsupported source bundle header".to_string());
        }
        index += 1;
    }
    let mut reader = Reader {
        bytes,
        at: magic.len(),
    };
    let root = reader.text()?;
    let tag = reader.byte()?;
    let library = if tag == 0u8 {
        None
    } else if tag == 1u8 {
        Some(reader.text()?)
    } else {
        return Err("invalid optional library tag".to_string());
    };
    let count = reader.length()?;
    // Every file needs two length fields, even when both strings are empty.
    if count > (bytes.len() - reader.at) / 16 {
        return Err("file count exceeds the remaining bundle".to_string());
    }
    let mut files: Vec<SourceInput> = Vec::new();
    let mut file = 0;
    while file < count {
        let path = reader.text()?;
        let text = reader.text()?;
        files.push(SourceInput { path, text });
        file += 1;
    }
    if reader.at != bytes.len() {
        return Err(format!("trailing bytes at offset {}", reader.at));
    }
    Ok(SourceBundle {
        root,
        library,
        files,
    })
}
