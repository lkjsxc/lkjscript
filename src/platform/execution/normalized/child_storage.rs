//! Opaque immutable children for callable prefixes and ordinary record fields.
//!
//! Scratch handoff: Function/Record, collection, and affine/view ownership
//! transitions share one drain using metadata reserved during construction.
//! This module confers no admission or custody authority and changes no policy
//! charge. Recursive compiler and evaluator work runs on the host-sized semantic stack.

use super::value::{NormalizedRecord, NormalizedValue};
use crate::platform::execution::ExecutionError;
use crate::platform::kernel::Name;
use std::fmt;
use std::ops::Deref;
use triomphe::{Arc, UniqueArc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StorageFailure {
    Extent,
    Allocation,
}

impl From<StorageFailure> for ExecutionError {
    fn from(value: StorageFailure) -> Self {
        match value {
            StorageFailure::Extent => Self::resource_static(
                "normalized_child_extent",
                "immutable child storage exceeds the contiguous allocation domain",
            ),
            StorageFailure::Allocation => Self::resource_static(
                "normalized_child_allocation",
                "immutable child storage allocation failed",
            ),
        }
    }
}

impl fmt::Display for StorageFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Extent => "immutable child storage extent is not representable",
            Self::Allocation => "immutable child storage allocation failed",
        })
    }
}

impl std::error::Error for StorageFailure {}

#[cfg(test)]
thread_local! {
    static OWNER_FACTORY_ALLOWANCE: std::cell::Cell<Option<usize>> = const {
        std::cell::Cell::new(None)
    };
}

/// Test-only refusal of this thread's opaque child-owner factories. The guard
/// cannot move to another thread; nested scopes restore the previous allowance.
/// This injects a factory error, not a physical allocator or process OOM event.
#[cfg(test)]
pub(crate) struct OwnerFactoryRefusal {
    previous: Option<usize>,
    _current_thread: std::marker::PhantomData<std::rc::Rc<()>>,
}

#[cfg(test)]
pub(crate) fn refuse_owner_factory_after(allowed_attempts: usize) -> OwnerFactoryRefusal {
    let previous =
        OWNER_FACTORY_ALLOWANCE.with(|allowance| allowance.replace(Some(allowed_attempts)));
    OwnerFactoryRefusal {
        previous,
        _current_thread: std::marker::PhantomData,
    }
}

#[cfg(test)]
impl Drop for OwnerFactoryRefusal {
    fn drop(&mut self) {
        OWNER_FACTORY_ALLOWANCE.with(|allowance| allowance.set(self.previous));
    }
}

fn allocate_node<T>(node: T) -> Result<Arc<T>, StorageFailure> {
    #[cfg(test)]
    if OWNER_FACTORY_ALLOWANCE.with(|allowance| match allowance.get() {
        None => false,
        Some(0) => true,
        Some(remaining) => {
            allowance.set(Some(remaining - 1));
            false
        }
    }) {
        // A moved ValueNode/FieldNode already owns and guards every child, so
        // this early return follows exactly the normal owner-allocation error
        // teardown path without allocating an error message or a drain stack.
        return Err(StorageFailure::Allocation);
    }
    Arc::try_new(node).map_err(|_| StorageFailure::Allocation)
}

fn check_extent<T>(items: &[T], additional: usize) -> Result<(), StorageFailure> {
    let count = items
        .len()
        .checked_add(additional)
        .ok_or(StorageFailure::Extent)?;
    let bytes = count
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(StorageFailure::Extent)?;
    if bytes > isize::MAX as usize {
        return Err(StorageFailure::Extent);
    }
    Ok(())
}

/// Unknown frontiers and streams use amortized growth. If the preferred growth
/// target cannot be reserved, try the exact minimum before reporting failure.
pub(crate) fn reserve<T>(items: &mut Vec<T>, additional: usize) -> Result<(), StorageFailure> {
    check_extent(items, additional)?;
    items
        .try_reserve(additional)
        .or_else(|_| items.try_reserve_exact(additional))
        .map_err(|_| StorageFailure::Allocation)
}

