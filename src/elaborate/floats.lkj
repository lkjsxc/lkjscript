//! Decimal literal conversion uses one exact rational rounding step. This is
//! ordinary producer code; no host float parser or new IR operation is required.
//! Decimal magnitude shortcuts classify values outside binary64's range. They
//! never limit the number of source digits or reject a representable literal.

pub(super) enum Parsed {
    Finite(u64),
    Overflow,
    Invalid,
}

#[derive(Clone)]
struct Natural {
    // Little-endian base 2^32 limbs, with no high zero limbs. Zero is empty.
    limbs: Vec<u32>,
}
impl Natural {
    fn zero() -> Self {
        Self { limbs: Vec::new() }
    }
    fn one() -> Self {
        Self { limbs: vec![1u32] }
    }
    fn multiply_add(&mut self, factor: u32, add: u32) {
        let mut carry = add as u64;
        let mut index = 0;
        while index < self.limbs.len() {
            let value = self.limbs[index] as u64 * factor as u64 + carry;
            self.limbs[index] = value as u32;
            carry = value >> 32;
            index += 1;
        }
        if carry != 0 {
            self.limbs.push(carry as u32);
        }
    }
    fn bits(&self) -> usize {
        if self.limbs.is_empty() {
            return 0;
        }
        let mut high = self.limbs[self.limbs.len() - 1];
        let mut bits = (self.limbs.len() - 1) * 32;
        while high != 0 {
            bits += 1;
            high >>= 1;
        }
        bits
    }
    fn compare(&self, other: &Self) -> i32 {
        if self.limbs.len() < other.limbs.len() {
            return -1;
        }
        if self.limbs.len() > other.limbs.len() {
            return 1;
        }
        let mut index = self.limbs.len();
        while index > 0 {
            index -= 1;
            if self.limbs[index] < other.limbs[index] {
                return -1;
            }
            if self.limbs[index] > other.limbs[index] {
                return 1;
            }
        }
        0
    }
    fn shifted(&self, count: usize) -> Self {
        if self.limbs.is_empty() {
            return Self::zero();
        }
        let mut limbs = Vec::new();
        let mut index = 0;
        while index < count / 32 {
            limbs.push(0u32);
            index += 1;
        }
        let shift = count % 32;
        let mut carry = 0u64;
        for limb in &self.limbs {
            let value = ((*limb as u64) << shift) | carry;
            limbs.push(value as u32);
            carry = value >> 32;
        }
        if carry != 0 {
            limbs.push(carry as u32);
        }
        Self { limbs }
    }
    // The caller compares first, so subtraction is nonnegative.
    fn subtract(&mut self, other: &Self) {
        let mut borrow = 0u64;
        let mut index = 0;
        while index < self.limbs.len() {
            let left = self.limbs[index] as u64;
            let right = if index < other.limbs.len() {
                other.limbs[index] as u64
            } else {
                0
            } + borrow;
            if left >= right {
                self.limbs[index] = (left - right) as u32;
                borrow = 0;
            } else {
                self.limbs[index] = ((1u64 << 32) + left - right) as u32;
                borrow = 1;
            }
            index += 1;
        }
        while !self.limbs.is_empty() {
            let last = self.limbs.len() - 1;
            if self.limbs[last] != 0 {
                break;
            }
            self.limbs.pop();
        }
    }
}

// A magnitude beyond the whole input length plus 400 cannot be cancelled back
// into binary64's decimal range by the mantissa's decimal-point position.
fn exponent_limit(length: usize) -> usize {
    length.checked_add(400).unwrap_or(!0usize)
}
fn signed_sum(
    left_negative: bool,
    left: usize,
    right_negative: bool,
    right: usize,
) -> (bool, usize) {
    if left_negative == right_negative {
        (left_negative, left.checked_add(right).unwrap_or(!0usize))
    } else if left >= right {
        (left_negative, left - right)
    } else {
        (right_negative, right - left)
    }
}

// The scaled rational has a quotient of at most 53 bits. The remainder is kept
// exactly so midpoint ties are resolved from the low significand bit once.
fn rounded_quotient(mut numerator: Natural, denominator: &Natural) -> u64 {
    let numerator_bits = numerator.bits();
    let denominator_bits = denominator.bits();
    let mut shift = if numerator_bits >= denominator_bits {
        numerator_bits - denominator_bits
    } else {
        0
    };
    let mut quotient = 0u64;
    loop {
        let candidate = denominator.shifted(shift);
        if numerator.compare(&candidate) >= 0 {
            numerator.subtract(&candidate);
            quotient |= 1u64 << shift;
        }
        if shift == 0 {
            break;
        }
        shift -= 1;
    }
    let doubled = numerator.shifted(1);
    let comparison = doubled.compare(denominator);
    if comparison > 0 || (comparison == 0 && quotient & 1 != 0) {
        quotient += 1;
    }
    quotient
}

