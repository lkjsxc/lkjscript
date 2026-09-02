//! Task-owned opaque resources for normalized execution.

use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{DeclarationReference, RequirementReference};
use crate::platform::queue::JobLease;
use crate::platform::stream::StreamLease;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

const MAXIMUM_TASK_RESOURCES: usize = 65_536;
static NEXT_RESOURCE_SCOPE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum NormalizedResourceKind {
    ByteStream,
    QueueLease,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct NormalizedResourceHandle {
    scope: NormalizedResourceScopeId,
    slot: u64,
    kind: NormalizedResourceKind,
    authority: RequirementReference,
    interface: DeclarationReference,
}

impl NormalizedResourceHandle {
    pub(crate) const fn is_affine_capability(self) -> bool {
        matches!(self.kind, NormalizedResourceKind::QueueLease)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct NormalizedResourceScopeId(u64);

pub(crate) struct NormalizedResourceScope {
    id: NormalizedResourceScopeId,
    maximum_entries: usize,
    state: Mutex<ResourceState>,
}

struct ResourceState {
    next_slot: u64,
    entries: BTreeMap<u64, ResourceEntry>,
}

enum ResourceEntry {
    ByteStream(Arc<StreamLease>),
    ReservedQueueLease,
    QueueLease(JobLease),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QueueLeaseInfo {
    pub(crate) job_id: String,
    pub(crate) payload: Vec<u8>,
    pub(crate) attempt_number: u32,
    pub(crate) lease_until_milliseconds: i64,
}

pub(crate) struct QueueLeaseReservation<'a> {
    scope: &'a NormalizedResourceScope,
    handle: NormalizedResourceHandle,
    active: bool,
}

impl NormalizedResourceScope {
    pub(crate) fn new() -> Result<Self, ExecutionError> {
        let id = NEXT_RESOURCE_SCOPE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| {
                ExecutionError::resource(
                    "normalized_resource_scope_exhausted",
                    "process-local task resource scope identity is exhausted",
                )
            })?;
        Ok(Self::with_id(NormalizedResourceScopeId(id)))
    }

    fn with_id(id: NormalizedResourceScopeId) -> Self {
        Self::with_id_and_limit(id, MAXIMUM_TASK_RESOURCES)
    }

    fn with_id_and_limit(id: NormalizedResourceScopeId, maximum_entries: usize) -> Self {
        Self {
            id,
            maximum_entries,
            state: Mutex::new(ResourceState {
                next_slot: 1,
                entries: BTreeMap::new(),
            }),
        }
    }

    pub(crate) fn register_byte_stream(
        &self,
        authority: RequirementReference,
        interface: DeclarationReference,
        lease: StreamLease,
    ) -> Result<NormalizedResourceHandle, ExecutionError> {
        let mut state = lock_unpoisoned(&self.state);
        if state.entries.len() >= self.maximum_entries {
            return Err(ExecutionError::resource(
                "normalized_resource_limit",
                "task resource scope reached its live-handle limit",
            ));
        }
        let slot = state.next_slot;
        state.next_slot = state.next_slot.checked_add(1).ok_or_else(|| {
            ExecutionError::resource(
                "normalized_resource_identity_exhausted",
                "task resource handle identity is exhausted",
            )
        })?;
        if slot == 0
            || state
                .entries
                .insert(slot, ResourceEntry::ByteStream(Arc::new(lease)))
                .is_some()
        {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "normalized_resource_identity_reused",
                "task resource handle identity was unexpectedly reused",
            ));
        }
        Ok(NormalizedResourceHandle {
            scope: self.id,
            slot,
            kind: NormalizedResourceKind::ByteStream,
            authority,
            interface,
        })
    }

    pub(crate) fn reserve_queue_lease(
        &self,
        authority: RequirementReference,
        interface: DeclarationReference,
    ) -> Result<QueueLeaseReservation<'_>, ExecutionError> {
        let mut state = lock_unpoisoned(&self.state);
        if state.entries.len() >= self.maximum_entries {
            return Err(ExecutionError::resource(
                "normalized_resource_limit",
                "task resource scope reached its live-handle limit before queue effect",
            ));
        }
        let slot = state.next_slot;
        state.next_slot = state.next_slot.checked_add(1).ok_or_else(|| {
            ExecutionError::resource(
                "normalized_resource_identity_exhausted",
                "task resource handle identity is exhausted before queue effect",
            )
        })?;
        if slot == 0
            || state
                .entries
                .insert(slot, ResourceEntry::ReservedQueueLease)
                .is_some()
        {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "normalized_resource_identity_reused",
                "task resource handle identity was unexpectedly reused",
            ));
        }
        Ok(QueueLeaseReservation {
            scope: self,
            handle: NormalizedResourceHandle {
                scope: self.id,
                slot,
                kind: NormalizedResourceKind::QueueLease,
                authority,
                interface,
            },
            active: true,
        })
    }

    pub(crate) fn borrow_queue_lease(
        &self,
        authority: RequirementReference,
        interface: DeclarationReference,
        handle: NormalizedResourceHandle,
    ) -> Result<QueueLeaseInfo, ExecutionError> {
        self.validate_capability_handle(
            handle,
            NormalizedResourceKind::QueueLease,
            authority,
            interface,
        )?;
        match lock_unpoisoned(&self.state).entries.get(&handle.slot) {
            Some(ResourceEntry::QueueLease(lease)) => Ok(QueueLeaseInfo {
                job_id: lease.job_id.clone(),
                payload: lease.payload.clone(),
                attempt_number: lease.attempt_number,
                lease_until_milliseconds: lease.lease_until_milliseconds,
            }),
            Some(ResourceEntry::ReservedQueueLease) | None => Err(closed_resource()),
            Some(ResourceEntry::ByteStream(_)) => Err(resource_kind_entry()),
        }
    }

    pub(crate) fn consume_queue_lease(
        &self,
        authority: RequirementReference,
        interface: DeclarationReference,
        handle: NormalizedResourceHandle,
    ) -> Result<JobLease, ExecutionError> {
        self.validate_capability_handle(
            handle,
            NormalizedResourceKind::QueueLease,
            authority,
            interface,
        )?;
        let mut state = lock_unpoisoned(&self.state);
        if !matches!(
            state.entries.get(&handle.slot),
            Some(ResourceEntry::QueueLease(_))
        ) {
            return match state.entries.get(&handle.slot) {
                Some(ResourceEntry::ByteStream(_)) => Err(resource_kind_entry()),
                Some(ResourceEntry::ReservedQueueLease) | None => Err(closed_resource()),
                Some(ResourceEntry::QueueLease(_)) => Err(closed_resource()),
            };
        }
        match state.entries.remove(&handle.slot) {
            Some(ResourceEntry::QueueLease(lease)) => Ok(lease),
            _ => Err(closed_resource()),
        }
    }

    pub(crate) fn validate_queue_lease_transfer(
        &self,
        authority: RequirementReference,
        interface: DeclarationReference,
        handle: NormalizedResourceHandle,
    ) -> Result<(), ExecutionError> {
        self.validate_capability_handle(
            handle,
            NormalizedResourceKind::QueueLease,
            authority,
            interface,
        )?;
        match lock_unpoisoned(&self.state).entries.get(&handle.slot) {
            Some(ResourceEntry::QueueLease(_)) => Ok(()),
            Some(ResourceEntry::ByteStream(_)) => Err(resource_kind_entry()),
            Some(ResourceEntry::ReservedQueueLease) | None => Err(closed_resource()),
        }
    }

    pub(crate) fn read_byte_stream(
        &self,
        authority: RequirementReference,
        handle: NormalizedResourceHandle,
        control: &ExecutionControl,
    ) -> Result<Option<Vec<u8>>, ExecutionError> {
        self.byte_stream(authority, handle)?.read(control)
    }

    pub(crate) fn read_all_byte_stream(
        &self,
        authority: RequirementReference,
        handle: NormalizedResourceHandle,
        maximum_bytes: usize,
        control: &ExecutionControl,
    ) -> Result<Vec<u8>, ExecutionError> {
        let lease = self.remove_byte_stream(authority, handle)?;
        lease.read_all(maximum_bytes, control)
    }

    pub(crate) fn close(
        &self,
        authority: RequirementReference,
        handle: NormalizedResourceHandle,
    ) -> Result<(), ExecutionError> {
        self.validate_handle(handle, NormalizedResourceKind::ByteStream, authority)?;
        if let Some(ResourceEntry::ByteStream(lease)) =
            lock_unpoisoned(&self.state).entries.remove(&handle.slot)
        {
            lease.close_registered();
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn live_resources(&self) -> usize {
        lock_unpoisoned(&self.state).entries.len()
    }

    fn byte_stream(
        &self,
        authority: RequirementReference,
        handle: NormalizedResourceHandle,
    ) -> Result<Arc<StreamLease>, ExecutionError> {
        self.validate_handle(handle, NormalizedResourceKind::ByteStream, authority)?;
        match lock_unpoisoned(&self.state).entries.get(&handle.slot) {
            Some(ResourceEntry::ByteStream(lease)) => Ok(Arc::clone(lease)),
            Some(ResourceEntry::ReservedQueueLease) | Some(ResourceEntry::QueueLease(_)) => {
                Err(resource_kind_entry())
            }
            None => Err(closed_resource()),
        }
    }

    fn remove_byte_stream(
        &self,
        authority: RequirementReference,
        handle: NormalizedResourceHandle,
    ) -> Result<Arc<StreamLease>, ExecutionError> {
        self.validate_handle(handle, NormalizedResourceKind::ByteStream, authority)?;
        match lock_unpoisoned(&self.state).entries.remove(&handle.slot) {
            Some(ResourceEntry::ByteStream(lease)) => Ok(lease),
            Some(ResourceEntry::ReservedQueueLease) | Some(ResourceEntry::QueueLease(_)) => {
                Err(resource_kind_entry())
            }
            None => Err(closed_resource()),
        }
    }

    fn validate_handle(
        &self,
        handle: NormalizedResourceHandle,
        expected: NormalizedResourceKind,
        authority: RequirementReference,
    ) -> Result<(), ExecutionError> {
        if handle.scope != self.id {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Capability,
                "normalized_resource_foreign_scope",
                "runtime resource handle belongs to another task scope",
            ));
        }
        if handle.kind != expected {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Capability,
                "normalized_resource_kind",
                "runtime resource handle has a foreign resource kind",
            ));
        }
        if handle.authority != authority {
            return Err(foreign_authority());
        }
        Ok(())
    }

    fn validate_capability_handle(
        &self,
        handle: NormalizedResourceHandle,
        expected: NormalizedResourceKind,
        authority: RequirementReference,
        interface: DeclarationReference,
    ) -> Result<(), ExecutionError> {
        self.validate_handle(handle, expected, authority)?;
        if handle.interface != interface {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Capability,
                "normalized_resource_interface",
                "runtime resource belongs to another exact capability interface",
            ));
        }
        Ok(())
    }
}

