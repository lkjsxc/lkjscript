//! Checked arithmetic returns an Option through ordinary guarded source code.
use crate::prelude::Option;
use crate::prelude::Option::{None, Some};

pub trait CheckedArithmetic: Copy {
    fn checked_add(self, other: Self) -> Option<Self>;
    fn checked_sub(self, other: Self) -> Option<Self>;
    fn checked_mul(self, other: Self) -> Option<Self>;
}

fn add_unsigned(left: u64, right: u64, maximum: u64) -> Option<u64> {
    if left > maximum - right {
        None
    } else {
        Some(left + right)
    }
}
fn sub_unsigned(left: u64, right: u64) -> Option<u64> {
    if left < right {
        None
    } else {
        Some(left - right)
    }
}
fn mul_unsigned(left: u64, right: u64, maximum: u64) -> Option<u64> {
    if right != 0 && left > maximum / right {
        None
    } else {
        Some(left * right)
    }
}
fn add_signed(left: i64, right: i64, minimum: i64, maximum: i64) -> Option<i64> {
    if (right > 0 && left > maximum - right) || (right < 0 && left < minimum - right) {
        None
    } else {
        Some(left + right)
    }
}
fn sub_signed(left: i64, right: i64, minimum: i64, maximum: i64) -> Option<i64> {
    if (right > 0 && left < minimum + right) || (right < 0 && left > maximum + right) {
        None
    } else {
        Some(left - right)
    }
}
fn mul_signed(left: i64, right: i64, minimum: i64, maximum: i64) -> Option<i64> {
    if left == 0 || right == 0 {
        return Some(0);
    }
    let overflow = if left > 0 {
        if right > 0 {
            left > maximum / right
        } else {
            right < minimum / left
        }
    } else {
        if right > 0 {
            left < minimum / right
        } else {
            left < maximum / right
        }
    };
    if overflow { None } else { Some(left * right) }
}

impl CheckedArithmetic for u8 {
    fn checked_add(self, other: u8) -> Option<u8> {
        match add_unsigned(self as u64, other as u64, 255u64) {
            Some(value) => Some(value as u8),
            None => None,
        }
    }
    fn checked_sub(self, other: u8) -> Option<u8> {
        match sub_unsigned(self as u64, other as u64) {
            Some(value) => Some(value as u8),
            None => None,
        }
    }
    fn checked_mul(self, other: u8) -> Option<u8> {
        match mul_unsigned(self as u64, other as u64, 255u64) {
            Some(value) => Some(value as u8),
            None => None,
        }
    }
}

impl CheckedArithmetic for u16 {
    fn checked_add(self, other: u16) -> Option<u16> {
        match add_unsigned(self as u64, other as u64, 65535u64) {
            Some(value) => Some(value as u16),
            None => None,
        }
    }
    fn checked_sub(self, other: u16) -> Option<u16> {
        match sub_unsigned(self as u64, other as u64) {
            Some(value) => Some(value as u16),
            None => None,
        }
    }
    fn checked_mul(self, other: u16) -> Option<u16> {
        match mul_unsigned(self as u64, other as u64, 65535u64) {
            Some(value) => Some(value as u16),
            None => None,
        }
    }
}

impl CheckedArithmetic for u32 {
    fn checked_add(self, other: u32) -> Option<u32> {
        match add_unsigned(self as u64, other as u64, 4294967295u64) {
            Some(value) => Some(value as u32),
            None => None,
        }
    }
    fn checked_sub(self, other: u32) -> Option<u32> {
        match sub_unsigned(self as u64, other as u64) {
            Some(value) => Some(value as u32),
            None => None,
        }
    }
    fn checked_mul(self, other: u32) -> Option<u32> {
        match mul_unsigned(self as u64, other as u64, 4294967295u64) {
            Some(value) => Some(value as u32),
            None => None,
        }
    }
}

impl CheckedArithmetic for u64 {
    fn checked_add(self, other: u64) -> Option<u64> {
        add_unsigned(self, other, 18446744073709551615u64)
    }
    fn checked_sub(self, other: u64) -> Option<u64> {
        sub_unsigned(self, other)
    }
    fn checked_mul(self, other: u64) -> Option<u64> {
        mul_unsigned(self, other, 18446744073709551615u64)
    }
}