/// Known complete payload sizes are reserved once, before accepting children.
fn reserve_exact<T>(items: &mut Vec<T>, additional: usize) -> Result<(), StorageFailure> {
    check_extent(items, additional)?;
    items
        .try_reserve_exact(additional)
        .map_err(|_| StorageFailure::Allocation)
}

// Node types remain distinct so the readers have statically typed slices. Their
// cleanup links share one private enum; no marker can occur in semantic values.
pub(super) enum Pending {
    Values(UniqueArc<ValueNode>),
    Fields(UniqueArc<FieldNode>),
    Element(UniqueArc<super::list::Element>),
    Entry(UniqueArc<super::map::Entry>),
    Owned(super::owned_storage::RetiredStorage),
    View(Box<super::read_view_storage::ReadyCleanup>),
}

/// One allocation-free drain shared by all value-bearing ownership families.
#[derive(Default)]
pub(super) struct DropQueue {
    pending: Option<Pending>,
}

impl DropQueue {
    pub(super) fn push_value(&mut self, value: NormalizedValue) {
        release_value(value, self);
    }

    pub(super) fn push_shared_value(&mut self, value: std::sync::Arc<NormalizedValue>) {
        if let Some(value) = std::sync::Arc::into_inner(value) {
            self.push_value(value);
        }
    }

    pub(super) fn enqueue_element(&mut self, value: Arc<super::list::Element>) {
        enqueue(Arc::into_unique(value).map(Pending::Element), self);
    }

    pub(super) fn enqueue_entry(&mut self, value: Arc<super::map::Entry>) {
        enqueue(Arc::into_unique(value).map(Pending::Entry), self);
    }

    pub(super) fn enqueue_owned(&mut self, value: Option<super::owned_storage::RetiredStorage>) {
        enqueue(value.map(Pending::Owned), self);
    }

    pub(super) fn enqueue_view(&mut self, value: Box<super::read_view_storage::ReadyCleanup>) {
        enqueue(Some(Pending::View(value)), self);
    }

    pub(super) fn drain(&mut self) {
        drain_pending(self);
    }
}

impl Drop for DropQueue {
    fn drop(&mut self) {
        self.drain();
    }
}

pub(super) fn release_root(value: NormalizedValue) {
    let mut queue = DropQueue::default();
    queue.push_value(value);
    queue.drain();
}

pub(super) fn release_roots(values: Vec<NormalizedValue>) {
    let mut queue = DropQueue::default();
    for value in values {
        queue.push_value(value);
    }
    queue.drain();
}

