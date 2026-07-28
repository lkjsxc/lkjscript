mod execution;

use super::*;

impl<'a, J: RuntimeTier> Vm<'a, J> {
    pub fn new(
        chunk: &'a ValidatedChunk,
        jit: J,
        inputs: ExecutionInputs,
        config: ExecutionConfig,
    ) -> Self {
        Self {
            chunk,
            globals: vec![Value::INVALID; chunk.global_names().len()],
            stack: Vec::new(),
            frames: Vec::new(),
            arena: Arena::new(GcConfig {
                max_allocations: config.max_allocations,
                max_heap_bytes: config.max_heap_bytes,
                ..GcConfig::default()
            }),
            jit,
            exit_code: None,
            inputs,
            resources: ResourceTable::new(config.max_handles, config.cleanup_failure_limits),
            unique: unique::UniqueRuntime::new(&config),
            fuel_remaining: config.instruction_fuel,
            output_bytes: 0,
            allocation_error: None,
            cleanup_failures: CleanupFailures::new(config.cleanup_failure_limits),
            logical_aggregate_constructions: 0,
            started: Instant::now(),
            config,
        }
    }

    pub fn run(mut self) -> ExecutionOutcome {
        self.run_inner()
    }

    pub(super) fn run_inner(&mut self) -> ExecutionOutcome {
        let stopped = self.run_loop();
        let failed = stopped.is_err();
        let outcome = match stopped {
            Ok(Stop::Returned(value))
                if matches!(
                    self.chunk.main().return_unique,
                    Some(
                        lkjscript_core::UniqueValueKind::Bytes
                            | lkjscript_core::UniqueValueKind::ByteVector
                    )
                ) =>
            {
                let transferred = if let Some(index) = value.as_static_bytes() {
                    self.chunk
                        .constants()
                        .get(usize::from(index))
                        .and_then(|constant| match constant {
                            lkjscript_core::Constant::StaticBytes(bytes) => {
                                lkjscript_core::OwnedValue::from_unique_bytes(bytes.to_vec()).ok()
                            }
                            _ => None,
                        })
                        .ok_or_else(|| Error::msg("invalid returned static bytes constant"))
                } else {
                    self.unique.export_owner(value)
                };
                match transferred {
                    Ok(value) => ExecutionOutcome::Returned(value),
                    Err(error) => ExecutionOutcome::Trapped(Trap::new(format!(
                        "invalid returned VM unique bytes owner: {error}"
                    ))),
                }
            }
            Ok(Stop::Returned(value)) => {
                let arena = std::mem::take(&mut self.arena);
                match arena.into_owned(value) {
                    Ok(value) => ExecutionOutcome::Returned(value),
                    Err(error) => ExecutionOutcome::Trapped(Trap::new(format!(
                        "invalid returned VM value: {error}"
                    ))),
                }
            }
            Ok(Stop::Exited(code)) => ExecutionOutcome::Exited(code),
            Err(error) => outcome_from_error(error),
        };

        let unique_cleanup = self.unique.verify_empty().inspect_err(|_| {
            let _cleanup = self.unique.cleanup();
        });
        let mut cleanup_failures = std::mem::replace(
            &mut self.cleanup_failures,
            CleanupFailures::new(self.config.cleanup_failure_limits),
        );
        if let Err(error) = unique_cleanup {
            cleanup_failures.push(
                if failed {
                    CleanupPhase::Emergency
                } else {
                    CleanupPhase::RuntimeTeardown
                },
                CleanupSubject::UniqueStorage,
                format!("unique byte cleanup {error}"),
            );
        }

        let resource_teardown = self.resources.teardown();
        let restore_error = self
            .inputs
            .capabilities
            .contains(&lkjscript_core::CapabilityKind::Terminal)
            .then(crate::host_term::restore_tty)
            .and_then(Result::err);
        let flush_error = self
            .inputs
            .capabilities
            .contains(&lkjscript_core::CapabilityKind::Stdio)
            .then(crate::host::flush_out)
            .and_then(Result::err);
        cleanup_failures.append(resource_teardown.cleanup_failures().clone());
        if let Some(error) = restore_error {
            cleanup_failures.push(
                CleanupPhase::RuntimeTeardown,
                CleanupSubject::Terminal,
                error.to_string(),
            );
        }
        if let Some(error) = flush_error {
            cleanup_failures.push(
                CleanupPhase::RuntimeTeardown,
                CleanupSubject::StandardOutput,
                format!("stdout cleanup {error}"),
            );
        }
        outcome.with_cleanup_failures(cleanup_failures)
    }
}
