use super::*;

pub(super) extern "C" fn runtime_island_static_bytes_length(
    state: *mut IslandCallState<'_>,
    identity: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let Some(bytes) = resolve(state, identity) else {
        return 0;
    };
    match i64::try_from(bytes.len()) {
        Ok(len) => len as u64,
        Err(_) => {
            service_error(state, NativeServiceError::Trap);
            0
        }
    }
}

pub(super) extern "C" fn runtime_island_static_bytes_byte_at(
    state: *mut IslandCallState<'_>,
    identity: u64,
    index: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let Some(bytes) = resolve(state, identity) else {
        return 0;
    };
    let Some(byte) = usize::try_from(index as i64)
        .ok()
        .and_then(|index| bytes.get(index))
        .copied()
    else {
        service_error(state, NativeServiceError::Trap);
        return 0;
    };
    u64::from(byte)
}

pub(super) extern "C" fn runtime_island_static_bytes_clone(
    state: *mut IslandCallState<'_>,
    identity: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let Some(bytes) = resolve(state, identity) else {
        return 0;
    };
    let result = state.services.clone_static_bytes(bytes);
    unique_result(state, result, UniqueType::Bytes)
}

pub(super) extern "C" fn runtime_island_static_bytes_copy_slice(
    state: *mut IslandCallState<'_>,
    identity: u64,
    start: u64,
    len: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let Some(bytes) = resolve(state, identity) else {
        return 0;
    };
    let result = state
        .services
        .copy_static_bytes_slice(bytes, start as i64, len as i64);
    unique_result(state, result, UniqueType::Bytes)
}

pub(super) extern "C" fn runtime_island_static_bytes_thaw(
    state: *mut IslandCallState<'_>,
    identity: u64,
) -> u64 {
    let Some(state) = active_state(state) else {
        return 0;
    };
    let Some(bytes) = resolve(state, identity) else {
        return 0;
    };
    let result = state.services.thaw_static_bytes(bytes);
    unique_result(state, result, UniqueType::ByteVector)
}

fn resolve<'a>(state: &mut IslandCallState<'a>, identity: u64) -> Option<&'a [u8]> {
    let bytes = state
        .image
        .resolve_static_bytes(NativeStaticBytes::new(identity));
    if bytes.is_none() {
        service_error(state, NativeServiceError::Trap);
    }
    bytes
}
