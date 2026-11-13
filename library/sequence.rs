//! Vec operations are ordinary source over checked element access.
use crate::prelude::Option;
pub fn __lkj_vec_compare<T: Ord>(left: &Vec<T>, right: &Vec<T>) -> i32 {
    let mut i: usize = 0;
    while i < left.len() && i < right.len() {
        if left[i] < right[i] {
            return -1;
        }
        if left[i] > right[i] {
            return 1;
        }
        i = i + 1;
    }
    if left.len() < right.len() {
        return -1;
    }
    if left.len() > right.len() {
        return 1;
    }
    0
}

pub fn __lkj_vec_clone<T: Clone>(source: &Vec<T>) -> Vec<T> {
    let mut result: Vec<T> = Vec::new();
    let mut index: usize = 0;
    while index < source.len() {
        result.push(source[index].clone());
        index = index + 1;
    }
    result
}

pub fn __lkj_vec_equal<T: PartialEq>(left: &Vec<T>, right: &Vec<T>) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut index: usize = 0;
    while index < left.len() {
        // Equality uses the element's eq contract; ne may be independently overridden.
        if !(left[index] == right[index]) {
            return false;
        }
        index = index + 1;
    }
    true
}

// Clone the first count-1 elements and move the original into the last slot.
// The element expression is evaluated even when count is zero.
pub fn __lkj_vec_repeat<T: Clone>(value: T, count: usize) -> Vec<T> {
    let mut result: Vec<T> = Vec::new();
    if count == 0 {
        return result;
    }
    let mut remaining = count - 1;
    while remaining > 0 {
        result.push(value.clone());
        remaining = remaining - 1;
    }
    result.push(value);
    result
}

/// Shared inspection does not require an element equality or copying contract.
pub trait VecAccess<T> {
    fn is_empty(&self) -> bool;
    fn get<'a>(&'a self, index: usize) -> Option<&'a T>;
    fn first<'a>(&'a self) -> Option<&'a T>;
}

impl<T> VecAccess<T> for Vec<T> {
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn get<'a>(&'a self, index: usize) -> Option<&'a T> {
        if index < self.len() {
            Option::Some(&self[index])
        } else {
            Option::None
        }
    }
    fn first<'a>(&'a self) -> Option<&'a T> {
        self.get(0)
    }
}

/// Equality search borrows its elements and stops at the first match.
pub trait VecContains<T: PartialEq> {
    fn contains(&self, value: &T) -> bool;
}

impl<T: PartialEq> VecContains<T> for Vec<T> {
    fn contains(&self, value: &T) -> bool {
        let mut index: usize = 0;
        while index < self.len() {
            if self[index] == *value {
                return true;
            }
            index = index + 1;
        }
        false
    }
}

/// Append independent clones in source order; the shared source remains usable.
pub trait VecExtend<T: Clone> {
    fn extend_from_slice(&mut self, values: &[T]);
}

impl<T: Clone> VecExtend<T> for Vec<T> {
    fn extend_from_slice(&mut self, values: &[T]) {
        let mut index: usize = 0;
        while index < values.len() {
            self.push(values[index].clone());
            index = index + 1;
        }
    }
}
