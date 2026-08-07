use super::*;

impl InstalledImage {
    pub fn prepare_invocation<'a>(
        &'a self,
        entry: FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
        services: &'a mut dyn NativeIslandRuntimeServices,
    ) -> Result<PreparedInvocation<'a>, PreEntryError> {
        if self.image.execution_domain() != NativeExecutionDomain::CollectorFree {
            return Err(PreEntryError::ExecutionDomain);
        }
        let prepared = self.prepare_entry(entry, arguments, config)?;
        let state = IslandCallState::new(
            &self.image,
            &self.entry_mapping,
            config,
            prepared.deadline_ms,
            prepared.native_stack_bounds,
            services,
        )?;
        validate_prepared_deadline(prepared.deadline_ms)?;
        Ok(PreparedInvocation {
            installed: self,
            entry: prepared.entry,
            offset: prepared.offset,
            arguments: prepared.arguments,
            state: PreparedInvocationState::CollectorFree(state),
        })
    }
}

pub(super) fn finish_collector_free(
    raw: RawReturn,
    signature: &Signature,
    mut state: IslandCallState<'_>,
) -> Result<InvocationReport, EnteredInvocationError> {
    debug_assert!(state.entry_started, "entered result without native entry");
    validate_entered_state(EnteredStateSummary {
        native_stack_error: state.native_stack_error,
        invalid_entry_accounting: state.invalid_entry_accounting,
        bookkeeping_allocation_failed: state.bookkeeping_allocation_failed,
        metadata_invalid: state.metadata_invalid,
        active_frames: state.active_frames.len(),
        pending_reservation: state.pending_reservation.is_some(),
        reserved_native_stack_bytes: state.reserved_native_stack_bytes,
        active_value_homes: state.active_value_homes,
    })?;
    let trap_site = (state.status == 1
        && state.trap == TrapCode::Explicit.as_u32()
        && state.trap_site_present == 1)
        .then(|| u64::from_ne_bytes(state.payload.to_ne_bytes()));
    let outcome = entered_outcome(state.status, state.trap, state.payload, raw, signature)?;
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
