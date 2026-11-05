//! VM language/observation equality over borrowed values.
//!
//! A false result never skips remaining comparability checks. This frontier
//! contains occurrences, not visited owners: sharing is not a shortcut past a
//! Function or other non-comparable descendant. Scratch reservations are actual
//! fallible allocator work, not new modeled value/allocation policy charges.

use super::{ExecutionControl, ExecutionError, NormalizedRecord, NormalizedValue, trap_error};

type Pair<'a> = (&'a NormalizedValue, &'a NormalizedValue);

struct Pending<'a> {
    current: Option<Pair<'a>>,
    later: Vec<Pair<'a>>,
}

impl<'a> Pending<'a> {
    fn push(&mut self, pair: Pair<'a>) -> Result<(), ExecutionError> {
        if let Some(previous) = self.current {
            reserve(&mut self.later)?;
            self.later.push(previous);
        }
        self.current = Some(pair);
        Ok(())
    }

    fn optional(
        &mut self,
        left: Option<&'a NormalizedValue>,
        right: Option<&'a NormalizedValue>,
    ) -> Result<bool, ExecutionError> {
        match (left, right) {
            (Some(left), Some(right)) => {
                self.push((left, right))?;
                Ok(true)
            }
            (Some(value), None) | (None, Some(value)) => {
                self.push((value, value))?;
                Ok(false)
            }
            (None, None) => Ok(true),
        }
    }

    fn pop(&mut self) -> Option<Pair<'a>> {
        self.current.take().or_else(|| self.later.pop())
    }
}

fn reserve(frontier: &mut Vec<Pair<'_>>) -> Result<(), ExecutionError> {
    #[cfg(test)]
    if FRONTIER_ALLOWANCE.with(|allowance| match allowance.get() {
        None => false,
        Some(0) => true,
        Some(remaining) => {
            allowance.set(Some(remaining - 1));
            false
        }
    }) {
        return Err(allocation_error());
    }
    frontier.try_reserve(1).map_err(|_| allocation_error())
}

fn allocation_error() -> ExecutionError {
    ExecutionError::resource_static(
        "normalized_semantic_comparison_allocation",
        "value equality could not reserve its pending borrowed occurrences",
    )
}

fn bytes_equal(
    mut left: &[u8],
    mut right: &[u8],
    control: &ExecutionControl,
) -> Result<bool, ExecutionError> {
    if left.len() != right.len() {
        return Ok(false);
    }
    while !left.is_empty() {
        control.check()?;
        // A checkpoint interval bounds uninterrupted scanning, not admission.
        let width = left.len().min(4096);
        let (left_head, left_tail) = left.split_at(width);
        let (right_head, right_tail) = right.split_at(width);
        if left_head != right_head {
            return Ok(false);
        }
        left = left_tail;
        right = right_tail;
    }
    Ok(true)
}

