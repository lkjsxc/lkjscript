//! Neutral persistent ordered maps. Storage sharing conveys no type or affine authority.
//!
//! AVL path copying retains immutable entry handles, including keys and payloads.
//! Each new node reserves one collection slot and its node plus two Arc counters;
//! each new entry reserves its key/value header plus two Arc counters. Owned key
//! buffers and payloads move from their already reserved construction owner. The
//! borrowed-key helper reserves its copied key buffer before cloning it. Existing
//! entry handles and subtrees are shared, never charged as fresh deep payloads.
//! Temporary rotation nodes are charged even when the resulting tree omits them.
//! Zero reservations check cancellation on entry and before successful exposure.
//! Traversal and codecs see key order, never tree shape. Raw ingress is admitted
//! independently by each evaluator; this carrier has no semantic certificate.

use super::value::{NormalizedMapKey, NormalizedValue, RawValueWork, release_raw_value};
use crate::platform::execution::{ExecutionError, ExecutionFailureClass};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::Arc;

// An AVL tree of representable length has height below twice the address width.
// This is a representation bound, not an independent finite value-admission rule.
const MAXIMUM_HEIGHT: usize = 2 * usize::BITS as usize;
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

/// New storage requests only; every evaluator owns its own cumulative ledger.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Charge {
    pub slots: u64,
    pub bytes: u64,
}

/// Storage observations, not global allocator measurements or execution permission.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize)]
pub(crate) struct Work {
    pub node_visits: u64,
    pub nodes_allocated: u64,
    pub entry_handles_allocated: u64,
    pub entry_handle_copies: u64,
    pub key_bytes_copied: u64,
}

thread_local! {
    static WORK: Cell<Work> = const { Cell::new(Work {
        node_visits: 0,
        nodes_allocated: 0,
        entry_handles_allocated: 0,
        entry_handle_copies: 0,
        key_bytes_copied: 0,
    }) };
}

impl Work {
    pub(super) fn current() -> Self {
        WORK.get()
    }

    pub(super) fn since(self) -> Self {
        let end = Self::current();
        Self {
            node_visits: end.node_visits.saturating_sub(self.node_visits),
            nodes_allocated: end.nodes_allocated.saturating_sub(self.nodes_allocated),
            entry_handles_allocated: end
                .entry_handles_allocated
                .saturating_sub(self.entry_handles_allocated),
            entry_handle_copies: end
                .entry_handle_copies
                .saturating_sub(self.entry_handle_copies),
            key_bytes_copied: end.key_bytes_copied.saturating_sub(self.key_bytes_copied),
        }
    }
}

fn observe(update: impl FnOnce(&mut Work)) {
    let mut work = WORK.get();
    update(&mut work);
    WORK.set(work);
}

fn visit() {
    observe(|work| work.node_visits = work.node_visits.saturating_add(1));
}

/// The evaluator calls this after its reserved owned-key conversion completes.
pub(super) fn key_copied(bytes: u64) {
    observe(|work| work.key_bytes_copied = work.key_bytes_copied.saturating_add(bytes));
}

fn share(entry: &Arc<Entry>) -> Arc<Entry> {
    observe(|work| work.entry_handle_copies = work.entry_handle_copies.saturating_add(1));
    Arc::clone(entry)
}

/// Empty maps have no heap storage. The inline header belongs to the value slot.
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