impl CheckedArithmetic for usize {
    fn checked_add(self, other: usize) -> Option<usize> {
        match add_unsigned(self as u64, other as u64, 18446744073709551615u64) {
            Some(value) => Some(value as usize),
            None => None,
        }
    }
    fn checked_sub(self, other: usize) -> Option<usize> {
        match sub_unsigned(self as u64, other as u64) {
            Some(value) => Some(value as usize),
            None => None,
        }
    }
    fn checked_mul(self, other: usize) -> Option<usize> {
        match mul_unsigned(self as u64, other as u64, 18446744073709551615u64) {
            Some(value) => Some(value as usize),
            None => None,
        }
    }
}

impl CheckedArithmetic for i8 {
    fn checked_add(self, other: i8) -> Option<i8> {
        match add_signed(self as i64, other as i64, -128i64, 127i64) {
            Some(value) => Some(value as i8),
            None => None,
        }
    }
    fn checked_sub(self, other: i8) -> Option<i8> {
        match sub_signed(self as i64, other as i64, -128i64, 127i64) {
            Some(value) => Some(value as i8),
            None => None,
        }
    }
    fn checked_mul(self, other: i8) -> Option<i8> {
        match mul_signed(self as i64, other as i64, -128i64, 127i64) {
            Some(value) => Some(value as i8),
            None => None,
        }
    }
}

impl CheckedArithmetic for i16 {
    fn checked_add(self, other: i16) -> Option<i16> {
        match add_signed(self as i64, other as i64, -32768i64, 32767i64) {
            Some(value) => Some(value as i16),
            None => None,
        }
    }
    fn checked_sub(self, other: i16) -> Option<i16> {
        match sub_signed(self as i64, other as i64, -32768i64, 32767i64) {
            Some(value) => Some(value as i16),
            None => None,
        }
    }
    fn checked_mul(self, other: i16) -> Option<i16> {
        match mul_signed(self as i64, other as i64, -32768i64, 32767i64) {
            Some(value) => Some(value as i16),
            None => None,
        }
    }
}

impl CheckedArithmetic for i32 {
    fn checked_add(self, other: i32) -> Option<i32> {
        match add_signed(self as i64, other as i64, -2147483648i64, 2147483647i64) {
            Some(value) => Some(value as i32),
            None => None,
        }
    }
    fn checked_sub(self, other: i32) -> Option<i32> {
        match sub_signed(self as i64, other as i64, -2147483648i64, 2147483647i64) {
            Some(value) => Some(value as i32),
            None => None,
        }
    }
    fn checked_mul(self, other: i32) -> Option<i32> {
        match mul_signed(self as i64, other as i64, -2147483648i64, 2147483647i64) {
            Some(value) => Some(value as i32),
            None => None,
        }
    }
}

impl CheckedArithmetic for i64 {
    fn checked_add(self, other: i64) -> Option<i64> {
        add_signed(self, other, -9223372036854775808i64, 9223372036854775807i64)
    }
    fn checked_sub(self, other: i64) -> Option<i64> {
        sub_signed(self, other, -9223372036854775808i64, 9223372036854775807i64)
    }
    fn checked_mul(self, other: i64) -> Option<i64> {
        mul_signed(self, other, -9223372036854775808i64, 9223372036854775807i64)
    }
}

impl CheckedArithmetic for isize {
    fn checked_add(self, other: isize) -> Option<isize> {
        match add_signed(
            self as i64,
            other as i64,
            -9223372036854775808i64,
            9223372036854775807i64,
        ) {
            Some(value) => Some(value as isize),
            None => None,
        }
    }
    fn checked_sub(self, other: isize) -> Option<isize> {
        match sub_signed(
            self as i64,
            other as i64,
            -9223372036854775808i64,
            9223372036854775807i64,
        ) {
            Some(value) => Some(value as isize),
            None => None,
        }
    }
    fn checked_mul(self, other: isize) -> Option<isize> {
        match mul_signed(
            self as i64,
            other as i64,
            -9223372036854775808i64,
            9223372036854775807i64,
        ) {
            Some(value) => Some(value as isize),
            None => None,
        }
    }
}