pub(super) fn compare(
    left: &NormalizedValue,
    right: &NormalizedValue,
    observation: bool,
    control: &ExecutionControl,
) -> Result<bool, ExecutionError> {
    let mut pending = Pending {
        current: Some((left, right)),
        later: Vec::new(),
    };
    let mut equal = true;
    while let Some((left, right)) = pending.pop() {
        control.check()?;
        equal &= match (left, right) {
            (NormalizedValue::Unit, NormalizedValue::Unit) => true,
            (NormalizedValue::Bool(left), NormalizedValue::Bool(right)) => left == right,
            (NormalizedValue::I64(left), NormalizedValue::I64(right)) => left == right,
            (NormalizedValue::F64(left), NormalizedValue::F64(right)) => {
                if observation {
                    left.bits() == right.bits()
                } else {
                    left.to_float() == right.to_float()
                }
            }
            (NormalizedValue::Bytes(left), NormalizedValue::Bytes(right)) => {
                bytes_equal(left.as_ref(), right.as_ref(), control)?
            }
            (NormalizedValue::Text(left), NormalizedValue::Text(right))
            | (NormalizedValue::StaticText(left), NormalizedValue::StaticText(right)) => {
                bytes_equal(left.as_bytes(), right.as_bytes(), control)?
            }
            (
                NormalizedValue::Record(NormalizedRecord::Nominal {
                    layout: left_layout,
                    fields: left,
                }),
                NormalizedValue::Record(NormalizedRecord::Nominal {
                    layout: right_layout,
                    fields: right,
                }),
            ) => {
                for index in (0..left.len().max(right.len())).rev() {
                    control.check()?;
                    pending.optional(left.get(index), right.get(index))?;
                }
                left_layout == right_layout && left.len() == right.len()
            }
            (
                NormalizedValue::Record(NormalizedRecord::Structural { fields: left }),
                NormalizedValue::Record(NormalizedRecord::Structural { fields: right }),
            ) => {
                let mut same = left.len() == right.len();
                for index in (0..left.len().max(right.len())).rev() {
                    control.check()?;
                    match (left.get(index), right.get(index)) {
                        (Some((left_name, left)), Some((right_name, right))) => {
                            same &= left_name == right_name;
                            pending.push((left, right))?;
                        }
                        (Some((_, value)), None) | (None, Some((_, value))) => {
                            pending.push((value, value))?;
                        }
                        (None, None) => {}
                    }
                }
                same
            }
            (
                NormalizedValue::Variant {
                    layout: left_layout,
                    case: left_case,
                    payload: left,
                },
                NormalizedValue::Variant {
                    layout: right_layout,
                    case: right_case,
                    payload: right,
                },
            ) => {
                let present = pending.optional(left.as_deref(), right.as_deref())?;
                present && left_layout == right_layout && left_case == right_case
            }
            (NormalizedValue::Option(left), NormalizedValue::Option(right)) => {
                pending.optional(left.as_deref(), right.as_deref())?
            }
            (
                NormalizedValue::Result {
                    success: left_case,
                    value: left,
                },
                NormalizedValue::Result {
                    success: right_case,
                    value: right,
                },
            ) => {
                pending.push((left, right))?;
                left_case == right_case
            }
            (NormalizedValue::List(left), NormalizedValue::List(right)) => {
                let mut left_values = left.iter();
                let mut right_values = right.iter();
                // Align the longer reversed iterator before comparing the common
                // prefix. All unmatched occurrences still receive a self check.
                for _ in right.len()..left.len() {
                    control.check()?;
                    pending.optional(left_values.next_back(), None)?;
                }
                for _ in left.len()..right.len() {
                    control.check()?;
                    pending.optional(None, right_values.next_back())?;
                }
                for (left, right) in left_values.rev().zip(right_values.rev()) {
                    control.check()?;
                    pending.push((left, right))?;
                }
                left.len() == right.len()
            }
            (NormalizedValue::Map(left), NormalizedValue::Map(right)) => {
                let mut same = left.len() == right.len();
                // Stack scheduling is the reverse of semantic visitation: left
                // key order first, then right-only keys in their original order.
                for (key, value) in right.iter().rev() {
                    control.check()?;
                    if !left.contains_key(key) {
                        pending.push((value, value))?;
                        same = false;
                    }
                }
                for (key, value) in left.iter().rev() {
                    control.check()?;
                    same &= pending.optional(Some(value), right.get(key))?;
                }
                same
            }
            _ if left.memory_form().is_some() || right.memory_form().is_some() => {
                return Err(trap_error(
                    "normalized_value_not_comparable",
                    "owned memory does not support semantic equality",
                ));
            }
            (NormalizedValue::ReadView(_), _) | (_, NormalizedValue::ReadView(_)) => {
                return Err(trap_error(
                    "normalized_value_not_comparable",
                    "read views do not support semantic equality",
                ));
            }
            (NormalizedValue::Function { .. }, _) | (_, NormalizedValue::Function { .. }) => {
                return Err(trap_error(
                    "normalized_value_not_comparable",
                    "functions do not support semantic equality",
                ));
            }
            (NormalizedValue::Resource(_), _) | (_, NormalizedValue::Resource(_)) => {
                return Err(trap_error(
                    "normalized_value_not_comparable",
                    "live resources do not support semantic equality",
                ));
            }
            (
                NormalizedValue::RegionPointer {
                    ty: left_type,
                    pointer: left,
                },
                NormalizedValue::RegionPointer {
                    ty: right_type,
                    pointer: right,
                },
            ) => {
                // Coordinates are comparable without dereferencing or lending
                // custody. Pointer equality includes arena, slot, generation
                // and offset; the exact type retains brand and item identity.
                left_type == right_type && left == right
            }
            (NormalizedValue::RegionCap { .. }, NormalizedValue::RegionCap { .. }) => {
                return Err(trap_error(
                    "normalized_value_not_comparable",
                    "region capabilities do not support semantic equality",
                ));
            }
            _ => {
                // A kind mismatch is unequal only after both complete values
                // have been checked. Do not skip a hidden non-comparable child.
                pending.push((right, right))?;
                pending.push((left, left))?;
                false
            }
        };
    }
    Ok(equal)
}

#[cfg(test)]
thread_local! {
    static FRONTIER_ALLOWANCE: std::cell::Cell<Option<usize>> = const {
        std::cell::Cell::new(None)
    };
}

#[cfg(test)]
pub(crate) struct FrontierRefusal {
    previous: Option<usize>,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}

#[cfg(test)]
pub(crate) fn refuse_frontier_after(allowed_reservations: usize) -> FrontierRefusal {
    FrontierRefusal {
        previous: FRONTIER_ALLOWANCE.with(|slot| slot.replace(Some(allowed_reservations))),
        _thread: std::marker::PhantomData,
    }
}

#[cfg(test)]
impl Drop for FrontierRefusal {
    fn drop(&mut self) {
        FRONTIER_ALLOWANCE.with(|slot| slot.set(self.previous));
    }
}
