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

#[must_use = "enter the prepared invocation or explicitly drop it before VM fallback"]
pub struct PreparedInvocation<'a> {
    installed: &'a InstalledImage,
    entry: &'a lkjscript_native::EntryMetadata,
    offset: usize,
    arguments: Vec<MachineArgument>,
    state: PreparedInvocationState<'a>,
}

enum PreparedInvocationState<'a> {
    InvocationRegion(NativeCallState<'a>),
    CollectorFree(IslandCallState<'a>),
}

struct PreparedEntry<'a> {
    entry: &'a lkjscript_native::EntryMetadata,
    offset: usize,
    arguments: Vec<MachineArgument>,
    deadline_ms: i64,
    native_stack_bounds: Option<platform::NativeStackBounds>,
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

    pub fn prepare_region_invocation<'a>(
        &'a self,
        entry: FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
        services: &'a mut dyn NativeRuntimeServices,
    ) -> Result<PreparedInvocation<'a>, PreEntryError> {
        if self.image.execution_domain() != NativeExecutionDomain::InvocationRegion {
            return Err(PreEntryError::ExecutionDomain);
        }
        let prepared = self.prepare_entry(entry, arguments, config)?;
        let state = NativeCallState::new(
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
            state: PreparedInvocationState::InvocationRegion(state),
        })
    }

    fn prepare_entry<'a>(
        &'a self,
        entry: FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
    ) -> Result<PreparedEntry<'a>, PreEntryError> {
        let entry = self
            .image
            .entries()
            .iter()
            .find(|candidate| candidate.function() == entry)
            .ok_or(PreEntryError::UnknownEntry)?;
        validate_arguments(entry.signature(), arguments)?;
        let deadline_ms = prepare_deadline(config.wall_time)?;
        let arguments = prepare_machine_arguments(entry.signature(), arguments)?;
        let offset = usize::try_from(entry.offset())
            .map_err(|_| PreEntryError::EntryAddressRepresentation)?;
        self.mapping.validate_entry(offset)?;
        validate_pre_entry_policy(&self.image, entry, config)?;
        let native_stack_bounds = platform::native_stack_bounds();
        if let Some(required_bytes) = config.native_stack_requirement {
            let bounds = native_stack_bounds.ok_or(PreEntryError::NativeStackUnavailable(
                NativeStackError::ThreadExtentUnavailable,
            ))?;
            platform::native_stack_requirement_fits(required_bytes, bounds)
                .map_err(PreEntryError::NativeStackUnavailable)?;
        }
        Ok(PreparedEntry {
            entry,
            offset,
            arguments,
            deadline_ms,
            native_stack_bounds,
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

impl PreparedInvocation<'_> {
    pub fn enter(self) -> Result<InvocationReport, EnteredInvocationError> {
        let Self {
            installed,
            entry,
            offset,
            arguments,
            state,
        } = self;
        match state {
            PreparedInvocationState::InvocationRegion(mut state) => {
                let raw =
                    installed
                        .mapping
                        .enter(offset, entry.signature(), &arguments, &mut state);
                finish_invocation_region(raw, entry.signature(), state)
            }
            PreparedInvocationState::CollectorFree(mut state) => {
                let raw = installed.mapping.enter_island(
                    offset,
                    entry.signature(),
                    &arguments,
                    &mut state,
                );
                island::finish_collector_free(raw, entry.signature(), state)
            }
        }
    }
}

fn validate_pre_entry_policy(
    image: &InstallableImage,
    entry: &lkjscript_native::EntryMetadata,
    config: &NativeInvocationConfig,
) -> Result<(), PreEntryError> {
    if config.cancellation_requested {
        return Err(PreEntryError::Cancelled);
    }
    if config.poll_fuel == Some(0) {
        return Err(PreEntryError::ResourceLimitExceeded(
            NativeResourceLimitKind::PollFuel,
        ));
    }
    if config.max_active_frames == Some(0) {
        return Err(PreEntryError::ResourceLimitExceeded(
            NativeResourceLimitKind::ActiveFrames,
        ));
    }
    if let Some(maximum) = config.max_active_values {
        let entry_values = image
            .frames()
            .iter()
            .find(|frame| frame.function() == entry.function())
            .map(|frame| frame.homes().len())
            .unwrap_or(0);
        if entry_values > maximum {
            return Err(PreEntryError::ResourceLimitExceeded(
                NativeResourceLimitKind::ActiveValues,
            ));
        }
    }
    Ok(())
}

fn prepare_deadline(wall_time: Option<Duration>) -> Result<i64, PreEntryError> {
    let Some(duration) = wall_time else {
        return Ok(-1);
    };
    let delta = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
    if delta == 0 {
        return Err(PreEntryError::DeadlineExceeded);
    }
    Ok(crate::now_ms_monotonic()
        .checked_add(delta)
        .unwrap_or(i64::MAX))
}

fn validate_prepared_deadline(deadline_ms: i64) -> Result<(), PreEntryError> {
    if deadline_ms >= 0 && crate::now_ms_monotonic() >= deadline_ms {
        return Err(PreEntryError::DeadlineExceeded);
    }
    Ok(())
}

fn finish_invocation_region(
    raw: RawReturn,
    signature: &Signature,
    mut state: NativeCallState<'_>,
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
    let trap_site = trap_site(&state);
    let outcome = entered_outcome(state.status, state.trap, state.payload, raw, signature)?;
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

pub(super) struct EnteredStateSummary {
    pub(super) native_stack_error: Option<NativeStackError>,
    pub(super) invalid_entry_accounting: Option<u64>,
    pub(super) bookkeeping_allocation_failed: bool,
    pub(super) metadata_invalid: bool,
    pub(super) active_frames: usize,
    pub(super) pending_reservation: bool,
    pub(super) reserved_native_stack_bytes: usize,
    pub(super) active_value_homes: usize,
}

pub(super) fn validate_entered_state(
    state: EnteredStateSummary,
) -> Result<(), EnteredInvocationError> {
    if let Some(error) = state.native_stack_error {
        return Err(EnteredInvocationError::NativeStackViolation(error));
    }
    if let Some(source) = state.invalid_entry_accounting {
        return Err(EnteredInvocationError::InvalidNativeEntryAccounting(source));
    }
    if state.bookkeeping_allocation_failed {
        return Err(EnteredInvocationError::BookkeepingAllocationFailed);
    }
    if state.metadata_invalid {
        return Err(EnteredInvocationError::InvalidActiveFrame);
    }
    if state.active_frames != 0 {
        return Err(EnteredInvocationError::LeakedActiveFrames(
            state.active_frames,
        ));
    }
    if state.pending_reservation
        || state.reserved_native_stack_bytes != 0
        || state.active_value_homes != 0
    {
        return Err(EnteredInvocationError::InvalidActiveFrame);
    }
    Ok(())
}

pub(super) fn trap_site(state: &NativeCallState<'_>) -> Option<u64> {
    (state.status == 1 && state.trap == TrapCode::Explicit.as_u32() && state.trap_site_present == 1)
        .then(|| u64::from_ne_bytes(state.payload.to_ne_bytes()))
}

pub(super) fn entered_outcome(
    status: u32,
    trap: u32,
    payload: i64,
    raw: RawReturn,
    signature: &Signature,
) -> Result<InvocationOutcome, EnteredInvocationError> {
    match status {
        0 => Ok(InvocationOutcome::Returned(
            raw.into_value(signature.result())?,
        )),
        1 => Ok(InvocationOutcome::Trapped(match trap {
            1 => TrapCode::I64Overflow,
            2 => TrapCode::DivisionByZero,
            3 => TrapCode::Explicit,
            other => return Err(EnteredInvocationError::InvalidNativeTrap(other)),
        })),
        2 => Ok(InvocationOutcome::Exited(payload)),
        3 => Ok(InvocationOutcome::DeadlineExceeded),
        4 => Ok(InvocationOutcome::ResourceLimitExceeded(match payload {
            1 => NativeResourceLimitKind::PollFuel,
            2 => NativeResourceLimitKind::ActiveFrames,
            4 => NativeResourceLimitKind::RuntimeService,
            6 => NativeResourceLimitKind::ActiveValues,
            _ => return Err(EnteredInvocationError::InvalidNativeStatus(status)),
        })),
        5 => Ok(InvocationOutcome::HostFailure),
        other => Err(EnteredInvocationError::InvalidNativeStatus(other)),
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
