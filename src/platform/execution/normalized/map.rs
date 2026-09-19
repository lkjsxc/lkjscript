//! Neutral persistent ordered maps. Storage sharing conveys no type or affine authority.
//!
//! AVL path copying retains immutable entry handles, including keys and payloads. An
//! edit allocates only its search/rotation paths. Traversal and codecs see key order,
//! never tree shape. Raw ingress is admitted independently by each evaluator.

use super::value::{NormalizedMapKey, NormalizedValue, release_raw_values};
use crate::platform::execution::{ExecutionError, ExecutionFailureClass};
use std::collections::BTreeMap;
use std::sync::Arc;

pub(super) const MAXIMUM_LENGTH: usize = 1_000_000;
// An AVL tree with at most one million entries has height below 32.
const MAXIMUM_HEIGHT: usize = 32;
type Link = Option<Arc<Node>>;

struct Entry {
    key: NormalizedMapKey,
    value: NormalizedValue,
}

struct Node {
    entry: Arc<Entry>,
    left: Link,
    right: Link,
    height: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Charge {
    pub slots: u64,
    pub bytes: u64,
}

#[derive(Clone, Default)]
pub struct Map {
    length: usize,
    root: Link,
}

fn storage_error() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Resource,
        "normalized_map_storage",
        "persistent map exceeds its checked length, height, or storage bound",
    )
}

fn shape_error() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "normalized_map_shape",
        "persistent map has an invalid internal shape",
    )
}

fn height(link: &Link) -> usize {
    link.as_ref().map_or(0, |node| node.height)
}

fn node_bytes() -> u64 {
    (std::mem::size_of::<Node>() + 2 * std::mem::size_of::<usize>()) as u64
}

fn entry_bytes() -> u64 {
    (std::mem::size_of::<Entry>() + 2 * std::mem::size_of::<usize>()) as u64
}

fn key_bytes(key: &NormalizedMapKey) -> usize {
    match key {
        NormalizedMapKey::Bytes(value) => value.len(),
        NormalizedMapKey::Text(value) => value.len(),
        NormalizedMapKey::Bool(_) | NormalizedMapKey::I64(_) => 0,
    }
}

