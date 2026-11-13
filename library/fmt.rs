//! Formatting is source code over String appends. Macros select these declared
//! trait contracts and never implement numeric or character encoding themselves.
use crate::{Option, String};
pub struct Error;
pub type Result = crate::Result<(), Error>;
pub struct Formatter<'a> {
    output: &'a mut String,
    width: usize,
    flags: u8,
    fill: char,
}
pub trait Display {
    fn fmt(&self, formatter: &mut Formatter) -> Result;
}
pub trait LowerHex {
    fn fmt(&self, formatter: &mut Formatter) -> Result;
}
pub trait UpperHex {
    fn fmt(&self, formatter: &mut Formatter) -> Result;
}
pub trait Binary {
    fn fmt(&self, formatter: &mut Formatter) -> Result;
}
pub trait Octal {
    fn fmt(&self, formatter: &mut Formatter) -> Result;
}
pub trait Write {
    fn write_str(&mut self, value: &str) -> Result;
}
impl Write for String {
    fn write_str(&mut self, value: &str) -> Result {
        self.push_str(value);
        crate::Result::Ok(())
    }
}
impl Formatter<'_> {
    pub fn write_str(&mut self, value: &str) -> Result {
        self.output.push_str(value);
        crate::Result::Ok(())
    }
    pub fn write_char(&mut self, value: char) -> Result {
        self.output.push(value);
        crate::Result::Ok(())
    }
    pub fn width(&self) -> Option<usize> {
        if self.flags & 32u8 == 0 {
            Option::None
        } else {
            Option::Some(self.width)
        }
    }
    pub fn sign_plus(&self) -> bool {
        self.flags & 1u8 != 0
    }
    pub fn alternate(&self) -> bool {
        self.flags & 2u8 != 0
    }
    pub fn sign_aware_zero_pad(&self) -> bool {
        self.flags & 4u8 != 0
    }
    pub fn fill(&self) -> char {
        self.fill
    }
    fn padding(&mut self, count: usize) {
        let mut i = 0usize;
        while i < count {
            self.output.push(self.fill);
            i = i + 1;
        }
    }
    pub fn pad(&mut self, value: &str) -> Result {
        let bytes = value.as_bytes();
        let mut length = 0usize;
        let mut i = 0usize;
        while i < bytes.len() {
            if bytes[i] & 0xc0u8 != 0x80u8 {
                length = length + 1;
            }
            i = i + 1;
        }
        let padding = if self.width > length {
            self.width - length
        } else {
            0usize
        };
        let align = self.flags & 24u8;
        let before = if align == 16u8 {
            padding
        } else if align == 24u8 {
            padding / 2usize
        } else {
            0usize
        };
        self.padding(before);
        self.output.push_str(value);
        self.padding(padding - before);
        crate::Result::Ok(())
    }
    fn number(&mut self, value: u64, negative: bool, base: u64, upper: bool) -> Result {
        let mut digits: Vec<u8> = Vec::new();
        let mut remaining = value;
        loop {
            let digit = (remaining % base) as u8;
            digits.push(if digit < 10u8 {
                digit + 48u8
            } else if upper {
                digit - 10u8 + 65u8
            } else {
                digit - 10u8 + 97u8
            });
            remaining = remaining / base;
            if remaining == 0u64 {
                break;
            }
        }
        let mut prefix = String::new();
        if negative {
            prefix.push('-');
        } else if self.sign_plus() {
            prefix.push('+');
        }
        if self.alternate() {
            if base == 16u64 {
                prefix.push_str("0x");
            } else if base == 2u64 {
                prefix.push_str("0b");
            } else if base == 8u64 {
                prefix.push_str("0o");
            }
        }
        let length = prefix.len() + digits.len();
        let padding = if self.width > length {
            self.width - length
        } else {
            0usize
        };
        let zero = self.sign_aware_zero_pad();
        let align = self.flags & 24u8;
        let before = if zero || align == 8u8 {
            0usize
        } else if align == 24u8 {
            padding / 2usize
        } else {
            padding
        };
        self.padding(before);
        self.output.push_str(prefix.as_str());
        if zero {
            let mut i = 0usize;
            while i < padding {
                self.output.push('0');
                i = i + 1;
            }
        }
        let mut i = digits.len();
        while i > 0 {
            i = i - 1;
            self.output.push(digits[i] as char);
        }
        if !zero {
            self.padding(padding - before);
        }
        crate::Result::Ok(())
    }
}
impl Write for Formatter<'_> {
    fn write_str(&mut self, value: &str) -> Result {
        self.output.push_str(value);
        crate::Result::Ok(())
    }
}
impl Display for String {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.pad(self.as_str())
    }
}
impl Display for str {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.pad(self)
    }
}
impl Display for bool {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.pad(if *self { "true" } else { "false" })
    }
}
impl Display for char {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut value = String::new();
        value.push(*self);
        f.pad(value.as_str())
    }
}
pub fn __lkj_new<'a>(output: &'a mut String) -> Formatter<'a> {
    Formatter {
        output,
        width: 0usize,
        flags: 0u8,
        fill: ' ',
    }
}
pub fn __lkj_nested<'a, 'b>(outer: &'a mut Formatter<'b>) -> Formatter<'a> {
    Formatter {
        output: &mut *outer.output,
        width: 0usize,
        flags: 0u8,
        fill: ' ',
    }
}
pub fn __lkj_settings(formatter: &mut Formatter, width: usize, flags: u8, fill: char) {
    formatter.width = width;
    formatter.flags = flags;
    formatter.fill = fill;
}
pub fn __lkj_literal(formatter: &mut Formatter, value: &str) -> Result {
    formatter.write_str(value)
}
pub fn __lkj_display<T: Display>(formatter: &mut Formatter, value: &T) -> Result {
    value.fmt(formatter)
}
pub fn __lkj_display_str(formatter: &mut Formatter, value: &str) -> Result {
    formatter.pad(value)
}
pub fn __lkj_lower_hex<T: LowerHex>(formatter: &mut Formatter, value: &T) -> Result {
    value.fmt(formatter)
}
pub fn __lkj_upper_hex<T: UpperHex>(formatter: &mut Formatter, value: &T) -> Result {
    value.fmt(formatter)
}
pub fn __lkj_binary<T: Binary>(formatter: &mut Formatter, value: &T) -> Result {
    value.fmt(formatter)
}
pub fn __lkj_octal<T: Octal>(formatter: &mut Formatter, value: &T) -> Result {
    value.fmt(formatter)
}