impl QueueLeaseReservation<'_> {
    pub(crate) fn commit(
        mut self,
        lease: JobLease,
    ) -> Result<NormalizedResourceHandle, ExecutionError> {
        let mut state = lock_unpoisoned(&self.scope.state);
        match state.entries.get_mut(&self.handle.slot) {
            Some(entry @ ResourceEntry::ReservedQueueLease) => {
                *entry = ResourceEntry::QueueLease(lease);
                self.active = false;
                Ok(self.handle)
            }
            Some(ResourceEntry::ByteStream(_)) | Some(ResourceEntry::QueueLease(_)) | None => {
                Err(ExecutionError::new(
                    ExecutionFailureClass::Infrastructure,
                    "normalized_resource_reservation_lost",
                    "queue lease reservation was lost before authority could be installed",
                ))
            }
        }
    }
}

impl Drop for QueueLeaseReservation<'_> {
    fn drop(&mut self) {
        if self.active {
            let mut state = lock_unpoisoned(&self.scope.state);
            if matches!(
                state.entries.get(&self.handle.slot),
                Some(ResourceEntry::ReservedQueueLease)
            ) {
                state.entries.remove(&self.handle.slot);
            }
        }
    }
}

fn closed_resource() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Capability,
        "normalized_resource_closed",
        "runtime resource handle is closed or absent from its task scope",
    )
}

