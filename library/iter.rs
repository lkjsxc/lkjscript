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

pub struct Enumerate<I> {
    iterator: I,
    index: usize,
}
pub fn enumerate<I: Iterator>(iterator: I) -> Enumerate<I> {
    Enumerate { iterator, index: 0 }
}
impl<I: Iterator> Iterator for Enumerate<I> {
    type Item = (usize, I::Item);
    fn next(&mut self) -> Option<Self::Item> {
        match self.iterator.next() {
            Some(value) => {
                let index = self.index;
                self.index = index + 1;
                Some((index, value))
            }
            None => None,
        }
    }
}
impl<I: Iterator> IntoIterator for Enumerate<I> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}

pub struct Take<I> {
    iterator: I,
    remaining: usize,
}
pub fn take<I: Iterator>(iterator: I, remaining: usize) -> Take<I> {
    Take {
        iterator,
        remaining,
    }
}
impl<I: Iterator> Iterator for Take<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining = self.remaining - 1;
        self.iterator.next()
    }
}
impl<I: Iterator> IntoIterator for Take<I> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}

pub struct Skip<I> {
    iterator: I,
    remaining: usize,
}
pub fn skip<I: Iterator>(iterator: I, remaining: usize) -> Skip<I> {
    Skip {
        iterator,
        remaining,
    }
}
impl<I: Iterator> Iterator for Skip<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        while self.remaining > 0 {
            self.remaining = self.remaining - 1;
            match self.iterator.next() {
                Some(_) => (),
                None => {
                    self.remaining = 0;
                    return None;
                }
            }
        }
        self.iterator.next()
    }
}
impl<I: Iterator> IntoIterator for Skip<I> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}

pub struct Zip<I, J> {
    left: I,
    right: J,
}
pub fn zip<I: Iterator, J: IntoIterator>(left: I, right: J) -> Zip<I, J::IntoIter> {
    Zip {
        left,
        right: right.into_iter(),
    }
}
impl<I: Iterator, J: Iterator> Iterator for Zip<I, J> {
    type Item = (I::Item, J::Item);
    fn next(&mut self) -> Option<Self::Item> {
        match self.left.next() {
            Some(left) => match self.right.next() {
                Some(right) => Some((left, right)),
                None => None,
            },
            None => None,
        }
    }
}
impl<I: Iterator, J: Iterator> IntoIterator for Zip<I, J> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}

// Until associated-type equality is available, two identical iterator types
// establish Item equality without an unchecked conversion or hidden cast.
pub struct Chain<I> {
    left: Option<I>,
    right: I,
}
pub fn chain<I: Iterator>(left: I, right: I) -> Chain<I> {
    Chain {
        left: Some(left),
        right,
    }
}
impl<I: Iterator> Iterator for Chain<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.left {
            Some(left) => match left.next() {
                Some(value) => return Some(value),
                None => (),
            },
            None => (),
        }
        self.left = None;
        self.right.next()
    }
}
impl<I: Iterator> IntoIterator for Chain<I> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}

pub trait IteratorExt<I: Iterator> {
    fn enumerate(self) -> Enumerate<I>;
    fn take(self, remaining: usize) -> Take<I>;
    fn skip(self, remaining: usize) -> Skip<I>;
    fn zip<J: IntoIterator>(self, right: J) -> Zip<I, J::IntoIter>;
    fn chain(self, right: I) -> Chain<I>;
}
impl<I: Iterator> IteratorExt<I> for I {
    fn enumerate(self) -> Enumerate<I> {
        enumerate(self)
    }
    fn take(self, remaining: usize) -> Take<I> {
        take(self, remaining)
    }
    fn skip(self, remaining: usize) -> Skip<I> {
        skip(self, remaining)
    }
    fn zip<J: IntoIterator>(self, right: J) -> Zip<I, J::IntoIter> {
        zip(self, right)
    }
    fn chain(self, right: I) -> Chain<I> {
        chain(self, right)
    }
}