fn entry(
    value: Entry,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<Arc<Entry>, ExecutionError> {
    // Entry already owns the raw payload, including on refused reservation.
    reserve(Charge {
        slots: 0,
        bytes: entry_bytes(),
    })?;
    observe(|work| work.entry_handles_allocated = work.entry_handles_allocated.saturating_add(1));
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
    observe(|work| work.nodes_allocated = work.nodes_allocated.saturating_add(1));
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
            return node(
                share(&pivot.entry),
                pivot.left.clone(),
                Some(right),
                reserve,
            );
        }
        let middle = pivot.right.as_ref().ok_or_else(shape_error)?;
        let left = node(
            share(&pivot.entry),
            pivot.left.clone(),
            middle.left.clone(),
            reserve,
        )?;
        let right = node(entry, middle.right.clone(), right, reserve)?;
        return node(share(&middle.entry), Some(left), Some(right), reserve);
    }
    if height(&right) > height(&left).saturating_add(1) {
        let pivot = right.as_ref().ok_or_else(shape_error)?;
        if height(&pivot.right) >= height(&pivot.left) {
            let left = node(entry, left, pivot.left.clone(), reserve)?;
            return node(
                share(&pivot.entry),
                Some(left),
                pivot.right.clone(),
                reserve,
            );
        }
        let middle = pivot.left.as_ref().ok_or_else(shape_error)?;
        let left = node(entry, left, middle.left.clone(), reserve)?;
        let right = node(
            share(&pivot.entry),
            middle.right.clone(),
            pivot.right.clone(),
            reserve,
        )?;
        return node(share(&middle.entry), Some(left), Some(right), reserve);
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
    visit();
    match added.key.cmp(&root.entry.key) {
        std::cmp::Ordering::Less => {
            let left = insert(&root.left, added, reserve)?;
            balance(share(&root.entry), Some(left), root.right.clone(), reserve)
        }
        std::cmp::Ordering::Greater => {
            let right = insert(&root.right, added, reserve)?;
            balance(share(&root.entry), root.left.clone(), Some(right), reserve)
        }
        std::cmp::Ordering::Equal => node(added, root.left.clone(), root.right.clone(), reserve),
    }
}

