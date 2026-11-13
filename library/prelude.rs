//! The source prelude. These enums own their payloads and work with affine values.
pub use crate::integer::CheckedArithmetic;
pub use crate::iter::{IntoIterator, IterSource, Iterator, IteratorExt};
#[derive(Clone, Copy, PartialEq, Eq)]
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
