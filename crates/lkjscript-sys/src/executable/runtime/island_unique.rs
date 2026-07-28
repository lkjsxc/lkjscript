use super::*;

pub(super) extern "C" fn runtime_island_byte_vector_new(
    state: *mut IslandCallState<'_>,
    size: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let result = state.services.new_byte_vector(size as i64);
    unique_result(state, result)
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
    unique_result(state, result)
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
    end_borrow(state, NativeLoan::byte_slice(loan));
}

pub(super) extern "C" fn runtime_island_byte_slice_mut_end(
    state: *mut IslandCallState<'_>,
    loan: u64,
) {
    end_borrow(state, NativeLoan::byte_slice_mut(loan));
}

pub(super) extern "C" fn runtime_island_byte_vector_drop(
    state: *mut IslandCallState<'_>,
    owner: u64,
) {
    let Some(state) = active_state(state) else {
        return;
    };
    let result = state
        .services
        .drop_byte_vector(NativeUnique::byte_vector(owner));
    unit_result(state, result);
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

fn end_borrow(state: *mut IslandCallState<'_>, loan: NativeLoan) {
    let Some(state) = active_state(state) else {
        return;
    };
    let result = state.services.end_byte_vector_borrow(loan);
    unit_result(state, result);
}

fn active_state<'a>(state: *mut IslandCallState<'a>) -> Option<&'a mut IslandCallState<'a>> {
    // SAFETY: collector-free relocations pass the live invocation-owned state.
    let state = unsafe { state.as_mut() }?;
    if state.status != 0 {
        return None;
    }
    state.unique_calls = state.unique_calls.saturating_add(1);
    Some(state)
}

fn unique_result(
    state: &mut IslandCallState<'_>,
    result: Result<NativeUnique, NativeServiceError>,
) -> u64 {
    match result {
        Ok(owner) if owner.unique_type() == UniqueType::ByteVector && owner.opaque_word() != 0 => {
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

fn scalar_result(state: &mut IslandCallState<'_>, result: Result<i64, NativeServiceError>) -> u64 {
    match result {
        Ok(value) => value as u64,
        Err(error) => {
            service_error(state, error);
            0
        }
    }
}

fn unit_result(state: &mut IslandCallState<'_>, result: Result<(), NativeServiceError>) {
    if let Err(error) = result {
        service_error(state, error);
    }
}

fn service_error(state: &mut IslandCallState<'_>, error: NativeServiceError) {
    match error {
        NativeServiceError::Trap => {
            state.status = 1;
            state.trap = TrapCode::Explicit.as_u32();
            state.payload = -1;
        }
        NativeServiceError::ResourceLimitExceeded => {
            state.status = 4;
            state.payload = 4;
        }
        NativeServiceError::HostFailure => state.status = 5,
    }
}
