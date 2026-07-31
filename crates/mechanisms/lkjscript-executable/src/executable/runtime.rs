use super::*;

mod island;
mod island_bytes;
mod island_bytes_static;
mod island_results;
mod island_structural;
mod island_unique;
mod symbols;
use island::*;
use island_bytes::*;
use island_bytes_static::*;
use island_results::*;
use island_structural::*;
use island_unique::*;
pub(super) use symbols::runtime_symbol;

extern "C" fn runtime_identity_i64(_state: *mut NativeCallState<'_>, value: u64) -> u64 {
    value
}

extern "C" fn runtime_poll(state: *mut NativeCallState<'_>) {
    // SAFETY: generated runtime calls receive the live invocation state as the
    // implicit first argument. The mapping and call cannot outlive this stack
    // value, and generated code cannot replace the context pointer.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return;
    };
    state.poll();
}

extern "C" fn runtime_enter_function(state: *mut NativeCallState<'_>, function: u64) {
    // SAFETY: the context provenance is identical to runtime_poll.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return;
    };
    if state.status != 0 {
        return;
    }
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

extern "C" fn runtime_reserve_frame(
    state: *mut NativeCallState<'_>,
    function_ordinal: u64,
    frame_bytes: u64,
    rbp: *mut u8,
) -> *mut NativeCallState<'_> {
    // SAFETY: generated canonical minimal prologues pass their invocation context
    // and current frame base before touching generated-frame storage.
    let Some(invocation) = (unsafe { state.as_mut() }) else {
        return state;
    };
    let Ok(function_ordinal) = u32::try_from(function_ordinal) else {
        invocation.invalidate_active_frame();
        return state;
    };
    invocation.reserve_frame(function_ordinal, frame_bytes, rbp);
    state
}

extern "C" fn runtime_register_frame(
    state: *mut NativeCallState<'_>,
    function_ordinal: u64,
    rbp: *mut u8,
) {
    // SAFETY: generated canonical prologues pass their invocation context and
    // current frame base. Both remain live until the matching epilogue.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return;
    };
    let Ok(function_ordinal) = u32::try_from(function_ordinal) else {
        state.invalidate_active_frame();
        return;
    };
    state.register_frame(function_ordinal, rbp);
}

extern "C" fn runtime_publish_safepoint(state: *mut NativeCallState<'_>, safepoint: u64) {
    // SAFETY: context provenance is identical to runtime_register_frame.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return;
    };
    let Ok(safepoint) = u32::try_from(safepoint) else {
        state.invalidate_active_frame();
        return;
    };
    state.publish_safepoint(safepoint);
}

extern "C" fn runtime_unregister_frame(
    state: *mut NativeCallState<'_>,
    function_ordinal: u64,
    rbp: *mut u8,
) {
    // SAFETY: context/frame provenance is identical to registration.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return;
    };
    let Ok(function_ordinal) = u32::try_from(function_ordinal) else {
        state.invalidate_active_frame();
        return;
    };
    state.unregister_frame(function_ordinal, rbp);
}

extern "C" fn runtime_heap_dispatch(state: *mut NativeCallState<'_>, site: u64) {
    // SAFETY: generated heap sites publish and pass their retained dense site
    // identity. Raw frame homes are validated and accessed only in this module.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return;
    };
    let Ok(site) = u32::try_from(site) else {
        state.invalidate_active_frame();
        return;
    };
    state.dispatch_heap_operation(site);
}

extern "C" fn runtime_collect_reference(state: *mut NativeCallState<'_>, reference: u64) -> u64 {
    // SAFETY: generated collecting calls publish an exact safepoint before
    // entering this trampoline; all raw frame access remains in this module.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return reference;
    };
    state.collect_references(reference)
}
