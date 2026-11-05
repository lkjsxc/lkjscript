//! Independent, ordered semantic comparison for the canonical reference tier.
//!
//! Values stay borrowed. A cursor continuation retains only unfinished siblings;
//! a unary descent replaces the current pair without reserving a continuation.
//! Unequal metadata never skips a child or an unmatched comparability scan.

use super::super::{list, map};
use super::{NormalizedRecord, NormalizedValue, reference_error, reference_trap};
use crate::platform::execution::{ExecutionControl, ExecutionError};
use crate::platform::kernel::Name;

type Pair<'a> = (&'a NormalizedValue, &'a NormalizedValue);

struct Sequence<'a> {
    left: std::slice::Iter<'a, NormalizedValue>,
    right: std::slice::Iter<'a, NormalizedValue>,
}

impl<'a> Sequence<'a> {
    fn next(&mut self) -> Option<Pair<'a>> {
        next_pair(self.left.next(), self.right.next())
    }

    fn finished(&self) -> bool {
        self.left.len() == 0 && self.right.len() == 0
    }
}

struct Fields<'a> {
    left: std::slice::Iter<'a, (Name, NormalizedValue)>,
    right: std::slice::Iter<'a, (Name, NormalizedValue)>,
}

impl<'a> Fields<'a> {
    fn next(&mut self, equal: &mut bool) -> Option<Pair<'a>> {
        match (self.left.next(), self.right.next()) {
            (Some((left_name, left)), Some((right_name, right))) => {
                *equal &= left_name == right_name;
                Some((left, right))
            }
            (Some((_, value)), None) | (None, Some((_, value))) => Some((value, value)),
            (None, None) => None,
        }
    }

    fn finished(&self) -> bool {
        self.left.len() == 0 && self.right.len() == 0
    }
}

struct Lists<'a> {
    left: list::Iter<'a>,
    right: list::Iter<'a>,
}

impl<'a> Lists<'a> {
    fn next(&mut self, control: &ExecutionControl) -> Result<Option<Pair<'a>>, ExecutionError> {
        control.check()?;
        let left = self.left.next();
        control.check()?;
        let right = self.right.next();
        Ok(next_pair(left, right))
    }

    fn finished(&self) -> bool {
        self.left.len() == 0 && self.right.len() == 0
    }
}

struct Maps<'a> {
    left: &'a map::Map,
    right: &'a map::Map,
    cursor: map::Iter<'a>,
    right_only: bool,
}

impl<'a> Maps<'a> {
    fn next(
        &mut self,
        equal: &mut bool,
        control: &ExecutionControl,
    ) -> Result<Option<Pair<'a>>, ExecutionError> {
        loop {
            control.check()?;
            if let Some((key, value)) = self.cursor.next() {
                control.check()?;
                if self.right_only {
                    if !self.left.contains_key(key) {
                        *equal = false;
                        return Ok(Some((value, value)));
                    }
                } else if let Some(other) = self.right.get(key) {
                    return Ok(Some((value, other)));
                } else {
                    *equal = false;
                    return Ok(Some((value, value)));
                }
            } else if self.right_only {
                return Ok(None);
            } else {
                // Preserve the reference tier's complete left-key pass followed
                // by its right-only pass, reusing the same iterator storage.
                self.cursor.restart(self.right);
                self.right_only = true;
            }
        }
    }

    fn finished(&self) -> bool {
        self.right_only && self.cursor.len() == 0
    }
}

