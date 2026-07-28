use super::*;

pub(in crate::executable::runtime) extern "C" fn runtime_island_byte_slice_read_u32_little_endian(
    state: *mut IslandCallState<'_>,
    loan: u64,
    index: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let result = state
        .services
        .byte_slice_read_u32_little_endian(NativeLoan::byte_slice(loan), index as i64);
    scalar_result(state, result)
}

pub(in crate::executable::runtime) extern "C" fn runtime_island_byte_slice_mut_write_u32_little_endian(
    state: *mut IslandCallState<'_>,
    loan: u64,
    index: u64,
    word: u64,
) {
    let Some(state) = active_state(state) else {
        return;
    };
    let result = state.services.byte_slice_mut_write_u32_little_endian(
        NativeLoan::byte_slice_mut(loan),
        index as i64,
        word as i64,
    );
    unit_result(state, result);
}
