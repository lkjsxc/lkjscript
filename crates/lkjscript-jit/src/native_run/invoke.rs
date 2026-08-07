use crate::attempt::{BaselineRegionAttempt, BaselineScalarAttempt};
use crate::*;
mod cleanup;
use cleanup::native_cleanup_failures;

impl NativeRun {
    pub(crate) fn invoke_scalar(
        &mut self,
        function: FunctionId,
        arguments: &[NativeValue],
        execution: &ExecutionPolicy,
    ) -> Result<ScalarInvocation, EngineError> {
        let (native, execution_domain, native_stack_requirement) = {
            let object = self.object.as_ref().ok_or_else(|| {
                EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "native run has no installed group",
                )
            })?;
            let native = object
                .entries
                .iter()
                .find(|entry| entry.source_function().get() == function.raw())
                .map(EntryMetadata::function)
                .ok_or_else(|| {
                    EngineError::new(
                        FailureCode::InvocationFailure,
                        Some(function),
                        "installed group has no source-function entry",
                    )
                })?;
            let requirement = object
                .entry_stack_requirements
                .iter()
                .find_map(|(candidate, bytes)| (*candidate == function).then_some(*bytes));
            (native, object.installed.execution_domain(), requirement)
        };
        self.reset_invocation_state();
        let mut config = invocation_config(execution);
        if let Some(required_bytes) = native_stack_requirement {
            config = config.with_native_stack_requirement(required_bytes);
        }
        let invocation_started = self.config.collect_metrics.then(Instant::now);
        let report = match execution_domain {
            lkjscript_native::NativeExecutionDomain::CollectorFree => {
                self.invoke_collector_free(function, native, arguments, &config, execution)
            }
            lkjscript_native::NativeExecutionDomain::InvocationRegion => {
                self.invoke_invocation_region(function, native, arguments, &config, execution)
            }
        };
        let entry_begun = report
            .as_ref()
            .map_or_else(|error| !is_pre_entry_failure(error.code()), |_| true);
        if entry_begun {
            self.record_native_timing(invocation_started, None);
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
        let report = report?;
        Ok(self.complete_scalar_invocation(function, report, execution))
    }

