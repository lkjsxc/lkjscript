use crate::error::{ErrorCode, LkError, Result};
use crate::ids::NodeId;
use std::num::NonZeroU64;

const INDEX_BITS: u32 = 30;
const INDEX_MASK: u32 = (1_u32 << INDEX_BITS) - 1;
const VIEW_KIND: u32 = 1;
const BACKING_KIND: u32 = 2;

pub const MAX_RUN_MANAGED_VISIBLE_BYTES: usize = 1024 * 1024;
pub const MAX_RUN_RETAINED_BACKING_BYTES: usize = 256 * 1024;
pub const MAX_RUN_MANAGED_OBJECTS: usize = 4_096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ByteHandle(NonZeroU64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BackingHandle(NonZeroU64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExecutionMode {
    #[cfg(test)]
    Oracle,
    Ownership,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ManagedLimits {
    pub cumulative_visible_bytes: usize,
    pub live_backing_bytes: usize,
    pub live_objects: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ManagedMetrics {
    pub cumulative_visible_bytes: usize,
    pub cumulative_allocated_bytes: usize,
    pub live_backing_bytes: usize,
    pub peak_live_backing_bytes: usize,
    pub live_objects: usize,
    pub peak_live_objects: usize,
    pub cumulative_objects: usize,
    pub retained_by_views: usize,
    pub peak_capacity_bytes: usize,
    pub copied_bytes: usize,
    pub reference_count_increments: u64,
    pub reference_count_decrements: u64,
    pub ownership_transfers: u64,
    pub borrowed_uses: u64,
    pub reuse_attempts: u64,
    pub reuse_hits: u64,
    pub reuse_fallbacks: u64,
}

#[derive(Debug)]
struct ByteView {
    backing: BackingHandle,
    start: usize,
    length: usize,
    owners: u32,
    full_backing: bool,
}

#[derive(Debug)]
struct ByteBacking {
    bytes: Vec<u8>,
    view_owners: u32,
    partial_views: u32,
}

#[derive(Debug)]
struct Slot<T> {
    generation: u32,
    value: Option<T>,
    retired: bool,
}

pub(crate) struct ManagedStore {
    limits: ManagedLimits,
    mode: ExecutionMode,
    backings: Vec<Slot<ByteBacking>>,
    views: Vec<Slot<ByteView>>,
    free_backings: Vec<usize>,
    free_views: Vec<usize>,
    metrics: ManagedMetrics,
    #[cfg(test)]
    drop_witness: Option<std::sync::Arc<std::sync::atomic::AtomicUsize>>,
    #[cfg(test)]
    fail_next_concat_allocation: bool,
    #[cfg(test)]
    fail_next_concat_growth: bool,
}

impl Default for ManagedStore {
    fn default() -> Self {
        Self::new(
            ManagedLimits {
                cumulative_visible_bytes: MAX_RUN_MANAGED_VISIBLE_BYTES,
                live_backing_bytes: MAX_RUN_RETAINED_BACKING_BYTES,
                live_objects: MAX_RUN_MANAGED_OBJECTS,
            },
            ExecutionMode::Ownership,
        )
    }
}

impl ManagedStore {
    pub(crate) fn new(limits: ManagedLimits, mode: ExecutionMode) -> Self {
        Self {
            limits,
            mode,
            backings: Vec::new(),
            views: Vec::new(),
            free_backings: Vec::new(),
            free_views: Vec::new(),
            metrics: ManagedMetrics::default(),
            #[cfg(test)]
            drop_witness: None,
            #[cfg(test)]
            fail_next_concat_allocation: false,
            #[cfg(test)]
            fail_next_concat_growth: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_drop_witness(
        limits: ManagedLimits,
        mode: ExecutionMode,
        witness: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    ) -> Self {
        let mut result = Self::new(limits, mode);
        result.drop_witness = Some(witness);
        result
    }

    #[cfg(test)]
    pub(crate) fn metrics(&self) -> ManagedMetrics {
        self.metrics
    }

    #[cfg(test)]
    fn fail_next_concat_allocation(&mut self) {
        self.fail_next_concat_allocation = true;
    }

    #[cfg(test)]
    fn fail_next_concat_growth(&mut self) {
        self.fail_next_concat_growth = true;
    }

    pub(crate) fn record_borrow(&mut self) -> Result<()> {
        self.metrics.borrowed_uses = self
            .metrics
            .borrowed_uses
            .checked_add(1)
            .ok_or_else(|| internal("managed borrow metric overflowed"))?;
        Ok(())
    }

    pub(crate) fn record_transfer(&mut self) -> Result<()> {
        self.metrics.ownership_transfers = self
            .metrics
            .ownership_transfers
            .checked_add(1)
            .ok_or_else(|| internal("managed transfer metric overflowed"))?;
        Ok(())
    }

    pub(crate) fn allocate_backing(&mut self, bytes: &[u8], origin: NodeId) -> Result<ByteHandle> {
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(bytes.len())
            .map_err(|_| allocation_error(origin))?;
        owned.extend_from_slice(bytes);
        self.metrics.copied_bytes = self
            .metrics
            .copied_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| internal("managed copied-byte metric overflowed"))?;
        self.allocate_owned(owned, bytes.len(), origin)
    }

    fn allocate_owned(
        &mut self,
        bytes: Vec<u8>,
        visible: usize,
        origin: NodeId,
    ) -> Result<ByteHandle> {
        self.preflight_visible(visible, origin)?;
        self.preflight_live_backing(bytes.len(), 0, origin)?;
        self.preflight_objects(2, origin)?;
        self.reserve_backing_slot(origin)?;
        self.reserve_view_slot(origin)?;

        let capacity = bytes.capacity();
        let backing = self.insert_backing(
            ByteBacking {
                bytes,
                view_owners: 1,
                partial_views: 0,
            },
            origin,
        )?;
        let length = self.backing(backing, origin)?.bytes.len();
        let view = match self.insert_view(
            ByteView {
                backing,
                start: 0,
                length,
                owners: 1,
                full_backing: true,
            },
            origin,
        ) {
            Ok(view) => view,
            Err(error) => {
                self.remove_backing_after_failed_view(backing, origin)?;
                return Err(error);
            }
        };
        self.metrics.cumulative_visible_bytes = self
            .metrics
            .cumulative_visible_bytes
            .checked_add(visible)
            .ok_or_else(|| internal("managed visible-byte metric overflowed"))?;
        self.metrics.cumulative_allocated_bytes = self
            .metrics
            .cumulative_allocated_bytes
            .checked_add(capacity)
            .ok_or_else(|| internal("managed allocation metric overflowed"))?;
        self.metrics.live_backing_bytes = self
            .metrics
            .live_backing_bytes
            .checked_add(length)
            .ok_or_else(|| internal("managed live-byte metric overflowed"))?;
        self.metrics.peak_live_backing_bytes = self
            .metrics
            .peak_live_backing_bytes
            .max(self.metrics.live_backing_bytes);
        self.metrics.peak_capacity_bytes = self.metrics.peak_capacity_bytes.max(capacity);
        Ok(view)
    }

    pub(crate) fn slice(
        &mut self,
        source: ByteHandle,
        start: i64,
        length: i64,
        origin: NodeId,
    ) -> Result<ByteHandle> {
        let start = usize::try_from(start).map_err(|_| slice_error(origin))?;
        let length = usize::try_from(length).map_err(|_| slice_error(origin))?;
        let (source_backing, source_start, source_length) = {
            let source_view = self.view(source, origin)?;
            (source_view.backing, source_view.start, source_view.length)
        };
        let relative_end = start
            .checked_add(length)
            .ok_or_else(|| slice_error(origin))?;
        if start > source_length || relative_end > source_length {
            return Err(slice_error(origin));
        }
        let absolute_start = source_start
            .checked_add(start)
            .ok_or_else(|| slice_error(origin))?;
        self.preflight_visible(length, origin)?;
        self.preflight_objects(1, origin)?;
        self.reserve_view_slot(origin)?;
        self.increment_backing_owner(source_backing, true, origin)?;
        let handle = match self.insert_view(
            ByteView {
                backing: source_backing,
                start: absolute_start,
                length,
                owners: 1,
                full_backing: false,
            },
            origin,
        ) {
            Ok(handle) => handle,
            Err(error) => {
                self.decrement_backing_owner(source_backing, true, origin)?;
                return Err(error);
            }
        };
        self.metrics.cumulative_visible_bytes = self
            .metrics
            .cumulative_visible_bytes
            .checked_add(length)
            .ok_or_else(|| internal("managed visible-byte metric overflowed"))?;
        self.refresh_retained_by_views()?;
        Ok(handle)
    }

    pub(crate) fn bytes(&self, handle: ByteHandle, origin: NodeId) -> Result<&[u8]> {
        let view = self.view(handle, origin)?;
        let backing = self.backing(view.backing, origin)?;
        let end = view
            .start
            .checked_add(view.length)
            .ok_or_else(|| invalid_handle(origin, "byte view range overflows"))?;
        backing
            .bytes
            .get(view.start..end)
            .ok_or_else(|| invalid_handle(origin, "byte view range exceeds its backing"))
    }

    pub(crate) fn share(&mut self, handle: ByteHandle, origin: NodeId) -> Result<()> {
        let (_, generation) = decode(handle.0, VIEW_KIND, origin)?;
        let index = decode(handle.0, VIEW_KIND, origin)?.0;
        let view = self
            .views
            .get_mut(index)
            .filter(|slot| slot.generation == generation)
            .and_then(|slot| slot.value.as_mut())
            .ok_or_else(|| invalid_handle(origin, "managed byte handle is stale"))?;
        view.owners = view.owners.checked_add(1).ok_or_else(|| {
            LkError::new(
                ErrorCode::ExecutionMemoryExhausted,
                "managed ownership count overflowed",
            )
            .for_node(origin)
        })?;
        self.metrics.reference_count_increments = self
            .metrics
            .reference_count_increments
            .checked_add(1)
            .ok_or_else(|| internal("reference-count increment metric overflowed"))?;
        Ok(())
    }

    pub(crate) fn drop_claim(&mut self, handle: ByteHandle, origin: NodeId) -> Result<()> {
        let (index, generation) = decode(handle.0, VIEW_KIND, origin)?;
        let (backing, partial, reclaim) = {
            let view = self
                .views
                .get_mut(index)
                .filter(|slot| slot.generation == generation)
                .and_then(|slot| slot.value.as_mut())
                .ok_or_else(|| invalid_handle(origin, "managed byte handle is stale"))?;
            view.owners = view.owners.checked_sub(1).ok_or_else(|| {
                invalid_handle(origin, "managed byte ownership count was already zero")
            })?;
            (view.backing, !view.full_backing, view.owners == 0)
        };
        self.metrics.reference_count_decrements = self
            .metrics
            .reference_count_decrements
            .checked_add(1)
            .ok_or_else(|| internal("reference-count decrement metric overflowed"))?;
        if reclaim {
            self.reclaim_view(index, generation, origin)?;
            self.decrement_backing_owner(backing, partial, origin)?;
            self.refresh_retained_by_views()?;
        }
        Ok(())
    }

    pub(crate) fn concat(
        &mut self,
        lhs: ByteHandle,
        rhs: ByteHandle,
        reuse_candidate: bool,
        maximum_value_bytes: usize,
        origin: NodeId,
    ) -> Result<(ByteHandle, bool)> {
        let lhs_len = self.bytes(lhs, origin)?.len();
        let rhs_len = self.bytes(rhs, origin)?.len();
        let result_len = lhs_len.checked_add(rhs_len).ok_or_else(|| {
            LkError::new(
                ErrorCode::ByteValueTooLarge,
                "byte concatenation length overflowed",
            )
            .for_node(origin)
        })?;
        if result_len > maximum_value_bytes {
            return Err(LkError::new(
                ErrorCode::ByteValueTooLarge,
                "byte concatenation result exceeds the byte value policy",
            )
            .for_node(origin));
        }
        self.preflight_visible(result_len, origin)?;
        if matches!(self.mode, ExecutionMode::Ownership) && reuse_candidate {
            self.metrics.reuse_attempts = self
                .metrics
                .reuse_attempts
                .checked_add(1)
                .ok_or_else(|| internal("concat reuse-attempt metric overflowed"))?;
            if self.can_reuse_left(lhs, rhs, origin)? {
                self.reuse_left(lhs, rhs, result_len, origin)?;
                self.metrics.reuse_hits = self
                    .metrics
                    .reuse_hits
                    .checked_add(1)
                    .ok_or_else(|| internal("concat reuse-hit metric overflowed"))?;
                self.metrics.cumulative_visible_bytes = self
                    .metrics
                    .cumulative_visible_bytes
                    .checked_add(result_len)
                    .ok_or_else(|| internal("managed visible-byte metric overflowed"))?;
                return Ok((lhs, true));
            }
            self.metrics.reuse_fallbacks = self
                .metrics
                .reuse_fallbacks
                .checked_add(1)
                .ok_or_else(|| internal("concat reuse-fallback metric overflowed"))?;
        }
        let mut bytes = Vec::new();
        #[cfg(test)]
        if std::mem::take(&mut self.fail_next_concat_allocation) {
            return Err(allocation_error(origin));
        }
        bytes
            .try_reserve_exact(result_len)
            .map_err(|_| allocation_error(origin))?;
        bytes.extend_from_slice(self.bytes(lhs, origin)?);
        bytes.extend_from_slice(self.bytes(rhs, origin)?);
        self.metrics.copied_bytes = self
            .metrics
            .copied_bytes
            .checked_add(result_len)
            .ok_or_else(|| internal("managed copied-byte metric overflowed"))?;
        Ok((self.allocate_owned(bytes, result_len, origin)?, false))
    }

    fn can_reuse_left(&self, lhs: ByteHandle, rhs: ByteHandle, origin: NodeId) -> Result<bool> {
        let left = self.view(lhs, origin)?;
        let right = self.view(rhs, origin)?;
        if lhs == rhs || left.backing == right.backing || left.owners != 1 || !left.full_backing {
            return Ok(false);
        }
        let backing = self.backing(left.backing, origin)?;
        Ok(backing.view_owners == 1 && left.start == 0 && left.length == backing.bytes.len())
    }

    fn reuse_left(
        &mut self,
        lhs: ByteHandle,
        rhs: ByteHandle,
        result_len: usize,
        origin: NodeId,
    ) -> Result<()> {
        let (right_backing, right_start, right_length) = {
            let view = self.view(rhs, origin)?;
            (view.backing, view.start, view.length)
        };
        let left_backing = self.view(lhs, origin)?.backing;
        let old_len = self.backing(left_backing, origin)?.bytes.len();
        let old_capacity = self.backing(left_backing, origin)?.bytes.capacity();
        self.preflight_live_backing(result_len, old_len, origin)?;
        #[cfg(test)]
        if std::mem::take(&mut self.fail_next_concat_growth) {
            return Err(allocation_error(origin));
        }

        let (left_index, left_generation) = decode(left_backing.0, BACKING_KIND, origin)?;
        let (right_index, right_generation) = decode(right_backing.0, BACKING_KIND, origin)?;
        if left_index == right_index
            || left_index >= self.backings.len()
            || right_index >= self.backings.len()
        {
            return Err(invalid_handle(
                origin,
                "concat reuse requires distinct live backing slots",
            ));
        }
        {
            let (left_slot, right_slot) = if left_index < right_index {
                let (left_slots, right_slots) = self.backings.split_at_mut(right_index);
                (&mut left_slots[left_index], &right_slots[0])
            } else {
                let (right_slots, left_slots) = self.backings.split_at_mut(left_index);
                (&mut left_slots[0], &right_slots[right_index])
            };
            if left_slot.generation != left_generation || right_slot.generation != right_generation
            {
                return Err(invalid_handle(
                    origin,
                    "concat reuse backing generation is stale",
                ));
            }
            let left_bytes = &mut left_slot
                .value
                .as_mut()
                .ok_or_else(|| invalid_handle(origin, "concat reuse left backing is dead"))?
                .bytes;
            let right_bytes = &right_slot
                .value
                .as_ref()
                .ok_or_else(|| invalid_handle(origin, "concat reuse right backing is dead"))?
                .bytes;
            let right_end = right_start
                .checked_add(right_length)
                .ok_or_else(|| invalid_handle(origin, "concat reuse right range overflows"))?;
            let right_range = right_bytes.get(right_start..right_end).ok_or_else(|| {
                invalid_handle(origin, "concat reuse right range exceeds its backing")
            })?;
            left_bytes
                .try_reserve(right_length)
                .map_err(|_| allocation_error(origin))?;
            left_bytes.extend_from_slice(right_range);
        }
        let capacity = self.backing(left_backing, origin)?.bytes.capacity();
        if capacity > old_capacity {
            self.metrics.cumulative_allocated_bytes = self
                .metrics
                .cumulative_allocated_bytes
                .checked_add(capacity)
                .ok_or_else(|| internal("managed allocation metric overflowed"))?;
        }
        let (index, generation) = decode(lhs.0, VIEW_KIND, origin)?;
        let view = self
            .views
            .get_mut(index)
            .filter(|slot| slot.generation == generation)
            .and_then(|slot| slot.value.as_mut())
            .ok_or_else(|| invalid_handle(origin, "concat reuse view became stale"))?;
        view.length = result_len;
        self.metrics.live_backing_bytes = self
            .metrics
            .live_backing_bytes
            .checked_sub(old_len)
            .and_then(|value| value.checked_add(result_len))
            .ok_or_else(|| internal("concat reuse live-byte accounting overflowed"))?;
        self.metrics.peak_live_backing_bytes = self
            .metrics
            .peak_live_backing_bytes
            .max(self.metrics.live_backing_bytes);
        self.metrics.peak_capacity_bytes = self.metrics.peak_capacity_bytes.max(capacity);
        self.metrics.copied_bytes = self
            .metrics
            .copied_bytes
            .checked_add(right_length)
            .ok_or_else(|| internal("concat reuse copied-byte metric overflowed"))?;
        Ok(())
    }

    fn preflight_visible(&self, added: usize, origin: NodeId) -> Result<()> {
        if self
            .metrics
            .cumulative_visible_bytes
            .checked_add(added)
            .is_none_or(|value| value > self.limits.cumulative_visible_bytes)
        {
            return Err(LkError::new(
                ErrorCode::ManagedVisibleBytePolicyExceeded,
                "invocation cumulative managed visible byte policy exceeded",
            )
            .for_node(origin));
        }
        Ok(())
    }

    fn preflight_live_backing(&self, added: usize, replaced: usize, origin: NodeId) -> Result<()> {
        if self
            .metrics
            .live_backing_bytes
            .checked_sub(replaced)
            .and_then(|value| value.checked_add(added))
            .is_none_or(|value| value > self.limits.live_backing_bytes)
        {
            return Err(LkError::new(
                ErrorCode::RetainedBytePolicyExceeded,
                "invocation live backing byte policy exceeded",
            )
            .for_node(origin));
        }
        Ok(())
    }

    fn preflight_objects(&self, added: usize, origin: NodeId) -> Result<()> {
        if self
            .metrics
            .live_objects
            .checked_add(added)
            .is_none_or(|value| value > self.limits.live_objects)
        {
            return Err(LkError::new(
                ErrorCode::ManagedObjectPolicyExceeded,
                "invocation live managed object policy exceeded",
            )
            .for_node(origin));
        }
        Ok(())
    }

    fn reserve_backing_slot(&mut self, origin: NodeId) -> Result<()> {
        if self.free_backings.is_empty() {
            self.backings
                .try_reserve(1)
                .map_err(|_| allocation_error(origin))?;
        }
        Ok(())
    }

    fn reserve_view_slot(&mut self, origin: NodeId) -> Result<()> {
        if self.free_views.is_empty() {
            self.views
                .try_reserve(1)
                .map_err(|_| allocation_error(origin))?;
        }
        Ok(())
    }

    fn insert_backing(&mut self, value: ByteBacking, origin: NodeId) -> Result<BackingHandle> {
        let (index, generation) =
            insert_slot(&mut self.backings, &mut self.free_backings, value, origin)?;
        self.record_object_creation()?;
        Ok(BackingHandle(encode(
            index,
            generation,
            BACKING_KIND,
            origin,
        )?))
    }

    fn insert_view(&mut self, value: ByteView, origin: NodeId) -> Result<ByteHandle> {
        let (index, generation) =
            insert_slot(&mut self.views, &mut self.free_views, value, origin)?;
        self.record_object_creation()?;
        Ok(ByteHandle(encode(index, generation, VIEW_KIND, origin)?))
    }

    fn record_object_creation(&mut self) -> Result<()> {
        self.metrics.live_objects = self
            .metrics
            .live_objects
            .checked_add(1)
            .ok_or_else(|| internal("managed live-object metric overflowed"))?;
        self.metrics.cumulative_objects = self
            .metrics
            .cumulative_objects
            .checked_add(1)
            .ok_or_else(|| internal("managed cumulative-object metric overflowed"))?;
        self.metrics.peak_live_objects = self
            .metrics
            .peak_live_objects
            .max(self.metrics.live_objects);
        Ok(())
    }

    fn view(&self, handle: ByteHandle, origin: NodeId) -> Result<&ByteView> {
        let (index, generation) = decode(handle.0, VIEW_KIND, origin)?;
        self.views
            .get(index)
            .filter(|slot| slot.generation == generation)
            .and_then(|slot| slot.value.as_ref())
            .ok_or_else(|| invalid_handle(origin, "managed byte handle is stale or out of bounds"))
    }

    fn backing(&self, handle: BackingHandle, origin: NodeId) -> Result<&ByteBacking> {
        let (index, generation) = decode(handle.0, BACKING_KIND, origin)?;
        self.backings
            .get(index)
            .filter(|slot| slot.generation == generation)
            .and_then(|slot| slot.value.as_ref())
            .ok_or_else(|| {
                invalid_handle(origin, "managed backing handle is stale or out of bounds")
            })
    }

    fn backing_mut(&mut self, handle: BackingHandle, origin: NodeId) -> Result<&mut ByteBacking> {
        let (index, generation) = decode(handle.0, BACKING_KIND, origin)?;
        self.backings
            .get_mut(index)
            .filter(|slot| slot.generation == generation)
            .and_then(|slot| slot.value.as_mut())
            .ok_or_else(|| {
                invalid_handle(origin, "managed backing handle is stale or out of bounds")
            })
    }

    fn increment_backing_owner(
        &mut self,
        handle: BackingHandle,
        partial: bool,
        origin: NodeId,
    ) -> Result<()> {
        let backing = self.backing_mut(handle, origin)?;
        backing.view_owners = backing.view_owners.checked_add(1).ok_or_else(|| {
            LkError::new(
                ErrorCode::ExecutionMemoryExhausted,
                "managed backing ownership count overflowed",
            )
            .for_node(origin)
        })?;
        if partial {
            backing.partial_views = backing.partial_views.checked_add(1).ok_or_else(|| {
                LkError::new(
                    ErrorCode::ExecutionMemoryExhausted,
                    "managed partial-view count overflowed",
                )
                .for_node(origin)
            })?;
        }
        self.metrics.reference_count_increments = self
            .metrics
            .reference_count_increments
            .checked_add(1)
            .ok_or_else(|| internal("reference-count increment metric overflowed"))?;
        Ok(())
    }

    fn decrement_backing_owner(
        &mut self,
        handle: BackingHandle,
        partial: bool,
        origin: NodeId,
    ) -> Result<()> {
        let (index, generation) = decode(handle.0, BACKING_KIND, origin)?;
        let reclaim = {
            let backing = self
                .backings
                .get_mut(index)
                .filter(|slot| slot.generation == generation)
                .and_then(|slot| slot.value.as_mut())
                .ok_or_else(|| invalid_handle(origin, "managed backing handle is stale"))?;
            backing.view_owners = backing.view_owners.checked_sub(1).ok_or_else(|| {
                invalid_handle(origin, "managed backing ownership count was already zero")
            })?;
            if partial {
                backing.partial_views = backing.partial_views.checked_sub(1).ok_or_else(|| {
                    invalid_handle(
                        origin,
                        "managed backing partial-view count was already zero",
                    )
                })?;
            }
            backing.view_owners == 0
        };
        self.metrics.reference_count_decrements = self
            .metrics
            .reference_count_decrements
            .checked_add(1)
            .ok_or_else(|| internal("reference-count decrement metric overflowed"))?;
        if reclaim {
            self.reclaim_backing(index, generation, origin)?;
        }
        Ok(())
    }

    fn reclaim_view(&mut self, index: usize, generation: u32, origin: NodeId) -> Result<()> {
        let slot = self
            .views
            .get_mut(index)
            .filter(|slot| slot.generation == generation)
            .ok_or_else(|| invalid_handle(origin, "managed view slot is stale"))?;
        if slot.value.take().is_none() {
            return Err(invalid_handle(origin, "managed view was reclaimed twice"));
        }
        retire_or_free(slot, index, &mut self.free_views);
        self.metrics.live_objects = self
            .metrics
            .live_objects
            .checked_sub(1)
            .ok_or_else(|| internal("managed live-object metric underflowed"))?;
        Ok(())
    }

    fn reclaim_backing(&mut self, index: usize, generation: u32, origin: NodeId) -> Result<()> {
        let slot = self
            .backings
            .get_mut(index)
            .filter(|slot| slot.generation == generation)
            .ok_or_else(|| invalid_handle(origin, "managed backing slot is stale"))?;
        let backing = slot
            .value
            .take()
            .ok_or_else(|| invalid_handle(origin, "managed backing was reclaimed twice"))?;
        if backing.view_owners != 0 {
            return Err(invalid_handle(
                origin,
                "managed backing reclaimed with live view owners",
            ));
        }
        self.metrics.live_backing_bytes = self
            .metrics
            .live_backing_bytes
            .checked_sub(backing.bytes.len())
            .ok_or_else(|| internal("managed live-byte metric underflowed"))?;
        retire_or_free(slot, index, &mut self.free_backings);
        self.metrics.live_objects = self
            .metrics
            .live_objects
            .checked_sub(1)
            .ok_or_else(|| internal("managed live-object metric underflowed"))?;
        Ok(())
    }

    fn remove_backing_after_failed_view(
        &mut self,
        handle: BackingHandle,
        origin: NodeId,
    ) -> Result<()> {
        let (index, generation) = decode(handle.0, BACKING_KIND, origin)?;
        let slot = self
            .backings
            .get_mut(index)
            .filter(|slot| slot.generation == generation)
            .ok_or_else(|| invalid_handle(origin, "failed backing allocation slot is stale"))?;
        slot.value = None;
        retire_or_free(slot, index, &mut self.free_backings);
        self.metrics.live_objects = self
            .metrics
            .live_objects
            .checked_sub(1)
            .ok_or_else(|| internal("failed backing object accounting underflowed"))?;
        Ok(())
    }

    fn refresh_retained_by_views(&mut self) -> Result<()> {
        self.metrics.retained_by_views =
            self.backings.iter().try_fold(0_usize, |total, slot| {
                let retained = slot
                    .value
                    .as_ref()
                    .filter(|backing| {
                        backing.partial_views > 0 && backing.partial_views == backing.view_owners
                    })
                    .map_or(0, |backing| backing.bytes.len());
                total
                    .checked_add(retained)
                    .ok_or_else(|| internal("view-retained backing metric overflowed"))
            })?;
        Ok(())
    }
}

impl Drop for ManagedStore {
    fn drop(&mut self) {
        #[cfg(test)]
        if let Some(witness) = &self.drop_witness {
            witness.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
}

fn insert_slot<T>(
    slots: &mut Vec<Slot<T>>,
    free: &mut Vec<usize>,
    value: T,
    origin: NodeId,
) -> Result<(usize, u32)> {
    if let Some(index) = free.pop() {
        let slot = slots
            .get_mut(index)
            .ok_or_else(|| invalid_handle(origin, "managed free-list index is out of bounds"))?;
        if slot.retired || slot.value.is_some() {
            return Err(invalid_handle(
                origin,
                "managed free-list slot is not reusable",
            ));
        }
        slot.generation = slot
            .generation
            .checked_add(1)
            .ok_or_else(|| invalid_handle(origin, "managed handle generation would wrap"))?;
        slot.value = Some(value);
        return Ok((index, slot.generation));
    }
    if slots.len() >= INDEX_MASK as usize {
        return Err(invalid_handle(
            origin,
            "managed handle index domain is exhausted",
        ));
    }
    slots.push(Slot {
        generation: 1,
        value: Some(value),
        retired: false,
    });
    Ok((slots.len() - 1, 1))
}

fn retire_or_free<T>(slot: &mut Slot<T>, index: usize, free: &mut Vec<usize>) {
    if slot.generation == u32::MAX {
        slot.retired = true;
    } else {
        free.push(index);
    }
}

fn encode(index: usize, generation: u32, kind: u32, origin: NodeId) -> Result<NonZeroU64> {
    let one_based = index
        .checked_add(1)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value <= INDEX_MASK)
        .ok_or_else(|| invalid_handle(origin, "managed handle index domain is exhausted"))?;
    if generation == 0 || kind == 0 || kind > 3 {
        return Err(invalid_handle(
            origin,
            "managed handle components are invalid",
        ));
    }
    let low = (kind << INDEX_BITS) | one_based;
    NonZeroU64::new((u64::from(generation) << 32) | u64::from(low))
        .ok_or_else(|| invalid_handle(origin, "managed handle encoded the invalid sentinel"))
}

fn decode(handle: NonZeroU64, expected_kind: u32, origin: NodeId) -> Result<(usize, u32)> {
    let raw = handle.get();
    let generation = u32::try_from(raw >> 32)
        .map_err(|_| invalid_handle(origin, "managed handle generation overflows u32"))?;
    let low = raw as u32;
    let kind = low >> INDEX_BITS;
    let one_based = low & INDEX_MASK;
    if generation == 0 || kind != expected_kind || one_based == 0 {
        return Err(invalid_handle(
            origin,
            "managed handle domain or kind is invalid",
        ));
    }
    let index = usize::try_from(one_based - 1)
        .map_err(|_| invalid_handle(origin, "managed handle index overflows host indexes"))?;
    Ok((index, generation))
}

fn invalid_handle(origin: NodeId, message: &str) -> LkError {
    LkError::new(ErrorCode::InvalidManagedHandle, message).for_node(origin)
}

fn slice_error(origin: NodeId) -> LkError {
    LkError::new(
        ErrorCode::ByteSliceOutOfBounds,
        "byte slice start and length must select a valid range",
    )
    .for_node(origin)
}

fn allocation_error(origin: NodeId) -> LkError {
    LkError::new(
        ErrorCode::ExecutionMemoryExhausted,
        "managed invocation allocation could not be reserved",
    )
    .for_node(origin)
}

fn internal(message: &str) -> LkError {
    LkError::new(ErrorCode::OwnershipPlanInvalid, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::WorkspaceId;

    fn origin() -> NodeId {
        NodeId::new(WorkspaceId::from_bytes([0x6d; 16]), 1).unwrap()
    }

    fn store(mode: ExecutionMode) -> ManagedStore {
        ManagedStore::new(
            ManagedLimits {
                cumulative_visible_bytes: 1_000_000,
                live_backing_bytes: 100_000,
                live_objects: 64,
            },
            mode,
        )
    }

    #[test]
    fn shared_views_reclaim_at_zero_and_reused_slots_reject_stale_generations() {
        let origin = origin();
        let mut store = store(ExecutionMode::Ownership);
        let root = store.allocate_backing(b"abcdef", origin).unwrap();
        let view = store.slice(root, 1, 2, origin).unwrap();
        store.share(view, origin).unwrap();
        store.drop_claim(root, origin).unwrap();
        assert_eq!(store.bytes(view, origin).unwrap(), b"bc");
        store.drop_claim(view, origin).unwrap();
        assert_eq!(store.bytes(view, origin).unwrap(), b"bc");
        store.drop_claim(view, origin).unwrap();
        assert_eq!(
            store.bytes(view, origin).unwrap_err().code,
            ErrorCode::InvalidManagedHandle
        );
        let replacement = store.allocate_backing(b"z", origin).unwrap();
        assert_eq!(store.bytes(replacement, origin).unwrap(), b"z");
        assert_eq!(
            store.bytes(view, origin).unwrap_err().code,
            ErrorCode::InvalidManagedHandle
        );
        store.drop_claim(replacement, origin).unwrap();
        assert_eq!(store.metrics().live_objects, 0);
        assert_eq!(store.metrics().live_backing_bytes, 0);
    }

    #[test]
    fn unique_concat_reuses_and_shared_concat_falls_back_exactly() {
        let origin = origin();
        let mut optimized = store(ExecutionMode::Ownership);
        let left = optimized.allocate_backing(b"ab", origin).unwrap();
        let right = optimized.allocate_backing(b"cd", origin).unwrap();
        let (joined, reused) = optimized.concat(left, right, true, 65_536, origin).unwrap();
        assert!(reused);
        assert_eq!(joined, left);
        assert_eq!(optimized.bytes(joined, origin).unwrap(), b"abcd");

        let mut shared = store(ExecutionMode::Ownership);
        let left = shared.allocate_backing(b"ab", origin).unwrap();
        shared.share(left, origin).unwrap();
        let right = shared.allocate_backing(b"cd", origin).unwrap();
        let (joined, reused) = shared.concat(left, right, true, 65_536, origin).unwrap();
        assert!(!reused);
        assert_ne!(joined, left);
        assert_eq!(shared.bytes(joined, origin).unwrap(), b"abcd");
        assert_eq!(shared.bytes(left, origin).unwrap(), b"ab");
    }

    #[test]
    fn concat_allocation_failures_leave_every_input_and_claim_valid() {
        let origin = origin();
        let mut growth = store(ExecutionMode::Ownership);
        let left = growth.allocate_backing(b"left", origin).unwrap();
        let right = growth.allocate_backing(b"right", origin).unwrap();
        growth.fail_next_concat_growth();
        assert_eq!(
            growth
                .concat(left, right, true, 65_536, origin)
                .unwrap_err()
                .code,
            ErrorCode::ExecutionMemoryExhausted
        );
        assert_eq!(growth.bytes(left, origin).unwrap(), b"left");
        assert_eq!(growth.bytes(right, origin).unwrap(), b"right");
        growth.drop_claim(left, origin).unwrap();
        growth.drop_claim(right, origin).unwrap();
        assert_eq!(growth.metrics().live_objects, 0);

        let mut fallback = store(ExecutionMode::Ownership);
        let left = fallback.allocate_backing(b"left", origin).unwrap();
        fallback.share(left, origin).unwrap();
        let right = fallback.allocate_backing(b"right", origin).unwrap();
        fallback.fail_next_concat_allocation();
        assert_eq!(
            fallback
                .concat(left, right, true, 65_536, origin)
                .unwrap_err()
                .code,
            ErrorCode::ExecutionMemoryExhausted
        );
        assert_eq!(fallback.bytes(left, origin).unwrap(), b"left");
        assert_eq!(fallback.bytes(right, origin).unwrap(), b"right");
        fallback.drop_claim(left, origin).unwrap();
        fallback.drop_claim(left, origin).unwrap();
        fallback.drop_claim(right, origin).unwrap();
        assert_eq!(fallback.metrics().live_objects, 0);
    }

    #[test]
    fn handle_kind_index_generation_and_count_boundaries_reject() {
        let origin = origin();
        let mut store = store(ExecutionMode::Ownership);
        let root = store.allocate_backing(b"root", origin).unwrap();
        let backing = store.view(root, origin).unwrap().backing;
        let wrong_kind = ByteHandle(backing.0);
        assert_eq!(
            store.bytes(wrong_kind, origin).unwrap_err().code,
            ErrorCode::InvalidManagedHandle
        );
        let out_of_range = ByteHandle(encode(17, 1, VIEW_KIND, origin).unwrap());
        assert_eq!(
            store.bytes(out_of_range, origin).unwrap_err().code,
            ErrorCode::InvalidManagedHandle
        );

        let (index, generation) = decode(root.0, VIEW_KIND, origin).unwrap();
        store.views[index].value.as_mut().unwrap().owners = u32::MAX;
        assert_eq!(
            store.share(root, origin).unwrap_err().code,
            ErrorCode::ExecutionMemoryExhausted
        );
        store.views[index].value.as_mut().unwrap().owners = 1;
        assert_eq!(store.views[index].generation, generation);

        store.views[index].generation = u32::MAX;
        let final_generation = ByteHandle(encode(index, u32::MAX, VIEW_KIND, origin).unwrap());
        store.drop_claim(final_generation, origin).unwrap();
        assert!(store.views[index].retired);
        let replacement = store.allocate_backing(b"replacement", origin).unwrap();
        assert_ne!(decode(replacement.0, VIEW_KIND, origin).unwrap().0, index);
        assert_eq!(
            store.bytes(final_generation, origin).unwrap_err().code,
            ErrorCode::InvalidManagedHandle
        );
    }

    #[test]
    fn reclaimed_descriptors_are_reused_beyond_the_live_object_limit() {
        let origin = origin();
        let mut store = ManagedStore::new(
            ManagedLimits {
                cumulative_visible_bytes: 10_000,
                live_backing_bytes: 10_000,
                live_objects: 2,
            },
            ExecutionMode::Ownership,
        );
        let mut previous = None;
        for _ in 0..1_000 {
            let handle = store.allocate_backing(b"x", origin).unwrap();
            if let Some(previous) = previous {
                assert_eq!(
                    store.bytes(previous, origin).unwrap_err().code,
                    ErrorCode::InvalidManagedHandle
                );
            }
            store.drop_claim(handle, origin).unwrap();
            previous = Some(handle);
        }
        assert_eq!(store.metrics().live_objects, 0);
        assert_eq!(store.metrics().peak_live_objects, 2);
        assert_eq!(store.metrics().cumulative_objects, 2_000);
        assert_eq!(store.views.len(), 1);
        assert_eq!(store.backings.len(), 1);
    }

    fn append_one_octet_workload(
        mode: ExecutionMode,
        octets: usize,
        force_shared_accumulator: bool,
    ) -> (Vec<u8>, ManagedMetrics) {
        let origin = origin();
        let mut store = ManagedStore::new(
            ManagedLimits {
                cumulative_visible_bytes: 1_000_000,
                live_backing_bytes: 100_000,
                live_objects: 64,
            },
            mode,
        );
        let mut accumulator = store.allocate_backing(b"", origin).unwrap();
        for index in 0..octets {
            let octet = [u8::try_from(index % 251 + 1).unwrap()];
            let right = store.allocate_backing(&octet, origin).unwrap();
            if force_shared_accumulator {
                store.share(accumulator, origin).unwrap();
            }
            let (next, reused) = store
                .concat(accumulator, right, true, 65_536, origin)
                .unwrap();
            store.drop_claim(right, origin).unwrap();
            if !reused {
                store.drop_claim(accumulator, origin).unwrap();
            }
            if force_shared_accumulator {
                assert!(!reused);
                store.drop_claim(accumulator, origin).unwrap();
            }
            accumulator = next;
        }
        let result = store.bytes(accumulator, origin).unwrap().to_vec();
        store.drop_claim(accumulator, origin).unwrap();
        let metrics = store.metrics();
        assert_eq!(metrics.live_objects, 0);
        assert_eq!(metrics.live_backing_bytes, 0);
        (result, metrics)
    }

    #[test]
    fn loop_carried_construction_reclaims_early_and_reuses_unique_storage() {
        let octets = 512;
        let (oracle, oracle_metrics) =
            append_one_octet_workload(ExecutionMode::Oracle, octets, false);
        let (optimized, optimized_metrics) =
            append_one_octet_workload(ExecutionMode::Ownership, octets, false);
        let (shared, shared_metrics) =
            append_one_octet_workload(ExecutionMode::Ownership, octets, true);
        assert_eq!(optimized, oracle);
        assert_eq!(shared, oracle);
        assert_eq!(
            optimized_metrics.cumulative_visible_bytes,
            oracle_metrics.cumulative_visible_bytes
        );
        assert_eq!(optimized_metrics.reuse_attempts, octets as u64);
        assert_eq!(optimized_metrics.reuse_hits, octets as u64);
        assert_eq!(optimized_metrics.reuse_fallbacks, 0);
        assert!(optimized_metrics.peak_live_backing_bytes < oracle_metrics.peak_live_backing_bytes);
        assert!(optimized_metrics.copied_bytes < oracle_metrics.copied_bytes);
        assert!(
            optimized_metrics.cumulative_allocated_bytes
                < oracle_metrics.cumulative_allocated_bytes
        );
        assert_eq!(shared_metrics.reuse_attempts, octets as u64);
        assert_eq!(shared_metrics.reuse_hits, 0);
        assert_eq!(shared_metrics.reuse_fallbacks, octets as u64);
        assert_eq!(shared_metrics.reference_count_increments, octets as u64);
        assert_eq!(shared_metrics.copied_bytes, oracle_metrics.copied_bytes);
        eprintln!(
            "managed-construction octets={octets} oracle={oracle_metrics:?} optimized={optimized_metrics:?} shared={shared_metrics:?}"
        );
    }
}
