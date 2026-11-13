//! Shared iteration is ordinary source over a borrowed slice and a cursor.
use crate::prelude::Option;
use crate::prelude::Option::{None, Some};
pub trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}

// Item is defined once, by the iterator. Associated equality syntax is not
// needed to make the two protocols agree.
pub trait IntoIterator {
    type IntoIter: Iterator;
    fn into_iter(self) -> Self::IntoIter;
}

pub struct Iter<'a, T> {
    items: &'a [T],
    index: usize,
}

pub fn shared<'a, T>(items: &'a [T]) -> Iter<'a, T> {
    Iter { items, index: 0 }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        if self.index == self.items.len() {
            return None;
        }
        let index = self.index;
        self.index = index + 1;
        Some(&self.items[index])
    }
}

impl<'a, T> IntoIterator for Iter<'a, T> {
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self
    }
}
impl<'a, T> IntoIterator for &'a [T] {
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        shared(self)
    }
}
impl<'a, T> IntoIterator for &'a Vec<T> {
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        shared(self.as_slice())
    }
}

// Structural for lowering resolves these declarations by library identity.
// Both remain ordinary generic source functions, checked like user functions.
pub fn __lkj_into_iter<I: IntoIterator>(value: I) -> I::IntoIter {
    value.into_iter()
}
pub fn __lkj_next<I: Iterator>(iterator: &mut I) -> Option<I::Item> {
    iterator.next()
}

/// A source extension for collections whose storage can be borrowed as a slice.
/// The returned iterator borrows the collection for its declared lifetime.
pub trait IterSource<T> {
    fn iter<'a>(&'a self) -> Iter<'a, T>;
}
impl<T> IterSource<T> for Vec<T> {
    fn iter<'a>(&'a self) -> Iter<'a, T> {
        shared(self.as_slice())
    }
}
impl<T> IterSource<T> for [T] {
    fn iter<'a>(&'a self) -> Iter<'a, T> {
        shared(self)
    }
}