enum Continuation<'a> {
    Sequence(Sequence<'a>),
    Fields(Fields<'a>),
    List,
    Map,
    RightScan(&'a NormalizedValue),
}

#[derive(Default)]
struct Cursors<'a> {
    pending: Vec<Continuation<'a>>,
    // Keep collection iterators in separate typed stacks. Their bounded inline
    // traversal buffers must not inflate every scalar/slice continuation.
    lists: Vec<Lists<'a>>,
    maps: Vec<Maps<'a>>,
}

impl<'a> Cursors<'a> {
    fn sequence(
        &mut self,
        left: &'a [NormalizedValue],
        right: &'a [NormalizedValue],
        control: &ExecutionControl,
    ) -> Result<Option<Pair<'a>>, ExecutionError> {
        control.check()?;
        let mut cursor = Sequence {
            left: left.iter(),
            right: right.iter(),
        };
        let pair = cursor.next();
        if !cursor.finished() {
            reserve(&mut self.pending, control)?;
            self.pending.push(Continuation::Sequence(cursor));
        }
        Ok(pair)
    }

    fn fields(
        &mut self,
        left: &'a [(Name, NormalizedValue)],
        right: &'a [(Name, NormalizedValue)],
        equal: &mut bool,
        control: &ExecutionControl,
    ) -> Result<Option<Pair<'a>>, ExecutionError> {
        control.check()?;
        let mut cursor = Fields {
            left: left.iter(),
            right: right.iter(),
        };
        let pair = cursor.next(equal);
        if !cursor.finished() {
            reserve(&mut self.pending, control)?;
            self.pending.push(Continuation::Fields(cursor));
        }
        Ok(pair)
    }

    fn list(
        &mut self,
        left: &'a list::List,
        right: &'a list::List,
        control: &ExecutionControl,
    ) -> Result<Option<Pair<'a>>, ExecutionError> {
        let mut cursor = Lists {
            left: left.iter(),
            right: right.iter(),
        };
        let pair = cursor.next(control)?;
        if !cursor.finished() {
            reserve(&mut self.pending, control)?;
            reserve(&mut self.lists, control)?;
            self.lists.push(cursor);
            self.pending.push(Continuation::List);
        }
        Ok(pair)
    }

    fn map(
        &mut self,
        left: &'a map::Map,
        right: &'a map::Map,
        equal: &mut bool,
        control: &ExecutionControl,
    ) -> Result<Option<Pair<'a>>, ExecutionError> {
        let mut cursor = Maps {
            left,
            right,
            cursor: left.iter(),
            right_only: false,
        };
        let pair = cursor.next(equal, control)?;
        if !cursor.finished() {
            reserve(&mut self.pending, control)?;
            reserve(&mut self.maps, control)?;
            self.maps.push(cursor);
            self.pending.push(Continuation::Map);
        }
        Ok(pair)
    }

    fn right_scan(
        &mut self,
        value: &'a NormalizedValue,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        reserve(&mut self.pending, control)?;
        self.pending.push(Continuation::RightScan(value));
        Ok(())
    }

    fn next(
        &mut self,
        equal: &mut bool,
        control: &ExecutionControl,
    ) -> Result<Option<Pair<'a>>, ExecutionError> {
        loop {
            control.check()?;
            let Some(continuation) = self.pending.last_mut() else {
                return Ok(None);
            };
            let pair = match continuation {
                Continuation::Sequence(cursor) => {
                    let pair = cursor.next();
                    if cursor.finished() {
                        self.pending.pop();
                    }
                    pair
                }
                Continuation::Fields(cursor) => {
                    let pair = cursor.next(equal);
                    if cursor.finished() {
                        self.pending.pop();
                    }
                    pair
                }
                Continuation::List => {
                    let cursor = self.lists.last_mut().ok_or_else(cursor_error)?;
                    let pair = cursor.next(control)?;
                    if cursor.finished() {
                        self.lists.pop();
                        self.pending.pop();
                    }
                    pair
                }
                Continuation::Map => {
                    let cursor = self.maps.last_mut().ok_or_else(cursor_error)?;
                    let pair = cursor.next(equal, control)?;
                    if cursor.finished() {
                        self.maps.pop();
                        self.pending.pop();
                    }
                    pair
                }
                Continuation::RightScan(value) => {
                    let value = *value;
                    self.pending.pop();
                    Some((value, value))
                }
            };
            if pair.is_some() {
                return Ok(pair);
            }
        }
    }
}

