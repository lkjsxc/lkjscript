//! Checked immutable UTF-8 views use the safe std::str::from_utf8 operation.
//! The current error value is an opaque failure marker.
#[derive(Copy, Clone)]
pub struct Utf8Error;

use crate::Option;
use crate::string::String;
// A pattern is a read-only byte matcher with a known UTF-8 extent. Primitive
// str methods and String methods share this source contract.
pub trait Pattern {
    fn byte_len(&self) -> usize;
    fn matches_at(&self, value: &[u8], position: usize) -> bool;
}
fn matches_bytes(value: &[u8], needle: &[u8], position: usize) -> bool {
    if position > value.len() || needle.len() > value.len() - position {
        return false;
    }
    let mut i = 0usize;
    while i < needle.len() {
        if value[position + i] != needle[i] {
            return false;
        }
        i = i + 1;
    }
    true
}
impl Pattern for &str {
    fn byte_len(&self) -> usize {
        self.len()
    }
    fn matches_at(&self, value: &[u8], position: usize) -> bool {
        matches_bytes(value, self.as_bytes(), position)
    }
}
impl Pattern for &String {
    fn byte_len(&self) -> usize {
        self.len()
    }
    fn matches_at(&self, value: &[u8], position: usize) -> bool {
        matches_bytes(value, self.as_bytes(), position)
    }
}
impl Pattern for char {
    fn byte_len(&self) -> usize {
        self.len_utf8()
    }
    fn matches_at(&self, value: &[u8], position: usize) -> bool {
        let mut text = String::new();
        text.push(*self);
        matches_bytes(value, text.as_bytes(), position)
    }
}
pub fn __lkj_starts_with<P: Pattern>(value: &str, pattern: P) -> bool {
    pattern.matches_at(value.as_bytes(), 0usize)
}
pub fn __lkj_ends_with<P: Pattern>(value: &str, pattern: P) -> bool {
    let length = pattern.byte_len();
    if length > value.len() {
        false
    } else {
        pattern.matches_at(value.as_bytes(), value.len() - length)
    }
}
pub fn __lkj_find<P: Pattern>(value: &str, pattern: P) -> Option<usize> {
    let length = pattern.byte_len();
    if length > value.len() {
        return Option::None;
    }
    let mut position = 0usize;
    loop {
        if pattern.matches_at(value.as_bytes(), position) {
            return Option::Some(position);
        }
        if position == value.len() - length {
            break;
        }
        position = position + 1;
    }
    Option::None
}
pub fn __lkj_rfind<P: Pattern>(value: &str, pattern: P) -> Option<usize> {
    let length = pattern.byte_len();
    if length > value.len() {
        return Option::None;
    }
    let mut position = value.len() - length;
    loop {
        if pattern.matches_at(value.as_bytes(), position) {
            return Option::Some(position);
        }
        if position == 0usize {
            break;
        }
        position = position - 1;
    }
    Option::None
}
pub fn __lkj_contains<P: Pattern>(value: &str, pattern: P) -> bool {
    __lkj_find(value, pattern).is_some()
}
pub fn __lkj_is_ascii(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] >= 128u8 {
            return false;
        }
        i = i + 1;
    }
    true
}
fn lower(value: u8) -> u8 {
    if value >= 65u8 && value <= 90u8 {
        value + 32u8
    } else {
        value
    }
}
fn upper(value: u8) -> u8 {
    if value >= 97u8 && value <= 122u8 {
        value - 32u8
    } else {
        value
    }
}
pub fn __lkj_eq_ignore_ascii_case(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let a = left.as_bytes();
    let b = right.as_bytes();
    let mut i = 0usize;
    while i < a.len() {
        if lower(a[i]) != lower(b[i]) {
            return false;
        }
        i = i + 1;
    }
    true
}
pub fn __lkj_to_ascii_lowercase(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut result: Vec<u8> = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        result.push(lower(bytes[i]));
        i = i + 1;
    }
    String::from_utf8(result).unwrap()
}
pub fn __lkj_to_ascii_uppercase(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut result: Vec<u8> = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        result.push(upper(bytes[i]));
        i = i + 1;
    }
    String::from_utf8(result).unwrap()
}
pub fn __lkj_repeat(value: &str, count: usize) -> String {
    let mut result = String::new();
    let mut i = 0usize;
    while i < count {
        result.push_str(value);
        i = i + 1;
    }
    result
}
pub fn __lkj_replace<P: Pattern>(value: &str, pattern: P, replacement: &str) -> String {
    let bytes = value.as_bytes();
    let length = pattern.byte_len();
    let mut result: Vec<u8> = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if length == 0usize {
            if bytes[i] & 0xc0u8 != 0x80u8 {
                crate::bytes::append_bytes(&mut result, replacement.as_bytes());
            }
            result.push(bytes[i]);
            i = i + 1;
        } else if pattern.matches_at(bytes, i) {
            crate::bytes::append_bytes(&mut result, replacement.as_bytes());
            i = i + length;
        } else {
            result.push(bytes[i]);
            i = i + 1;
        }
    }
    if length == 0usize {
        crate::bytes::append_bytes(&mut result, replacement.as_bytes());
    }
    String::from_utf8(result).unwrap()
}

/// UTF-8 equality compares bytes; no allocation or locale affects the result.
pub fn __lkj_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    matches_bytes(left.as_bytes(), right.as_bytes(), 0usize)
}
