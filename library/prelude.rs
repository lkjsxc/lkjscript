//! The source prelude. These enums own their payloads and work with affine values.
pub use crate::integer::{ByteAscii, CheckedArithmetic, ScalarOrder};
pub use crate::iter::{IntoIterator, IterSource, Iterator, IteratorExt};
pub use crate::sequence::{
    SequencePrefix, SliceClone, SliceCopy, SliceSplitSource, VecAccess, VecAccessMut, VecClear,
    VecContains, VecExtend, VecMutate, VecSort,
};
pub use crate::str::{ParseInteger, TextView};
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Option<T> {
    None,
    Some(T),
}

impl<T> Option<T> {
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            Option::Some(value) => Option::Some(value),
            Option::None => Option::None,
        }
    }
    pub fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            Option::Some(value) => Option::Some(value),
            Option::None => Option::None,
        }
    }
    pub fn expect(self, _message: &str) -> T {
        self.unwrap()
    }
    pub fn and<U>(self, other: Option<U>) -> Option<U> {
        match self {
            Option::Some(_) => other,
            Option::None => Option::None,
        }
    }
    pub fn or(self, other: Option<T>) -> Option<T> {
        match self {
            Option::Some(value) => Option::Some(value),
            Option::None => other,
        }
    }
    pub fn xor(self, other: Option<T>) -> Option<T> {
        match (self, other) {
            (Option::Some(value), Option::None) | (Option::None, Option::Some(value)) => {
                Option::Some(value)
            }
            _ => Option::None,
        }
    }
    pub fn zip<U>(self, other: Option<U>) -> Option<(T, U)> {
        match (self, other) {
            (Option::Some(left), Option::Some(right)) => Option::Some((left, right)),
            _ => Option::None,
        }
    }
    pub fn replace(&mut self, value: T) -> Option<T> {
        std::mem::replace(self, Option::Some(value))
    }
    pub fn insert(&mut self, value: T) -> &mut T {
        *self = Option::Some(value);
        match self {
            Option::Some(value) => value,
            Option::None => unreachable!(),
        }
    }
    pub fn get_or_insert(&mut self, value: T) -> &mut T {
        if self.is_none() {
            *self = Option::Some(value);
        }
        match self {
            Option::Some(value) => value,
            Option::None => unreachable!(),
        }
    }

    pub fn is_some(&self) -> bool {
        match self {
            Option::None => false,
            Option::Some(_) => true,
        }
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    pub fn is_some_and<F: FnOnce(T) -> bool>(self, predicate: F) -> bool {
        match self {
            Option::Some(value) => predicate(value),
            Option::None => false,
        }
    }

    pub fn is_none_or<F: FnOnce(T) -> bool>(self, predicate: F) -> bool {
        match self {
            Option::Some(value) => predicate(value),
            Option::None => true,
        }
    }

    pub fn ok_or<E>(self, error: E) -> Result<T, E> {
        match self {
            Option::Some(value) => Result::Ok(value),
            Option::None => Result::Err(error),
        }
    }

    pub fn ok_or_else<E, F: FnOnce() -> E>(self, error: F) -> Result<T, E> {
        match self {
            Option::Some(value) => Result::Ok(value),
            Option::None => Result::Err(error()),
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            Option::Some(value) => return value,
            Option::None => panic!("called Option::unwrap on None"),
        }
    }

    pub fn take(&mut self) -> Option<T> {
        std::mem::replace(self, Option::None)
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Option::Some(value) => return value,
            Option::None => return default,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Result<T, E> {
    Ok(T),
    Err(E),
}

impl<T, E> Result<T, E> {
    pub fn as_ref(&self) -> Result<&T, &E> {
        match self {
            Result::Ok(value) => Result::Ok(value),
            Result::Err(error) => Result::Err(error),
        }
    }
    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> {
        match self {
            Result::Ok(value) => Result::Ok(value),
            Result::Err(error) => Result::Err(error),
        }
    }
    pub fn and<U>(self, other: Result<U, E>) -> Result<U, E> {
        match self {
            Result::Ok(_) => other,
            Result::Err(error) => Result::Err(error),
        }
    }
    pub fn or<F>(self, other: Result<T, F>) -> Result<T, F> {
        match self {
            Result::Ok(value) => Result::Ok(value),
            Result::Err(_) => other,
        }
    }

    pub fn is_ok(&self) -> bool {
        match self {
            Result::Ok(_) => true,
            Result::Err(_) => false,
        }
    }

    pub fn is_err(&self) -> bool {
        !self.is_ok()
    }

    pub fn unwrap(self) -> T {
        match self {
            Result::Ok(value) => return value,
            Result::Err(_) => panic!("called Result::unwrap on Err"),
        }
    }

    pub fn unwrap_err(self) -> E {
        match self {
            Result::Err(value) => return value,
            Result::Ok(_) => panic!("called Result::unwrap_err on Ok"),
        }
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Result::Ok(value) => return value,
            Result::Err(_) => return default,
        }
    }

    pub fn ok(self) -> Option<T> {
        match self {
            Result::Ok(value) => return Option::Some(value),
            Result::Err(_) => return Option::None,
        }
    }

    pub fn err(self) -> Option<E> {
        match self {
            Result::Err(value) => return Option::Some(value),
            Result::Ok(_) => return Option::None,
        }
    }
}

impl<T> Option<Option<T>> {
    pub fn flatten(self) -> Option<T> {
        match self {
            Option::Some(value) => value,
            Option::None => Option::None,
        }
    }
}
impl<T, E> Option<Result<T, E>> {
    pub fn transpose(self) -> Result<Option<T>, E> {
        match self {
            Option::Some(Result::Ok(value)) => Result::Ok(Option::Some(value)),
            Option::Some(Result::Err(error)) => Result::Err(error),
            Option::None => Result::Ok(Option::None),
        }
    }
}
impl<T, E> Result<Option<T>, E> {
    pub fn transpose(self) -> Option<Result<T, E>> {
        match self {
            Result::Ok(Option::Some(value)) => Option::Some(Result::Ok(value)),
            Result::Ok(Option::None) => Option::None,
            Result::Err(error) => Option::Some(Result::Err(error)),
        }
    }
}
impl<T, U> Option<(T, U)> {
    pub fn unzip(self) -> (Option<T>, Option<U>) {
        match self {
            Option::Some((left, right)) => (Option::Some(left), Option::Some(right)),
            Option::None => (Option::None, Option::None),
        }
    }
}
impl<T> Option<T> {
    pub fn map<U, F: FnOnce(T) -> U>(self, function: F) -> Option<U> {
        match self {
            Option::Some(value) => Option::Some(function(value)),
            Option::None => Option::None,
        }
    }
    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, function: F) -> U {
        match self {
            Option::Some(value) => function(value),
            Option::None => default,
        }
    }
    pub fn map_or_else<U, D: FnOnce() -> U, F: FnOnce(T) -> U>(self, default: D, function: F) -> U {
        match self {
            Option::Some(value) => function(value),
            Option::None => default(),
        }
    }
    pub fn and_then<U, F: FnOnce(T) -> Option<U>>(self, function: F) -> Option<U> {
        match self {
            Option::Some(value) => function(value),
            Option::None => Option::None,
        }
    }
    pub fn or_else<F: FnOnce() -> Option<T>>(self, function: F) -> Option<T> {
        match self {
            Option::Some(value) => Option::Some(value),
            Option::None => function(),
        }
    }
    pub fn unwrap_or_else<F: FnOnce() -> T>(self, function: F) -> T {
        match self {
            Option::Some(value) => value,
            Option::None => function(),
        }
    }
    pub fn filter<P: FnOnce(&T) -> bool>(self, predicate: P) -> Option<T> {
        match self {
            Option::Some(value) => {
                if predicate(&value) {
                    Option::Some(value)
                } else {
                    Option::None
                }
            }
            Option::None => Option::None,
        }
    }
}

