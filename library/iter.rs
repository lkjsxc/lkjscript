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
    fn fold<B, F: FnMut(B, I::Item) -> B>(self, initial: B, function: F) -> B;
    fn for_each<F: FnMut(I::Item)>(self, function: F);
    fn map<B, F: FnMut(I::Item) -> B>(self, function: F) -> Map<I, F>;
    fn filter<F: FnMut(&I::Item) -> bool>(self, predicate: F) -> Filter<I, F>;
    fn filter_map<B, F: FnMut(I::Item) -> Option<B>>(self, function: F) -> FilterMap<I, F>;
    fn inspect<F: FnMut(&I::Item)>(self, function: F) -> Inspect<I, F>;
    fn any<F: FnMut(I::Item) -> bool>(&mut self, predicate: F) -> bool;
    fn all<F: FnMut(I::Item) -> bool>(&mut self, predicate: F) -> bool;
    fn find<F: FnMut(&I::Item) -> bool>(&mut self, predicate: F) -> Option<I::Item>;
    fn position<F: FnMut(I::Item) -> bool>(&mut self, predicate: F) -> Option<usize>;
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
    fn fold<B, F: FnMut(B, I::Item) -> B>(self, initial: B, function: F) -> B {
        fold(self, initial, function)
    }
    fn for_each<F: FnMut(I::Item)>(self, function: F) {
        for_each(self, function)
    }
    fn map<B, F: FnMut(I::Item) -> B>(self, function: F) -> Map<I, F> {
        map(self, function)
    }
    fn filter<F: FnMut(&I::Item) -> bool>(self, predicate: F) -> Filter<I, F> {
        filter(self, predicate)
    }
    fn filter_map<B, F: FnMut(I::Item) -> Option<B>>(self, function: F) -> FilterMap<I, F> {
        filter_map(self, function)
    }
    fn inspect<F: FnMut(&I::Item)>(self, function: F) -> Inspect<I, F> {
        inspect(self, function)
    }
    fn any<F: FnMut(I::Item) -> bool>(&mut self, predicate: F) -> bool {
        any(self, predicate)
    }
    fn all<F: FnMut(I::Item) -> bool>(&mut self, predicate: F) -> bool {
        all(self, predicate)
    }
    fn find<F: FnMut(&I::Item) -> bool>(&mut self, predicate: F) -> Option<I::Item> {
        find(self, predicate)
    }
    fn position<F: FnMut(I::Item) -> bool>(&mut self, predicate: F) -> Option<usize> {
        position(self, predicate)
    }
}

pub struct Map<I, F> {
    iterator: I,
    function: F,
}
pub fn map<I: Iterator, B, F: FnMut(I::Item) -> B>(iterator: I, function: F) -> Map<I, F> {
    Map { iterator, function }
}
impl<I: Iterator, B, F: FnMut(I::Item) -> B> Iterator for Map<I, F> {
    type Item = B;
    fn next(&mut self) -> Option<B> {
        match self.iterator.next() {
            Some(value) => Some((self.function)(value)),
            None => None,
        }
    }
}
pub struct Filter<I, F> {
    iterator: I,
    predicate: F,
}
pub fn filter<I: Iterator, F: FnMut(&I::Item) -> bool>(iterator: I, predicate: F) -> Filter<I, F> {
    Filter {
        iterator,
        predicate,
    }
}
impl<I: Iterator, F: FnMut(&I::Item) -> bool> Iterator for Filter<I, F> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        loop {
            match self.iterator.next() {
                Some(value) => {
                    if (self.predicate)(&value) {
                        return Some(value);
                    }
                }
                None => return None,
            }
        }
    }
}
pub struct FilterMap<I, F> {
    iterator: I,
    function: F,
}
pub fn filter_map<I: Iterator, B, F: FnMut(I::Item) -> Option<B>>(
    iterator: I,
    function: F,
) -> FilterMap<I, F> {
    FilterMap { iterator, function }
}
impl<I: Iterator, B, F: FnMut(I::Item) -> Option<B>> Iterator for FilterMap<I, F> {
    type Item = B;
    fn next(&mut self) -> Option<B> {
        loop {
            match self.iterator.next() {
                Some(value) => match (self.function)(value) {
                    Some(output) => return Some(output),
                    None => (),
                },
                None => return None,
            }
        }
    }
}
pub struct Inspect<I, F> {
    iterator: I,
    function: F,
}
pub fn inspect<I: Iterator, F: FnMut(&I::Item)>(iterator: I, function: F) -> Inspect<I, F> {
    Inspect { iterator, function }
}
impl<I: Iterator, F: FnMut(&I::Item)> Iterator for Inspect<I, F> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        match self.iterator.next() {
            Some(value) => {
                (self.function)(&value);
                Some(value)
            }
            None => None,
        }
    }
}

