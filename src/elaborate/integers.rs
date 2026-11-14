//! Integer constants share one typed validation path in expressions and patterns.
use super::{Builder, Result, a, err, k};
use std::collections::BTreeMap;

fn integer_suffix(index: usize) -> &'static str {
    match index {
        0 => "usize",
        1 => "isize",
        2 => "u64",
        3 => "i64",
        4 => "u32",
        5 => "i32",
        6 => "u16",
        7 => "i16",
        8 => "u8",
        9 => "i8",
        _ => unreachable!(),
    }
}

pub(super) fn magnitude(text: &str, radix: u64) -> Option<u64> {
    let bytes = text.as_bytes();
    let mut index = if bytes.first() == Some(&b'+') { 1 } else { 0 };
    if index == bytes.len() {
        return None;
    }
    let mut value = 0_u64;
    while index < bytes.len() {
        let byte = bytes[index];
        let digit = if byte >= b'0' && byte <= b'9' {
            (byte - b'0') as u64
        } else if byte >= b'a' && byte <= b'f' {
            (byte - b'a' + 10) as u64
        } else if byte >= b'A' && byte <= b'F' {
            (byte - b'A' + 10) as u64
        } else {
            return None;
        };
        if digit >= radix || value > (!0_u64 - digit) / radix {
            return None;
        }
        value = value * radix + digit;
        index += 1;
    }
    Some(value)
}

pub(super) fn suffix_type(names: &BTreeMap<String, usize>, text: &str) -> Option<usize> {
    for index in 0..10 {
        let suffix = integer_suffix(index);
        if text.ends_with(suffix) {
            return names.get(&suffix.to_string()).copied();
        }
    }
    None
}

pub(super) fn literal(
    types: &[k::Type],
    names: &BTreeMap<String, usize>,
    text: &str,
    expected: Option<usize>,
    negative: bool,
    span: a::Span,
) -> Result<(i64, usize)> {
    let mut digits = text.replace('_', "");
    let mut ty = expected.unwrap_or(k::I32);
    for index in 0..10 {
        let suffix = integer_suffix(index);
        if digits.ends_with(suffix) {
            ty = *names
                .get(&suffix.to_string())
                .ok_or_else(|| err(span, "E_TYPE", "unknown integer suffix"))?;
            let shortened = digits[..digits.len() - suffix.len()].to_string();
            digits = shortened;
            if expected.is_some_and(|expected| expected != ty) {
                return Err(err(
                    span,
                    "E_TYPE",
                    "integer literal suffix differs from the expected type",
                ));
            }
            break;
        }
    }
    let radix = if digits.starts_with("0x") {
        16
    } else if digits.starts_with("0b") {
        2
    } else if digits.starts_with("0o") {
        8
    } else {
        10
    };
    let magnitude = magnitude(
        if radix == 10 {
            digits.as_str()
        } else {
            &digits[2..]
        },
        radix,
    )
    .ok_or_else(|| err(span, "E_TYPE", "integer literal exceeds its representation"))?;
    let (bits, signed) = match types[ty].kind {
        k::TypeKind::Int { bits, signed } => (bits, signed),
        _ => {
            return Err(err(
                span,
                "E_TYPE",
                "integer literal requires an integer type",
            ));
        }
    };
    if negative && !signed {
        return Err(err(span, "E_TYPE", "unsigned integers cannot be negated"));
    }
    let maximum = if signed {
        if negative {
            1_u64 << (bits - 1)
        } else {
            (1_u64 << (bits - 1)) - 1
        }
    } else if bits == 64 {
        !0_u64
    } else {
        (1_u64 << bits) - 1
    };
    if magnitude > maximum {
        return Err(err(span, "E_TYPE", "integer literal does not fit its type"));
    }
    let value = if negative {
        0_u64.wrapping_sub(magnitude) as i64
    } else {
        magnitude as i64
    };
    Ok((value, ty))
}

