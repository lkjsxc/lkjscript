use super::*;

mod island;

#[derive(Debug)]
pub struct InstalledImage {
    pub(super) installer: Arc<InstallerState>,
    pub(super) image: InstallableImage,
    pub(super) entry_mapping: NativeEntryMapping,
    pub(super) mapping: platform::Mapping,
    pub(super) usage: ExecutableUsage,
}

impl InstalledImage {
    #[must_use]
    pub const fn execution_domain(&self) -> NativeExecutionDomain {
        self.image.execution_domain()
    }

    #[must_use]
    pub fn entries(&self) -> &[lkjscript_native::EntryMetadata] {
        self.image.entries()
    }

    pub fn resolve_static_bytes(&self, identity: NativeStaticBytes) -> Option<&[u8]> {
        self.image.resolve_static_bytes(identity)
    }

    pub fn invoke(
        &self,
        entry: FunctionId,
        arguments: &[NativeValue],
    ) -> Result<InvocationOutcome, InvocationError> {
        self.invoke_with_config(entry, arguments, &NativeInvocationConfig::unrestricted())
            .map(|report| report.outcome)
    }

    pub fn invoke_with_config(
        &self,
        entry: FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
    ) -> Result<InvocationReport, InvocationError> {
        match self.image.execution_domain() {
            NativeExecutionDomain::CollectorFree => {
                let mut services = NoopNativeIslandRuntimeServices;
                self.invoke_island_with_services(entry, arguments, config, &mut services)
            }
            NativeExecutionDomain::InvocationRegion => {
                let mut services = NoopNativeRuntimeServices;
                self.invoke_with_services(entry, arguments, config, &mut services)
            }
        }
    }

    pub fn invoke_with_services(
        &self,
        entry: FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
        services: &mut dyn NativeRuntimeServices,
    ) -> Result<InvocationReport, InvocationError> {
        if self.image.execution_domain() != NativeExecutionDomain::InvocationRegion {
            return Err(InvocationError::ExecutionDomain);
        }
        let entry = self
            .image
            .entries()
            .iter()
            .find(|candidate| candidate.function() == entry)
            .ok_or(InvocationError::UnknownEntry)?;
        validate_arguments(entry.signature(), arguments)?;
        let mut state = NativeCallState::new(&self.image, &self.entry_mapping, config, services)?;
        let raw = self.mapping.invoke(
            entry.offset() as usize,
            entry.signature(),
            arguments,
            &mut state,
        )?;
        if let Some(boundary) = state.native_stack_boundary {
            return Err(InvocationError::NativeStackBoundary {
                boundary,
                retry_safe: state.peak_active_depth == 0,
            });
        }
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
            let leaked = state.active_frames.len();
            state.active_frames.clear();
            state.pending_reservation = None;
            state.reserved_native_stack_bytes = 0;
            state.active_value_homes = 0;
            return Err(InvocationError::LeakedActiveFrames(leaked));
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
            active_frame_depth: state.active_frames.len(),
            peak_native_stack_bytes: state.peak_native_stack_bytes,
            reserved_native_stack_bytes: state.reserved_native_stack_bytes,
            heap_operation_attempts: state.heap_operation_attempts,
            heap_operation_successes: state.heap_operation_successes,
            peak_active_value_homes: state.peak_active_value_homes,
            active_value_homes: state.active_value_homes,
            resource_calls: 0,
            unique_calls: 0,
            structural_calls: 0,
            cleanup_failures: Vec::new(),
            omitted_cleanup_failures: 0,
        })
    }

    pub fn permissions(&self) -> Result<MappingPermissions, PermissionProbeError> {
        self.mapping.permissions()
    }

    #[must_use]
    pub fn accounted_allocation_bytes(&self) -> u64 {
        u64::try_from(self.mapping.allocation_length()).unwrap_or(u64::MAX)
    }

    #[must_use]
    pub fn wx_transition_verified(&self) -> bool {
        self.mapping.wx_transition_verified()
    }
}

impl Drop for InstalledImage {
    fn drop(&mut self) {
        let mut current = match self.installer.usage.lock() {
            Ok(usage) => usage,
            Err(poisoned) => poisoned.into_inner(),
        };
        *current = ExecutableUsage {
            code_bytes: current.code_bytes.saturating_sub(self.usage.code_bytes),
            metadata_bytes: current
                .metadata_bytes
                .saturating_sub(self.usage.metadata_bytes),
            work_units: current.work_units.saturating_sub(self.usage.work_units),
            objects: current.objects.saturating_sub(1),
        };
    }
}
