#![allow(unsafe_code)]

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
    state.register_frame(function_ordinal, rbp);
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
    state.unregister_frame(function_ordinal, rbp);
}

extern "C" fn runtime_value_dispatch(state: *mut NativeCallState<'_>, site: u64) {
    // SAFETY: generated runtime-value sites pass their retained dense site
    // identity. Raw frame homes are validated and accessed only in this module.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return;
    };
    state.dispatch_runtime_value_operation(site);
}
