#![allow(unsafe_code)]

use super::*;

pub(super) extern "C" fn runtime_island_poll(state: *mut IslandCallState<'_>) {
    // SAFETY: a collector-free image is relocated only to island trampolines.
    if let Some(state) = unsafe { state.as_mut() } {
        state.poll();
    }
}

pub(super) extern "C" fn runtime_island_enter(state: *mut IslandCallState<'_>, function: u64) {
    // SAFETY: context provenance is identical to runtime_island_poll.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return;
    };
    if state.status != 0 {
        return;
    }
    let Some(ordinal) = state.entry_mapping.ordinal_for_source(function) else {
        state.invalid_entry_accounting = Some(function);
        state.status = 5;
        return;
    };
    if !record_native_entry(&mut state.native_entries, ordinal) {
        state.invalid_entry_accounting = Some(function);
        state.status = 5;
    }
}

pub(super) extern "C" fn runtime_island_stdin(
    state: *mut IslandCallState<'_>,
    capability: u64,
) -> u64 {
    // SAFETY: the generated call uses the contract-bound island state.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return 0;
    };
    if state.status != 0 || capability != capability_word(CapabilityKind::Stdio) {
        state.status = 5;
        return 0;
    }
    state.resource_calls = state.resource_calls.saturating_add(1);
    match state.services.borrow_standard_input() {
        Ok(resource)
            if resource.resource_kind() == ResourceKind::InputStream
                && resource.opaque_word() != 0 =>
        {
            resource.opaque_word()
        }
        Ok(_) | Err(NativeServiceError::HostFailure) => {
            state.status = 5;
            0
        }
        Err(NativeServiceError::Trap) => {
            state.status = 1;
            state.trap = TrapCode::Explicit.as_u32();
            0
        }
        Err(NativeServiceError::ResourceLimitExceeded) => {
            state.status = 4;
            state.payload = 4;
            0
        }
    }
}

pub(super) extern "C" fn runtime_island_value_dispatch(state: *mut IslandCallState<'_>, site: u64) {
    // SAFETY: collector-free relocation binds this call to its live island state.
    if let Some(state) = unsafe { state.as_mut() } {
        state.dispatch_runtime_value_operation(site);
    }
}

pub(super) extern "C" fn runtime_island_structural_dispatch(
    state: *mut IslandCallState<'_>,
    site: u64,
    first: u64,
    second: u64,
    third: u64,
) -> u64 {
    // SAFETY: collector-free relocation binds this call to its live island state.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return 0;
    };
    dispatch_structural(state, site, first, second, third)
}

pub(super) extern "C" fn runtime_island_reserve(
    state: *mut IslandCallState<'_>,
    function: u64,
    bytes: u64,
    rbp: *mut u8,
) -> *mut IslandCallState<'_> {
    // SAFETY: canonical prologues pass their live island context and frame base.
    if let Some(invocation) = unsafe { state.as_mut() } {
        invocation.reserve_frame(function, bytes, rbp);
        invocation.entry_rejected = invocation.status != 0;
    }
    state
}

pub(super) extern "C" fn runtime_island_take_rejected_entry(
    state: *mut IslandCallState<'_>,
) -> u64 {
    // SAFETY: generated call-failure paths pass their live island context.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return 0;
    };
    let rejected = state.entry_rejected;
    state.entry_rejected = false;
    u64::from(rejected)
}

pub(super) extern "C" fn runtime_island_register(
    state: *mut IslandCallState<'_>,
    function: u64,
    rbp: *mut u8,
) {
    // SAFETY: context provenance is identical to runtime_island_reserve.
    if let Some(state) = unsafe { state.as_mut() } {
        state.register_frame(function, rbp);
    }
}

pub(super) extern "C" fn runtime_island_unregister(
    state: *mut IslandCallState<'_>,
    function: u64,
    rbp: *mut u8,
) {
    // SAFETY: context provenance is identical to runtime_island_reserve.
    if let Some(state) = unsafe { state.as_mut() } {
        state.unregister_frame(function, rbp);
    }
}