fn next_pair<'a>(
    left: Option<&'a NormalizedValue>,
    right: Option<&'a NormalizedValue>,
) -> Option<Pair<'a>> {
    match (left, right) {
        (Some(left), Some(right)) => Some((left, right)),
        (Some(value), None) | (None, Some(value)) => Some((value, value)),
        (None, None) => None,
    }
}

fn reserve<T>(pending: &mut Vec<T>, control: &ExecutionControl) -> Result<(), ExecutionError> {
    control.check()?;
    #[cfg(test)]
    if FRONTIER_ALLOWANCE.with(|allowance| match allowance.get() {
        None => false,
        Some(0) => true,
        Some(remaining) => {
            allowance.set(Some(remaining - 1));
            false
        }
    }) {
        return Err(storage_error());
    }
    pending
        .try_reserve(1)
        .or_else(|_| pending.try_reserve_exact(1))
        .map_err(|_| storage_error())
}

fn storage_error() -> ExecutionError {
    ExecutionError::resource_static(
        "normalized_reference_semantic_comparison_allocation",
        "reference semantic comparison could not reserve its cursor frontier",
    )
}

fn cursor_error() -> ExecutionError {
    reference_error(
        "normalized_reference_semantic_comparison_cursor",
        "reference semantic comparison continuation has no matching cursor",
    )
}

