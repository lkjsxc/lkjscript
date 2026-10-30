//! Neutral immutable sequences. No type, origin, affinity, or serialized tree authority.
//!
//! Every node reserves 32 slots. Appending copies at most one tail (32 handles) and,
//! only when that tail is full, at most height + 1 branch nodes. Existing payloads
//! are behind immutable element handles: neither append nor tree rotation clones them.

use super::value::{NormalizedValue, RawValueWork, release_raw_value};
use crate::platform::execution::{ExecutionError, ExecutionFailureClass};
use std::cell::Cell;
use std::sync::Arc;

pub(super) const FANOUT: usize = 32;
// Each radix level consumes five index bits. These inline worklists cover every
// usize index, including the partially used top level; they are not admission
// policies. The leaf consumes one level, so a reachable root has at most
// INDEX_LEVELS - 1 branch layers. Its leaf capacity still fits usize even when
// the corresponding complete value capacity would not.
const INDEX_LEVELS: usize = usize::BITS.div_ceil(FANOUT.ilog2()) as usize;
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
    /// True marks loss of exactness; counters remain passive lower bounds.
    pub saturated: bool,
}

thread_local! {
    // Observations only. Saturation cannot affect admission or execution.
    static WORK: Cell<Work> = const { Cell::new(Work::ZERO) };
}

/// A worker job starts with fresh observations and restores its caller on unwind.
pub(super) struct WorkScope(Work);

impl WorkScope {
    pub(super) fn enter() -> Self {
        Self(WORK.replace(Work::ZERO))
    }
}

impl Drop for WorkScope {
    fn drop(&mut self) {
        WORK.set(self.0);
    }
}

impl Work {
    /// Join observations from a completed off-thread child into this invocation.
    pub(super) fn include_joined(child: Self) {
        WORK.with(|slot| {
            let mut value = slot.get();
            value.saturated |= child.saturated;
            macro_rules! add { ($($field:ident),* $(,)?) => { $(add_observed(&mut value.$field, child.$field, &mut value.saturated);)* }; }
            add!(node_visits, element_handle_copies, element_handle_allocations, element_slots_reserved, branch_slot_copies, branch_slots_reserved, nodes_allocated, full_materializations, materialized_elements);
            slot.set(value);
        });
    }
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
        saturated: false,
    };

    pub(super) fn current() -> Self {
        WORK.get()
    }

    /// Difference within the same observation scope; loss of exactness is sticky.
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
            saturated: self.saturated || end.saturated,
        }
    }
}

fn add_observed(counter: &mut u64, amount: u64, saturated: &mut bool) {
    match counter.checked_add(amount) {
        Some(next) => *counter = next,
        None => {
            *counter = u64::MAX;
            *saturated = true;
        }
    }
}

fn observe(update: impl FnOnce(&mut Work)) {
    let mut work = WORK.get();
    update(&mut work);
    WORK.set(work);
}

fn visit() {
    observe(|work| add_observed(&mut work.node_visits, 1, &mut work.saturated));
}

/// Explicit external-buffer conversion; ordinary runtime list operations never call this.
pub(super) fn materialized(elements: usize) {
    observe(|work| {
        add_observed(&mut work.full_materializations, 1, &mut work.saturated);
        let elements = u64::try_from(elements).unwrap_or_else(|_| {
            work.saturated = true;
            u64::MAX
        });
        add_observed(
            &mut work.materialized_elements,
            elements,
            &mut work.saturated,
        );
    });
}

fn failure() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Resource,
        "normalized_list_storage",
        "immutable list exceeds its explicit length policy or representable storage extent",
    )
}

/// Exact caller policy, separate from physical node reservations and observations.
pub(super) fn admit_length(length: usize, maximum: Option<u64>) -> Result<(), ExecutionError> {
    let length = u64::try_from(length).map_err(|_| failure())?;
    if maximum.is_some_and(|maximum| length > maximum) {
        Err(failure())
    } else {
        Ok(())
    }
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
        add_observed(&mut work.element_handle_allocations, 1, &mut work.saturated);
    });
    Ok(Arc::new(value))
}

fn leaf(
    source: Option<&Node>,
    reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
) -> Result<Leaves, ExecutionError> {
    reserve(node_charge())?;
    observe(|work| {
        add_observed(
            &mut work.element_slots_reserved,
            FANOUT as u64,
            &mut work.saturated,
        );
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
                    add_observed(&mut work.element_handle_copies, 1, &mut work.saturated);
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
        add_observed(
            &mut work.branch_slots_reserved,
            FANOUT as u64,
            &mut work.saturated,
        );
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
                observe(|work| {
                    add_observed(&mut work.branch_slot_copies, 1, &mut work.saturated);
                });
            }
        }
    }
    Ok(slots)
}

