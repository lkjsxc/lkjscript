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
    let Ok(index) = usize::try_from(function) else {
        state.status = 5;
        return;
    };
    let Some(entries) = state.native_entries.get_mut(index) else {
        state.status = 5;
        return;
    };
    *entries = entries.saturating_add(1);
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

pub(super) extern "C" fn runtime_island_reserve(
    state: *mut IslandCallState<'_>,
    function: u64,
    bytes: u64,
    rbp: *mut u8,
) -> *mut IslandCallState<'_> {
    // SAFETY: canonical prologues pass their live island context and frame base.
    if let Some(invocation) = unsafe { state.as_mut() } {
        if let Ok(function) = u32::try_from(function) {
            invocation.reserve_frame(function, bytes, rbp);
        } else {
            invocation.invalidate_frame();
        }
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
        if let Ok(function) = u32::try_from(function) {
            state.register_frame(function, rbp);
        } else {
            state.invalidate_frame();
        }
    }
}

pub(super) extern "C" fn runtime_island_unregister(
    state: *mut IslandCallState<'_>,
    function: u64,
    rbp: *mut u8,
) {
    // SAFETY: context provenance is identical to runtime_island_reserve.
    if let Some(state) = unsafe { state.as_mut() } {
        if let Ok(function) = u32::try_from(function) {
            state.unregister_frame(function, rbp);
        } else {
            state.invalidate_frame();
        }
    }
}
