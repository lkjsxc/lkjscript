use std::fmt::Write;

pub(crate) fn append_framed(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    output.extend_from_slice(bytes);
}

pub(crate) fn escape_compact(text: &str) -> String {
    text.replace('%', "%25")
        .replace(';', "%3B")
        .replace('=', "%3D")
        .replace('\n', "%0A")
        .replace('\r', "%0D")
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}
