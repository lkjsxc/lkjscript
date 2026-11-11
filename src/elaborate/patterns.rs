//! Typed scalar patterns and interval coverage, independent of source arm order.
use super::{Builder, Result, a, err, integers, k};

#[derive(Clone, Copy)]
pub(super) struct IntegerRange {
    start: Option<i64>,
    end: Option<i64>,
    inclusive: bool,
}

#[derive(Default)]
pub(super) struct ArmPattern {
    pub(super) range: Option<IntegerRange>,
    pub(super) tag: Option<usize>,
}

#[derive(Clone, Copy)]
struct Domain {
    maximum: u64,
    sign: u64,
}
impl Domain {
    fn new(kind: &k::TypeKind) -> Option<Self> {
        if let k::TypeKind::Int { bits, signed } = kind {
            Some(Domain {
                maximum: if *bits == 64 {
                    u64::MAX
                } else {
                    (1u64 << bits) - 1
                },
                sign: if *signed { 1u64 << (bits - 1) } else { 0 },
            })
        } else {
            None
        }
    }
    fn rank(self, value: i64) -> u64 {
        (value as u64 & self.maximum) ^ self.sign
    }
}

fn covered(mut intervals: Vec<(u64, u64)>, maximum: u64) -> bool {
    intervals.sort_unstable();
    let Some((first, mut last)) = intervals.first().copied() else {
        return false;
    };
    if first != 0 {
        return false;
    }
    for (start, end) in intervals.into_iter().skip(1) {
        if last == maximum {
            return true;
        }
        if start > last && last.checked_add(1) != Some(start) {
            return false;
        }
        last = last.max(end);
    }
    last == maximum
}

impl Builder<'_, '_> {
    pub(super) fn pattern_hint(&self, arms: &[a::Arm]) -> Option<usize> {
        for arm in arms {
            let ids = match &self.compiler.source.patterns[arm.pattern].kind {
                a::Pattern::Literal(id) => vec![*id],
                a::Pattern::Range { start, end, .. } => {
                    start.iter().chain(end.iter()).copied().collect()
                }
                _ => Vec::new(),
            };
            for mut id in ids {
                if let a::Expr::Unary { op, value } = &self.compiler.source.exprs[id].kind {
                    if op == "-" {
                        id = *value;
                    }
                }
                if let a::Expr::Int(text) = &self.compiler.source.exprs[id].kind {
                    if let Some(ty) = integers::suffix_type(&self.compiler.type_names, text) {
                        return Some(ty);
                    }
                }
            }
        }
        None
    }

    fn integer_pattern(&self, expression: usize, ty: usize, range: bool) -> Result<i64> {
        let node = &self.compiler.source.exprs[expression];
        let (text, negative) = match &node.kind {
            a::Expr::Int(text) => (text, false),
            a::Expr::Unary { op, value } if op == "-" => {
                if let a::Expr::Int(text) = &self.compiler.source.exprs[*value].kind {
                    (text, true)
                } else {
                    return Err(err(
                        node.span,
                        "E_TYPE",
                        "negative pattern bound requires an integer literal",
                    ));
                }
            }
            a::Expr::Bool(_) | a::Expr::Float(_) => {
                return Err(err(
                    node.span,
                    "E_TYPE",
                    "range endpoints require integer literals",
                ));
            }
            _ => {
                return Err(err(
                    node.span,
                    "E_UNSUPPORTED",
                    if range {
                        "range endpoints require integer or byte literals"
                    } else {
                        "integer pattern requires a literal"
                    },
                ));
            }
        };
        integers::literal(
            &self.compiler.types,
            &self.compiler.type_names,
            text,
            Some(ty),
            negative,
            node.span,
        )
        .map(|(value, _)| value)
    }

    fn irrefutable_fields(&self, fields: &[usize]) -> Result<()> {
        let mut pending = fields.to_vec();
        while let Some(id) = pending.pop() {
            let pattern = &self.compiler.source.patterns[id];
            match &pattern.kind {
                a::Pattern::Wild | a::Pattern::Bind { .. } => {}
                a::Pattern::Tuple(fields) => pending.extend_from_slice(fields),
                _ => {
                    return Err(err(
                        pattern.span,
                        "E_UNSUPPORTED",
                        "nested refutable match patterns are not implemented",
                    ));
                }
            }
        }
        Ok(())
    }