fn bytes_equal(
    left: &[u8],
    right: &[u8],
    control: &ExecutionControl,
) -> Result<bool, ExecutionError> {
    if left.len() != right.len() {
        return Ok(false);
    }
    // This is a cancellation interval, not a value-size or work quota.
    for (left, right) in left.chunks(4096).zip(right.chunks(4096)) {
        control.check()?;
        if left != right {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn compare(
    left: &NormalizedValue,
    right: &NormalizedValue,
    observation: bool,
    control: &ExecutionControl,
) -> Result<bool, ExecutionError> {
    let mut current = (left, right);
    let mut cursors = Cursors::default();
    let mut equal = true;
    loop {
        control.check()?;
        let (left, right) = current;
        // Semantic equality must inspect every occurrence, even when the two
        // references or immutable child owners are physically identical.
        let child = match (left, right) {
            (NormalizedValue::Unit, NormalizedValue::Unit) => None,
            (NormalizedValue::Bool(left), NormalizedValue::Bool(right)) => {
                equal &= left == right;
                None
            }
            (NormalizedValue::I64(left), NormalizedValue::I64(right)) => {
                equal &= left == right;
                None
            }
            (NormalizedValue::F64(left), NormalizedValue::F64(right)) => {
                equal &= if observation {
                    left.bits() == right.bits()
                } else {
                    left.to_float() == right.to_float()
                };
                None
            }
            (NormalizedValue::Bytes(left), NormalizedValue::Bytes(right)) => {
                equal &= bytes_equal(left.as_ref(), right.as_ref(), control)?;
                None
            }
            (NormalizedValue::Text(left), NormalizedValue::Text(right))
            | (NormalizedValue::StaticText(left), NormalizedValue::StaticText(right)) => {
                equal &= bytes_equal(left.as_bytes(), right.as_bytes(), control)?;
                None
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
                equal &= left_layout == right_layout && left.len() == right.len();
                cursors.sequence(left, right, control)?
            }
            (
                NormalizedValue::Record(NormalizedRecord::Structural { fields: left }),
                NormalizedValue::Record(NormalizedRecord::Structural { fields: right }),
            ) => {
                equal &= left.len() == right.len();
                cursors.fields(left, right, &mut equal, control)?
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
                equal &= left_layout == right_layout
                    && left_case == right_case
                    && left.is_some() == right.is_some();
                next_pair(left.as_deref(), right.as_deref())
            }
            (NormalizedValue::Option(left), NormalizedValue::Option(right)) => {
                equal &= left.is_some() == right.is_some();
                next_pair(left.as_deref(), right.as_deref())
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
                equal &= left_case == right_case;
                Some((left.as_ref(), right.as_ref()))
            }
            (NormalizedValue::List(left), NormalizedValue::List(right)) => {
                equal &= left.len() == right.len();
                cursors.list(left, right, control)?
            }
            (NormalizedValue::Map(left), NormalizedValue::Map(right)) => {
                equal &= left.len() == right.len();
                cursors.map(left, right, &mut equal, control)?
            }
            (NormalizedValue::ReadView(_), _) | (_, NormalizedValue::ReadView(_)) => {
                return Err(reference_trap(
                    "normalized_reference_value_not_comparable",
                    "read-view handles do not support semantic equality",
                ));
            }
            (NormalizedValue::Function { .. }, _) | (_, NormalizedValue::Function { .. }) => {
                return Err(reference_trap(
                    "normalized_reference_value_not_comparable",
                    "functions do not support semantic equality",
                ));
            }
            (NormalizedValue::Resource(_), _) | (_, NormalizedValue::Resource(_)) => {
                return Err(reference_trap(
                    "normalized_reference_value_not_comparable",
                    "live resources do not support semantic equality",
                ));
            }
            (
                NormalizedValue::ByteBuffer(_)
                | NormalizedValue::OwnedI64Cell(_)
                | NormalizedValue::OwnedProduct(_)
                | NormalizedValue::OwnedChoice(_)
                | NormalizedValue::OwnedSequence(_),
                _,
            )
            | (
                _,
                NormalizedValue::ByteBuffer(_)
                | NormalizedValue::OwnedI64Cell(_)
                | NormalizedValue::OwnedProduct(_)
                | NormalizedValue::OwnedChoice(_)
                | NormalizedValue::OwnedSequence(_),
            ) => {
                return Err(reference_trap(
                    "normalized_reference_value_not_comparable",
                    "owned memory does not support semantic equality",
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
                // Compare the complete typed relative coordinate, never the
                // referent or capability. This grants no access to an arena.
                equal &= left_type == right_type && left == right;
                None
            }
            (NormalizedValue::RegionCap { .. }, NormalizedValue::RegionCap { .. }) => {
                // A capability is authority, not a comparable coordinate.
                // Cross-shape pairs retain the left-before-right scan.
                return Err(reference_trap(
                    "normalized_reference_value_not_comparable",
                    "region capabilities do not support semantic equality",
                ));
            }
            _ => {
                equal = false;
                cursors.right_scan(right, control)?;
                Some((left, left))
            }
        };
        current = match child {
            Some(pair) => pair,
            None => match cursors.next(&mut equal, control)? {
                Some(pair) => pair,
                None => return Ok(equal),
            },
        };
    }
}

#[cfg(test)]
thread_local! {
    static FRONTIER_ALLOWANCE: std::cell::Cell<Option<usize>> = const {
        std::cell::Cell::new(None)
    };
}

/// Refuse this thread's cursor-vector reservation attempts after `allowed`.
/// This tests the fallible frontier path, not the process allocator. A list/map
/// continuation reserves both its compact marker and its separate cursor row.
#[cfg(test)]
pub(crate) fn refuse_frontier_after(allowed: usize) -> FrontierRefusal {
    let previous = FRONTIER_ALLOWANCE.with(|allowance| allowance.replace(Some(allowed)));
    FrontierRefusal {
        previous,
        _current_thread: std::marker::PhantomData,
    }
}

#[cfg(test)]
pub(crate) struct FrontierRefusal {
    previous: Option<usize>,
    _current_thread: std::marker::PhantomData<std::rc::Rc<()>>,
}

#[cfg(test)]
impl Drop for FrontierRefusal {
    fn drop(&mut self) {
        FRONTIER_ALLOWANCE.with(|allowance| allowance.set(self.previous));
    }
}