macro_rules! child_owner {
    ($owner:ident, $builder:ident, $node:ident, $item:ty, $variant:ident, $release:ident) => {
        pub(super) struct $node {
            items: Vec<$item>,
            next_cleanup: Option<Pending>,
            #[cfg(test)]
            probe: Option<std::sync::Arc<DropProbe>>,
        }

        impl $node {
            fn new(items: Vec<$item>) -> Self {
                Self {
                    items,
                    next_cleanup: None,
                    #[cfg(test)]
                    probe: None,
                }
            }
        }

        impl Drop for $node {
            fn drop(&mut self) {
                #[cfg(test)]
                let _observation = DropObservation::enter(self.probe.take());
                let mut pending = DropQueue {
                    pending: self.next_cleanup.take(),
                };
                while let Some(item) = self.items.pop() {
                    $release(item, &mut pending);
                }
                drain_pending(&mut pending);
            }
        }

        /// Immutable raw storage. Sharing is not a type or origin certificate.
        pub struct $owner {
            inner: Option<Arc<$node>>,
        }

        impl $owner {
            pub(crate) fn try_new(items: Vec<$item>) -> Result<Self, StorageFailure> {
                // The node guards its moved children before the owner allocation.
                let node = $node::new(items);
                allocate_node(node).map(|inner| Self { inner: Some(inner) })
            }

            pub fn as_slice(&self) -> &[$item] {
                // None is private and exists only after a consuming extraction
                // or while Drop is running. No live reader sees that state.
                self.inner
                    .as_ref()
                    .map_or(&[], |inner| inner.items.as_slice())
            }

            pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
                match (&self.inner, &other.inner) {
                    (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                    (None, None) => true,
                    _ => false,
                }
            }

            fn into_pending(mut self) -> Option<Pending> {
                self.inner
                    .take()
                    .and_then(Arc::into_unique)
                    .map(Pending::$variant)
            }
        }

        impl Clone for $owner {
            fn clone(&self) -> Self {
                Self {
                    inner: self.inner.as_ref().map(Arc::clone),
                }
            }
        }

        impl Drop for $owner {
            fn drop(&mut self) {
                if let Some(inner) = self.inner.take() {
                    // Unlike try_unique(...).ok(), the None path cannot become
                    // an ordinary recursive last-owner drop in a race.
                    drop(Arc::into_unique(inner));
                }
            }
        }

        impl Deref for $owner {
            type Target = [$item];

            fn deref(&self) -> &Self::Target {
                self.as_slice()
            }
        }

        impl AsRef<[$item]> for $owner {
            fn as_ref(&self) -> &[$item] {
                self.as_slice()
            }
        }

        impl<'a> IntoIterator for &'a $owner {
            type Item = &'a $item;
            type IntoIter = std::slice::Iter<'a, $item>;

            fn into_iter(self) -> Self::IntoIter {
                self.as_slice().iter()
            }
        }

        impl fmt::Debug for $owner {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($owner))
                    .field("len", &self.len())
                    .finish()
            }
        }

        /// An unpublished guarded payload. Creating this guard allocates nothing.
        pub(crate) struct $builder {
            node: $node,
        }

        impl Default for $builder {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $builder {
            pub(crate) fn new() -> Self {
                Self {
                    node: $node::new(Vec::new()),
                }
            }

            pub(crate) fn with_capacity(count: usize) -> Result<Self, StorageFailure> {
                let mut builder = Self::new();
                reserve_exact(&mut builder.node.items, count)?;
                Ok(builder)
            }

            pub(crate) fn try_push(&mut self, item: $item) -> Result<(), StorageFailure> {
                // Rejected raw children may contain a deep unary spine before
                // reaching an opaque owner. Release through the same queue.
                if let Err(error) = reserve(&mut self.node.items, 1) {
                    let mut pending = DropQueue::default();
                    $release(item, &mut pending);
                    pending.drain();
                    return Err(error);
                }
                self.node.items.push(item);
                Ok(())
            }

            pub(crate) fn finish(self) -> Result<$owner, StorageFailure> {
                allocate_node(self.node).map(|inner| $owner { inner: Some(inner) })
            }
        }
    };
}

child_owner!(
    Values,
    ValuesBuilder,
    ValueNode,
    NormalizedValue,
    Values,
    release_value
);
child_owner!(
    Fields,
    FieldsBuilder,
    FieldNode,
    (Name, NormalizedValue),
    Fields,
    release_field
);

impl ValuesBuilder {
    pub(crate) fn len(&self) -> usize {
        self.node.items.len()
    }

    #[cfg(test)]
    pub(crate) fn try_reserve(&mut self, additional: usize) -> Result<(), StorageFailure> {
        reserve(&mut self.node.items, additional)
    }
}

#[cfg(test)]
impl Values {
    /// Passive ownership evidence for existing collection-sharing controls.
    pub(crate) fn strong_count(&self) -> usize {
        self.inner.as_ref().map_or(0, Arc::strong_count)
    }
}

impl FieldsBuilder {
    /// Preserve authored order for equal names. The temporary index buffer is
    /// reserved before mutation; the payload stays guarded throughout sorting.
    pub(crate) fn sort_by_name(&mut self) -> Result<(), StorageFailure> {
        sort_named(&mut self.node.items)
    }
}

