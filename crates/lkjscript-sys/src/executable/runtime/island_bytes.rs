use super::*;

pub(super) extern "C" fn runtime_island_bytes_move(
    state: *mut IslandCallState<'_>,
    owner: u64,
) -> u64 {
    bytes_owner_result(state, |services| {
        services.move_bytes(NativeUnique::bytes(owner))
    })
}

pub(super) extern "C" fn runtime_island_bytes_borrow(
    state: *mut IslandCallState<'_>,
    owner: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    match state.services.borrow_bytes(NativeUnique::bytes(owner)) {
        Ok(loan) if loan.loan_type() == LoanType::Bytes && loan.opaque_word() != 0 => {
            loan.opaque_word()
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

pub(super) extern "C" fn runtime_island_bytes_length(
    state: *mut IslandCallState<'_>,
    loan: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let result = state.services.bytes_length(NativeLoan::bytes(loan));
    scalar_result(state, result)
}

pub(super) extern "C" fn runtime_island_bytes_byte_at(
    state: *mut IslandCallState<'_>,
    loan: u64,
    index: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let result = state
        .services
        .bytes_byte_at(NativeLoan::bytes(loan), index as i64);
    scalar_result(state, result)
}

pub(super) extern "C" fn runtime_island_bytes_clone(
    state: *mut IslandCallState<'_>,
    loan: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let result = state.services.clone_bytes(NativeLoan::bytes(loan));
    unique_result(state, result, UniqueType::Bytes)
}

pub(super) extern "C" fn runtime_island_bytes_copy_slice(
    state: *mut IslandCallState<'_>,
    loan: u64,
    start: u64,
    len: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let result = state
        .services
        .copy_bytes_slice(NativeLoan::bytes(loan), start as i64, len as i64);
    unique_result(state, result, UniqueType::Bytes)
}

pub(super) extern "C" fn runtime_island_bytes_end(state: *mut IslandCallState<'_>, loan: u64) {
    let Some(state) = active_state(state) else {
        return;
    };
    let result = state.services.end_bytes_borrow(NativeLoan::bytes(loan));
    unit_result(state, result);
}

pub(super) extern "C" fn runtime_island_bytes_drop(state: *mut IslandCallState<'_>, owner: u64) {
    let Some(state) = active_state(state) else {
        return;
    };
    let result = state.services.drop_bytes(NativeUnique::bytes(owner));
    unit_result(state, result);
}

pub(super) extern "C" fn runtime_island_freeze_byte_vector(
    state: *mut IslandCallState<'_>,
    owner: u64,
) -> u64 {
    bytes_owner_result(state, |services| {
        services.freeze_byte_vector(NativeUnique::byte_vector(owner))
    })
}

pub(super) extern "C" fn runtime_island_thaw_bytes(
    state: *mut IslandCallState<'_>,
    owner: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let result = state.services.thaw_bytes(NativeUnique::bytes(owner));
    unique_result(state, result, UniqueType::ByteVector)
}

fn bytes_owner_result(
    state: *mut IslandCallState<'_>,
    operation: impl FnOnce(
        &mut dyn NativeIslandRuntimeServices,
    ) -> Result<NativeUnique, NativeServiceError>,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let result = operation(state.services);
    unique_result(state, result, UniqueType::Bytes)
}
