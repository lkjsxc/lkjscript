use crate::*;
mod cleanup;
use cleanup::native_cleanup_failures;

impl JitSession {
    pub fn invoke_scalar(
        &mut self,
        function: FunctionId,
        arguments: &[NativeValue],
        execution: &ExecutionPolicy,
    ) -> Result<ScalarInvocation, EngineError> {
        let index = function.index().ok_or_else(|| {
            EngineError::new(
                FailureCode::InvocationFailure,
                Some(function),
                "function ID cannot index tier state",
            )
        })?;
        let expected_state = match self.program.tier() {
            Tier::Baseline => TierState::BaselineNative,
            Tier::Optimizing => TierState::OptimizedNative,
        };
        let object_id = self
            .functions
            .get(index)
            .filter(|record| record.state == expected_state)
            .and_then(|record| record.code_object)
            .ok_or_else(|| {
                EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "function has no installed code object for the selected tier",
                )
            })?;
        let object_index = self
            .objects
            .iter()
            .position(|object| object.identity == object_id && !object.invalidated)
            .ok_or_else(|| {
                EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "installed code object is unavailable",
                )
            })?;
        let native = self.objects[object_index]
            .entries
            .iter()
            .find(|entry| entry.source_function().get() == function.raw())
            .map(EntryMetadata::function)
            .ok_or_else(|| {
                EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "code object has no source-function entry",
                )
            })?;
        let execution_domain = self.objects[object_index].installed.execution_domain();
        let native_stack_requirement = self
            .links
            .as_ref()
            .map(|_| {
                self.objects[object_index]
                    .automatic_stack_requirements
                    .iter()
                    .find_map(|(candidate, bytes)| (*candidate == function).then_some(*bytes))
                    .ok_or_else(|| {
                        EngineError::new(
                            FailureCode::NativeStackBoundary,
                            Some(function),
                            "automatic native stack requirement is unavailable",
                        )
                    })
            })
            .transpose()?;
        self.last_runtime_trap = None;
        self.last_runtime_resource = None;
        self.last_runtime_failure = None;
        self.returned_unique = None;
        let mut config = match execution.limited_policy() {
            Some(policy) => {
                NativeInvocationConfig::limited(policy.instruction_fuel, policy.wall_time)
                    .with_max_active_frames(policy.max_frames)
                    .with_max_active_values(policy.max_stack_values)
                    .with_max_cleanup_failures(policy.cleanup_retention.max_failures())
            }
            None => NativeInvocationConfig::unrestricted(),
        };
        if let Some(required_bytes) = native_stack_requirement {
            config = config.with_native_stack_requirement(required_bytes);
        }
        let invocation_started = self.config.collect_metrics.then(Instant::now);
        if self.links.is_some() {
            self.vm_to_native_transitions = self.vm_to_native_transitions.saturating_add(1);
        }
        let report = match execution_domain {
            lkjscript_native::NativeExecutionDomain::CollectorFree => self.invoke_collector_free(
                function,
                object_index,
                native,
                arguments,
                &config,
                execution,
            ),
            lkjscript_native::NativeExecutionDomain::InvocationRegion => self
                .invoke_invocation_region(
                    function,
                    object_index,
                    native,
                    arguments,
                    &config,
                    execution,
                ),
        };
        if self.links.is_some() {
            self.native_to_vm_transitions = self.native_to_vm_transitions.saturating_add(1);
        }
        let entry_begun = report
            .as_ref()
            .map_or_else(|error| !is_pre_entry_failure(error.code()), |_| true);
        if entry_begun {
            if self.time_to_first_native_entry.is_none() {
                self.time_to_first_native_entry =
                    self.metrics_started.map(|started| started.elapsed());
            }
            if let Some(started) = invocation_started {
                let elapsed = started.elapsed();
                self.native_invocations = self.native_invocations.saturating_add(1);
                self.native_execution = self.native_execution.saturating_add(elapsed);
                if self.first_native_call.is_none() {
                    self.first_native_call = Some(elapsed);
                }
            }
        }
        if let Err(error) = &report {
            let outcome = match error.code() {
                FailureCode::PreEntryCancelled => Some(ScalarInvocationOutcome::HostFailure),
                FailureCode::PreEntryDeadline => Some(ScalarInvocationOutcome::DeadlineExceeded),
                FailureCode::PreEntryPollFuel => {
                    Some(ScalarInvocationOutcome::ResourceLimitExceeded(
                        ResourceLimitKind::InstructionFuel,
                    ))
                }
                FailureCode::PreEntryActiveFrames => Some(
                    ScalarInvocationOutcome::ResourceLimitExceeded(ResourceLimitKind::FrameDepth),
                ),
                FailureCode::PreEntryActiveValues => Some(
                    ScalarInvocationOutcome::ResourceLimitExceeded(ResourceLimitKind::StackValues),
                ),
                FailureCode::PreEntryRuntimeService => Some(
                    ScalarInvocationOutcome::ResourceLimitExceeded(ResourceLimitKind::Allocations),
                ),
                _ => None,
            };
            if let Some(outcome) = outcome {
                return Ok(ScalarInvocation {
                    outcome,
                    poll_count: 0,
                    cleanup_failures: CleanupFailures::with_retention(
                        execution.cleanup_retention(),
                    ),
                });
            }
        }
        let report = match report {
            Ok(report) => report,
            Err(error)
                if self.links.is_some()
                    && matches!(
                        error.code(),
                        FailureCode::NativeBookkeeping | FailureCode::NativeStackBoundary
                    ) =>
            {
                let object = self.objects[object_index].identity;
                self.objects[object_index].invalidated = true;
                for record in &mut self.functions {
                    if record.code_object == Some(object) {
                        record.code_object = None;
                        record.state = TierState::Disabled;
                        record.last_failure = Some(error.code());
                    }
                }
                self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
                return Err(error);
            }
            Err(error) => return Err(error),
        };
        Ok(self.complete_scalar_invocation(function, index, object_index, report, execution))
    }

    pub(crate) fn invoke_baseline_scalar_attempt(
        &mut self,
        function: FunctionId,
        arguments: &[NativeValue],
        execution: &ExecutionPolicy,
    ) -> BaselineScalarAttempt {
        let preparation_started = Instant::now();
        let Some(index) = function.index() else {
            return BaselineScalarAttempt::PreparationFailure {
                error: EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "function ID cannot index baseline state",
                ),
                preparation: preparation_started.elapsed(),
            };
        };
        let Some(object_id) = self
            .functions
            .get(index)
            .filter(|record| record.state == TierState::BaselineNative)
            .and_then(|record| record.code_object)
        else {
            return BaselineScalarAttempt::PreparationFailure {
                error: EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "function has no installed baseline object",
                ),
                preparation: preparation_started.elapsed(),
            };
        };
        let Some(object_index) = self
            .objects
            .iter()
            .position(|object| object.identity == object_id && !object.invalidated)
        else {
            return BaselineScalarAttempt::PreparationFailure {
                error: EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "installed baseline object is unavailable",
                ),
                preparation: preparation_started.elapsed(),
            };
        };
        let Some(native) = self.objects[object_index]
            .entries
            .iter()
            .find(|entry| entry.source_function().get() == function.raw())
            .map(EntryMetadata::function)
        else {
            return BaselineScalarAttempt::PreparationFailure {
                error: EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "baseline object has no source-function entry",
                ),
                preparation: preparation_started.elapsed(),
            };
        };
        let execution_domain = self.objects[object_index].installed.execution_domain();
        let Some(native_stack_requirement) = self.objects[object_index]
            .automatic_stack_requirements
            .iter()
            .find_map(|(candidate, bytes)| (*candidate == function).then_some(*bytes))
        else {
            return BaselineScalarAttempt::PreparationFailure {
                error: EngineError::new(
                    FailureCode::NativeStackBoundary,
                    Some(function),
                    "baseline group stack requirement is unavailable",
                ),
                preparation: preparation_started.elapsed(),
            };
        };
        self.last_runtime_trap = None;
        self.last_runtime_resource = None;
        self.last_runtime_failure = None;
        self.returned_unique = None;
        let mut config = match execution.limited_policy() {
            Some(policy) => {
                NativeInvocationConfig::limited(policy.instruction_fuel, policy.wall_time)
                    .with_max_active_frames(policy.max_frames)
                    .with_max_active_values(policy.max_stack_values)
                    .with_max_cleanup_failures(policy.cleanup_retention.max_failures())
            }
            None => NativeInvocationConfig::unrestricted(),
        };
        config = config.with_native_stack_requirement(native_stack_requirement);
        let attempt = match execution_domain {
            lkjscript_native::NativeExecutionDomain::CollectorFree => self
                .invoke_baseline_collector_free_attempt(object_index, native, arguments, &config),
            lkjscript_native::NativeExecutionDomain::InvocationRegion => self
                .invoke_baseline_region_attempt(
                    function,
                    object_index,
                    native,
                    arguments,
                    &config,
                    execution,
                ),
        };
        match attempt {
            BaselineRegionAttempt::Declined { error, preparation } => {
                BaselineScalarAttempt::Declined { error, preparation }
            }
            BaselineRegionAttempt::PreparationFailure { error, preparation } => {
                BaselineScalarAttempt::PreparationFailure { error, preparation }
            }
            BaselineRegionAttempt::Entered {
                result,
                preparation,
                native_execution,
            } => {
                if self.time_to_first_native_entry.is_none() {
                    self.time_to_first_native_entry =
                        self.metrics_started.map(|started| started.elapsed());
                }
                if self.config.collect_metrics {
                    self.native_invocations = self.native_invocations.saturating_add(1);
                    self.native_execution = self.native_execution.saturating_add(native_execution);
                    if self.first_native_call.is_none() {
                        self.first_native_call = Some(native_execution);
                    }
                }
                match *result {
                    Ok(report) => BaselineScalarAttempt::Executed {
                        invocation: self.complete_scalar_invocation(
                            function,
                            index,
                            object_index,
                            report,
                            execution,
                        ),
                        preparation,
                        native_execution,
                    },
                    Err(error) => BaselineScalarAttempt::EnteredFailure {
                        error,
                        preparation,
                        native_execution,
                    },
                }
            }
        }
    }

    fn invoke_baseline_collector_free_attempt(
        &self,
        object_index: usize,
        native: lkjscript_native::FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
    ) -> BaselineRegionAttempt {
        let preparation_started = Instant::now();
        let mut services = NoopNativeIslandRuntimeServices;
        let prepared = self.objects[object_index].installed.prepare_invocation(
            native,
            arguments,
            config,
            &mut services,
        );
        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                return BaselineRegionAttempt::Declined {
                    error,
                    preparation: preparation_started.elapsed(),
                };
            }
        };
        let preparation = preparation_started.elapsed();
        let native_started = Instant::now();
        let result = prepared.enter();
        BaselineRegionAttempt::Entered {
            result: Box::new(result),
            preparation,
            native_execution: native_started.elapsed(),
        }
    }

    fn complete_scalar_invocation(
        &mut self,
        function: FunctionId,
        index: usize,
        object_index: usize,
        report: InvocationReport,
        execution: &ExecutionPolicy,
    ) -> ScalarInvocation {
        self.poll_calls = self.poll_calls.saturating_add(report.poll_count());
        self.resource_runtime_calls = self
            .resource_runtime_calls
            .saturating_add(report.resource_calls());
        self.unique_runtime_calls = self
            .unique_runtime_calls
            .saturating_add(report.unique_calls());
        self.structural_runtime_calls = self
            .structural_runtime_calls
            .saturating_add(report.structural_calls());
        self.runtime_heap_attempts = self
            .runtime_heap_attempts
            .saturating_add(report.heap_operation_attempts());
        self.runtime_heap_successes = self
            .runtime_heap_successes
            .saturating_add(report.heap_operation_successes());
        self.peak_native_frame_depth = self
            .peak_native_frame_depth
            .max(report.peak_active_frame_depth());
        self.peak_native_stack_bytes = self
            .peak_native_stack_bytes
            .max(report.peak_native_stack_bytes());
        let mut invocation_entries = 0_u64;
        for count in report.native_entries() {
            invocation_entries = invocation_entries.saturating_add(count.entries());
            self.native_entries = self.native_entries.saturating_add(count.entries());
            if count.source_function() != function.raw() {
                self.direct_native_calls = self.direct_native_calls.saturating_add(count.entries());
            }
            if let Ok(source_index) = usize::try_from(count.source_function()) {
                if let Some(record) = self.functions.get_mut(source_index) {
                    record.native_entries = record.native_entries.saturating_add(count.entries());
                }
            }
        }
        if invocation_entries == 0 {
            invocation_entries = 1;
            self.native_entries = self.native_entries.saturating_add(1);
            if let Some(record) = self.functions.get_mut(index) {
                record.native_entries = record.native_entries.saturating_add(1);
            }
        }
        self.objects[object_index].native_entry_count = self.objects[object_index]
            .native_entry_count
            .saturating_add(invocation_entries);
        let cleanup_failures = native_cleanup_failures(&report, execution);
        let outcome = match self.last_runtime_failure.take() {
            Some(NativeServiceError::ResourceLimitExceeded) => {
                ScalarInvocationOutcome::ResourceLimitExceeded(
                    self.last_runtime_resource
                        .unwrap_or(ResourceLimitKind::Allocations),
                )
            }
            Some(NativeServiceError::Trap | NativeServiceError::HostFailure) => {
                ScalarInvocationOutcome::HostFailure
            }
            None => match report.outcome() {
                InvocationOutcome::Returned(value) => ScalarInvocationOutcome::Returned(value),
                InvocationOutcome::Trapped(trap) => {
                    ScalarInvocationOutcome::Trapped(trap, report.trap_site())
                }
                InvocationOutcome::Exited(code) => ScalarInvocationOutcome::Exited(code),
                InvocationOutcome::DeadlineExceeded => ScalarInvocationOutcome::DeadlineExceeded,
                InvocationOutcome::ResourceLimitExceeded(kind) => {
                    let kind = match kind {
                        NativeResourceLimitKind::PollFuel => ResourceLimitKind::InstructionFuel,
                        NativeResourceLimitKind::ActiveFrames => ResourceLimitKind::FrameDepth,
                        NativeResourceLimitKind::ActiveValues => ResourceLimitKind::StackValues,
                        NativeResourceLimitKind::RuntimeService => self
                            .last_runtime_resource
                            .unwrap_or(ResourceLimitKind::Allocations),
                    };
                    ScalarInvocationOutcome::ResourceLimitExceeded(kind)
                }
                InvocationOutcome::HostFailure => ScalarInvocationOutcome::HostFailure,
            },
        };
        ScalarInvocation {
            outcome,
            poll_count: report.poll_count(),
            cleanup_failures,
        }
    }
}

const fn is_pre_entry_failure(code: FailureCode) -> bool {
    matches!(
        code,
        FailureCode::NativeBookkeeping
            | FailureCode::NativeStackBoundary
            | FailureCode::PreEntryCancelled
            | FailureCode::PreEntryDeadline
            | FailureCode::PreEntryPollFuel
            | FailureCode::PreEntryActiveFrames
            | FailureCode::PreEntryActiveValues
            | FailureCode::PreEntryRuntimeService
            | FailureCode::PreEntryFailure
    )
}
