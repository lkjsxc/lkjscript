//! Host-independent Edition 2 I64/F64 conversions.

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericError {
    NonFinite,
    OutOfRange,
    Fractional,
    Inexact,
}

impl NumericError {
    pub const fn name(self) -> &'static str {
        match self {
            Self::NonFinite => "NonFinite",
            Self::OutOfRange => "OutOfRange",
            Self::Fractional => "Fractional",
            Self::Inexact => "Inexact",
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::NonFinite => 0,
            Self::OutOfRange => 1,
            Self::Fractional => 2,
            Self::Inexact => 3,
        }
    }

    pub const fn variant_id(self) -> [u8; 32] {
        crate::numeric_variant(self.index())
    }

    pub const fn physical_tag(self) -> u16 {
        match self {
            Self::NonFinite => 0,
            Self::OutOfRange => 3,
            Self::Fractional => 1,
            Self::Inexact => 2,
        }
    }
}

pub fn f64_from_i64_rounded(value: i64) -> f64 {
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

pub fn f64_from_i64_exact(value: i64) -> Result<f64, NumericError> {
    let magnitude = value.unsigned_abs();
    if magnitude != 0 {
        let exponent = 63_u32 - magnitude.leading_zeros();
        if exponent > 52 && magnitude & ((1_u64 << (exponent - 52)) - 1) != 0 {
            return Err(NumericError::Inexact);
        }
    }
    Ok(f64_from_i64_rounded(value))
}

pub fn i64_from_f64_exact(value: f64) -> Result<i64, NumericError> {
    decode_f64(value, true)
}

pub fn i64_from_f64_trunc(value: f64) -> Result<i64, NumericError> {
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