fn entry(
    value: Entry,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<Arc<Entry>, ExecutionError> {
    let bytes = entry_bytes()
        .checked_add(key_bytes(&value.key) as u64)
        .ok_or_else(storage_error)?;
    reserve(Charge { slots: 0, bytes })?;
    Ok(Arc::new(value))
}

fn node(
    entry: Arc<Entry>,
    left: Link,
    right: Link,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<Arc<Node>, ExecutionError> {
    let height = height(&left)
        .max(height(&right))
        .checked_add(1)
        .filter(|height| *height <= MAXIMUM_HEIGHT)
        .ok_or_else(storage_error)?;
    reserve(Charge {
        slots: 1,
        bytes: node_bytes(),
    })?;
    Ok(Arc::new(Node {
        entry,
        left,
        right,
        height,
    }))
}

fn balance(
    entry: Arc<Entry>,
    left: Link,
    right: Link,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<Arc<Node>, ExecutionError> {
    if height(&left) > height(&right).saturating_add(1) {
        let pivot = left.as_ref().ok_or_else(shape_error)?;
        if height(&pivot.left) >= height(&pivot.right) {
            let right = node(entry, pivot.right.clone(), right, reserve)?;
            return node(pivot.entry.clone(), pivot.left.clone(), Some(right), reserve);
        }
        let middle = pivot.right.as_ref().ok_or_else(shape_error)?;
        let left = node(
            pivot.entry.clone(),
            pivot.left.clone(),
            middle.left.clone(),
            reserve,
        )?;
        let right = node(entry, middle.right.clone(), right, reserve)?;
        return node(middle.entry.clone(), Some(left), Some(right), reserve);
    }
    if height(&right) > height(&left).saturating_add(1) {
        let pivot = right.as_ref().ok_or_else(shape_error)?;
        if height(&pivot.right) >= height(&pivot.left) {
            let left = node(entry, left, pivot.left.clone(), reserve)?;
            return node(pivot.entry.clone(), Some(left), pivot.right.clone(), reserve);
        }
        let middle = pivot.left.as_ref().ok_or_else(shape_error)?;
        let left = node(entry, left, middle.left.clone(), reserve)?;
        let right = node(
            pivot.entry.clone(),
            middle.right.clone(),
            pivot.right.clone(),
            reserve,
        )?;
        return node(middle.entry.clone(), Some(left), Some(right), reserve);
    }
    node(entry, left, right, reserve)
}

fn insert(
    root: &Link,
    added: Arc<Entry>,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<Arc<Node>, ExecutionError> {
    let Some(root) = root else {
        return node(added, None, None, reserve);
    };
    match added.key.cmp(&root.entry.key) {
        std::cmp::Ordering::Less => {
            let left = insert(&root.left, added, reserve)?;
            balance(root.entry.clone(), Some(left), root.right.clone(), reserve)
        }
        std::cmp::Ordering::Greater => {
            let right = insert(&root.right, added, reserve)?;
            balance(root.entry.clone(), root.left.clone(), Some(right), reserve)
        }
        std::cmp::Ordering::Equal => node(added, root.left.clone(), root.right.clone(), reserve),
    }
}

fn remove_first(
    root: &Arc<Node>,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<(Arc<Entry>, Link), ExecutionError> {
    let Some(left) = &root.left else {
        return Ok((root.entry.clone(), root.right.clone()));
    };
    let (entry, left) = remove_first(left, reserve)?;
    let root = balance(root.entry.clone(), left, root.right.clone(), reserve)?;
    Ok((entry, Some(root)))
}

fn remove(
    root: &Link,
    key: &NormalizedMapKey,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<Link, ExecutionError> {
    let root = root.as_ref().ok_or_else(shape_error)?;
    match key.cmp(&root.entry.key) {
        std::cmp::Ordering::Less => {
            let left = remove(&root.left, key, reserve)?;
            balance(root.entry.clone(), left, root.right.clone(), reserve).map(Some)
        }
        std::cmp::Ordering::Greater => {
            let right = remove(&root.right, key, reserve)?;
            balance(root.entry.clone(), root.left.clone(), right, reserve).map(Some)
        }
        std::cmp::Ordering::Equal => match (&root.left, &root.right) {
            (None, _) => Ok(root.right.clone()),
            (_, None) => Ok(root.left.clone()),
            (Some(_), Some(right)) => {
                let (entry, right) = remove_first(right, reserve)?;
                balance(entry, root.left.clone(), right, reserve).map(Some)
            }
        },
    }
}

// Own all unadmitted payloads while a bounded bulk construction can still fail.
struct RawEntries(std::collections::btree_map::IntoIter<NormalizedMapKey, NormalizedValue>);

impl Drop for RawEntries {
    fn drop(&mut self) {
        release_raw_values(self.0.by_ref().map(|(_, value)| value).collect());
    }
}

fn sorted(
    entries: &mut RawEntries,
    count: usize,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<Link, ExecutionError> {
    if count == 0 {
        return Ok(None);
    }
    let left_count = count / 2;
    let left = sorted(entries, left_count, reserve)?;
    let (key, value) = entries.0.next().ok_or_else(shape_error)?;
    let entry = entry(Entry { key, value }, reserve)?;
    let right = sorted(entries, count - left_count - 1, reserve)?;
    node(entry, left, right, reserve).map(Some)
}

impl Map {
    pub(super) fn from_items(
        entries: BTreeMap<NormalizedMapKey, NormalizedValue>,
        reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        let length = entries.len();
        let mut entries = RawEntries(entries.into_iter());
        if length > MAXIMUM_LENGTH {
            return Err(storage_error());
        }
        reserve(Charge::default())?;
        let root = sorted(&mut entries, length, reserve)?;
        Ok(Self { length, root })
    }

    pub(super) fn len(&self) -> usize {
        self.length
    }

    pub(super) fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub(super) fn get(&self, key: &NormalizedMapKey) -> Option<&NormalizedValue> {
        let mut current = self.root.as_deref();
        while let Some(node) = current {
            current = match key.cmp(&node.entry.key) {
                std::cmp::Ordering::Less => node.left.as_deref(),
                std::cmp::Ordering::Greater => node.right.as_deref(),
                std::cmp::Ordering::Equal => return Some(&node.entry.value),
            };
        }
        None
    }

    pub(super) fn contains_key(&self, key: &NormalizedMapKey) -> bool {
        self.get(key).is_some()
    }

    pub(super) fn insert(
        &self,
        key: NormalizedMapKey,
        value: NormalizedValue,
        maximum_length: u64,
        reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        let added = Entry { key, value };
        reserve(Charge::default())?;
        let length = self
            .length
            .checked_add(usize::from(!self.contains_key(&added.key)))
            .filter(|length| *length <= MAXIMUM_LENGTH && *length as u64 <= maximum_length)
            .ok_or_else(storage_error)?;
        let added = entry(added, reserve)?;
        let root = insert(&self.root, added, reserve)?;
        Ok(Self {
            length,
            root: Some(root),
        })
    }

    pub(super) fn remove(
        &self,
        key: &NormalizedMapKey,
        reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        reserve(Charge::default())?;
        if !self.contains_key(key) {
            return Ok(self.clone());
        }
        let length = self.length.checked_sub(1).ok_or_else(shape_error)?;
        let root = remove(&self.root, key, reserve)?;
        Ok(Self { length, root })
    }

    pub(super) fn iter(&self) -> Iter<'_> {
        Iter {
            forward: Cursor::new(self.root.as_deref(), false),
            backward: Cursor::new(self.root.as_deref(), true),
            remaining: self.length,
        }
    }

    pub(super) fn keys(
        &self,
    ) -> impl DoubleEndedIterator<Item = &NormalizedMapKey> + ExactSizeIterator {
        self.iter().map(|(key, _)| key)
    }

    pub(super) fn values(
        &self,
    ) -> impl DoubleEndedIterator<Item = &NormalizedValue> + ExactSizeIterator {
        self.iter().map(|(_, value)| value)
    }

    // Admission already charges each logical (key, value) occurrence. These bytes
    // account for the additional neutral node and entry reference-count storage.
    pub(super) fn metadata_bytes(&self) -> Result<u64, ExecutionError> {
        let overhead = node_bytes()
            .checked_add((2 * std::mem::size_of::<usize>()) as u64)
            .ok_or_else(storage_error)?;
        (self.length as u64)
            .checked_mul(overhead)
            .ok_or_else(storage_error)
    }

    pub(super) fn drain_unique(&mut self, values: &mut Vec<NormalizedValue>) {
        let mut pending = Vec::new();
        pending.extend(self.root.take());
        self.length = 0;
        while let Some(node) = pending.pop() {
            if let Some(node) = Arc::into_inner(node) {
                pending.extend(node.left);
                pending.extend(node.right);
                if let Some(mut entry) = Arc::into_inner(node.entry) {
                    values.push(std::mem::replace(&mut entry.value, NormalizedValue::Unit));
                }
            }
        }
    }
}

impl Drop for Entry {
    fn drop(&mut self) {
        if !matches!(self.value, NormalizedValue::Unit) {
            release_raw_values(vec![std::mem::replace(&mut self.value, NormalizedValue::Unit)]);
        }
    }
}

impl Drop for Map {
    fn drop(&mut self) {
        let mut values = Vec::new();
        self.drain_unique(&mut values);
        release_raw_values(values);
    }
}

impl std::fmt::Debug for Map {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_map().entries(self.iter()).finish()
    }
}

impl PartialEq for Map {
    fn eq(&self, other: &Self) -> bool {
        self.length == other.length && self.iter().eq(other.iter())
    }
}
impl Eq for Map {}

struct Cursor<'a> {
    stack: [Option<&'a Node>; MAXIMUM_HEIGHT],
    depth: usize,
    next: Option<&'a Node>,
    reverse: bool,
}

impl<'a> Cursor<'a> {
    fn new(root: Option<&'a Node>, reverse: bool) -> Self {
        Self {
            stack: [None; MAXIMUM_HEIGHT],
            depth: 0,
            next: root,
            reverse,
        }
    }

    fn next(&mut self) -> Option<(&'a NormalizedMapKey, &'a NormalizedValue)> {
        while let Some(node) = self.next.take() {
            *self.stack.get_mut(self.depth)? = Some(node);
            self.depth += 1;
            self.next = if self.reverse {
                node.right.as_deref()
            } else {
                node.left.as_deref()
            };
        }
        self.depth = self.depth.checked_sub(1)?;
        let node = self.stack.get_mut(self.depth)?.take()?;
        self.next = if self.reverse {
            node.left.as_deref()
        } else {
            node.right.as_deref()
        };
        Some((&node.entry.key, &node.entry.value))
    }
}

pub(super) struct Iter<'a> {
    forward: Cursor<'a>,
    backward: Cursor<'a>,
    remaining: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a NormalizedMapKey, &'a NormalizedValue);
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let item = self.forward.next()?;
        self.remaining -= 1;
        Some(item)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl DoubleEndedIterator for Iter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let item = self.backward.next()?;
        self.remaining -= 1;
        Some(item)
    }
}
impl ExactSizeIterator for Iter<'_> {}

#[cfg(test)]
#[path = "map_tests.rs"]
mod tests;
