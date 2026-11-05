//! Fallible raw representation comparison for differential callers.
//!
//! This preserves the former derived NormalizedValue equality, including
//! Function metadata and capture contents. It is not language equality: the
//! independent VM/reference semantic comparators still reject Function values.

use super::value::{NormalizedRecord, NormalizedValue};
use crate::platform::execution::{ExecutionControl, ExecutionError};

type Pair<'a> = (&'a NormalizedValue, &'a NormalizedValue);

#[derive(Default)]
struct Pending<'a> {
    current: Option<Pair<'a>>,
    others: Vec<Pair<'a>>,
}

impl<'a> Pending<'a> {
    fn push(&mut self, pair: Pair<'a>) -> Result<(), ExecutionError> {
        if let Some(previous) = self.current {
            #[cfg(test)]
            if REFUSE_FRONTIER_RESERVATION.with(std::cell::Cell::get) {
                return Err(storage_error());
            }
            self.others.try_reserve(1).map_err(|_| storage_error())?;
            self.others.push(previous);
        }
        self.current = Some(pair);
        Ok(())
    }

    fn pop(&mut self) -> Option<Pair<'a>> {
        self.current.take().or_else(|| self.others.pop())
    }
}

fn storage_error() -> ExecutionError {
    ExecutionError::resource_static(
        "normalized_raw_comparison_allocation",
        "raw structural comparison could not reserve its pending references",
    )
}