impl Builder<'_, '_> {
    pub(super) fn wrapping_method(
        &mut self,
        place: &k::Place,
        ty: usize,
        name: &str,
        types: &[usize],
        args: &[usize],
        span: a::Span,
    ) -> Result<Option<(usize, usize)>> {
        if super::methods::direct(
            super::methods::receiver(&self.compiler.types[ty].kind),
            name,
        ) != Some(super::methods::Direct::Wrapping)
        {
            return Ok(None);
        }
        if args.len() != 1 || !types.is_empty() {
            return Err(err(
                span,
                "E_TYPE",
                "wrapping arithmetic takes one value of the same integer type",
            ));
        }
        let left = self.read(place.clone(), span.line)?.0;
        let right = self.expr(args[0], Some(ty))?.0;
        let dst = self.temp(ty);
        self.emit(
            k::Op::Binary {
                dst,
                op: name.to_string(),
                left,
                right,
            },
            span.line,
        );
        Ok(Some((dst, ty)))
    }
}

impl Builder<'_, '_> {
    pub(super) fn primitive_value_type(&self, ty: usize) -> usize {
        match self.compiler.types[ty].kind {
            k::TypeKind::Ref {
                inner,
                mutable: false,
            } if matches!(
                self.compiler.types[inner].kind,
                k::TypeKind::Bool | k::TypeKind::Int { .. } | k::TypeKind::F64
            ) =>
            {
                inner
            }
            _ => ty,
        }
    }

    pub(super) fn primitive_operand(
        &mut self,
        expression: usize,
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let inferred = self.inferred.expressions.get(&expression).copied();
        let inferred = match inferred {
            Some(ty) => Some(ty),
            None => self.hint(expression),
        };
        let borrowed = inferred.is_some_and(|ty| self.primitive_value_type(ty) != ty);
        let value = self.expr(expression, if borrowed { None } else { expected })?;
        if self.primitive_value_type(value.1) != value.1 {
            self.read(
                k::Place {
                    local: value.0,
                    projections: vec![k::Projection::Deref],
                },
                span.line,
            )
        } else {
            Ok(value)
        }
    }

    pub(super) fn shift_expr(
        &mut self,
        operation: &str,
        left: usize,
        right: usize,
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let expected = self
            .hint(left)
            .or(expected)
            .map(|ty| self.primitive_value_type(ty));
        let (left, ty) = self.primitive_operand(left, expected, span)?;
        let (mut right, rt) = self.primitive_operand(right, None, span)?;
        if !matches!(self.compiler.types[ty].kind, k::TypeKind::Int { .. })
            || !matches!(self.compiler.types[rt].kind, k::TypeKind::Int { .. })
        {
            return Err(err(span, "E_TYPE", "shift operands must be integers"));
        }
        if ty != rt {
            let k::TypeKind::Int { bits, .. } = self.compiler.types[ty].kind else {
                unreachable!()
            };
            let k::TypeKind::Int {
                bits: right_bits, ..
            } = self.compiler.types[rt].kind
            else {
                unreachable!()
            };
            if right_bits > bits {
                // Validate before narrowing: an invalid amount such as 256_u64
                // must not turn into a valid zero when shifted into a u8 value.
                // Casting a negative amount to u64 also preserves its invalidity.
                // An ordinary checked K shift provides the arithmetic trap.
                let amount = self.temp(k::U64);
                self.emit(
                    k::Op::Intrinsic {
                        dst: amount,
                        name: "cast".to_string(),
                        args: vec![right],
                    },
                    span.line,
                );
                let zero = self.temp(k::U64);
                self.emit(
                    k::Op::Int {
                        dst: zero,
                        value: 0,
                    },
                    span.line,
                );
                let check_amount = self.read(k::Place::local(amount), span.line)?.0;
                let checked = self.temp(k::U64);
                self.emit(
                    k::Op::Binary {
                        dst: checked,
                        op: "<<".to_string(),
                        left: zero,
                        right: check_amount,
                    },
                    span.line,
                );
                right = amount;
            }
            let amount = self.temp(ty);
            self.emit(
                k::Op::Intrinsic {
                    dst: amount,
                    name: "cast".to_string(),
                    args: vec![right],
                },
                span.line,
            );
            right = amount;
        }
        let dst = self.temp(ty);
        self.emit(
            k::Op::Binary {
                dst,
                op: operation.to_string(),
                left,
                right,
            },
            span.line,
        );
        Ok((dst, ty))
    }
}
