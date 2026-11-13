//! Integer constants share one typed validation path in expressions and patterns.
use super::{Builder, Result, a, err, k};
use std::collections::BTreeMap;

const SUFFIXES: [&str; 10] = [
    "usize", "isize", "u64", "i64", "u32", "i32", "u16", "i16", "u8", "i8",
];

pub(super) fn suffix_type(names: &BTreeMap<String, usize>, text: &str) -> Option<usize> {
    for suffix in SUFFIXES {
        if text.ends_with(suffix) {
            return names.get(suffix).copied();
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
    for suffix in SUFFIXES {
        if digits.ends_with(suffix) {
            ty = *names
                .get(suffix)
                .ok_or_else(|| err(span, "E_TYPE", "unknown integer suffix"))?;
            digits.truncate(digits.len() - suffix.len());
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
    let magnitude = u64::from_str_radix(if radix == 10 { &digits } else { &digits[2..] }, radix)
        .map_err(|_| err(span, "E_TYPE", "integer literal exceeds its representation"))?;
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
        u64::MAX
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
        if !matches!(self.compiler.types[ty].kind, k::TypeKind::Int { .. })
            || !matches!(name, "wrapping_add" | "wrapping_sub" | "wrapping_mul")
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
