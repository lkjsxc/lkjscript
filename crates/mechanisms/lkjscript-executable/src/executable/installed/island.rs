use super::*;

impl InstalledImage {
    pub fn invoke_island_with_services(
        &self,
        entry: FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
        services: &mut dyn NativeIslandRuntimeServices,
    ) -> Result<InvocationReport, InvocationError> {
        if self.image.execution_domain() != NativeExecutionDomain::CollectorFree {
            return Err(InvocationError::ExecutionDomain);
        }
        let entry = self
            .image
            .entries()
            .iter()
            .find(|candidate| candidate.function() == entry)
            .ok_or(InvocationError::UnknownEntry)?;
        validate_arguments(entry.signature(), arguments)?;
        let mut state = IslandCallState::new(&self.image, config, services);
        let raw = self.mapping.invoke_island(
            entry.offset() as usize,
            entry.signature(),
            arguments,
            &mut state,
        )?;
        if state.metadata_invalid {
            return Err(InvocationError::InvalidActiveFrame);
        }
        if state.active_depth != 0 {
            return Err(InvocationError::LeakedActiveFrames(state.active_depth));
        }
        if state.pending_reservation.is_some()
            || state.reserved_native_stack_bytes != 0
            || state.active_value_homes != 0
        {
            return Err(InvocationError::InvalidActiveFrame);
        }
        let trap_site = (state.status == 1 && state.trap == TrapCode::Explicit.as_u32())
            .then(|| u32::try_from(state.payload).ok())
            .flatten();
        let outcome = match state.status {
            0 => InvocationOutcome::Returned(raw.into_value(entry.signature().result())?),
            1 => InvocationOutcome::Trapped(match state.trap {
                1 => TrapCode::I64Overflow,
                2 => TrapCode::DivisionByZero,
                3 => TrapCode::Explicit,
                other => return Err(InvocationError::InvalidNativeTrap(other)),
            }),
            2 => InvocationOutcome::Exited(state.payload),
            3 => InvocationOutcome::DeadlineExceeded,
            4 => InvocationOutcome::ResourceLimitExceeded(match state.payload {
                1 => NativeResourceLimitKind::PollFuel,
                2 => NativeResourceLimitKind::ActiveFrames,
                4 => NativeResourceLimitKind::RuntimeService,
                5 => NativeResourceLimitKind::NativeStackBytes,
                6 => NativeResourceLimitKind::ActiveValues,
                _ => return Err(InvocationError::InvalidNativeStatus(state.status)),
            }),
            5 => InvocationOutcome::HostFailure,
            other => return Err(InvocationError::InvalidNativeStatus(other)),
        };
        let native_entries = state
            .native_entries
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, count)| *count != 0)
            .filter_map(|(source, entries)| {
                u32::try_from(source)
                    .ok()
                    .map(|source_function| NativeEntryCount {
                        source_function,
                        entries,
                    })
            })
            .collect();
        Ok(InvocationReport {
            outcome,
            trap_site,
            poll_count: state.poll_count,
            native_entries,
            peak_active_frame_depth: state.peak_active_depth,
            active_frame_depth: 0,
            collection_calls: 0,
            maximum_roots: 0,
            exact_root_counts: Vec::new(),
            peak_native_stack_bytes: state.peak_native_stack_bytes,
            reserved_native_stack_bytes: 0,
            heap_operation_attempts: 0,
            heap_operation_successes: 0,
            barrier_count: 0,
            peak_active_value_homes: state.peak_active_value_homes,
            active_value_homes: 0,
            resource_calls: state.resource_calls,
            unique_calls: state.unique_calls,
            structural_calls: state.structural_calls,
            cleanup_failures: state.cleanup_failures,
            omitted_cleanup_failures: state.omitted_cleanup_failures,
            collector_runtime: false,
        })
    }
}
