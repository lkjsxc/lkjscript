use super::*;

pub(super) fn scalar_result(
    state: &mut IslandCallState<'_>,
    result: Result<i64, NativeServiceError>,
) -> u64 {
    match result {
        Ok(value) => value as u64,
        Err(error) => {
            service_error(state, error);
            0
        }
    }
}

pub(super) fn unit_result(state: &mut IslandCallState<'_>, result: Result<(), NativeServiceError>) {
    if let Err(error) = result {
        service_error(state, error);
    }
}

pub(super) fn service_error(state: &mut IslandCallState<'_>, error: NativeServiceError) {
    match error {
        NativeServiceError::Trap => {
            state.status = 1;
            state.trap = TrapCode::Explicit.as_u32();
            state.payload = 0;
            state.trap_site_present = 0;
        }
        NativeServiceError::ResourceLimitExceeded => {
            state.status = 4;
            state.payload = 4;
        }
        NativeServiceError::HostFailure => state.status = 5,
    }
}
