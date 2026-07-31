use super::*;

impl<'a, J: RuntimeTier> Vm<'a, J> {
    pub fn run(mut self) -> ExecutionOutcome {
        self.run_inner()
    }

    pub(in crate::run) fn run_inner(&mut self) -> ExecutionOutcome {
        let stopped = self.run_loop();
        let failed = stopped.is_err();
        let mut outcome = match stopped {
            Ok(Stop::Returned(value)) if self.chunk.main().return_structural.is_some() => {
                let representation = self
                    .chunk
                    .main()
                    .return_structural
                    .ok_or_else(|| Error::msg("structural return metadata disappeared"));
                let transferred = representation.and_then(|representation| {
                    structural_ops::export_return(self, value, representation)
                });
                match transferred {
                    Ok(value) => ExecutionOutcome::Returned(value),
                    Err(error) => ExecutionOutcome::Trapped(Trap::new(format!(
                        "invalid returned VM structural owner: {error}"
                    ))),
                }
            }
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
            Ok(Stop::Returned(_)) if self.chunk.main().return_region_product.is_some() => {
                ExecutionOutcome::Trapped(Trap::new(
                    "invocation-region product cannot cross the process boundary",
                ))
            }
            Ok(Stop::Returned(value))
                if value.as_static_string().is_some() || value.as_structural_root().is_some() =>
            {
                match structural_ops::export_plain_return(self, value) {
                    Ok(Some(value)) => ExecutionOutcome::Returned(value),
                    Ok(None) => ExecutionOutcome::Trapped(Trap::new(
                        "structural owner escaped without exact return metadata",
                    )),
                    Err(error) => ExecutionOutcome::Trapped(Trap::new(format!(
                        "invalid returned VM structural leaf: {error}"
                    ))),
                }
            }
            Ok(Stop::Returned(value)) => match self.snapshot_list_aware_return(value) {
                Ok(value) => ExecutionOutcome::Returned(value),
                Err(error) => ExecutionOutcome::Trapped(Trap::new(format!(
                    "invalid returned VM value: {error}"
                ))),
            },
            Ok(Stop::Exited(code)) => ExecutionOutcome::Exited(code),
            Err(error) => outcome_from_error(error),
        };

        let structural_cleanup = structural_ops::teardown(self);
        let unique_cleanup = self.unique.verify_empty().inspect_err(|_| {
            let _cleanup = self.unique.cleanup();
        });
        let mut cleanup_failures = std::mem::replace(
            &mut self.cleanup_failures,
            CleanupFailures::new(self.config.cleanup_failure_limits),
        );
        if let Err(error) = structural_cleanup {
            cleanup_failures.push(
                if failed {
                    CleanupPhase::Emergency
                } else {
                    CleanupPhase::RuntimeTeardown
                },
                CleanupSubject::UniqueStorage,
                format!("structural value cleanup {error}"),
            );
            if !failed {
                outcome = ExecutionOutcome::Trapped(Trap::new(
                    "structural invocation retained live state after execution",
                ));
            }
        }
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
        let flush_error = if self
            .inputs
            .capabilities
            .contains(&lkjscript_core::CapabilityKind::Stdio)
        {
            match self.inputs.host.stdio.as_ref() {
                Some(provider) => crate::host::flush_out(provider.as_ref()).err(),
                None => Some(Error::host("stdio capability has no granted provider")),
            }
        } else {
            None
        };
        if resource_teardown.ordinary_obligations() > 0 {
            cleanup_failures.push(
                CleanupPhase::RuntimeTeardown,
                CleanupSubject::ResourceTable,
                format!(
                    "resource invocation retained {} ordinary ownership obligation(s)",
                    resource_teardown.ordinary_obligations()
                ),
            );
        }
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