fn foreign_authority() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Capability,
        "normalized_resource_authority",
        "runtime resource belongs to another exact capability requirement",
    )
}

fn resource_kind_entry() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Capability,
        "normalized_resource_entry_kind",
        "runtime resource slot contains another resource kind",
    )
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::execution::normalized::reference::reference_equal;
    use crate::platform::execution::normalized::value::NormalizedValue;
    use crate::platform::execution::normalized::vm::normalized_equal;
    use crate::platform::kernel::PackageId;
    use crate::platform::semantic_id::{DeclarationId, RequirementId};
    use crate::platform::stream::{StreamLimits, StreamRegistry};

    fn queue_lease() -> JobLease {
        JobLease {
            job_id: "job-1".to_owned(),
            attempt_id: "private-attempt-1".to_owned(),
            worker_id: "private-worker-1".to_owned(),
            payload: b"payload".to_vec(),
            attempt_number: 1,
            lease_until_milliseconds: 20,
        }
    }

    fn authority(ordinal: u64) -> RequirementReference {
        RequirementReference {
            package: PackageId::migrate(b"normalized-resource-test", 0),
            requirement: RequirementId::migrate(b"normalized-resource-test", ordinal),
        }
    }

    fn interface(ordinal: u64) -> DeclarationReference {
        DeclarationReference {
            package: PackageId::migrate(b"normalized-resource-test", 0),
            declaration: DeclarationId::migrate(b"normalized-resource-test", ordinal),
        }
    }

    #[test]
    fn stream_handles_are_scope_bound_and_scope_drop_closes_leases() {
        let registry = StreamRegistry::new(StreamLimits::default()).expect("stream registry");
        let first = NormalizedResourceScope::with_id(NormalizedResourceScopeId(101));
        let second = NormalizedResourceScope::with_id(NormalizedResourceScopeId(102));
        let handle = first
            .register_byte_stream(
                authority(0),
                interface(0),
                registry
                    .register_memory(b"bounded".to_vec())
                    .expect("memory stream"),
            )
            .expect("resource handle");
        assert_eq!(handle.slot, 1);
        assert_eq!(first.live_resources(), 1);
        assert_eq!(registry.live_streams(), 1);

        let error = second
            .read_byte_stream(authority(0), handle, &ExecutionControl::uncancelled())
            .expect_err("foreign scope must reject");
        assert_eq!(error.code, "normalized_resource_foreign_scope");
        assert_eq!(
            first
                .read_byte_stream(authority(0), handle, &ExecutionControl::uncancelled())
                .expect("stream read"),
            Some(b"bounded".to_vec())
        );

        drop(first);
        assert_eq!(registry.live_streams(), 0);
    }

    #[test]
    fn close_removes_handle_exactly() {
        let registry = StreamRegistry::new(StreamLimits::default()).expect("stream registry");
        let scope = NormalizedResourceScope::with_id(NormalizedResourceScopeId(103));
        let handle = scope
            .register_byte_stream(
                authority(0),
                interface(0),
                registry
                    .register_memory(b"closed".to_vec())
                    .expect("memory stream"),
            )
            .expect("resource handle");
        let foreign = scope
            .close(authority(1), handle)
            .expect_err("foreign requirement must not close the resource");
        assert_eq!(foreign.code, "normalized_resource_authority");
        scope.close(authority(0), handle).expect("resource close");
        assert_eq!(scope.live_resources(), 0);
        assert_eq!(registry.live_streams(), 0);
        scope
            .close(authority(0), handle)
            .expect("resource close is idempotent");
    }

    #[test]
    fn live_resources_have_no_semantic_equality() {
        let registry = StreamRegistry::new(StreamLimits::default()).expect("stream registry");
        let scope = NormalizedResourceScope::with_id(NormalizedResourceScopeId(104));
        let handle = scope
            .register_byte_stream(
                authority(0),
                interface(0),
                registry
                    .register_memory(b"opaque".to_vec())
                    .expect("memory stream"),
            )
            .expect("resource handle");
        let value = NormalizedValue::Resource(handle);
        assert_eq!(
            normalized_equal(&value, &value)
                .expect_err("production equality must reject resources")
                .code,
            "normalized_value_not_comparable"
        );
        assert_eq!(
            reference_equal(&value, &value)
                .expect_err("reference equality must reject resources")
                .code,
            "normalized_reference_value_not_comparable"
        );
    }

    #[test]
    fn queue_lease_reservation_precedes_effect_and_drop_releases_capacity() {
        let scope = NormalizedResourceScope::with_id_and_limit(NormalizedResourceScopeId(105), 1);
        let reservation = scope
            .reserve_queue_lease(authority(0), interface(0))
            .expect("first reservation");
        assert_eq!(scope.live_resources(), 1);
        let exhausted = scope
            .reserve_queue_lease(authority(0), interface(0))
            .err()
            .expect("capacity must reject before a queue effect can begin");
        assert_eq!(exhausted.code, "normalized_resource_limit");
        drop(reservation);
        assert_eq!(scope.live_resources(), 0);

        let handle = scope
            .reserve_queue_lease(authority(0), interface(0))
            .expect("replacement reservation")
            .commit(queue_lease())
            .expect("commit reserved lease");
        assert_eq!(handle.slot, 2);
        assert_eq!(scope.live_resources(), 1);
    }

    #[test]
    fn queue_lease_borrow_preserves_and_consume_closes_exact_authority() {
        let scope = NormalizedResourceScope::with_id(NormalizedResourceScopeId(106));
        let handle = scope
            .reserve_queue_lease(authority(0), interface(0))
            .expect("lease reservation")
            .commit(queue_lease())
            .expect("lease commit");

        let info = scope
            .borrow_queue_lease(authority(0), interface(0), handle)
            .expect("first metadata borrow");
        assert_eq!(info.job_id, "job-1");
        assert_eq!(info.payload, b"payload");
        assert_eq!(info.attempt_number, 1);
        assert_eq!(scope.live_resources(), 1);
        assert_eq!(
            scope
                .borrow_queue_lease(authority(1), interface(0), handle)
                .expect_err("foreign requirement")
                .code,
            "normalized_resource_authority"
        );
        assert_eq!(
            scope
                .borrow_queue_lease(authority(0), interface(1), handle)
                .expect_err("foreign interface")
                .code,
            "normalized_resource_interface"
        );

        let lease = scope
            .consume_queue_lease(authority(0), interface(0), handle)
            .expect("terminal consume");
        assert_eq!(lease.attempt_id, "private-attempt-1");
        assert_eq!(lease.worker_id, "private-worker-1");
        assert_eq!(scope.live_resources(), 0);
        assert_eq!(
            scope
                .consume_queue_lease(authority(0), interface(0), handle)
                .expect_err("duplicate hostile handle copy must be closed")
                .code,
            "normalized_resource_closed"
        );
    }

    #[test]
    fn queue_lease_rejects_foreign_scope_kind_and_uncommitted_slot() {
        let first = NormalizedResourceScope::with_id(NormalizedResourceScopeId(107));
        let second = NormalizedResourceScope::with_id(NormalizedResourceScopeId(108));
        let reservation = first
            .reserve_queue_lease(authority(0), interface(0))
            .expect("lease reservation");
        let handle = reservation.handle;
        assert_eq!(
            first
                .borrow_queue_lease(authority(0), interface(0), handle)
                .expect_err("uncommitted slot")
                .code,
            "normalized_resource_closed"
        );
        assert_eq!(
            second
                .borrow_queue_lease(authority(0), interface(0), handle)
                .expect_err("foreign scope")
                .code,
            "normalized_resource_foreign_scope"
        );
        let mut wrong_kind = handle;
        wrong_kind.kind = NormalizedResourceKind::ByteStream;
        assert_eq!(
            first
                .borrow_queue_lease(authority(0), interface(0), wrong_kind)
                .expect_err("foreign resource kind")
                .code,
            "normalized_resource_kind"
        );
        drop(reservation);
        assert_eq!(first.live_resources(), 0);
    }

    #[test]
    fn queue_lease_transfer_rechecks_exact_live_authority_without_consuming_it() {
        let first = NormalizedResourceScope::with_id(NormalizedResourceScopeId(109));
        let second = NormalizedResourceScope::with_id(NormalizedResourceScopeId(110));
        let handle = first
            .reserve_queue_lease(authority(0), interface(0))
            .expect("lease reservation")
            .commit(queue_lease())
            .expect("lease commit");

        first
            .validate_queue_lease_transfer(authority(0), interface(0), handle)
            .expect("exact live handoff validation");
        assert_eq!(first.live_resources(), 1);
        assert_eq!(
            first
                .validate_queue_lease_transfer(authority(1), interface(0), handle)
                .expect_err("foreign requirement")
                .code,
            "normalized_resource_authority"
        );
        assert_eq!(
            first
                .validate_queue_lease_transfer(authority(0), interface(1), handle)
                .expect_err("foreign interface")
                .code,
            "normalized_resource_interface"
        );
        assert_eq!(
            second
                .validate_queue_lease_transfer(authority(0), interface(0), handle)
                .expect_err("foreign task scope")
                .code,
            "normalized_resource_foreign_scope"
        );

        first
            .consume_queue_lease(authority(0), interface(0), handle)
            .expect("terminal consume after handoff validation");
        assert_eq!(first.live_resources(), 0);
        assert_eq!(
            first
                .validate_queue_lease_transfer(authority(0), interface(0), handle)
                .expect_err("copied or revived closed handle")
                .code,
            "normalized_resource_closed"
        );
    }
}