impl Display for u8 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 10u64, false)
    }
}
impl LowerHex for u8 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 16u64, false)
    }
}
impl UpperHex for u8 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 16u64, true)
    }
}
impl Binary for u8 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 2u64, false)
    }
}
impl Octal for u8 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 8u64, false)
    }
}

impl Display for u16 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 10u64, false)
    }
}
impl LowerHex for u16 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 16u64, false)
    }
}
impl UpperHex for u16 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 16u64, true)
    }
}
impl Binary for u16 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 2u64, false)
    }
}
impl Octal for u16 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 8u64, false)
    }
}

impl Display for u32 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 10u64, false)
    }
}
impl LowerHex for u32 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 16u64, false)
    }
}
impl UpperHex for u32 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 16u64, true)
    }
}
impl Binary for u32 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 2u64, false)
    }
}
impl Octal for u32 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 8u64, false)
    }
}

impl Display for u64 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 10u64, false)
    }
}
impl LowerHex for u64 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 16u64, false)
    }
}
impl UpperHex for u64 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 16u64, true)
    }
}
impl Binary for u64 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 2u64, false)
    }
}
impl Octal for u64 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 8u64, false)
    }
}

impl Display for usize {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 10u64, false)
    }
}
impl LowerHex for usize {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 16u64, false)
    }
}
impl UpperHex for usize {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 16u64, true)
    }
}
impl Binary for usize {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 2u64, false)
    }
}
impl Octal for usize {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 8u64, false)
    }
}

impl Display for i8 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let value = *self as i64;
        let negative = value < 0i64;
        let magnitude = if negative {
            (-(value + 1i64)) as u64 + 1u64
        } else {
            value as u64
        };
        f.number(magnitude, negative, 10u64, false)
    }
}
impl LowerHex for i8 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number((*self as u8) as u64, false, 16u64, false)
    }
}
impl UpperHex for i8 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number((*self as u8) as u64, false, 16u64, true)
    }
}
impl Binary for i8 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number((*self as u8) as u64, false, 2u64, false)
    }
}
impl Octal for i8 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number((*self as u8) as u64, false, 8u64, false)
    }
}

impl Display for i16 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let value = *self as i64;
        let negative = value < 0i64;
        let magnitude = if negative {
            (-(value + 1i64)) as u64 + 1u64
        } else {
            value as u64
        };
        f.number(magnitude, negative, 10u64, false)
    }
}
impl LowerHex for i16 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number((*self as u16) as u64, false, 16u64, false)
    }
}
impl UpperHex for i16 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number((*self as u16) as u64, false, 16u64, true)
    }
}
impl Binary for i16 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number((*self as u16) as u64, false, 2u64, false)
    }
}
impl Octal for i16 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number((*self as u16) as u64, false, 8u64, false)
    }
}

impl Display for i32 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let value = *self as i64;
        let negative = value < 0i64;
        let magnitude = if negative {
            (-(value + 1i64)) as u64 + 1u64
        } else {
            value as u64
        };
        f.number(magnitude, negative, 10u64, false)
    }
}
impl LowerHex for i32 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number((*self as u32) as u64, false, 16u64, false)
    }
}
impl UpperHex for i32 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number((*self as u32) as u64, false, 16u64, true)
    }
}
impl Binary for i32 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number((*self as u32) as u64, false, 2u64, false)
    }
}
impl Octal for i32 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number((*self as u32) as u64, false, 8u64, false)
    }
}

impl Display for i64 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let value = *self as i64;
        let negative = value < 0i64;
        let magnitude = if negative {
            (-(value + 1i64)) as u64 + 1u64
        } else {
            value as u64
        };
        f.number(magnitude, negative, 10u64, false)
    }
}
impl LowerHex for i64 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 16u64, false)
    }
}
impl UpperHex for i64 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 16u64, true)
    }
}
impl Binary for i64 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 2u64, false)
    }
}
impl Octal for i64 {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 8u64, false)
    }
}

impl Display for isize {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let value = *self as i64;
        let negative = value < 0i64;
        let magnitude = if negative {
            (-(value + 1i64)) as u64 + 1u64
        } else {
            value as u64
        };
        f.number(magnitude, negative, 10u64, false)
    }
}
impl LowerHex for isize {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 16u64, false)
    }
}
impl UpperHex for isize {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 16u64, true)
    }
}
impl Binary for isize {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 2u64, false)
    }
}
impl Octal for isize {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.number(*self as u64, false, 8u64, false)
    }
}

pub fn __lkj_success() -> Result {
    crate::Result::Ok(())
}
