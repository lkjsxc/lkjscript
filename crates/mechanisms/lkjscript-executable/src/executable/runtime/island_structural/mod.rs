use super::*;

mod execute;
use execute::execute;

pub(super) fn dispatch_structural(
    state: &mut IslandCallState<'_>,
    site: u64,
    first: u64,
    second: u64,
    third: u64,
) -> u64 {
    let primary = state.status;
    let Some(site) = usize::try_from(site)
        .ok()
        .and_then(|index| state.image.structural_runtime_sites().get(index))
    else {
        state.status = 5;
        return 0;
    };
    let descriptor = site.descriptor().clone();
    if primary != 0
        && !matches!(
            descriptor.operation(),
            StructuralOperation::EndView(_)
                | StructuralOperation::Drop(_)
                | StructuralOperation::WitnessDisposeStatic(_)
                | StructuralOperation::DestinationAbort { .. }
        )
    {
        return 0;
    }
    state.structural_calls = state.structural_calls.saturating_add(1);
    if primary != 0 {
        state.status = 0;
        let result = execute(state, &descriptor, first, second, third);
        if let Err(error) = result {
            state.record_cleanup_failure(RuntimeCallSlot::StructuralDispatch, error);
        }
        state.status = primary;
        return 0;
    }
    let result = execute(state, &descriptor, first, second, third);
    structural_result(state, result, descriptor.signature().result())
}

fn structural_result(
    state: &mut IslandCallState<'_>,
    result: Result<NativeValue, NativeServiceError>,
    expected: ValueType,
) -> u64 {
    match result {
        Ok(value) if value.value_type() == expected => native_value_word(value, expected)
            .unwrap_or_else(|| {
                state.status = 5;
                0
            }),
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