pub(crate) fn raw_structural_equal(
    left: &NormalizedValue,
    right: &NormalizedValue,
    control: &ExecutionControl,
) -> Result<bool, ExecutionError> {
    let mut pending = Pending {
        current: Some((left, right)),
        others: Vec::new(),
    };
    while let Some((left, right)) = pending.pop() {
        control.check()?;
        // The former derived Eq is reflexive, including bitwise Binary64 and
        // identity-bearing leaves. This does not equate distinct allocations.
        if std::ptr::eq(left, right) {
            continue;
        }
        let equal = match (left, right) {
            (
                NormalizedValue::RegionCap { ty: a, token: left },
                NormalizedValue::RegionCap {
                    ty: b,
                    token: right,
                },
            ) => a == b && left == right,
            (
                NormalizedValue::RegionPointer {
                    ty: a,
                    pointer: left,
                },
                NormalizedValue::RegionPointer {
                    ty: b,
                    pointer: right,
                },
            ) => a == b && left == right,
            (NormalizedValue::ReadView(left), NormalizedValue::ReadView(right)) => left == right,
            (NormalizedValue::OwnedSequence(left), NormalizedValue::OwnedSequence(right)) => {
                left == right
            }
            (NormalizedValue::OwnedChoice(left), NormalizedValue::OwnedChoice(right)) => {
                left == right
            }
            (NormalizedValue::OwnedProduct(left), NormalizedValue::OwnedProduct(right)) => {
                left == right
            }
            (NormalizedValue::Unit, NormalizedValue::Unit) => true,
            (NormalizedValue::Bool(left), NormalizedValue::Bool(right)) => left == right,
            (NormalizedValue::I64(left), NormalizedValue::I64(right)) => left == right,
            (NormalizedValue::F64(left), NormalizedValue::F64(right)) => left == right,
            (NormalizedValue::ByteBuffer(left), NormalizedValue::ByteBuffer(right)) => {
                left == right
            }
            (NormalizedValue::OwnedI64Cell(left), NormalizedValue::OwnedI64Cell(right)) => {
                left == right
            }
            (NormalizedValue::Bytes(left), NormalizedValue::Bytes(right)) => left == right,
            (NormalizedValue::Text(left), NormalizedValue::Text(right))
            | (NormalizedValue::StaticText(left), NormalizedValue::StaticText(right)) => {
                left == right
            }
            (
                NormalizedValue::Record(NormalizedRecord::Nominal {
                    layout: a,
                    fields: left,
                }),
                NormalizedValue::Record(NormalizedRecord::Nominal {
                    layout: b,
                    fields: right,
                }),
            ) => {
                if a != b || left.len() != right.len() {
                    false
                } else {
                    if !left.ptr_eq(right) {
                        push_pairs(left.iter().zip(right.iter()).rev(), control, &mut pending)?;
                    }
                    true
                }
            }
            (
                NormalizedValue::Record(NormalizedRecord::Structural { fields: left }),
                NormalizedValue::Record(NormalizedRecord::Structural { fields: right }),
            ) => {
                if left.ptr_eq(right) {
                    true
                } else if left.len() != right.len() {
                    false
                } else {
                    for ((left_name, left), (right_name, right)) in
                        left.iter().zip(right.iter()).rev()
                    {
                        control.check()?;
                        if left_name != right_name {
                            return Ok(false);
                        }
                        pending.push((left, right))?;
                    }
                    true
                }
            }
            (
                NormalizedValue::Variant {
                    layout: a,
                    case: x,
                    payload: left,
                },
                NormalizedValue::Variant {
                    layout: b,
                    case: y,
                    payload: right,
                },
            ) => {
                a == b && x == y && push_optional(left.as_deref(), right.as_deref(), &mut pending)?
            }
            (NormalizedValue::Option(left), NormalizedValue::Option(right)) => {
                push_optional(left.as_deref(), right.as_deref(), &mut pending)?
            }
            (
                NormalizedValue::Result {
                    success: a,
                    value: left,
                },
                NormalizedValue::Result {
                    success: b,
                    value: right,
                },
            ) => {
                if a != b {
                    false
                } else {
                    pending.push((left, right))?;
                    true
                }
            }
            (NormalizedValue::List(left), NormalizedValue::List(right)) => {
                if left.len() != right.len() {
                    false
                } else {
                    push_pairs(left.iter().zip(right.iter()).rev(), control, &mut pending)?;
                    true
                }
            }
            (NormalizedValue::Map(left), NormalizedValue::Map(right)) => {
                if left.len() != right.len() {
                    false
                } else {
                    for ((left_key, left), (right_key, right)) in
                        left.iter().zip(right.iter()).rev()
                    {
                        control.check()?;
                        if left_key != right_key {
                            return Ok(false);
                        }
                        pending.push((left, right))?;
                    }
                    true
                }
            }
            (
                NormalizedValue::Function {
                    function: a,
                    type_arguments: at,
                    effect_arguments: ae,
                    requirement_arguments: ar,
                    read_scope_arguments: av,
                    bound_arguments: left,
                },
                NormalizedValue::Function {
                    function: b,
                    type_arguments: bt,
                    effect_arguments: be,
                    requirement_arguments: br,
                    read_scope_arguments: bv,
                    bound_arguments: right,
                },
            ) => {
                if a != b || at != bt || ae != be || ar != br || av != bv {
                    false
                } else {
                    match (left, right) {
                        (None, None) => true,
                        (Some(left), Some(right)) if left.len() == right.len() => {
                            // Shared storage may be skipped just as std Arc's Eq
                            // did. Distinct storage is compared by its children.
                            if !left.ptr_eq(right) {
                                push_pairs(
                                    left.iter().zip(right.iter()).rev(),
                                    control,
                                    &mut pending,
                                )?;
                            }
                            true
                        }
                        _ => false,
                    }
                }
            }
            (NormalizedValue::Resource(left), NormalizedValue::Resource(right)) => left == right,
            _ => false,
        };
        if !equal {
            return Ok(false);
        }
    }
    Ok(true)
}

fn push_pairs<'a>(
    pairs: impl Iterator<Item = Pair<'a>>,
    control: &ExecutionControl,
    pending: &mut Pending<'a>,
) -> Result<(), ExecutionError> {
    for pair in pairs {
        control.check()?;
        pending.push(pair)?;
    }
    Ok(())
}

fn push_optional<'a>(
    left: Option<&'a NormalizedValue>,
    right: Option<&'a NormalizedValue>,
    pending: &mut Pending<'a>,
) -> Result<bool, ExecutionError> {
    match (left, right) {
        (None, None) => Ok(true),
        (Some(left), Some(right)) => {
            pending.push((left, right))?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

#[cfg(test)]
#[path = "structural_equality_tests.rs"]
mod tests;

#[cfg(test)]
thread_local! {
    static REFUSE_FRONTIER_RESERVATION: std::cell::Cell<bool> = const {
        std::cell::Cell::new(false)
    };
}

#[cfg(test)]
#[path = "raw_comparison_assertions.rs"]
pub(crate) mod assertions;
