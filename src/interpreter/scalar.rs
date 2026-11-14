use crate::ir::{BinaryOp, TypeKind, UnaryOp};

pub(super) enum NumberError {
    Trap(u8),
    Invalid(&'static str),
}

pub(super) fn normalize(value: u64, bits: u8, signed: bool) -> u64 {
    if bits == 64 {
        value
    } else if signed {
        (((value << (64 - bits)) as i64) >> (64 - bits)) as u64
    } else {
        value & ((1u64 << bits) - 1)
    }
}

fn integer(kind: &TypeKind) -> Result<(u8, bool), NumberError> {
    match kind {
        TypeKind::Bool => Ok((1, false)),
        TypeKind::Char => Ok((32, false)),
        TypeKind::Int { bits, signed } if [8, 16, 32, 64].contains(bits) => Ok((*bits, *signed)),
        _ => Err(NumberError::Invalid("operation requires a scalar integer")),
    }
}

fn float(value: f64) -> u64 {
    if value.is_nan() {
        0x7ff8_0000_0000_0000
    } else {
        value.to_bits()
    }
}

pub(super) fn unary(kind: &TypeKind, op: UnaryOp, value: u64) -> Result<u64, NumberError> {
    if matches!(kind, TypeKind::Char) {
        return Err(NumberError::Invalid("invalid character unary operation"));
    }
    if matches!(kind, TypeKind::F64) {
        return match op {
            UnaryOp::Neg => Ok(float(-f64::from_bits(value))),
            _ => Err(NumberError::Invalid("invalid floating unary operation")),
        };
    }
    let (bits, signed) = integer(kind)?;
    match op {
        UnaryOp::Not if matches!(kind, TypeKind::Bool) => Ok(u64::from(value == 0)),
        UnaryOp::Not => Ok(normalize(!value, bits, signed)),
        UnaryOp::Neg if signed => {
            let value = value as i64 as i128;
            if value == -(1i128 << (bits - 1)) {
                Err(NumberError::Trap(1))
            } else {
                Ok((-value) as u64)
            }
        }
        _ => Err(NumberError::Invalid("invalid integer unary operation")),
    }
}

pub(super) fn binary(kind: &TypeKind, op: BinaryOp, a: u64, b: u64) -> Result<u64, NumberError> {
    if matches!(kind, TypeKind::Char)
        && ![
            BinaryOp::Eq,
            BinaryOp::Ne,
            BinaryOp::Lt,
            BinaryOp::Le,
            BinaryOp::Gt,
            BinaryOp::Ge,
        ]
        .contains(&op)
    {
        return Err(NumberError::Invalid("invalid character binary operation"));
    }
    if matches!(kind, TypeKind::F64) {
        let (a, b) = (f64::from_bits(a), f64::from_bits(b));
        return Ok(match op {
            BinaryOp::Add => float(a + b),
            BinaryOp::Sub => float(a - b),
            BinaryOp::Mul => float(a * b),
            BinaryOp::Div => float(a / b),
            BinaryOp::Eq => u64::from(a == b),
            BinaryOp::Ne => u64::from(a != b),
            BinaryOp::Lt => u64::from(a < b),
            BinaryOp::Le => u64::from(a <= b),
            BinaryOp::Gt => u64::from(a > b),
            BinaryOp::Ge => u64::from(a >= b),
            _ => return Err(NumberError::Invalid("invalid floating binary operation")),
        });
    }
    let (bits, signed) = integer(kind)?;
    if matches!(kind, TypeKind::Bool)
        && ![
            BinaryOp::Eq,
            BinaryOp::Ne,
            BinaryOp::BitAnd,
            BinaryOp::BitOr,
            BinaryOp::BitXor,
            BinaryOp::And,
            BinaryOp::Or,
        ]
        .contains(&op)
    {
        return Err(NumberError::Invalid("invalid boolean binary operation"));
    }
    if op == BinaryOp::Eq {
        return Ok(u64::from(a == b));
    }
    if op == BinaryOp::Ne {
        return Ok(u64::from(a != b));
    }
    if [BinaryOp::Lt, BinaryOp::Le, BinaryOp::Gt, BinaryOp::Ge].contains(&op) {
        let order = if signed {
            (a as i64).cmp(&(b as i64))
        } else {
            a.cmp(&b)
        };
        return Ok(u64::from(match op {
            BinaryOp::Lt => order.is_lt(),
            BinaryOp::Le => order.is_le(),
            BinaryOp::Gt => order.is_gt(),
            _ => order.is_ge(),
        }));
    }
    if op == BinaryOp::Shl || op == BinaryOp::Shr {
        if b >= u64::from(bits) {
            return Err(NumberError::Trap(1));
        }
        return Ok(normalize(
            if op == BinaryOp::Shl {
                a << b
            } else if signed {
                ((a as i64) >> b) as u64
            } else {
                a >> b
            },
            bits,
            signed,
        ));
    }
    match op {
        BinaryOp::BitAnd | BinaryOp::And => return Ok(normalize(a & b, bits, signed)),
        BinaryOp::BitOr | BinaryOp::Or => return Ok(normalize(a | b, bits, signed)),
        BinaryOp::BitXor => return Ok(normalize(a ^ b, bits, signed)),
        BinaryOp::WrappingAdd => return Ok(normalize(a.wrapping_add(b), bits, signed)),
        BinaryOp::WrappingSub => return Ok(normalize(a.wrapping_sub(b), bits, signed)),
        BinaryOp::WrappingMul => return Ok(normalize(a.wrapping_mul(b), bits, signed)),
        _ => {}
    }
    if op == BinaryOp::Div || op == BinaryOp::Rem {
        if b == 0 {
            return Err(NumberError::Trap(2));
        }
        if signed {
            let (a, b) = (a as i64 as i128, b as i64 as i128);
            if a == -(1i128 << (bits - 1)) && b == -1 {
                return Err(NumberError::Trap(1));
            }
            return Ok(if op == BinaryOp::Div { a / b } else { a % b } as u64);
        }
        return Ok(if op == BinaryOp::Div { a / b } else { a % b });
    }
    if signed {
        let (a, b) = (a as i64 as i128, b as i64 as i128);
        let value = match op {
            BinaryOp::Add => a + b,
            BinaryOp::Sub => a - b,
            BinaryOp::Mul => a * b,
            _ => return Err(NumberError::Invalid("unknown binary operation")),
        };
        if value < -(1i128 << (bits - 1)) || value > (1i128 << (bits - 1)) - 1 {
            return Err(NumberError::Trap(1));
        }
        Ok(value as u64)
    } else {
        let (a, b) = (u128::from(a), u128::from(b));
        let value = match op {
            BinaryOp::Add => a.checked_add(b),
            BinaryOp::Sub => a.checked_sub(b),
            BinaryOp::Mul => a.checked_mul(b),
            _ => return Err(NumberError::Invalid("unknown binary operation")),
        }
        .ok_or(NumberError::Trap(1))?;
        if value > (1u128 << bits) - 1 {
            return Err(NumberError::Trap(1));
        }
        Ok(value as u64)
    }
}

pub(super) fn cast(from: &TypeKind, to: &TypeKind, value: u64) -> Result<u64, NumberError> {
    match (from, to) {
        (TypeKind::F64, TypeKind::F64) => Ok(value),
        (TypeKind::Int { signed, .. }, TypeKind::F64) => Ok(float(if *signed {
            value as i64 as f64
        } else {
            value as f64
        })),
        (TypeKind::F64, TypeKind::Int { bits, signed }) => {
            let value = f64::from_bits(value);
            let value = if *signed {
                let value = value as i64 as i128;
                value.clamp(-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1) as u64
            } else {
                (value as u64).min(if *bits == 64 {
                    u64::MAX
                } else {
                    (1u64 << bits) - 1
                })
            };
            Ok(value)
        }
        (
            TypeKind::Int {
                bits: 8,
                signed: false,
            }
            | TypeKind::Char,
            TypeKind::Char,
        ) => Ok(value),
        (
            TypeKind::Int { .. } | TypeKind::Bool | TypeKind::Char,
            TypeKind::Int { bits, signed },
        ) => Ok(normalize(value, *bits, *signed)),
        _ => Err(NumberError::Invalid("invalid scalar cast")),
    }
}
