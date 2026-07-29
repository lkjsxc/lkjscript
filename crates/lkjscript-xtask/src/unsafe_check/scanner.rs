pub(super) fn contains_unsafe_token(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"//") {
            index = skip_line(bytes, index + 2);
        } else if bytes[index..].starts_with(b"/*") {
            index = skip_block(bytes, index + 2);
        } else if let Some(end) = skip_raw_string(bytes, index) {
            index = end;
        } else if matches!(bytes[index], b'b' | b'c') && bytes.get(index + 1) == Some(&b'"') {
            index = skip_string(bytes, index + 2);
        } else if bytes[index] == b'"' {
            index = skip_string(bytes, index + 1);
        } else if bytes[index] == b'\'' {
            index = skip_char_or_lifetime(source, index);
        } else if is_ident_start(bytes[index]) {
            let start = index;
            index += 1;
            while index < bytes.len() && is_ident_continue(bytes[index]) {
                index += 1;
            }
            if &bytes[start..index] == b"unsafe" && !is_raw_identifier(bytes, start) {
                return true;
            }
        } else {
            index += 1;
        }
    }
    false
}

fn skip_line(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index] != b'\n' {
        index += 1;
    }
    index
}

fn skip_block(bytes: &[u8], mut index: usize) -> usize {
    let mut depth = 1usize;
    while index < bytes.len() && depth > 0 {
        if bytes[index..].starts_with(b"/*") {
            depth += 1;
            index += 2;
        } else if bytes[index..].starts_with(b"*/") {
            depth -= 1;
            index += 2;
        } else {
            index += 1;
        }
    }
    index
}

fn skip_string(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = (index + 2).min(bytes.len()),
            b'"' => return index + 1,
            _ => index += 1,
        }
    }
    index
}

fn skip_raw_string(bytes: &[u8], start: usize) -> Option<usize> {
    let mut index = start;
    if matches!(bytes.get(index), Some(b'b') | Some(b'c')) {
        index += 1;
    }
    if bytes.get(index) != Some(&b'r') {
        return None;
    }
    index += 1;
    let hashes = bytes[index..]
        .iter()
        .take_while(|byte| **byte == b'#')
        .count();
    index += hashes;
    if bytes.get(index) != Some(&b'"') {
        return None;
    }
    index += 1;
    while index < bytes.len() {
        if bytes[index] == b'"'
            && bytes
                .get(index + 1..index + 1 + hashes)
                .is_some_and(|ending| ending.iter().all(|byte| *byte == b'#'))
        {
            return Some(index + 1 + hashes);
        }
        index += 1;
    }
    Some(index)
}

fn skip_char_or_lifetime(source: &str, start: usize) -> usize {
    let tail = &source[start + 1..];
    let Some(first) = tail.chars().next() else {
        return start + 1;
    };
    let mut end = start + 1 + first.len_utf8();
    if first == '\\' {
        end = escaped_char_end(source.as_bytes(), end);
    }
    if source.as_bytes().get(end) == Some(&b'\'') {
        end + 1
    } else {
        start + 1
    }
}

fn escaped_char_end(bytes: &[u8], mut index: usize) -> usize {
    if bytes.get(index) == Some(&b'u') && bytes.get(index + 1) == Some(&b'{') {
        index += 2;
        while index < bytes.len() && bytes[index] != b'}' {
            index += 1;
        }
        return (index + 1).min(bytes.len());
    }
    if bytes.get(index) == Some(&b'x') {
        return (index + 3).min(bytes.len());
    }
    (index + 1).min(bytes.len())
}

fn is_raw_identifier(bytes: &[u8], start: usize) -> bool {
    start >= 2 && &bytes[start - 2..start] == b"r#"
}

fn is_ident_start(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphabetic()
}

fn is_ident_continue(byte: u8) -> bool {
    is_ident_start(byte) || byte.is_ascii_digit()
}