/// Stable field ordering for both unpublished raw and checked record payloads.
/// Reserve the complete index buffer before mutating any child position.
pub(crate) fn sort_named<T>(items: &mut [(Name, T)]) -> Result<(), StorageFailure> {
    let mut order = Vec::new();
    reserve_exact(&mut order, items.len())?;
    order.extend(0..items.len());
    order.sort_unstable_by(|left, right| {
        items[*left]
            .0
            .cmp(&items[*right].0)
            .then_with(|| left.cmp(right))
    });
    // order maps sorted positions to original positions. Each cycle rotation
    // fixes one previously unvisited slot per step, and uses only swaps.
    for start in 0..order.len() {
        let mut slot = start;
        loop {
            let source = order[slot];
            order[slot] = slot;
            if source == start {
                break;
            }
            items.swap(slot, source);
            slot = source;
        }
    }
    Ok(())
}

/// Function environments have explicit fallible raw structural comparison.
/// No infallible recursive Eq implementation or identity-equality substitution is exposed.
#[derive(Clone)]
pub struct Captures(Values);

impl Captures {
    pub(crate) fn from_values(values: Values) -> Self {
        Self(values)
    }

    #[cfg(test)]
    pub(crate) fn try_new(values: Vec<NormalizedValue>) -> Result<Self, StorageFailure> {
        Values::try_new(values).map(Self)
    }

    pub fn as_slice(&self) -> &[NormalizedValue] {
        self.0.as_slice()
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }

    fn into_pending(self) -> Option<Pending> {
        self.0.into_pending()
    }
}

impl Deref for Captures {
    type Target = [NormalizedValue];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl AsRef<[NormalizedValue]> for Captures {
    fn as_ref(&self) -> &[NormalizedValue] {
        self.as_slice()
    }
}

impl<'a> IntoIterator for &'a Captures {
    type Item = &'a NormalizedValue;
    type IntoIter = std::slice::Iter<'a, NormalizedValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl fmt::Debug for Captures {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Captures")
            .field("len", &self.len())
            .finish()
    }
}

fn enqueue(node: Option<Pending>, queue: &mut DropQueue) {
    if let Some(mut node) = node {
        let previous = queue.pending.take();
        match &mut node {
            Pending::Values(inner) => inner.next_cleanup = previous,
            Pending::Fields(inner) => inner.next_cleanup = previous,
            Pending::Element(inner) => inner.1 = previous,
            Pending::Entry(inner) => inner.next_cleanup = previous,
            Pending::Owned(inner) => inner.set_next(previous),
            Pending::View(inner) => inner.next = previous,
        }
        queue.pending = Some(node);
    }
}

fn drain_pending(queue: &mut DropQueue) {
    while let Some(node) = queue.pending.take() {
        match node {
            Pending::Values(mut inner) => {
                queue.pending = inner.next_cleanup.take();
                while let Some(value) = inner.items.pop() {
                    release_value(value, queue);
                }
            }
            Pending::Fields(mut inner) => {
                queue.pending = inner.next_cleanup.take();
                while let Some(field) = inner.items.pop() {
                    release_field(field, queue);
                }
            }
            Pending::Element(mut inner) => {
                queue.pending = inner.1.take();
                release_value(
                    std::mem::replace(&mut inner.0, NormalizedValue::Unit),
                    queue,
                );
            }
            Pending::Entry(mut inner) => {
                queue.pending = inner.next_cleanup.take();
                release_value(
                    std::mem::replace(&mut inner.value, NormalizedValue::Unit),
                    queue,
                );
            }
            Pending::Owned(mut inner) => {
                queue.pending = inner.take_next();
                while let Some(value) = inner.pop() {
                    release_value(value, queue);
                }
            }
            Pending::View(mut inner) => {
                queue.pending = inner.next.take();
                super::read_view_storage::drain_ready(&mut inner, queue);
            }
        }
        // Every node now has no pending link or owned child. Its own destructor
        // performs only an empty drain, independent of graph nesting.
    }
}

