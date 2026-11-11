//! Byte operations used by readers. Every extent is a usize; there is no recursion.
pub fn starts_with(bytes: &Vec<u8>, prefix: &Vec<u8>) -> bool {
    if prefix.len() > bytes.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < prefix.len() {
        if bytes[i] != prefix[i] {
            return false;
        }
        i = i + 1;
    }
    true
}

pub fn find_byte(bytes: &Vec<u8>, needle: u8) -> Option<usize> {
    let mut i: usize = 0;
    while i < bytes.len() {
        if bytes[i] == needle {
            return Option::Some(i);
        }
        i = i + 1;
    }
    Option::None
}

pub fn append_bytes(target: &mut Vec<u8>, source: &Vec<u8>) {
    let mut i: usize = 0;
    while i < source.len() {
        target.push(source[i]);
        i = i + 1;
    }
}

fn continuation(byte: u8) -> bool {
    byte >= 128 && byte < 192
}

pub fn valid_utf8(bytes: &Vec<u8>) -> bool {
    let mut i: usize = 0;
    while i < bytes.len() {
        let first = bytes[i];
        if first < 128 {
            i = i + 1;
        } else if first >= 194 && first <= 223 {
            if bytes.len() - i < 2 {
                return false;
            }
            if !continuation(bytes[i + 1]) {
                return false;
            }
            i = i + 2;
        } else if first >= 224 && first <= 239 {
            if bytes.len() - i < 3 {
                return false;
            }
            let second = bytes[i + 1];
            if !continuation(second) || !continuation(bytes[i + 2]) {
                return false;
            }
            if first == 224 && second < 160 {
                return false;
            }
            if first == 237 && second >= 160 {
                return false;
            }
            i = i + 3;
        } else if first >= 240 && first <= 244 {
            if bytes.len() - i < 4 {
                return false;
            }
            let second = bytes[i + 1];
            if !continuation(second) || !continuation(bytes[i + 2]) || !continuation(bytes[i + 3]) {
                return false;
            }
            if first == 240 && second < 144 {
                return false;
            }
            if first == 244 && second >= 144 {
                return false;
            }
            i = i + 4;
        } else {
            return false;
        }
    }
    true
}
