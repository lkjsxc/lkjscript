//! Neutral immutable sequences. No type, origin, affinity, or serialized tree authority.
//!
//! Every node reserves 32 slots. Appending copies at most one tail (32 handles) and,
//! only when that tail is full, at most height + 1 branch nodes. Existing payloads
//! are behind immutable element handles: neither append nor tree rotation clones them.

use super::value::{NormalizedValue, release_raw_values};
use crate::platform::execution::{ExecutionError, ExecutionFailureClass};
use std::cell::Cell;
use std::sync::Arc;

pub(super) const FANOUT: usize = 32;
pub(super) const MAXIMUM_LENGTH: usize = 1_000_000;
const MAXIMUM_HEIGHT: usize = 3;
type Leaves = [Option<Arc<Element>>; FANOUT];
type Branches = [Option<Arc<Node>>; FANOUT];

struct Element(NormalizedValue);

enum Node {
    Leaf(Leaves),
    Branch(Branches),
}

/// Allocation requests are facts about new storage. Each evaluator owns its fuel ledger.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Charge {
    pub slots: u64,
    pub bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize)]
pub(crate) struct Work {
    pub node_visits: u64,
    pub element_handle_copies: u64,
    pub element_handle_allocations: u64,
    pub element_slots_reserved: u64,
    pub branch_slot_copies: u64,
    pub branch_slots_reserved: u64,
    pub nodes_allocated: u64,
    pub full_materializations: u64,
    pub materialized_elements: u64,
}

thread_local! {
    // Observations only. Saturation cannot affect admission or execution.
    static WORK: Cell<Work> = const { Cell::new(Work::ZERO) };
}

impl Work {
    const ZERO: Self = Self {
        node_visits: 0,
        element_handle_copies: 0,
        element_handle_allocations: 0,
        element_slots_reserved: 0,
        branch_slot_copies: 0,
        branch_slots_reserved: 0,
        nodes_allocated: 0,
        full_materializations: 0,
        materialized_elements: 0,
    };

    pub(super) fn current() -> Self {
        WORK.get()
    }

    pub(super) fn since(self) -> Self {
        let end = Self::current();
        Self {
            node_visits: end.node_visits.saturating_sub(self.node_visits),
            element_handle_copies: end
                .element_handle_copies
                .saturating_sub(self.element_handle_copies),
            element_handle_allocations: end
                .element_handle_allocations
                .saturating_sub(self.element_handle_allocations),
            element_slots_reserved: end
                .element_slots_reserved
                .saturating_sub(self.element_slots_reserved),
            branch_slot_copies: end
                .branch_slot_copies
                .saturating_sub(self.branch_slot_copies),
            branch_slots_reserved: end
                .branch_slots_reserved
                .saturating_sub(self.branch_slots_reserved),
            nodes_allocated: end.nodes_allocated.saturating_sub(self.nodes_allocated),
            full_materializations: end
                .full_materializations
                .saturating_sub(self.full_materializations),
            materialized_elements: end
                .materialized_elements
                .saturating_sub(self.materialized_elements),
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

/// Explicit external-buffer conversion; ordinary runtime list operations never call this.
pub(super) fn materialized(elements: usize) {
    observe(|work| {
        work.full_materializations = work.full_materializations.saturating_add(1);
        work.materialized_elements = work.materialized_elements.saturating_add(elements as u64);
    });
}

fn failure() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Resource,
        "normalized_list_storage",
        "immutable list exceeds its checked length, height, or storage bound",
    )
}

fn invalid() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "normalized_list_shape",
        "immutable list has an invalid internal shape",
    )
}

fn node_charge() -> Charge {
    Charge {
        slots: FANOUT as u64,
        bytes: (std::mem::size_of::<Node>() + 2 * std::mem::size_of::<usize>()) as u64,
    }
}

fn element_charge() -> Charge {
    Charge {
        slots: 0, // Its containing leaf reserves the element slot, including unused capacity.
        bytes: (std::mem::size_of::<Element>() + 2 * std::mem::size_of::<usize>()) as u64,
    }
}