pub(super) fn release_element(element: &mut super::list::Element) {
    let mut queue = DropQueue {
        pending: element.1.take(),
    };
    queue.push_value(std::mem::replace(&mut element.0, NormalizedValue::Unit));
    queue.drain();
}

pub(super) fn release_entry(entry: &mut super::map::Entry) {
    let mut queue = DropQueue {
        pending: entry.next_cleanup.take(),
    };
    queue.push_value(std::mem::replace(&mut entry.value, NormalizedValue::Unit));
    queue.drain();
}

pub(super) fn release_retired(retired: &mut super::owned_storage::RetiredStorage) {
    let mut queue = DropQueue {
        pending: retired.take_next(),
    };
    while let Some(value) = retired.pop() {
        queue.push_value(value);
    }
    queue.drain();
}

pub(super) fn release_ready(ready: &mut super::read_view_storage::ReadyCleanup) {
    let mut queue = DropQueue {
        pending: ready.next.take(),
    };
    super::read_view_storage::drain_ready(ready, &mut queue);
    queue.drain();
}

fn release_field((name, value): (Name, NormalizedValue), pending: &mut DropQueue) {
    drop(name);
    release_value(value, pending);
}

fn release_value(mut value: NormalizedValue, pending: &mut DropQueue) {
    loop {
        value = match value {
            NormalizedValue::Option(Some(child))
            | NormalizedValue::Result { value: child, .. }
            | NormalizedValue::Variant {
                payload: Some(child),
                ..
            } => *child,
            NormalizedValue::Function {
                bound_arguments, ..
            } => {
                enqueue(bound_arguments.and_then(Captures::into_pending), pending);
                return;
            }
            NormalizedValue::Record(NormalizedRecord::Nominal { fields, .. }) => {
                enqueue(fields.into_pending(), pending);
                return;
            }
            NormalizedValue::Record(NormalizedRecord::Structural { fields }) => {
                enqueue(fields.into_pending(), pending);
                return;
            }
            NormalizedValue::List(mut list) => {
                list.drain_unique(pending);
                return;
            }
            NormalizedValue::Map(mut map) => {
                map.drain_unique(pending);
                return;
            }
            NormalizedValue::OwnedProduct(mut owner) => {
                pending.enqueue_owned(owner.revoke_for_cleanup());
                return;
            }
            NormalizedValue::OwnedChoice(mut owner) => {
                pending.enqueue_owned(owner.storage.revoke_for_cleanup());
                return;
            }
            NormalizedValue::OwnedSequence(mut owner) => {
                pending.enqueue_owned(owner.storage.revoke_for_cleanup());
                return;
            }
            NormalizedValue::ReadView(mut view) => {
                view.release_into(pending);
                return;
            }
            leaf => {
                drop(leaf);
                return;
            }
        };
    }
}

#[cfg(test)]
#[derive(Default)]
struct DropProbe {
    finished: std::sync::atomic::AtomicUsize,
    maximum_depth: std::sync::atomic::AtomicUsize,
}

#[cfg(test)]
thread_local! {
    static DROP_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
struct DropObservation(Option<std::sync::Arc<DropProbe>>);

#[cfg(test)]
impl DropObservation {
    fn enter(probe: Option<std::sync::Arc<DropProbe>>) -> Self {
        if let Some(probe) = &probe {
            let depth = DROP_DEPTH.with(|slot| {
                let next = slot.get().saturating_add(1);
                slot.set(next);
                next
            });
            probe
                .maximum_depth
                .fetch_max(depth, std::sync::atomic::Ordering::Relaxed);
        }
        Self(probe)
    }
}

#[cfg(test)]
impl Drop for DropObservation {
    fn drop(&mut self) {
        if let Some(probe) = self.0.take() {
            DROP_DEPTH.with(|slot| slot.set(slot.get().saturating_sub(1)));
            probe
                .finished
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
#[path = "child_storage_tests.rs"]
mod tests;
