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
pub fn __lkjscript_starts_with<P: Pattern>(value: &str, pattern: P) -> bool {
    pattern.matches_at(value.as_bytes(), 0usize)
}
pub fn __lkjscript_ends_with<P: Pattern>(value: &str, pattern: P) -> bool {
    let length = pattern.byte_len();
    if length > value.len() {
        false
    } else {
        pattern.matches_at(value.as_bytes(), value.len() - length)
    }
}
pub fn __lkjscript_find<P: Pattern>(value: &str, pattern: P) -> Option<usize> {
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
pub fn __lkjscript_rfind<P: Pattern>(value: &str, pattern: P) -> Option<usize> {
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
pub fn __lkjscript_contains<P: Pattern>(value: &str, pattern: P) -> bool {
    __lkjscript_find(value, pattern).is_some()
}
pub fn __lkjscript_is_ascii(value: &str) -> bool {
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
pub fn __lkjscript_eq_ignore_ascii_case(left: &str, right: &str) -> bool {
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
pub fn __lkjscript_to_ascii_lowercase(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut result: Vec<u8> = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        result.push(lower(bytes[i]));
        i = i + 1;
    }
    String::from_utf8(result).unwrap()
}
pub fn __lkjscript_to_ascii_uppercase(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut result: Vec<u8> = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        result.push(upper(bytes[i]));
        i = i + 1;
    }
    String::from_utf8(result).unwrap()
}
pub fn __lkjscript_repeat(value: &str, count: usize) -> String {
    let mut result = String::new();
    let mut i = 0usize;
    while i < count {
        result.push_str(value);
        i = i + 1;
    }
    result
}
pub fn __lkjscript_replace<P: Pattern>(value: &str, pattern: P, replacement: &str) -> String {
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
pub fn __lkjscript_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    matches_bytes(left.as_bytes(), right.as_bytes(), 0usize)
}

/// Text order is lexicographic UTF-8 byte order.
pub fn __lkjscript_compare(left: &str, right: &str) -> i32 {
    crate::sequence::__lkjscript_slice_order(left.as_bytes(), right.as_bytes())
}

use crate::Result;
use crate::iter::{IntoIterator, Iterator};

/// Text iteration keeps a view of the input and an ordinary byte cursor.
pub struct Bytes<'a> {
    value: &'a [u8],
    index: usize,
}
impl<'a> Iterator for Bytes<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        if self.index == self.value.len() {
            return Option::None;
        }
        let index = self.index;
        self.index += 1;
        Option::Some(self.value[index])
    }
}
impl<'a> IntoIterator for Bytes<'a> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}

pub struct Chars<'a> {
    value: &'a [u8],
    index: usize,
}
impl<'a> Iterator for Chars<'a> {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        if self.index == self.value.len() {
            return Option::None;
        }
        let first = self.value[self.index];
        self.index += 1;
        if first < 128u8 {
            return Option::Some(first as char);
        }
        let mut remaining = if first < 224u8 {
            1usize
        } else if first < 240u8 {
            2usize
        } else {
            3usize
        };
        let mut value = if remaining == 1 {
            (first & 31u8) as u32
        } else if remaining == 2 {
            (first & 15u8) as u32
        } else {
            (first & 7u8) as u32
        };
        while remaining > 0 {
            value = value * 64u32 + (self.value[self.index] & 63u8) as u32;
            self.index += 1;
            remaining -= 1;
        }
        Option::Some(char::from_u32(value).unwrap())
    }
}
impl<'a> IntoIterator for Chars<'a> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}

pub struct Lines<'a> {
    value: &'a str,
    index: usize,
}
impl<'a> Iterator for Lines<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.index == self.value.len() {
            return Option::None;
        }
        let start = self.index;
        let bytes = self.value.as_bytes();
        let mut end = start;
        while end < bytes.len() && bytes[end] != 10u8 {
            end += 1;
        }
        if end < bytes.len() {
            self.index = end + 1;
            if end > start && bytes[end - 1] == 13u8 {
                end -= 1;
            }
        } else {
            self.index = end;
        }
        Option::Some(&self.value[start..end])
    }
}
impl<'a> IntoIterator for Lines<'a> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}

