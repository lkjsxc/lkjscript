//! Task-owned opaque resources for normalized execution.

use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::RequirementReference;
use crate::platform::stream::StreamLease;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

const MAXIMUM_TASK_RESOURCES: usize = 65_536;
static NEXT_RESOURCE_SCOPE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum NormalizedResourceKind {
    ByteStream,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct NormalizedResourceHandle {
    scope: NormalizedResourceScopeId,
    slot: u64,
    kind: NormalizedResourceKind,
    authority: RequirementReference,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct NormalizedResourceScopeId(u64);

pub(crate) struct NormalizedResourceScope {
    id: NormalizedResourceScopeId,
    state: Mutex<ResourceState>,
}

struct ResourceState {
    next_slot: u64,
    entries: BTreeMap<u64, ResourceEntry>,
}

enum ResourceEntry {
    ByteStream(Arc<StreamLease>),
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
        Self {
            id,
            state: Mutex::new(ResourceState {
                next_slot: 1,
                entries: BTreeMap::new(),
            }),
        }
    }

    pub(crate) fn register_byte_stream(
        &self,
        authority: RequirementReference,
        lease: StreamLease,
    ) -> Result<NormalizedResourceHandle, ExecutionError> {
        let mut state = lock_unpoisoned(&self.state);
        if state.entries.len() >= MAXIMUM_TASK_RESOURCES {
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
        })
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
    use crate::platform::semantic_id::RequirementId;
    use crate::platform::stream::{StreamLimits, StreamRegistry};

    fn authority(ordinal: u64) -> RequirementReference {
        RequirementReference {
            package: PackageId::migrate(b"normalized-resource-test", 0),
            requirement: RequirementId::migrate(b"normalized-resource-test", ordinal),
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
}