impl<T, E> Result<T, E> {
    pub fn map<U, F: FnOnce(T) -> U>(self, function: F) -> Result<U, E> {
        match self {
            Result::Ok(value) => Result::Ok(function(value)),
            Result::Err(error) => Result::Err(error),
        }
    }
    pub fn map_err<U, F: FnOnce(E) -> U>(self, function: F) -> Result<T, U> {
        match self {
            Result::Ok(value) => Result::Ok(value),
            Result::Err(error) => Result::Err(function(error)),
        }
    }
    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, function: F) -> U {
        match self {
            Result::Ok(value) => function(value),
            Result::Err(_) => default,
        }
    }
    pub fn map_or_else<U, D: FnOnce(E) -> U, F: FnOnce(T) -> U>(
        self,
        default: D,
        function: F,
    ) -> U {
        match self {
            Result::Ok(value) => function(value),
            Result::Err(error) => default(error),
        }
    }
    pub fn and_then<U, F: FnOnce(T) -> Result<U, E>>(self, function: F) -> Result<U, E> {
        match self {
            Result::Ok(value) => function(value),
            Result::Err(error) => Result::Err(error),
        }
    }
    pub fn or_else<U, F: FnOnce(E) -> Result<T, U>>(self, function: F) -> Result<T, U> {
        match self {
            Result::Ok(value) => Result::Ok(value),
            Result::Err(error) => function(error),
        }
    }
    pub fn unwrap_or_else<F: FnOnce(E) -> T>(self, function: F) -> T {
        match self {
            Result::Ok(value) => value,
            Result::Err(error) => function(error),
        }
    }
}