pub struct Split<'a, P> {
    value: &'a str,
    pattern: P,
    start: usize,
    empty_started: bool,
    done: bool,
}
impl<'a, P: Pattern> Iterator for Split<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.done {
            return Option::None;
        }
        let length = self.pattern.byte_len();
        let bytes = self.value.as_bytes();
        let start = self.start;
        if length == 0 {
            if !self.empty_started {
                self.empty_started = true;
                return Option::Some(&self.value[0..0]);
            }
            if start == bytes.len() {
                self.done = true;
                return Option::Some(&self.value[start..start]);
            }
            let mut end = start + 1;
            while end < bytes.len() && bytes[end] & 0xc0u8 == 0x80u8 {
                end += 1;
            }
            self.start = end;
            return Option::Some(&self.value[start..end]);
        }
        let mut position = start;
        while length <= bytes.len() - position {
            if self.pattern.matches_at(bytes, position) {
                self.start = position + length;
                return Option::Some(&self.value[start..position]);
            }
            position += 1;
        }
        self.done = true;
        Option::Some(&self.value[start..])
    }
}
impl<'a, P: Pattern> IntoIterator for Split<'a, P> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}

#[derive(Clone, Copy)]
pub struct ParseIntError;

// Integer parsing separates checked magnitude decoding from value construction.
pub trait ParseInteger {
    fn positive_maximum() -> u64;
    fn is_signed() -> bool;
    fn from_magnitude(value: u64, negative: bool) -> Self;
}

fn decimal_magnitude(
    value: &str,
    maximum: u64,
    signed: bool,
) -> Result<(u64, bool), ParseIntError> {
    let bytes = value.as_bytes();
    if bytes.len() == 0 {
        return Result::Err(ParseIntError);
    }
    let mut index = 0usize;
    let mut negative = false;
    if bytes[0] == 43u8 {
        index = 1;
    } else if bytes[0] == 45u8 {
        if !signed {
            return Result::Err(ParseIntError);
        }
        negative = true;
        index = 1;
    }
    if index == bytes.len() {
        return Result::Err(ParseIntError);
    }
    let limit = if negative { maximum + 1u64 } else { maximum };
    let mut result = 0u64;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte < 48u8 || byte > 57u8 {
            return Result::Err(ParseIntError);
        }
        let digit = (byte - 48u8) as u64;
        if result > (limit - digit) / 10u64 {
            return Result::Err(ParseIntError);
        }
        result = result * 10u64 + digit;
        index += 1;
    }
    Result::Ok((result, negative))
}

pub trait TextView {
    fn bytes<'a>(&'a self) -> Bytes<'a>;
    fn chars<'a>(&'a self) -> Chars<'a>;
    fn lines<'a>(&'a self) -> Lines<'a>;
    fn strip_prefix<'a, P: Pattern>(&'a self, prefix: P) -> Option<&'a str>;
    fn split_once<'a, P: Pattern>(&'a self, pattern: P) -> Option<(&'a str, &'a str)>;
    fn split<'a, P: Pattern>(&'a self, pattern: P) -> Split<'a, P>;
    fn parse<T: ParseInteger>(&self) -> Result<T, ParseIntError>;
}
impl TextView for str {
    fn bytes<'a>(&'a self) -> Bytes<'a> {
        Bytes {
            value: self.as_bytes(),
            index: 0,
        }
    }
    fn chars<'a>(&'a self) -> Chars<'a> {
        Chars {
            value: self.as_bytes(),
            index: 0,
        }
    }
    fn lines<'a>(&'a self) -> Lines<'a> {
        Lines {
            value: self,
            index: 0,
        }
    }
    fn strip_prefix<'a, P: Pattern>(&'a self, prefix: P) -> Option<&'a str> {
        let length = prefix.byte_len();
        if prefix.matches_at(self.as_bytes(), 0usize) {
            Option::Some(&self[length..])
        } else {
            Option::None
        }
    }
    fn split_once<'a, P: Pattern>(&'a self, pattern: P) -> Option<(&'a str, &'a str)> {
        let length = pattern.byte_len();
        match __lkjscript_find(self, pattern) {
            Option::Some(index) => Option::Some((&self[..index], &self[index + length..])),
            Option::None => Option::None,
        }
    }
    fn split<'a, P: Pattern>(&'a self, pattern: P) -> Split<'a, P> {
        Split {
            value: self,
            pattern,
            start: 0,
            empty_started: false,
            done: false,
        }
    }
    fn parse<T: ParseInteger>(&self) -> Result<T, ParseIntError> {
        match decimal_magnitude(self, T::positive_maximum(), T::is_signed()) {
            Result::Ok((value, negative)) => Result::Ok(T::from_magnitude(value, negative)),
            Result::Err(error) => Result::Err(error),
        }
    }
}

