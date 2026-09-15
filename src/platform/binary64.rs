//! Canonical binary64 representation. Rust equality here observes bits, never program equality.

use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

pub const CANONICAL_NAN_BITS: u64 = 0x7ff8_0000_0000_0000;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Binary64(u64);

impl Binary64 {
    pub fn from_float(value: f64) -> Self {
        Self(if value.is_nan() {
            CANONICAL_NAN_BITS
        } else {
            value.to_bits()
        })
    }

    /// Durable/raw readers reject noncanonical NaNs rather than silently repairing producer bytes.
    pub const fn from_bits(bits: u64) -> Option<Self> {
        let nan = bits & 0x7ff0_0000_0000_0000 == 0x7ff0_0000_0000_0000
            && bits & 0x000f_ffff_ffff_ffff != 0;
        if nan && bits != CANONICAL_NAN_BITS {
            None
        } else {
            Some(Self(bits))
        }
    }

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub const fn to_float(self) -> f64 {
        f64::from_bits(self.0)
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "nan" => Some(Self(CANONICAL_NAN_BITS)),
            "inf" => Some(Self(0x7ff0_0000_0000_0000)),
            "-inf" => Some(Self(0xfff0_0000_0000_0000)),
            _ if is_decimal(text) => text
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(Self::from_float),
            _ => None,
        }
    }

    /// Ryu supplies shortest round-trip finite digits; representation identity never uses text.
    pub fn to_text(self) -> String {
        match self.0 {
            CANONICAL_NAN_BITS => "nan".to_owned(),
            0x7ff0_0000_0000_0000 => "inf".to_owned(),
            0xfff0_0000_0000_0000 => "-inf".to_owned(),
            0x8000_0000_0000_0000 => "-0.0".to_owned(),
            _ => ryu::Buffer::new().format_finite(self.to_float()).to_owned(),
        }
    }
}

/// The JSON decimal grammar, also used for explicitly typed floating literals and text parsing.
pub fn is_decimal(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut offset = usize::from(bytes.first() == Some(&b'-'));
    match bytes.get(offset) {
        Some(b'0') => offset += 1,
        Some(b'1'..=b'9') => {
            offset += 1;
            while bytes.get(offset).is_some_and(u8::is_ascii_digit) {
                offset += 1;
            }
        }
        _ => return false,
    }
    if bytes.get(offset) == Some(&b'.') {
        offset += 1;
        let first = offset;
        while bytes.get(offset).is_some_and(u8::is_ascii_digit) {
            offset += 1;
        }
        if offset == first {
            return false;
        }
    }
    if matches!(bytes.get(offset), Some(b'e' | b'E')) {
        offset += 1;
        if matches!(bytes.get(offset), Some(b'+' | b'-')) {
            offset += 1;
        }
        let first = offset;
        while bytes.get(offset).is_some_and(u8::is_ascii_digit) {
            offset += 1;
        }
        if offset == first {
            return false;
        }
    }
    offset == bytes.len()
}

impl FromStr for Binary64 {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value).ok_or("expected a finite JSON decimal or exactly nan, inf, or -inf")
    }
}

impl fmt::Display for Binary64 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_text())
    }
}

impl Serialize for Binary64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_text())
    }
}

impl<'de> Deserialize<'de> for Binary64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

impl Encode for Binary64 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.0.to_le_bytes().encode(encoder)
    }
}

impl<Context> Decode<Context> for Binary64 {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let bytes = <[u8; 8]>::decode(decoder)?;
        Self::from_bits(u64::from_le_bytes(bytes))
            .ok_or(DecodeError::Other("noncanonical binary64 NaN"))
    }
}

bincode::impl_borrow_decode!(Binary64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary64_representation_is_fixed_little_endian_and_strict() {
        for bits in [
            0,
            0x8000_0000_0000_0000,
            1,
            0x0010_0000_0000_0000,
            0x3ff0_0000_0000_0000,
            0x7fef_ffff_ffff_ffff,
            0x7ff0_0000_0000_0000,
            0xfff0_0000_0000_0000,
            CANONICAL_NAN_BITS,
        ] {
            let value = Binary64::from_bits(bits).unwrap();
            let bytes = bincode::encode_to_vec(value, bincode::config::standard()).unwrap();
            assert_eq!(bytes, bits.to_le_bytes());
            let (decoded, used): (Binary64, usize) =
                bincode::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
            assert_eq!(used, 8);
            assert_eq!(value, decoded);
            assert_eq!(Binary64::parse(&value.to_text()), Some(value));
        }
        for bits in [
            0x7ff0_0000_0000_0001_u64,
            0x7ff8_0000_0000_0001,
            0xfff8_0000_0000_0000,
        ] {
            assert_eq!(Binary64::from_bits(bits), None);
            assert_eq!(
                Binary64::from_float(f64::from_bits(bits)).bits(),
                CANONICAL_NAN_BITS
            );
            assert!(
                bincode::decode_from_slice::<Binary64, _>(
                    &bits.to_le_bytes(),
                    bincode::config::standard()
                )
                .is_err()
            );
        }
        let nan = Binary64::parse("nan").unwrap();
        assert_eq!(nan, nan);
        assert!(!(nan.to_float() == nan.to_float()));
        assert_ne!(Binary64::parse("0"), Binary64::parse("-0"));
    }

    #[test]
    fn binary64_decimal_admission_uses_fixed_rounding_expectations() {
        for (text, bits) in [
            (
                "1.00000000000000011102230246251565404236316680908203125",
                0x3ff0_0000_0000_0000,
            ),
            (
                "1.00000000000000033306690738754696212708950042724609375",
                0x3ff0_0000_0000_0002,
            ),
            ("9007199254740993", 0x4340_0000_0000_0000),
            ("5e-324", 1),
            ("-1e-400", 0x8000_0000_0000_0000),
            ("-0.0", 0x8000_0000_0000_0000),
            ("1.7976931348623157e308", 0x7fef_ffff_ffff_ffff),
        ] {
            assert_eq!(Binary64::parse(text).unwrap().bits(), bits, "{text}");
        }
        for text in [
            "", " 1", "1 ", "+1", "01", "-01", ".1", "1.", "1e", "1e+", "NaN", "-nan", "Infinity",
            "0x1", "1_0", "1,5", "1e400",
        ] {
            assert_eq!(Binary64::parse(text), None, "{text}");
        }
    }
}
