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

/// Mutation visits values in their original order and keeps affine ownership.
pub trait VecMutate<T> {
    fn retain<F: FnMut(&T) -> bool>(&mut self, predicate: F);
}
impl<T> VecMutate<T> for Vec<T> {
    // Explicit replacement uses the declared primitive; mem::take is not provided.
    #[allow(clippy::mem_replace_with_default)]
    fn retain<F: FnMut(&T) -> bool>(&mut self, mut predicate: F) {
        let mut remaining = crate::iter::owned(std::mem::replace(self, Vec::new()));
        while let Option::Some(value) = crate::iter::__lkj_next(&mut remaining) {
            if predicate(&value) {
                self.push(value);
            }
        }
    }
}

/// Stable merge sorting requires only checked indexing and owned Option slots.
/// Each pass is linear; the number of passes grows logarithmically with length.
pub trait VecSort<T: Ord> {
    fn sort(&mut self);
}
impl<T: Ord> VecSort<T> for Vec<T> {
    #[allow(clippy::mem_replace_with_default)]
    fn sort(&mut self) {
        let mut remaining = crate::iter::owned(std::mem::replace(self, Vec::new()));
        let mut values: Vec<Option<T>> = Vec::new();
        while let Option::Some(value) = crate::iter::__lkj_next(&mut remaining) {
            values.push(Option::Some(value));
        }
        let count = values.len();
        let mut width = 1usize;
        while width < count {
            let mut merged: Vec<Option<T>> = Vec::new();
            let mut start = 0usize;
            while start < count {
                let left_count = if width < count - start {
                    width
                } else {
                    count - start
                };
                let middle = start + left_count;
                let right_count = if width < count - middle {
                    width
                } else {
                    count - middle
                };
                let end = middle + right_count;
                let mut left = start;
                let mut right = middle;
                while left < middle || right < end {
                    let use_left = if left == middle {
                        false
                    } else if right == end {
                        true
                    } else {
                        *values[left].as_ref().unwrap() <= *values[right].as_ref().unwrap()
                    };
                    if use_left {
                        merged.push(values[left].take());
                        left = left + 1;
                    } else {
                        merged.push(values[right].take());
                        right = right + 1;
                    }
                }
                start = end;
            }
            values = merged;
            width = if width > count / 2 { count } else { width * 2 };
        }
        let mut index = 0usize;
        while index < count {
            self.push(values[index].take().unwrap());
            index = index + 1;
        }
    }
}
