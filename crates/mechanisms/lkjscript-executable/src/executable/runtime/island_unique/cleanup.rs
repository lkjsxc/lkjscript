#![allow(unsafe_code)]

use super::*;

pub(super) fn end_borrow(state: *mut IslandCallState<'_>, loan: NativeLoan, slot: RuntimeCallSlot) {
    let Some((state, primary)) = cleanup_state(state) else {
        return;
    };
    let result = state.services.end_byte_vector_borrow(loan);
    cleanup_result(state, primary, slot, result);
}

pub(in crate::executable::runtime) fn cleanup_state<'a>(
    state: *mut IslandCallState<'a>,
) -> Option<(&'a mut IslandCallState<'a>, u32)> {
    // SAFETY: collector-free relocations pass the live invocation-owned state.
    let state = unsafe { state.as_mut() }?;
    let primary = state.status;
    state.unique_calls = state.unique_calls.saturating_add(1);
    Some((state, primary))
}

pub(in crate::executable::runtime) fn cleanup_result(
    state: &mut IslandCallState<'_>,
    primary: u32,
    slot: RuntimeCallSlot,
    result: Result<(), NativeServiceError>,
) {
    if primary == 0 {
        unit_result(state, result);
    } else {
        if let Err(error) = result {
            if state
                .maximum_cleanup_failures
                .is_none_or(|maximum| state.cleanup_failures.len() < maximum)
            {
                state
                    .cleanup_failures
                    .push(NativeCleanupFailure::new(slot, error));
            } else {
                state.omitted_cleanup_failures = state.omitted_cleanup_failures.saturating_add(1);
            }
        }
        state.status = primary;
    }
}