    pub(crate) fn invoke_baseline_scalar_attempt(
        &mut self,
        function: FunctionId,
        arguments: &[NativeValue],
        execution: &ExecutionPolicy,
    ) -> BaselineScalarAttempt {
        let preparation_started = Instant::now();
        let prepared_entry = self.object.as_ref().and_then(|object| {
            let native = object
                .entries
                .iter()
                .find(|entry| entry.source_function().get() == function.raw())
                .map(EntryMetadata::function)?;
            let stack_requirement = object
                .entry_stack_requirements
                .iter()
                .find_map(|(candidate, bytes)| (*candidate == function).then_some(*bytes))?;
            Some((
                native,
                object.installed.execution_domain(),
                stack_requirement,
            ))
        });
        let Some((native, execution_domain, native_stack_requirement)) = prepared_entry else {
            return BaselineScalarAttempt::PreparationFailure {
                error: EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "installed baseline group lacks an entry or pre-entry stack requirement",
                ),
                preparation: preparation_started.elapsed(),
            };
        };
        self.reset_invocation_state();
        let config =
            invocation_config(execution).with_native_stack_requirement(native_stack_requirement);
        let attempt = match execution_domain {
            lkjscript_native::NativeExecutionDomain::CollectorFree => self
                .invoke_baseline_collector_free_attempt(
                    function, native, arguments, &config, execution,
                ),
            lkjscript_native::NativeExecutionDomain::InvocationRegion => {
                self.invoke_baseline_region_attempt(function, native, arguments, &config, execution)
            }
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
                self.record_native_timing(None, Some(native_execution));
                match *result {
                    Ok(report) => BaselineScalarAttempt::Executed {
                        invocation: self.complete_scalar_invocation(function, report, execution),
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
        &mut self,
        function: FunctionId,
        native: lkjscript_native::FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
        execution: &ExecutionPolicy,
    ) -> BaselineRegionAttempt {
        let preparation_started = Instant::now();
        let Some(scope) = lkjscript_core::ScopeId::new(self.next_resource_scope) else {
            return BaselineRegionAttempt::PreparationFailure {
                error: EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "native resource scope exhausted",
                ),
                preparation: preparation_started.elapsed(),
            };
        };
        let Some(next_scope) = self.next_resource_scope.checked_add(1) else {
            return BaselineRegionAttempt::PreparationFailure {
                error: EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "native resource scope exhausted",
                ),
                preparation: preparation_started.elapsed(),
            };
        };
        self.next_resource_scope = next_scope;
        let witnesses = match NativeWitnessCatalog::build(self.program.program()) {
            Ok(witnesses) => witnesses,
            Err(error) => {
                return BaselineRegionAttempt::PreparationFailure {
                    error,
                    preparation: preparation_started.elapsed(),
                };
            }
        };
        let mut services = match JitIslandServices::with_witnesses(scope, execution, witnesses) {
            Ok(services) => services,
            Err(error) => {
                return BaselineRegionAttempt::PreparationFailure {
                    error,
                    preparation: preparation_started.elapsed(),
                };
            }
        };
        let Some(object) = self.object.as_ref() else {
            return BaselineRegionAttempt::PreparationFailure {
                error: EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "native run lost its installed group",
                ),
                preparation: preparation_started.elapsed(),
            };
        };
        let prepared =
            object
                .installed
                .prepare_invocation(native, arguments, config, &mut services);
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
        let native_execution = native_started.elapsed();
        let unique_export = result
            .as_ref()
            .ok()
            .and_then(|report| match report.outcome() {
                InvocationOutcome::Returned(NativeValue::Unique(owner)) => {
                    Some(services.export_unique(owner))
                }
                InvocationOutcome::Returned(NativeValue::StaticBytes(identity)) => Some(
                    object
                        .installed
                        .resolve_static_bytes(identity)
                        .map(<[u8]>::to_vec)
                        .ok_or(NativeServiceError::Trap),
                ),
                _ => None,
            });
        let structural_export = result
            .as_ref()
            .ok()
            .and_then(|report| match report.outcome() {
                InvocationOutcome::Returned(NativeValue::StructuralOwner(owner)) => {
                    Some(services.export_structural(owner))
                }
                _ => None,
            });
        let (resources, unique, structural, last_resource, last_trap, empty, lists) =
            services.finish();
        self.lists = Some(lists);
        self.native_resources.add(resources);
        self.native_unique.add(unique);
        self.native_structural.add(structural);
        self.last_runtime_resource = last_resource;
        self.last_runtime_trap = last_trap;
        if !empty {
            self.last_runtime_failure = Some(NativeServiceError::HostFailure);
        }
        match unique_export {
            Some(Ok(bytes)) => self.returned_unique = Some(bytes),
            Some(Err(error)) => self.last_runtime_failure = Some(error),
            None => {}
        }
        match structural_export {
            Some(Ok(value)) => {
                self.returned_structural = Some(ReturnedStructuralValue(value));
            }
            Some(Err(error)) => self.last_runtime_failure = Some(error),
            None => {}
        }
        BaselineRegionAttempt::Entered {
            result: Box::new(result),
            preparation,
            native_execution,
        }
    }

    fn reset_invocation_state(&mut self) {
        self.last_runtime_trap = None;
        self.last_runtime_resource = None;
        self.last_runtime_failure = None;
        self.returned_unique = None;
        self.returned_structural = None;
    }

    fn record_native_timing(
        &mut self,
        invocation_started: Option<Instant>,
        measured_execution: Option<Duration>,
    ) {
        if self.time_to_first_native_entry.is_none() {
            self.time_to_first_native_entry = self.metrics_started.map(|started| started.elapsed());
        }
        if !self.config.collect_metrics {
            return;
        }
        let elapsed = measured_execution
            .or_else(|| invocation_started.map(|started| started.elapsed()))
            .unwrap_or(Duration::ZERO);
        self.native_invocations = self.native_invocations.saturating_add(1);
        self.native_execution = self.native_execution.saturating_add(elapsed);
        if self.first_native_call.is_none() {
            self.first_native_call = Some(elapsed);
        }
    }

    fn complete_scalar_invocation(
        &mut self,
        function: FunctionId,
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
        }
        if invocation_entries == 0 {
            invocation_entries = 1;
            self.native_entries = self.native_entries.saturating_add(1);
        }
        if let Some(object) = self.object.as_mut() {
            object.native_entry_count =
                object.native_entry_count.saturating_add(invocation_entries);
        }
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

fn invocation_config(execution: &ExecutionPolicy) -> NativeInvocationConfig {
    match execution.limited_policy() {
        Some(policy) => NativeInvocationConfig::limited(policy.instruction_fuel, policy.wall_time)
            .with_max_active_frames(policy.max_frames)
            .with_max_active_values(policy.max_stack_values)
            .with_max_cleanup_failures(policy.cleanup_retention.max_failures()),
        None => NativeInvocationConfig::unrestricted(),
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
