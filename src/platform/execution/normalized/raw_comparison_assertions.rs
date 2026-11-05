//! Explicit fallible comparison adapters for raw-value test assertions.
//!
//! Use `assert_eq!(equal(&actual, &expected), Ok(true))`. A refused frontier
//! allocation or cancellation is reported as Err and fails the assertion.
//! This module is compiled only in tests; it never supplies a language Eq.

use super::raw_structural_equal;
use crate::platform::execution::normalized::{
    child_storage::{Captures, Fields, Values},
    list::List,
    map::Map,
    value::{NormalizedMapKey, NormalizedRecord, NormalizedValue},
};
use crate::platform::execution::{ExecutionControl, ExecutionError};

pub(crate) trait StructuralTestValue {
    fn raw_equal(&self, other: &Self, control: &ExecutionControl) -> Result<bool, ExecutionError>;
}

pub(crate) fn equal<T: StructuralTestValue + ?Sized>(
    left: &T,
    right: &T,
) -> Result<bool, ExecutionError> {
    left.raw_equal(right, &ExecutionControl::uncancelled())
}

impl StructuralTestValue for NormalizedValue {
    fn raw_equal(&self, other: &Self, control: &ExecutionControl) -> Result<bool, ExecutionError> {
        raw_structural_equal(self, other, control)
    }
}

macro_rules! wrapper {
    ($ty:ty, $variant:ident) => {
        impl StructuralTestValue for $ty {
            fn raw_equal(
                &self,
                other: &Self,
                control: &ExecutionControl,
            ) -> Result<bool, ExecutionError> {
                // These carriers clone only immutable owner handles; the
                // comparator still traverses distinct content.
                raw_structural_equal(
                    &NormalizedValue::$variant(self.clone()),
                    &NormalizedValue::$variant(other.clone()),
                    control,
                )
            }
        }
    };
}
wrapper!(NormalizedRecord, Record);
wrapper!(List, List);
wrapper!(Map, Map);

macro_rules! children {
    ($($ty:ty),* $(,)?) => {$(
        impl StructuralTestValue for $ty {
            fn raw_equal(
                &self,
                other: &Self,
                control: &ExecutionControl,
            ) -> Result<bool, ExecutionError> {
                self.as_slice().raw_equal(other.as_slice(), control)
            }
        }
    )*};
}
children!(Captures, Fields, Values);

macro_rules! leaves {
    ($($ty:ty),* $(,)?) => {$(
        impl StructuralTestValue for $ty {
            fn raw_equal(
                &self,
                other: &Self,
                control: &ExecutionControl,
            ) -> Result<bool, ExecutionError> {
                control.check()?;
                Ok(self == other)
            }
        }
    )*};
}
leaves!(
    (),
    bool,
    u8,
    u16,
    u32,
    u64,
    usize,
    i8,
    i16,
    i32,
    i64,
    isize,
    String,
    str,
    NormalizedMapKey,
    crate::platform::kernel::Name,
    crate::platform::binary64::Binary64,
    ExecutionError,
);

impl<T: StructuralTestValue + ?Sized> StructuralTestValue for &T {
    fn raw_equal(&self, other: &Self, control: &ExecutionControl) -> Result<bool, ExecutionError> {
        (**self).raw_equal(*other, control)
    }
}

impl<T: StructuralTestValue + ?Sized> StructuralTestValue for Box<T> {
    fn raw_equal(&self, other: &Self, control: &ExecutionControl) -> Result<bool, ExecutionError> {
        self.as_ref().raw_equal(other.as_ref(), control)
    }
}

impl<T: StructuralTestValue + ?Sized> StructuralTestValue for std::sync::Arc<T> {
    fn raw_equal(&self, other: &Self, control: &ExecutionControl) -> Result<bool, ExecutionError> {
        self.as_ref().raw_equal(other.as_ref(), control)
    }
}

impl<T: StructuralTestValue> StructuralTestValue for Option<T> {
    fn raw_equal(&self, other: &Self, control: &ExecutionControl) -> Result<bool, ExecutionError> {
        match (self, other) {
            (None, None) => Ok(true),
            (Some(left), Some(right)) => left.raw_equal(right, control),
            _ => Ok(false),
        }
    }
}

impl<T: StructuralTestValue> StructuralTestValue for [T] {
    fn raw_equal(&self, other: &Self, control: &ExecutionControl) -> Result<bool, ExecutionError> {
        control.check()?;
        if self.len() != other.len() {
            return Ok(false);
        }
        for (left, right) in self.iter().zip(other) {
            if !left.raw_equal(right, control)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl<T: StructuralTestValue> StructuralTestValue for Vec<T> {
    fn raw_equal(&self, other: &Self, control: &ExecutionControl) -> Result<bool, ExecutionError> {
        self.as_slice().raw_equal(other.as_slice(), control)
    }
}

impl<K: Ord + StructuralTestValue, V: StructuralTestValue> StructuralTestValue
    for std::collections::BTreeMap<K, V>
{
    fn raw_equal(&self, other: &Self, control: &ExecutionControl) -> Result<bool, ExecutionError> {
        control.check()?;
        if self.len() != other.len() {
            return Ok(false);
        }
        for ((left_key, left_value), (right_key, right_value)) in self.iter().zip(other) {
            if !left_key.raw_equal(right_key, control)?
                || !left_value.raw_equal(right_value, control)?
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl<T: StructuralTestValue, const N: usize> StructuralTestValue for [T; N] {
    fn raw_equal(&self, other: &Self, control: &ExecutionControl) -> Result<bool, ExecutionError> {
        self.as_slice().raw_equal(other.as_slice(), control)
    }
}

impl<A: StructuralTestValue, B: StructuralTestValue> StructuralTestValue for (A, B) {
    fn raw_equal(&self, other: &Self, control: &ExecutionControl) -> Result<bool, ExecutionError> {
        Ok(self.0.raw_equal(&other.0, control)? && self.1.raw_equal(&other.1, control)?)
    }
}

impl<A: StructuralTestValue, B: StructuralTestValue, C: StructuralTestValue> StructuralTestValue
    for (A, B, C)
{
    fn raw_equal(&self, other: &Self, control: &ExecutionControl) -> Result<bool, ExecutionError> {
        Ok(self.0.raw_equal(&other.0, control)?
            && self.1.raw_equal(&other.1, control)?
            && self.2.raw_equal(&other.2, control)?)
    }
}

impl<T: StructuralTestValue, E: PartialEq> StructuralTestValue for Result<T, E> {
    fn raw_equal(&self, other: &Self, control: &ExecutionControl) -> Result<bool, ExecutionError> {
        control.check()?;
        match (self, other) {
            (Ok(left), Ok(right)) => left.raw_equal(right, control),
            (Err(left), Err(right)) => Ok(left == right),
            _ => Ok(false),
        }
    }
}