impl<'a, T: Copy> Option<&'a T> {
    pub fn copied(self) -> Option<T> {
        match self {
            Option::Some(value) => Option::Some(*value),
            Option::None => Option::None,
        }
    }
}

impl<'a, T: Clone> Option<&'a T> {
    pub fn cloned(self) -> Option<T> {
        match self {
            Option::Some(value) => Option::Some((*value).clone()),
            Option::None => Option::None,
        }
    }
}

impl<'a, T: Copy> Option<&'a mut T> {
    pub fn copied(self) -> Option<T> {
        match self {
            Option::Some(value) => Option::Some(*value),
            Option::None => Option::None,
        }
    }
}

impl<'a, T: Clone> Option<&'a mut T> {
    pub fn cloned(self) -> Option<T> {
        match self {
            Option::Some(value) => Option::Some((*value).clone()),
            Option::None => Option::None,
        }
    }
}

/// An ordinary source contract for a canonical initial value.
pub trait Default {
    fn default() -> Self;
}

impl Default for bool {
    fn default() -> bool {
        false
    }
}

impl Default for char {
    fn default() -> char {
        '\0'
    }
}

impl Default for u8 {
    fn default() -> u8 {
        0u8
    }
}

impl Default for u16 {
    fn default() -> u16 {
        0u16
    }
}

impl Default for u32 {
    fn default() -> u32 {
        0u32
    }
}

impl Default for u64 {
    fn default() -> u64 {
        0u64
    }
}

impl Default for usize {
    fn default() -> usize {
        0usize
    }
}

impl Default for i8 {
    fn default() -> i8 {
        0i8
    }
}

impl Default for i16 {
    fn default() -> i16 {
        0i16
    }
}

impl Default for i32 {
    fn default() -> i32 {
        0i32
    }
}

impl Default for i64 {
    fn default() -> i64 {
        0i64
    }
}

impl Default for isize {
    fn default() -> isize {
        0isize
    }
}

impl Default for f64 {
    fn default() -> f64 {
        0.0
    }
}

impl<T> Default for Vec<T> {
    fn default() -> Vec<T> {
        Vec::new()
    }
}
impl<T> Default for Option<T> {
    fn default() -> Option<T> {
        Option::None
    }
}
impl Default for crate::string::String {
    fn default() -> crate::string::String {
        crate::string::String::new()
    }
}

// Derive expansion resolves this declaration by standard-library identity.
pub fn __lkj_default<T: Default>() -> T {
    T::default()
}
