//! Character operations use the checked Unicode scalar contract.
use crate::String;
pub fn __lkjscript_len_utf8(value: char) -> usize {
    let code = value as u32;
    if code < 0x80u32 {
        1
    } else if code < 0x800u32 {
        2
    } else if code < 0x10000u32 {
        3
    } else {
        4
    }
}
pub fn __lkjscript_len_utf16(value: char) -> usize {
    if (value as u32) < 0x10000u32 { 1 } else { 2 }
}
pub fn __lkjscript_is_ascii(value: char) -> bool {
    (value as u32) < 0x80u32
}
pub fn __lkjscript_is_ascii_digit(value: char) -> bool {
    value >= '0' && value <= '9'
}
pub fn __lkjscript_is_ascii_uppercase(value: char) -> bool {
    value >= 'A' && value <= 'Z'
}
pub fn __lkjscript_is_ascii_lowercase(value: char) -> bool {
    value >= 'a' && value <= 'z'
}
pub fn __lkjscript_is_ascii_alphabetic(value: char) -> bool {
    __lkjscript_is_ascii_uppercase(value) || __lkjscript_is_ascii_lowercase(value)
}
pub fn __lkjscript_is_ascii_alphanumeric(value: char) -> bool {
    __lkjscript_is_ascii_alphabetic(value) || __lkjscript_is_ascii_digit(value)
}
pub fn __lkjscript_is_ascii_hexdigit(value: char) -> bool {
    __lkjscript_is_ascii_digit(value) || (value >= 'a' && value <= 'f') || (value >= 'A' && value <= 'F')
}
pub fn __lkjscript_is_ascii_whitespace(value: char) -> bool {
    value == ' ' || value == '\t' || value == '\n' || value == '\r' || value == '\u{c}'
}
pub fn __lkjscript_is_whitespace(value: char) -> bool {
    let code = value as u32;
    (code >= 9u32 && code <= 13u32)
        || code == 32u32
        || code == 0x85u32
        || code == 0xa0u32
        || code == 0x1680u32
        || (code >= 0x2000u32 && code <= 0x200au32)
        || code == 0x2028u32
        || code == 0x2029u32
        || code == 0x202fu32
        || code == 0x205fu32
        || code == 0x3000u32
}
pub fn __lkjscript_to_ascii_lowercase(value: char) -> char {
    if __lkjscript_is_ascii_uppercase(value) {
        ((value as u8) + 32u8) as char
    } else {
        value
    }
}
pub fn __lkjscript_to_ascii_uppercase(value: char) -> char {
    if __lkjscript_is_ascii_lowercase(value) {
        ((value as u8) - 32u8) as char
    } else {
        value
    }
}
pub fn __lkjscript_to_string(value: char) -> String {
    let mut result = String::new();
    result.push(value);
    result
}

pub fn __lkjscript_is_control(value: char) -> bool {
    let code = value as u32;
    code <= 0x1fu32 || (code >= 0x7fu32 && code <= 0x9fu32)
}
pub fn __lkjscript_to_digit(value: char, radix: u32) -> crate::Option<u32> {
    if radix < 2u32 || radix > 36u32 {
        panic!("to_digit: invalid radix");
    }
    let code = value as u32;
    let digit = if code >= 48u32 && code <= 57u32 {
        code - 48u32
    } else if code >= 65u32 && code <= 90u32 {
        code - 65u32 + 10u32
    } else if code >= 97u32 && code <= 122u32 {
        code - 97u32 + 10u32
    } else {
        return crate::Option::None;
    };
    if digit < radix {
        crate::Option::Some(digit)
    } else {
        crate::Option::None
    }
}