impl ParseInteger for u8 {
    fn positive_maximum() -> u64 {
        255u64
    }
    fn is_signed() -> bool {
        false
    }
    fn from_magnitude(value: u64, negative: bool) -> u8 {
        let _ = negative;
        value as u8
    }
}

impl ParseInteger for u16 {
    fn positive_maximum() -> u64 {
        65535u64
    }
    fn is_signed() -> bool {
        false
    }
    fn from_magnitude(value: u64, negative: bool) -> u16 {
        let _ = negative;
        value as u16
    }
}

impl ParseInteger for u32 {
    fn positive_maximum() -> u64 {
        4294967295u64
    }
    fn is_signed() -> bool {
        false
    }
    fn from_magnitude(value: u64, negative: bool) -> u32 {
        let _ = negative;
        value as u32
    }
}

impl ParseInteger for u64 {
    fn positive_maximum() -> u64 {
        18446744073709551615u64
    }
    fn is_signed() -> bool {
        false
    }
    fn from_magnitude(value: u64, negative: bool) -> u64 {
        let _ = negative;
        value
    }
}

impl ParseInteger for usize {
    fn positive_maximum() -> u64 {
        18446744073709551615u64
    }
    fn is_signed() -> bool {
        false
    }
    fn from_magnitude(value: u64, negative: bool) -> usize {
        let _ = negative;
        value as usize
    }
}

impl ParseInteger for i8 {
    fn positive_maximum() -> u64 {
        127u64
    }
    fn is_signed() -> bool {
        true
    }
    fn from_magnitude(value: u64, negative: bool) -> i8 {
        let value = if negative {
            0i64.wrapping_sub(value as i64)
        } else {
            value as i64
        };
        value as i8
    }
}

impl ParseInteger for i16 {
    fn positive_maximum() -> u64 {
        32767u64
    }
    fn is_signed() -> bool {
        true
    }
    fn from_magnitude(value: u64, negative: bool) -> i16 {
        let value = if negative {
            0i64.wrapping_sub(value as i64)
        } else {
            value as i64
        };
        value as i16
    }
}

impl ParseInteger for i32 {
    fn positive_maximum() -> u64 {
        2147483647u64
    }
    fn is_signed() -> bool {
        true
    }
    fn from_magnitude(value: u64, negative: bool) -> i32 {
        let value = if negative {
            0i64.wrapping_sub(value as i64)
        } else {
            value as i64
        };
        value as i32
    }
}

impl ParseInteger for i64 {
    fn positive_maximum() -> u64 {
        9223372036854775807u64
    }
    fn is_signed() -> bool {
        true
    }
    fn from_magnitude(value: u64, negative: bool) -> i64 {
        if negative {
            0i64.wrapping_sub(value as i64)
        } else {
            value as i64
        }
    }
}

impl ParseInteger for isize {
    fn positive_maximum() -> u64 {
        9223372036854775807u64
    }
    fn is_signed() -> bool {
        true
    }
    fn from_magnitude(value: u64, negative: bool) -> isize {
        let value = if negative {
            0i64.wrapping_sub(value as i64)
        } else {
            value as i64
        };
        value as isize
    }
}