pub(super) fn decimal_bits(text: &str) -> Parsed {
    let bytes = text.as_bytes();
    let mut at = 0;
    let mut negative = false;
    if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') {
        negative = bytes[at] == b'-';
        at += 1;
    }
    let sign = if negative { 1u64 << 63 } else { 0 };
    let mut digits: Vec<u8> = Vec::new();
    let mut fractional = 0usize;
    let mut decimal_point = false;
    let mut saw_digit = false;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte.is_ascii_digit() {
            saw_digit = true;
            if decimal_point {
                fractional += 1;
            }
            if !digits.is_empty() || byte != b'0' {
                digits.push(byte - b'0');
            }
        } else if byte == b'.' && !decimal_point {
            decimal_point = true;
        } else {
            break;
        }
        at += 1;
    }
    if !saw_digit {
        return Parsed::Invalid;
    }
    let mut exponent = 0usize;
    let mut exponent_negative = false;
    if at < bytes.len() && (bytes[at] == b'e' || bytes[at] == b'E') {
        at += 1;
        if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') {
            exponent_negative = bytes[at] == b'-';
            at += 1;
        }
        let start = at;
        let limit = exponent_limit(text.len());
        while at < bytes.len() && bytes[at].is_ascii_digit() {
            let digit = (bytes[at] - b'0') as usize;
            if exponent > limit / 10 || (exponent == limit / 10 && digit > limit % 10) {
                exponent = limit;
            } else {
                exponent = exponent * 10 + digit;
            }
            at += 1;
        }
        if at == start {
            return Parsed::Invalid;
        }
    }
    if at != bytes.len() {
        return Parsed::Invalid;
    }
    if digits.is_empty() {
        return Parsed::Finite(sign);
    }

    let integer_position = digits.len() - 1;
    let (base_negative, base) = if integer_position >= fractional {
        (false, integer_position - fractional)
    } else {
        (true, fractional - integer_position)
    };
    let (order_negative, order) = signed_sum(exponent_negative, exponent, base_negative, base);
    if !order_negative && order > 308 {
        return Parsed::Overflow;
    }
    if order_negative && order > 324 {
        return Parsed::Finite(sign);
    }

    // Trailing zeros affect the power of ten, not the integer significand.
    loop {
        let last = digits.len() - 1;
        if digits[last] != 0 {
            break;
        }
        digits.pop();
    }
    let significant_position = digits.len() - 1;
    let (power_negative, power) = if order_negative {
        (true, significant_position + order)
    } else if significant_position >= order {
        (true, significant_position - order)
    } else {
        (false, order - significant_position)
    };
    let mut numerator = Natural::zero();
    for digit in digits {
        numerator.multiply_add(10, digit as u32);
    }
    let mut denominator = Natural::one();
    let mut index = 0;
    while index < power {
        if power_negative {
            denominator.multiply_add(10, 0);
        } else {
            numerator.multiply_add(10, 0);
        }
        index += 1;
    }

    let numerator_bits = numerator.bits();
    let denominator_bits = denominator.bits();
    let mut binary_exponent: i32 = if numerator_bits >= denominator_bits {
        (numerator_bits - denominator_bits) as i32
    } else {
        -((denominator_bits - numerator_bits) as i32)
    };
    let comparison = if binary_exponent >= 0 {
        let shifted = denominator.shifted(binary_exponent as usize);
        numerator.compare(&shifted)
    } else {
        let shifted = numerator.shifted((-binary_exponent) as usize);
        shifted.compare(&denominator)
    };
    if comparison < 0 {
        binary_exponent -= 1;
    }
    if binary_exponent > 1023 {
        return Parsed::Overflow;
    }
    let normal = binary_exponent >= -1022;
    let shift = if normal { 52 - binary_exponent } else { 1074 };
    let mut significand = if shift >= 0 {
        rounded_quotient(numerator.shifted(shift as usize), &denominator)
    } else {
        let shifted = denominator.shifted((-shift) as usize);
        rounded_quotient(numerator, &shifted)
    };
    if !normal {
        // 2^52 after rounding is exactly the smallest normal encoding.
        return Parsed::Finite(sign | significand);
    }
    if significand == 1u64 << 53 {
        significand >>= 1;
        binary_exponent += 1;
    }
    if binary_exponent > 1023 {
        return Parsed::Overflow;
    }
    let biased = (binary_exponent + 1023) as u64;
    Parsed::Finite(sign | (biased << 52) | (significand - (1u64 << 52)))
}