fn remove_first(
    root: &Arc<Node>,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<(Arc<Entry>, Link), ExecutionError> {
    visit();
    let Some(left) = &root.left else {
        return Ok((share(&root.entry), root.right.clone()));
    };
    let (entry, left) = remove_first(left, reserve)?;
    let root = balance(share(&root.entry), left, root.right.clone(), reserve)?;
    Ok((entry, Some(root)))
}

fn remove(
    root: &Link,
    key: &NormalizedMapKey,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<Link, ExecutionError> {
    let root = root.as_ref().ok_or_else(shape_error)?;
    visit();
    match key.cmp(&root.entry.key) {
        std::cmp::Ordering::Less => {
            let left = remove(&root.left, key, reserve)?;
            balance(share(&root.entry), left, root.right.clone(), reserve).map(Some)
        }
        std::cmp::Ordering::Greater => {
            let right = remove(&root.right, key, reserve)?;
            balance(share(&root.entry), root.left.clone(), right, reserve).map(Some)
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

// Own all unadmitted payloads while bulk construction can still fail. Drain each
// remaining payload iteratively without allocating another whole-entry buffer.
struct RawEntries(std::collections::btree_map::IntoIter<NormalizedMapKey, NormalizedValue>);

impl Drop for RawEntries {
    fn drop(&mut self) {
        for (_, value) in self.0.by_ref() {
            release_raw_value(value);
        }
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
        maximum_length: u64,
        reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        let length = entries.len();
        let mut entries = RawEntries(entries.into_iter());
        if length as u64 > maximum_length {
            return Err(storage_error());
        }
        reserve(Charge::default())?;
        let root = sorted(&mut entries, length, reserve)?;
        reserve(Charge::default())?;
        Ok(Self { length, root })
    }

    pub(super) fn len(&self) -> usize {
        self.length
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub(super) fn get_key_value(
        &self,
        key: &NormalizedMapKey,
    ) -> Option<(&NormalizedMapKey, &NormalizedValue)> {
        let mut current = self.root.as_deref();
        while let Some(node) = current {
            visit();
            current = match key.cmp(&node.entry.key) {
                std::cmp::Ordering::Less => node.left.as_deref(),
                std::cmp::Ordering::Greater => node.right.as_deref(),
                std::cmp::Ordering::Equal => return Some((&node.entry.key, &node.entry.value)),
            };
        }
        None
    }

    pub(super) fn get(&self, key: &NormalizedMapKey) -> Option<&NormalizedValue> {
        self.get_key_value(key).map(|(_, value)| value)
    }

    pub(super) fn contains_key(&self, key: &NormalizedMapKey) -> bool {
        self.get(key).is_some()
    }

    fn inserted_length(
        &self,
        key: &NormalizedMapKey,
        maximum_length: u64,
    ) -> Result<usize, ExecutionError> {
        self.length
            .checked_add(usize::from(!self.contains_key(key)))
            .filter(|length| *length as u64 <= maximum_length)
            .ok_or_else(storage_error)
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
        let length = self.inserted_length(&added.key, maximum_length)?;
        let added = entry(added, reserve)?;
        let root = insert(&self.root, added, reserve)?;
        reserve(Charge::default())?;
        Ok(Self {
            length,
            root: Some(root),
        })
    }

    // A bounded borrowed-key owner used to challenge reservation ordering. Normal
    // evaluator producers reserve conversion at their own key allocation boundary.
    #[cfg(test)]
    fn insert_borrowed(
        &self,
        key: &NormalizedMapKey,
        value: NormalizedValue,
        maximum_length: u64,
        reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        let mut added = Entry {
            key: NormalizedMapKey::Bool(false),
            value,
        };
        reserve(Charge::default())?;
        let length = self.inserted_length(key, maximum_length)?;
        let key_bytes = match key {
            NormalizedMapKey::Bytes(value) => value.len(),
            NormalizedMapKey::Text(value) => value.len(),
            NormalizedMapKey::Bool(_) | NormalizedMapKey::I64(_) => 0,
        } as u64;
        reserve(Charge {
            slots: 0,
            bytes: entry_bytes()
                .checked_add(key_bytes)
                .ok_or_else(storage_error)?,
        })?;
        added.key = key.clone();
        key_copied(key_bytes);
        observe(|work| {
            work.entry_handles_allocated = work.entry_handles_allocated.saturating_add(1);
        });
        let root = insert(&self.root, Arc::new(added), reserve)?;
        reserve(Charge::default())?;
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
        let result = if self.contains_key(key) {
            let length = self.length.checked_sub(1).ok_or_else(shape_error)?;
            let root = remove(&self.root, key, reserve)?;
            Self { length, root }
        } else {
            self.clone()
        };
        reserve(Charge::default())?;
        Ok(result)
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

    #[cfg(test)]
    pub(super) fn values(
        &self,
    ) -> impl DoubleEndedIterator<Item = &NormalizedValue> + ExactSizeIterator {
        self.iter().map(|(_, value)| value)
    }

    // Admission separately charges every logical value slot and key. Add neutral
    // AVL nodes, entry reference counts and any header alignment padding here.
    pub(super) fn metadata_bytes(&self) -> Result<u64, ExecutionError> {
        let entry_overhead = entry_bytes()
            .checked_sub(
                (std::mem::size_of::<NormalizedMapKey>() + std::mem::size_of::<NormalizedValue>())
                    as u64,
            )
            .ok_or_else(storage_error)?;
        let overhead = node_bytes()
            .checked_add(entry_overhead)
            .ok_or_else(storage_error)?;
        (self.length as u64)
            .checked_mul(overhead)
            .ok_or_else(storage_error)
    }

    pub(super) fn drain_unique(&mut self, values: &mut RawValueWork) {
        // A depth-first binary walk holds at most one sibling per AVL level.
        // Retiring an update path must not allocate a temporary node Vec.
        let mut pending: [Link; MAXIMUM_HEIGHT] = std::array::from_fn(|_| None);
        pending[0] = self.root.take();
        self.length = 0;
        let mut depth = usize::from(pending[0].is_some());
        while depth != 0 {
            depth -= 1;
            if let Some(node) = pending[depth].take().and_then(Arc::into_inner) {
                for child in [node.left, node.right].into_iter().flatten() {
                    pending[depth] = Some(child);
                    depth += 1;
                }
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
            release_raw_value(std::mem::replace(&mut self.value, NormalizedValue::Unit));
        }
    }
}

impl Drop for Map {
    fn drop(&mut self) {
        let mut values = RawValueWork::default();
        self.drain_unique(&mut values);
        values.release();
    }
}

impl std::fmt::Debug for Map {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_map().entries(self.iter()).finish()
    }
}

impl PartialEq for Map {
    fn eq(&self, other: &Self) -> bool {
        if self.length != other.length {
            return false;
        }
        // Borrow the bounded cursor buffers across recursive payload comparison.
        // Moving them through Iterator::eq duplicates both buffers in debug frames.
        let mut left = self.iter();
        let mut right = other.iter();
        (&mut left).eq(&mut right)
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
            visit();
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

impl<'a> Iter<'a> {
    /// Reuse the bounded traversal buffers rather than moving a second complete
    /// iterator into a recursive value walker's frame. Resetting the active depths
    /// makes old, non-owning path slots unreachable; subsequent pushes overwrite
    /// them before they can be read. No nodes, entries or buffers are allocated.
    pub(super) fn restart(&mut self, map: &'a Map) {
        self.forward.depth = 0;
        self.forward.next = map.root.as_deref();
        self.backward.depth = 0;
        self.backward.next = map.root.as_deref();
        self.remaining = map.length;
    }
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
