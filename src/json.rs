//! JSON string bytes used by the emitter's trap metadata.
//! Input is validated UTF-8; non-control bytes retain their original encoding.

fn hex_digit(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        10..=15 => b'a' + (value - 10),
        _ => panic!("invalid hexadecimal digit"),
    }
}

/// Append one quoted JSON string, preserving the emitter's lowercase escapes.
pub fn write_string(out: &mut Vec<u8>, text: &str) {
    out.push(b'"');
    let bytes = text.as_bytes();
    let mut index: usize = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b'"' => {
                out.push(b'\\');
                out.push(b'"');
            }
            b'\\' => {
                out.push(b'\\');
                out.push(b'\\');
            }
            0..=31 => {
                out.push(b'\\');
                out.push(b'u');
                out.push(b'0');
                out.push(b'0');
                out.push(hex_digit(byte >> 4));
                out.push(hex_digit(byte & 15));
            }
            _ => out.push(byte),
        }
        index = index + 1;
    }
    out.push(b'"');
}
