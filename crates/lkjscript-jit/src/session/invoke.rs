use crate::*;
mod cleanup;
use cleanup::native_cleanup_failures;

impl JitSession {
    pub fn invoke_scalar(
        &mut self,
        function: FunctionId,
        arguments: &[NativeValue],
        execution: &ExecutionConfig,
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
        self.last_runtime_trap = None;
        self.last_runtime_resource = None;
        self.returned_unique = None;
        let config = NativeInvocationConfig::new(execution.instruction_fuel, execution.wall_time)
            .with_max_active_frames(execution.max_frames)
            .with_max_active_values(execution.max_stack_values)
            .with_max_cleanup_failures(execution.cleanup_failure_limits.max_failures());
        if self.time_to_first_native_entry.is_none() {
            self.time_to_first_native_entry = self.metrics_started.map(|started| started.elapsed());
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
        if let Some(started) = invocation_started {
            let elapsed = started.elapsed();
            self.native_invocations = self.native_invocations.saturating_add(1);
            self.native_execution = self.native_execution.saturating_add(elapsed);
            if self.first_native_call.is_none() {
                self.first_native_call = Some(elapsed);
            }
        }
        let report = match report {
            Ok(report) => report,
            Err(error)
                if self.links.is_some() && error.code() == FailureCode::NativeBookkeeping =>
            {
                let object = self.objects[object_index].identity;
                self.objects[object_index].invalidated = true;
                for record in &mut self.functions {
                    if record.code_object == Some(object) {
                        record.code_object = None;
                        record.state = TierState::Disabled;
                        record.last_failure = Some(FailureCode::NativeBookkeeping);
                    }
                }
                self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
                return Err(error);
            }
            Err(error) => return Err(error),
        };
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
        let outcome = match report.outcome() {
            InvocationOutcome::Returned(value) => ScalarInvocationOutcome::Returned(value),
            InvocationOutcome::Trapped(trap) => {
                ScalarInvocationOutcome::Trapped(trap, report.trap_site())
            }
            InvocationOutcome::Exited(code) => ScalarInvocationOutcome::Exited(code),
            InvocationOutcome::DeadlineExceeded => ScalarInvocationOutcome::DeadlineExceeded,
            InvocationOutcome::ResourceLimitExceeded(kind) => {
                let kind = match kind {
                    NativeResourceLimitKind::PollFuel => ResourceLimitKind::InstructionFuel,
                    NativeResourceLimitKind::ActiveFrames
                    | NativeResourceLimitKind::NativeStackBytes => ResourceLimitKind::FrameDepth,
                    NativeResourceLimitKind::ActiveValues => ResourceLimitKind::StackValues,
                    NativeResourceLimitKind::RuntimeService => self
                        .last_runtime_resource
                        .unwrap_or(ResourceLimitKind::Allocations),
                };
                ScalarInvocationOutcome::ResourceLimitExceeded(kind)
            }
            InvocationOutcome::HostFailure => ScalarInvocationOutcome::HostFailure,
        };
        Ok(ScalarInvocation {
            outcome,
            poll_count: report.poll_count(),
            cleanup_failures,
        })
    }
}
