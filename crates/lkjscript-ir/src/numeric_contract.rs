//! Independent host-free oracle for Edition 2 numeric conversions.

#[derive(Clone, Copy)]
pub(super) enum NumericError {
    NonFinite,
    OutOfRange,
    Fractional,
    Inexact,
}

pub(super) const ERROR_ID: [u8; 32] = [
    0xcc, 0xb4, 0x99, 0xa6, 0x02, 0x8f, 0x3f, 0x73, 0xf1, 0x5c, 0xbd, 0xff, 0x96, 0x40, 0x79, 0xbf,
    0x28, 0x54, 0xb0, 0x59, 0x18, 0x49, 0xf2, 0x29, 0x63, 0x90, 0xb7, 0x97, 0x13, 0x57, 0xb6, 0x53,
];
pub(super) const ERROR_LAYOUT: [u8; 32] = [
    0xb8, 0xa9, 0x4c, 0xec, 0xee, 0xaa, 0x22, 0x12, 0xb3, 0x09, 0x14, 0x5f, 0xce, 0x37, 0x7c, 0x28,
    0x3e, 0x5c, 0x64, 0x55, 0xd2, 0xf1, 0x66, 0x1a, 0x2f, 0xe6, 0x90, 0x98, 0xd3, 0x8f, 0xcd, 0x7e,
];
const VARIANTS: [[u8; 32]; 4] = [
    [
        0x2a, 0x61, 0xae, 0xff, 0xbe, 0xe5, 0x48, 0x4c, 0x59, 0x80, 0x9d, 0xbc, 0x5f, 0x1a, 0xe2,
        0x9f, 0xfa, 0xd4, 0xdc, 0xc2, 0xb9, 0xcf, 0xc8, 0xbc, 0x52, 0x49, 0x79, 0x40, 0x49, 0x32,
        0xb6, 0x43,
    ],
    [
        0xa9, 0xfd, 0x52, 0xc5, 0x09, 0xa7, 0xce, 0xab, 0x52, 0x2e, 0xf9, 0x48, 0x28, 0x40, 0x85,
        0xe4, 0xe4, 0x84, 0xa1, 0xa2, 0xfb, 0x88, 0xd3, 0xe4, 0x08, 0xe8, 0x23, 0x6e, 0xea, 0xb7,
        0xa4, 0x70,
    ],
    [
        0x5b, 0x94, 0x60, 0x2d, 0xcd, 0x03, 0xa9, 0xc8, 0xff, 0xe7, 0x5c, 0x9c, 0x2d, 0xad, 0x2b,
        0x05, 0x24, 0x96, 0x01, 0x4b, 0x51, 0x80, 0xbb, 0x89, 0x24, 0x13, 0xc7, 0x1b, 0xce, 0x15,
        0x2b, 0x80,
    ],
    [
        0x67, 0xc7, 0x0b, 0x1f, 0xac, 0xac, 0x4b, 0xf7, 0x6f, 0x34, 0x53, 0xcf, 0x99, 0xcc, 0x58,
        0x27, 0xe7, 0x2f, 0x46, 0x27, 0x9f, 0x22, 0x21, 0x4c, 0x10, 0x4f, 0x3b, 0xf2, 0xcb, 0x3a,
        0xef, 0xf4,
    ],
];

impl NumericError {
    pub(super) const fn variant_id(self) -> [u8; 32] {
        match self {
            Self::NonFinite => VARIANTS[0],
            Self::OutOfRange => VARIANTS[1],
            Self::Fractional => VARIANTS[2],
            Self::Inexact => VARIANTS[3],
        }
    }

    pub(super) const fn physical_tag(self) -> u16 {
        match self {
            Self::NonFinite => 0,
            Self::Fractional => 1,
            Self::Inexact => 2,
            Self::OutOfRange => 3,
        }
    }
}

pub(super) fn f64_from_i64_rounded(value: i64) -> f64 {
    let sign = if value < 0 { 1_u64 << 63 } else { 0 };
    let magnitude = value.unsigned_abs();
    if magnitude == 0 {
        return f64::from_bits(0);
    }
    let mut exponent = 63_u32 - magnitude.leading_zeros();
    let mut significand = if exponent <= 52 {
        magnitude << (52 - exponent)
    } else {
        let shift = exponent - 52;
        let mut high = magnitude >> shift;
        let remainder = magnitude & ((1_u64 << shift) - 1);
        let halfway = 1_u64 << (shift - 1);
        if remainder > halfway || remainder == halfway && high & 1 == 1 {
            high += 1;
        }
        if high == 1_u64 << 53 {
            exponent += 1;
            high >>= 1;
        }
        high
    };
    significand &= (1_u64 << 52) - 1;
    f64::from_bits(sign | (u64::from(exponent + 1023) << 52) | significand)
}

pub(super) fn f64_from_i64_exact(value: i64) -> Result<f64, NumericError> {
    let magnitude = value.unsigned_abs();
    if magnitude != 0 {
        let exponent = 63_u32 - magnitude.leading_zeros();
        if exponent > 52 && magnitude & ((1_u64 << (exponent - 52)) - 1) != 0 {
            return Err(NumericError::Inexact);
        }
    }
    Ok(f64_from_i64_rounded(value))
}

pub(super) fn i64_from_f64_exact(value: f64) -> Result<i64, NumericError> {
    decode_f64(value, true)
}

pub(super) fn i64_from_f64_trunc(value: f64) -> Result<i64, NumericError> {
    decode_f64(value, false)
}

fn decode_f64(value: f64, exact: bool) -> Result<i64, NumericError> {
    let bits = value.to_bits();
    let negative = bits >> 63 != 0;
    let raw_exponent = ((bits >> 52) & 0x7ff) as u16;
    let fraction = bits & ((1_u64 << 52) - 1);
    if raw_exponent == 0x7ff {
        return Err(NumericError::NonFinite);
    }
    if raw_exponent == 0 && fraction == 0 {
        return Ok(0);
    }
    let exponent = if raw_exponent == 0 {
        -1022
    } else {
        i32::from(raw_exponent) - 1023
    };
    let significand = if raw_exponent == 0 {
        fraction
    } else {
        (1_u64 << 52) | fraction
    };
    if exponent < 0 {
        return if exact {
            Err(NumericError::Fractional)
        } else {
            Ok(0)
        };
    }
    if exponent > 63 {
        return Err(NumericError::OutOfRange);
    }
    let magnitude = if exponent >= 52 {
        significand << u32::try_from(exponent - 52).unwrap_or(0)
    } else {
        let shift = u32::try_from(52 - exponent).unwrap_or(0);
        if exact && significand & ((1_u64 << shift) - 1) != 0 {
            return Err(NumericError::Fractional);
        }
        significand >> shift
    };
    if negative {
        if magnitude == 1_u64 << 63 {
            Ok(i64::MIN)
        } else {
            i64::try_from(magnitude)
                .map(|integer| -integer)
                .map_err(|_| NumericError::OutOfRange)
        }
    } else {
        i64::try_from(magnitude).map_err(|_| NumericError::OutOfRange)
    }
}