impl<I: Iterator, B, F: FnMut(I::Item) -> B> IntoIterator for Map<I, F> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}

impl<I: Iterator, F: FnMut(&I::Item) -> bool> IntoIterator for Filter<I, F> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}

impl<I: Iterator, B, F: FnMut(I::Item) -> Option<B>> IntoIterator for FilterMap<I, F> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}

impl<I: Iterator, F: FnMut(&I::Item)> IntoIterator for Inspect<I, F> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}

/// Consume a sequence in order, carrying the checked accumulator through each call.
pub fn fold<I: Iterator, B, F: FnMut(B, I::Item) -> B>(
    mut iterator: I,
    mut accumulator: B,
    mut function: F,
) -> B {
    while let Some(value) = iterator.next() {
        accumulator = function(accumulator, value);
    }
    accumulator
}

/// Call the consumer once for each item; iterator and callback own their cleanup.
pub fn for_each<I: Iterator, F: FnMut(I::Item)>(mut iterator: I, mut function: F) {
    while let Some(value) = iterator.next() {
        function(value);
    }
}

/// Stop at the first true predicate or None, leaving later items in the iterator.
pub fn any<I: Iterator, F: FnMut(I::Item) -> bool>(iterator: &mut I, mut predicate: F) -> bool {
    while let Some(value) = iterator.next() {
        if predicate(value) {
            return true;
        }
    }
    false
}

/// Stop at the first false predicate or None; an empty traversal is true.
pub fn all<I: Iterator, F: FnMut(I::Item) -> bool>(iterator: &mut I, mut predicate: F) -> bool {
    while let Some(value) = iterator.next() {
        if !predicate(value) {
            return false;
        }
    }
    true
}

/// Owned iteration reverses the storage once so every next is a constant-time pop.
/// Remaining items are explicitly destroyed in source order when iteration ends.
pub struct IntoIter<T> {
    items: Vec<T>,
}
pub fn owned<T>(mut values: Vec<T>) -> IntoIter<T> {
    let mut items: Vec<T> = Vec::new();
    while values.len() > 0 {
        items.push(values.pop().unwrap());
    }
    IntoIter { items }
}
impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.items.len() == 0 {
            None
        } else {
            Some(self.items.pop().unwrap())
        }
    }
}
impl<T> IntoIterator for IntoIter<T> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}
impl<T> IntoIterator for Vec<T> {
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        owned(self)
    }
}
impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        while self.items.len() > 0 {
            self.items.pop();
        }
    }
}

/// Search consumes each visited item, including the matching item.
pub fn find<I: Iterator, F: FnMut(&I::Item) -> bool>(
    iterator: &mut I,
    mut predicate: F,
) -> Option<I::Item> {
    loop {
        match iterator.next() {
            Some(value) => {
                if predicate(&value) {
                    return Some(value);
                }
            }
            None => return None,
        }
    }
}
pub fn position<I: Iterator, F: FnMut(I::Item) -> bool>(
    iterator: &mut I,
    mut predicate: F,
) -> Option<usize> {
    let mut index = 0usize;
    loop {
        match iterator.next() {
            Some(value) => {
                if predicate(value) {
                    return Some(index);
                }
                index += 1;
            }
            None => return None,
        }
    }
}

pub struct OptionIter<T> {
    value: Option<T>,
}
impl<T> Iterator for OptionIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.value.take()
    }
}
impl<T> IntoIterator for Option<T> {
    type IntoIter = OptionIter<T>;
    fn into_iter(self) -> OptionIter<T> {
        OptionIter { value: self }
    }
}
impl<T> IntoIterator for OptionIter<T> {
    type IntoIter = Self;
    fn into_iter(self) -> Self {
        self
    }
}
