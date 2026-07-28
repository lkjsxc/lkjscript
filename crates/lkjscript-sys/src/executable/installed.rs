use super::*;

mod island;

#[derive(Debug)]
pub struct InstalledImage {
    pub(super) installer: Rc<InstallerState>,
    pub(super) image: InstallableImage,
    pub(super) mapping: platform::Mapping,
    pub(super) usage: ExecutableUsage,
    pub(super) not_send_or_sync: PhantomData<Rc<()>>,
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

    pub fn invoke(
        &self,
        entry: FunctionId,
        arguments: &[NativeValue],
    ) -> Result<InvocationOutcome, InvocationError> {
        self.invoke_with_config(entry, arguments, &NativeInvocationConfig::default())
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
            NativeExecutionDomain::LegacyHeap => {
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
        if self.image.execution_domain() != NativeExecutionDomain::LegacyHeap {
            return Err(InvocationError::ExecutionDomain);
        }
        let entry = self
            .image
            .entries()
            .iter()
            .find(|candidate| candidate.function() == entry)
            .ok_or(InvocationError::UnknownEntry)?;
        validate_arguments(entry.signature(), arguments)?;
        let mut state = NativeCallState::new(&self.image, config, services)?;
        let raw = self.mapping.invoke(
            entry.offset() as usize,
            entry.signature(),
            arguments,
            &mut state,
        )?;
        if state.metadata_invalid {
            return Err(InvocationError::InvalidActiveFrame);
        }
        if state.active_depth != 0 {
            let leaked = state.active_depth;
            state.active_depth = 0;
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
                3 => NativeResourceLimitKind::MaterializedRoots,
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
            .filter(|(_, entries)| *entries != 0)
            .filter_map(|(source_function, entries)| {
                u32::try_from(source_function)
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
            active_frame_depth: state.active_depth,
            collection_calls: state.collection_calls,
            maximum_roots: state.maximum_roots,
            exact_root_counts: state.exact_root_counts.clone(),
            peak_native_stack_bytes: state.peak_native_stack_bytes,
            reserved_native_stack_bytes: state.reserved_native_stack_bytes,
            heap_operation_attempts: state.heap_operation_attempts,
            heap_operation_successes: state.heap_operation_successes,
            barrier_count: state.barrier_count,
            peak_active_value_homes: state.peak_active_value_homes,
            active_value_homes: state.active_value_homes,
            resource_calls: 0,
            unique_calls: 0,
            collector_runtime: true,
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
        let current = self.installer.usage.get();
        self.installer.usage.set(ExecutableUsage {
            code_bytes: current.code_bytes.saturating_sub(self.usage.code_bytes),
            metadata_bytes: current
                .metadata_bytes
                .saturating_sub(self.usage.metadata_bytes),
            work_units: current.work_units.saturating_sub(self.usage.work_units),
            objects: current.objects.saturating_sub(1),
        });
    }
}
