mod cleanup;
use super::*;
use cleanup::end_borrow;
pub(super) use cleanup::{cleanup_result, cleanup_state};

mod word;
pub(super) use word::*;

pub(super) extern "C" fn runtime_island_byte_vector_new(
    state: *mut IslandCallState<'_>,
    size: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let result = state.services.new_byte_vector(size as i64);
    unique_result(state, result, UniqueType::ByteVector)
}

pub(super) extern "C" fn runtime_island_byte_vector_move(
    state: *mut IslandCallState<'_>,
    owner: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let result = state
        .services
        .move_byte_vector(NativeUnique::byte_vector(owner));
    unique_result(state, result, UniqueType::ByteVector)
}

pub(super) extern "C" fn runtime_island_borrow_shared(
    state: *mut IslandCallState<'_>,
    owner: u64,
) -> u64 {
    borrow(state, owner, LoanType::ByteSlice)
}

pub(super) extern "C" fn runtime_island_borrow_exclusive(
    state: *mut IslandCallState<'_>,
    owner: u64,
) -> u64 {
    borrow(state, owner, LoanType::ByteSliceMut)
}

pub(super) extern "C" fn runtime_island_byte_slice_length(
    state: *mut IslandCallState<'_>,
    loan: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let result = state
        .services
        .byte_slice_length(NativeLoan::byte_slice(loan));
    scalar_result(state, result)
}

pub(super) extern "C" fn runtime_island_byte_slice_byte_at(
    state: *mut IslandCallState<'_>,
    loan: u64,
    index: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let result = state
        .services
        .byte_slice_byte_at(NativeLoan::byte_slice(loan), index as i64);
    scalar_result(state, result)
}

pub(super) extern "C" fn runtime_island_byte_slice_mut_set_byte(
    state: *mut IslandCallState<'_>,
    loan: u64,
    index: u64,
    byte: u64,
) {
    let Some(state) = active_state(state) else {
        return;
    };
    let result = state.services.byte_slice_mut_set_byte(
        NativeLoan::byte_slice_mut(loan),
        index as i64,
        byte as i64,
    );
    unit_result(state, result);
}

pub(super) extern "C" fn runtime_island_byte_slice_end(state: *mut IslandCallState<'_>, loan: u64) {
    end_borrow(
        state,
        NativeLoan::byte_slice(loan),
        RuntimeCallSlot::ByteSliceEnd,
    );
}

pub(super) extern "C" fn runtime_island_byte_slice_mut_end(
    state: *mut IslandCallState<'_>,
    loan: u64,
) {
    end_borrow(
        state,
        NativeLoan::byte_slice_mut(loan),
        RuntimeCallSlot::ByteSliceMutEnd,
    );
}

pub(super) extern "C" fn runtime_island_byte_vector_drop(
    state: *mut IslandCallState<'_>,
    owner: u64,
) {
    let Some((state, primary)) = cleanup_state(state) else {
        return;
    };
    let result = state
        .services
        .drop_byte_vector(NativeUnique::byte_vector(owner));
    cleanup_result(state, primary, RuntimeCallSlot::ByteVectorDrop, result);
}

fn borrow(state: *mut IslandCallState<'_>, owner: u64, kind: LoanType) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let result = state
        .services
        .borrow_byte_vector(NativeUnique::byte_vector(owner), kind);
    match result {
        Ok(loan) if loan.loan_type() == kind && loan.opaque_word() != 0 => loan.opaque_word(),
        Ok(_) => {
            state.status = 5;
            0
        }
        Err(error) => {
            service_error(state, error);
            0
        }
    }
}

pub(super) fn active_state<'a>(
    state: *mut IslandCallState<'a>,
) -> Option<&'a mut IslandCallState<'a>> {
    // SAFETY: collector-free relocations pass the live invocation-owned state.
    let state = unsafe { state.as_mut() }?;
    if state.status != 0 {
        return None;
    }
    state.unique_calls = state.unique_calls.saturating_add(1);
    Some(state)
}

pub(super) fn unique_result(
    state: &mut IslandCallState<'_>,
    result: Result<NativeUnique, NativeServiceError>,
    expected: UniqueType,
) -> u64 {
    match result {
        Ok(owner) if owner.unique_type() == expected && owner.opaque_word() != 0 => {
            owner.opaque_word()
        }
        Ok(_) => {
            state.status = 5;
            0
        }
        Err(error) => {
            service_error(state, error);
            0
        }
    }
}