    pub(super) fn match_patterns(
        &mut self,
        ty: usize,
        arms: &[a::Arm],
    ) -> Result<(Vec<ArmPattern>, bool)> {
        let domain = Domain::new(&self.compiler.types[ty].kind);
        let mut intervals = Vec::new();
        let mut tags = Vec::new();
        let mut booleans = [false; 2];
        let mut exhaustive = false;
        let mut prepared = Vec::new();
        for arm in arms {
            if arm.guard.is_some() {
                return Err(err(
                    arm.span,
                    "E_UNSUPPORTED",
                    "match guards are not implemented yet",
                ));
            }
            let pattern = self.compiler.source.patterns[arm.pattern].clone();
            let mut info = ArmPattern::default();
            match pattern.kind {
                a::Pattern::Wild | a::Pattern::Bind { .. } => exhaustive = true,
                a::Pattern::Range {
                    start,
                    end,
                    inclusive,
                } => {
                    let domain = domain.ok_or_else(|| {
                        err(
                            pattern.span,
                            "E_TYPE",
                            "range pattern requires an integer scrutinee",
                        )
                    })?;
                    let start = start
                        .map(|id| self.integer_pattern(id, ty, true))
                        .transpose()?;
                    let end = end
                        .map(|id| self.integer_pattern(id, ty, true))
                        .transpose()?;
                    let first = start.map(|value| domain.rank(value)).unwrap_or(0);
                    let last = match end {
                        Some(value) if !inclusive => {
                            domain.rank(value).checked_sub(1).ok_or_else(|| {
                                err(pattern.span, "E_TYPE", "range pattern is empty or reversed")
                            })?
                        }
                        Some(value) => domain.rank(value),
                        None => domain.maximum,
                    };
                    if first > last {
                        return Err(err(
                            pattern.span,
                            "E_TYPE",
                            "range pattern is empty or reversed",
                        ));
                    }
                    intervals.push((first, last));
                    info.range = Some(IntegerRange {
                        start,
                        end,
                        inclusive,
                    });
                }
                a::Pattern::Literal(expression) => {
                    if let Some(domain) = domain {
                        let value = self.integer_pattern(expression, ty, false)?;
                        let rank = domain.rank(value);
                        intervals.push((rank, rank));
                    } else if let a::Expr::Bool(value) = self.compiler.source.exprs[expression].kind
                    {
                        if ty != k::BOOL {
                            return Err(err(
                                pattern.span,
                                "E_TYPE",
                                "literal pattern differs from scrutinee type",
                            ));
                        }
                        booleans[usize::from(value)] = true;
                    } else if ty != k::F64 {
                        return Err(err(
                            pattern.span,
                            "E_UNSUPPORTED",
                            "this scalar literal pattern is not implemented",
                        ));
                    }
                }
                a::Pattern::Variant { path, fields } => {
                    let (_, tag) = self.variant(&path, Some(ty), arm.span)?;
                    let k::TypeKind::Enum { variants, .. } = &self.compiler.types[ty].kind else {
                        unreachable!()
                    };
                    if fields.len() != variants[tag].fields.len() {
                        return Err(err(
                            pattern.span,
                            "E_TYPE",
                            "variant pattern field count differs",
                        ));
                    }
                    self.irrefutable_fields(&fields)?;
                    tags.push(tag);
                    info.tag = Some(tag);
                }
                a::Pattern::Path(path) => {
                    let (_, tag) = self.variant(&path, Some(ty), arm.span)?;
                    let k::TypeKind::Enum { variants, .. } = &self.compiler.types[ty].kind else {
                        unreachable!()
                    };
                    if !variants[tag].fields.is_empty() {
                        return Err(err(
                            pattern.span,
                            "E_TYPE",
                            "variant pattern requires its fields",
                        ));
                    }
                    tags.push(tag);
                    info.tag = Some(tag);
                }
                _ => {
                    return Err(err(
                        pattern.span,
                        "E_UNSUPPORTED",
                        "this match pattern is not implemented yet",
                    ));
                }
            }
            prepared.push(info);
        }
        if let Some(domain) = domain {
            exhaustive |= covered(intervals, domain.maximum);
        } else if ty == k::BOOL {
            exhaustive |= booleans[0] && booleans[1];
        } else if let k::TypeKind::Enum { variants, .. } = &self.compiler.types[ty].kind {
            tags.sort_unstable();
            tags.dedup();
            exhaustive |= tags.len() == variants.len();
        }
        Ok((prepared, exhaustive))
    }

    pub(super) fn scalar_match_read(
        &mut self,
        scrutinee: usize,
        borrowed: bool,
        line: usize,
    ) -> Result<usize> {
        let place = k::Place {
            local: scrutinee,
            projections: if borrowed {
                vec![k::Projection::Deref]
            } else {
                Vec::new()
            },
        };
        self.read(place, line).map(|value| value.0)
    }

    pub(super) fn range_condition(
        &mut self,
        scrutinee: usize,
        borrowed: bool,
        ty: usize,
        range: IntegerRange,
        line: usize,
    ) -> Result<usize> {
        let mut conditions = Vec::new();
        for (bound, operation) in [
            (range.start, ">="),
            (range.end, if range.inclusive { "<=" } else { "<" }),
        ] {
            if let Some(value) = bound {
                let left = self.scalar_match_read(scrutinee, borrowed, line)?;
                let right = self.temp(ty);
                self.emit(k::Op::Int { dst: right, value }, line);
                let condition = self.temp(k::BOOL);
                self.emit(
                    k::Op::Binary {
                        dst: condition,
                        op: operation.to_string(),
                        left,
                        right,
                    },
                    line,
                );
                conditions.push(condition);
            }
        }
        if conditions.len() == 2 {
            let result = self.temp(k::BOOL);
            self.emit(
                k::Op::Binary {
                    dst: result,
                    op: "&".to_string(),
                    left: conditions[0],
                    right: conditions[1],
                },
                line,
            );
            Ok(result)
        } else {
            Ok(conditions[0])
        }
    }
}