fn node(value: Node) -> Arc<Node> {
    observe(|work| add_observed(&mut work.nodes_allocated, 1, &mut work.saturated));
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
        self.append(value, None, &mut |_| control.check())
    }

    pub(super) fn len(&self) -> usize {
        self.length
    }

    fn tail_start(&self) -> usize {
        self.length.saturating_sub(1) / FANOUT * FANOUT
    }

    /// Completed-leaf capacity, which fits usize for every reachable branch height.
    fn capacity(height: usize) -> Result<usize, ExecutionError> {
        FANOUT
            .checked_pow(u32::try_from(height).map_err(|_| failure())?)
            .ok_or_else(failure)
    }

    /// Copy the path to one new completed leaf. Heights count branch layers, not values.
    fn insert_leaf(
        root: Option<&Node>,
        height: usize,
        index: usize,
        added: Arc<Node>,
        reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
    ) -> Result<Arc<Node>, ExecutionError> {
        let mut path: [Option<(Branches, usize)>; INDEX_LEVELS] = std::array::from_fn(|_| None);
        let mut depth = 0;
        let mut source = root;
        let mut remaining = height;
        let mut index = index;
        while remaining != 0 {
            let span = Self::capacity(remaining - 1)?;
            let slot = index / span;
            let children = branch(source, reserve)?;
            children.get(slot).ok_or_else(invalid)?;
            // Follow the borrowed original tree. The copied path may move between
            // frames, so it must not supply references to later frames.
            let next = match source {
                Some(Node::Branch(children)) => children.get(slot).ok_or_else(invalid)?.as_deref(),
                Some(Node::Leaf(_)) => return Err(invalid()),
                None => None,
            };
            *path.get_mut(depth).ok_or_else(invalid)? = Some((children, slot));
            depth += 1;
            source = next;
            index %= span;
            remaining -= 1;
        }
        if source.is_some() || index != 0 {
            return Err(invalid());
        }
        let mut child = added;
        while depth != 0 {
            depth -= 1;
            let (mut children, slot) = path[depth].take().ok_or_else(invalid)?;
            *children.get_mut(slot).ok_or_else(invalid)? = Some(child);
            child = node(Node::Branch(children));
        }
        Ok(child)
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
            let height = self.height.checked_add(1).ok_or_else(failure)?;
            Self::capacity(height)?;
            let mut children = branch(None, reserve)?;
            children[0] = Some(Arc::clone(root));
            observe(|work| {
                add_observed(&mut work.branch_slot_copies, 1, &mut work.saturated);
            });
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
        maximum_length: Option<u64>,
        reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        let value = Element(value);
        let length = self.length.checked_add(1).ok_or_else(failure)?;
        admit_length(length, maximum_length)?;
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
            add_observed(&mut work.element_handle_allocations, 1, &mut work.saturated);
        });
        items[slot] = Some(Arc::new(value));
        result.tail = Some(node(Node::Leaf(items)));
        result.length = length;
        reserve(Charge::default())?; // Independent cancellation check before result installation.
        Ok(result)
    }

    /// Private bulk ingress. Raw items move into each final node once.
    pub(super) fn from_items(
        items: Vec<NormalizedValue>,
        maximum_length: Option<u64>,
        reserve: &mut impl FnMut(Charge) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        bulk::from_items(items, maximum_length, reserve)
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
                let span = Self::capacity(height).ok()?;
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

    pub(super) fn drain_unique(&mut self, values: &mut RawValueWork) {
        // Each branch level adds at most FANOUT-1 pending siblings, in addition
        // to the two roots. Shared or scalar retired list payloads need no heap.
        let mut pending: [Option<Arc<Node>>; (FANOUT - 1) * (INDEX_LEVELS - 1) + 2] =
            std::array::from_fn(|_| None);
        let mut depth = 0;
        for root in [self.root.take(), self.tail.take()].into_iter().flatten() {
            pending[depth] = Some(root);
            depth += 1;
        }
        self.length = 0;
        self.height = 0;
        while depth != 0 {
            depth -= 1;
            if let Some(node) = pending[depth].take().and_then(Arc::into_inner) {
                visit();
                match node {
                    Node::Branch(children) => {
                        for child in children.into_iter().flatten() {
                            pending[depth] = Some(child);
                            depth += 1;
                        }
                    }
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
            release_raw_value(std::mem::replace(&mut self.0, NormalizedValue::Unit));
        }
    }
}

impl Drop for List {
    fn drop(&mut self) {
        let mut values = RawValueWork::default();
        self.drain_unique(&mut values);
        values.release();
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
    stack: [Option<(&'a Node, usize)>; INDEX_LEVELS],
    depth: usize,
    first: Option<&'a Node>,
    second: Option<&'a Node>,
    reverse: bool,
}

impl<'a> Cursor<'a> {
    fn new(first: Option<&'a Node>, second: Option<&'a Node>, reverse: bool) -> Self {
        Self {
            stack: [None; INDEX_LEVELS],
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

#[path = "list_bulk.rs"]
mod bulk;

#[cfg(test)]
#[path = "list_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "list_bulk_tests.rs"]
mod bulk_tests;

#[cfg(test)]
#[path = "list_bulk_failure_tests.rs"]
mod bulk_failure_tests;