fn element(
    value: NormalizedValue,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<Arc<Element>, ExecutionError> {
    // Own rejected raw payloads with the same stack-safe destruction as raw ingress.
    let value = Element(value);
    reserve(element_charge())?;
    observe(|work| {
        work.element_handle_allocations = work.element_handle_allocations.saturating_add(1);
    });
    Ok(Arc::new(value))
}

fn leaf(
    source: Option<&Node>,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<Leaves, ExecutionError> {
    reserve(node_charge())?;
    observe(|work| {
        work.element_slots_reserved = work.element_slots_reserved.saturating_add(FANOUT as u64)
    });
    let mut slots = std::array::from_fn(|_| None);
    if let Some(source) = source {
        visit();
        let Node::Leaf(items) = source else {
            return Err(invalid());
        };
        for (target, item) in slots.iter_mut().zip(items) {
            if let Some(item) = item {
                *target = Some(Arc::clone(item));
                observe(|work| {
                    work.element_handle_copies = work.element_handle_copies.saturating_add(1)
                });
            }
        }
    }
    Ok(slots)
}

fn branch(
    source: Option<&Node>,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<Branches, ExecutionError> {
    reserve(node_charge())?;
    observe(|work| {
        work.branch_slots_reserved = work.branch_slots_reserved.saturating_add(FANOUT as u64)
    });
    let mut slots = std::array::from_fn(|_| None);
    if let Some(source) = source {
        visit();
        let Node::Branch(children) = source else {
            return Err(invalid());
        };
        for (target, child) in slots.iter_mut().zip(children) {
            if let Some(child) = child {
                *target = Some(Arc::clone(child));
                observe(|work| work.branch_slot_copies = work.branch_slot_copies.saturating_add(1));
            }
        }
    }
    Ok(slots)
}

fn node(value: Node) -> Arc<Node> {
    observe(|work| work.nodes_allocated = work.nodes_allocated.saturating_add(1));
    Arc::new(value)
}

/// O(1) length and shallow clone, logarithmic indexing, linear ordered traversal.
/// Empty lists have no heap storage. The inline header is already part of the value slot.
#[derive(Clone, Default)]
pub struct List {
    length: usize,
    height: usize,
    root: Option<Arc<Node>>,
    tail: Option<Arc<Node>>,
}

impl List {
    pub(super) fn raw_append(
        &self,
        value: NormalizedValue,
        control: &crate::platform::execution::ExecutionControl,
    ) -> Result<Self, ExecutionError> {
        let mut bytes = 0_u64;
        let mut slots = 0_u64;
        self.append(value, MAXIMUM_LENGTH as u64, &mut |charge| {
            control.check()?;
            bytes = bytes
                .checked_add(charge.bytes)
                .filter(|n| *n <= 268_435_456)
                .ok_or_else(failure)?;
            slots = slots
                .checked_add(charge.slots)
                .filter(|n| *n <= 1_000_000)
                .ok_or_else(failure)?;
            Ok(())
        })
    }

    pub(super) fn len(&self) -> usize {
        self.length
    }

    fn tail_start(&self) -> usize {
        self.length.saturating_sub(1) / FANOUT * FANOUT
    }

    fn capacity(height: usize) -> Result<usize, ExecutionError> {
        if height > MAXIMUM_HEIGHT {
            return Err(failure());
        }
        FANOUT.checked_pow(height as u32).ok_or_else(failure)
    }

    /// Copy the path to one new completed leaf. Heights count branch layers, not values.
    fn insert_leaf(
        root: Option<&Node>,
        height: usize,
        index: usize,
        added: Arc<Node>,
        reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
    ) -> Result<Arc<Node>, ExecutionError> {
        if height == 0 {
            return if root.is_none() && index == 0 {
                Ok(added)
            } else {
                Err(invalid())
            };
        }
        let span = Self::capacity(height - 1)?;
        let slot = index / span;
        let mut children = branch(root, reserve)?;
        let target = children.get_mut(slot).ok_or_else(invalid)?;
        let child = Self::insert_leaf(target.as_deref(), height - 1, index % span, added, reserve)?;
        *target = Some(child);
        Ok(node(Node::Branch(children)))
    }

    fn promote(
        &mut self,
        tail: Arc<Node>,
        reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
    ) -> Result<(), ExecutionError> {
        let leaves = self.tail_start() / FANOUT;
        let Some(root) = self.root.as_ref() else {
            self.root = Some(tail);
            return Ok(());
        };
        if leaves == Self::capacity(self.height)? {
            let height = self
                .height
                .checked_add(1)
                .filter(|h| *h <= MAXIMUM_HEIGHT)
                .ok_or_else(failure)?;
            let mut children = branch(None, reserve)?;
            children[0] = Some(Arc::clone(root));
            observe(|work| work.branch_slot_copies = work.branch_slot_copies.saturating_add(1));
            children[1] = Some(Self::insert_leaf(None, self.height, 0, tail, reserve)?);
            self.root = Some(node(Node::Branch(children)));
            self.height = height;
        } else {
            self.root = Some(Self::insert_leaf(
                Some(root),
                self.height,
                leaves,
                tail,
                reserve,
            )?);
        }
        Ok(())
    }

    pub(super) fn append(
        &self,
        value: NormalizedValue,
        maximum_length: u64,
        reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        let value = Element(value);
        let length = self
            .length
            .checked_add(1)
            .filter(|n| *n <= MAXIMUM_LENGTH && (*n as u64) <= maximum_length)
            .ok_or_else(failure)?;
        let mut result = self.clone();
        let tail_length = self.length - self.tail_start();
        let mut items = if tail_length == FANOUT {
            let old = result.tail.take().ok_or_else(invalid)?;
            result.promote(old, reserve)?;
            leaf(None, reserve)?
        } else {
            leaf(result.tail.as_deref(), reserve)?
        };
        let slot = if tail_length == FANOUT {
            0
        } else {
            tail_length
        };
        reserve(element_charge())?;
        observe(|work| {
            work.element_handle_allocations = work.element_handle_allocations.saturating_add(1)
        });
        items[slot] = Some(Arc::new(value));
        result.tail = Some(node(Node::Leaf(items)));
        result.length = length;
        reserve(Charge::default())?; // Independent cancellation check before result installation.
        Ok(result)
    }

    /// Private bounded bulk ingress. Raw items are moved; no existing list is flattened.
    pub(super) fn from_items(
        items: Vec<NormalizedValue>,
        maximum_length: u64,
        reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        let mut pending = super::value::RawArguments::new(items);
        let length = pending.len();
        if length > MAXIMUM_LENGTH || length as u64 > maximum_length {
            return Err(failure());
        }
        let mut result = Self::default();
        while !pending.is_empty() {
            let mut slots = leaf(None, reserve)?;
            let count = pending.len().min(FANOUT);
            for slot in slots.iter_mut().take(count) {
                let item = pending.next().ok_or_else(invalid)?;
                *slot = Some(element(item, reserve)?);
            }
            if let Some(tail) = result.tail.take() {
                result.promote(tail, reserve)?;
            }
            result.tail = Some(node(Node::Leaf(slots)));
            result.length = result.length.checked_add(count).ok_or_else(failure)?;
        }
        reserve(Charge::default())?;
        Ok(result)
    }

    pub(super) fn get(&self, index: usize) -> Option<&NormalizedValue> {
        if index >= self.length {
            return None;
        }
        let current = if index >= self.tail_start() {
            self.tail.as_deref()?
        } else {
            let mut current = self.root.as_deref()?;
            for height in (1..=self.height).rev() {
                visit();
                let Node::Branch(children) = current else {
                    return None;
                };
                let span = FANOUT.checked_pow(height as u32)?;
                current = children.get(index / span % FANOUT)?.as_deref()?;
            }
            current
        };
        visit();
        let Node::Leaf(items) = current else {
            return None;
        };
        Some(&items.get(index % FANOUT)?.as_deref()?.0)
    }

    pub(super) fn iter(&self) -> Iter<'_> {
        Iter {
            forward: Cursor::new(self.root.as_deref(), self.tail.as_deref(), false),
            backward: Cursor::new(self.tail.as_deref(), self.root.as_deref(), true),
            remaining: self.length,
        }
    }

    /// Additional raw metadata beyond the logical value occurrences charged by admission.
    /// Sharing is never a certificate: each raw occurrence is still interpreted independently.
    pub(super) fn metadata_bytes(&self) -> Result<u64, ExecutionError> {
        let mut nodes = self.length.div_ceil(FANOUT);
        let mut leaves = self.tail_start() / FANOUT;
        while leaves > 1 {
            leaves = leaves.div_ceil(FANOUT);
            nodes = nodes.checked_add(leaves).ok_or_else(failure)?;
        }
        (nodes as u64)
            .checked_mul(node_charge().bytes)
            .and_then(|bytes| {
                (self.length as u64)
                    .checked_mul((2 * std::mem::size_of::<usize>()) as u64)
                    .and_then(|handles| bytes.checked_add(handles))
            })
            .ok_or_else(failure)
    }

    pub(super) fn drain_unique(&mut self, values: &mut Vec<NormalizedValue>) {
        let mut pending = Vec::new();
        pending.extend(self.root.take());
        pending.extend(self.tail.take());
        while let Some(node) = pending.pop() {
            if let Some(node) = Arc::into_inner(node) {
                visit();
                match node {
                    Node::Branch(children) => pending.extend(children.into_iter().flatten()),
                    Node::Leaf(items) => {
                        for item in items.into_iter().flatten() {
                            if let Some(mut item) = Arc::into_inner(item) {
                                values.push(std::mem::replace(&mut item.0, NormalizedValue::Unit));
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Drop for Element {
    fn drop(&mut self) {
        if !matches!(self.0, NormalizedValue::Unit) {
            release_raw_values(vec![std::mem::replace(&mut self.0, NormalizedValue::Unit)]);
        }
    }
}

impl Drop for List {
    fn drop(&mut self) {
        let mut values = Vec::new();
        self.drain_unique(&mut values);
        release_raw_values(values);
    }
}

impl std::fmt::Debug for List {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_list().entries(self.iter()).finish()
    }
}

impl PartialEq for List {
    fn eq(&self, other: &Self) -> bool {
        self.length == other.length && self.iter().eq(other.iter())
    }
}
impl Eq for List {}

struct Cursor<'a> {
    stack: [Option<(&'a Node, usize)>; MAXIMUM_HEIGHT + 1],
    depth: usize,
    first: Option<&'a Node>,
    second: Option<&'a Node>,
    reverse: bool,
}

impl<'a> Cursor<'a> {
    fn new(first: Option<&'a Node>, second: Option<&'a Node>, reverse: bool) -> Self {
        Self {
            stack: [None; MAXIMUM_HEIGHT + 1],
            depth: 0,
            first,
            second,
            reverse,
        }
    }

    fn next(&mut self) -> Option<&'a NormalizedValue> {
        loop {
            if self.depth == 0 {
                let next = self.first.take().or_else(|| self.second.take())?;
                self.stack[0] = Some((next, 0));
                self.depth = 1;
                visit();
            }
            let (node, next) = self.stack.get_mut(self.depth - 1)?.as_mut()?;
            if *next == FANOUT {
                self.stack[self.depth - 1] = None;
                self.depth -= 1;
                continue;
            }
            let index = if self.reverse {
                FANOUT - 1 - *next
            } else {
                *next
            };
            *next += 1;
            match node {
                Node::Leaf(items) => {
                    if let Some(item) = items.get(index)?.as_deref() {
                        return Some(&item.0);
                    }
                }
                Node::Branch(children) => {
                    if let Some(child) = children.get(index)?.as_deref() {
                        *self.stack.get_mut(self.depth)? = Some((child, 0));
                        self.depth += 1;
                        visit();
                    }
                }
            }
        }
    }
}

pub(super) struct Iter<'a> {
    forward: Cursor<'a>,
    backward: Cursor<'a>,
    remaining: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a NormalizedValue;
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
#[path = "list_tests.rs"]
mod tests;
