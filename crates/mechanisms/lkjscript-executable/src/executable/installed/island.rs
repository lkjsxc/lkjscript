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
        let mut state = IslandCallState::new(&self.image, &self.entry_mapping, config, services)?;
        let raw = self.mapping.invoke_island(
            entry.offset() as usize,
            entry.signature(),
            arguments,
            &mut state,
        )?;
        if let Some(source) = state.invalid_entry_accounting {
            return Err(InvocationError::InvalidNativeEntryAccounting(source));
        }
        if state.bookkeeping_allocation_failed {
            return Err(InvocationError::NativeBookkeepingAllocationFailed);
        }
        if state.metadata_invalid {
            return Err(InvocationError::InvalidActiveFrame);
        }
        if !state.active_frames.is_empty() {
            return Err(InvocationError::LeakedActiveFrames(
                state.active_frames.len(),
            ));
        }
        if state.pending_reservation.is_some()
            || state.reserved_native_stack_bytes != 0
            || state.active_value_homes != 0
        {
            return Err(InvocationError::InvalidActiveFrame);
        }
        let trap_site = (state.status == 1
            && state.trap == TrapCode::Explicit.as_u32()
            && state.trap_site_present == 1)
            .then(|| u64::from_ne_bytes(state.payload.to_ne_bytes()));
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
        state.native_entries.retain(|count| count.entries != 0);
        Ok(InvocationReport {
            outcome,
            trap_site,
            poll_count: state.poll_count,
            native_entries: state.native_entries,
            peak_active_frame_depth: state.peak_active_depth,
            active_frame_depth: 0,
            peak_native_stack_bytes: state.peak_native_stack_bytes,
            reserved_native_stack_bytes: 0,
            heap_operation_attempts: state.heap_operation_attempts,
            heap_operation_successes: state.heap_operation_successes,
            peak_active_value_homes: state.peak_active_value_homes,
            active_value_homes: 0,
            resource_calls: state.resource_calls,
            unique_calls: state.unique_calls,
            structural_calls: state.structural_calls,
            cleanup_failures: state.cleanup_failures,
            omitted_cleanup_failures: state.omitted_cleanup_failures,
        })
    }
}
