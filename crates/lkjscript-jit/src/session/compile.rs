use crate::*;

impl JitSession {
    pub(crate) fn compile_group(&mut self, root: FunctionId) -> Result<u64, EngineError> {
        if self.links.is_some() && self.scalar_signature(root).is_none() {
            return Err(EngineError::new(
                FailureCode::UnsupportedType,
                Some(root),
                "automatic tiering conservatively keeps reference-typed functions in the VM",
            ));
        }
        let tier = self.program.tier();
        if tier == Tier::Optimizing {
            if let Some(record) = root.index().and_then(|index| self.functions.get_mut(index)) {
                record.state = TierState::OptimizingCompiling;
            }
        }
        let started = Instant::now();
        let lowered = match &self.program {
            ProgramAuthority::Baseline(program) => {
                lower::lower_baseline_group(program, root, self.config.backend_limits)?
            }
            ProgramAuthority::Optimizing(program) => {
                lower::lower_optimizing_group(program, root, self.config.backend_limits)?
            }
        };
        let lowering_and_encoding = started.elapsed();
        if self.optimization_time.saturating_add(lowering_and_encoding)
            > self.config.max_object_compile_time
            || self
                .total_compile_time
                .saturating_add(lowering_and_encoding)
                > self.config.max_total_compile_time
        {
            return Err(EngineError::new(
                FailureCode::CompileWallTime,
                Some(root),
                "native compilation wall-time budget exceeded",
            ));
        }

        let accounting = lowered.image.accounting();
        let contracts = lowered.image.contracts();
        let diagnostic_machine_code = if self.config.retain_machine_code_diagnostics {
            let bytes = u64::try_from(lowered.image.bytes().len()).map_err(|_| {
                EngineError::new(
                    FailureCode::InstallLimit,
                    Some(root),
                    "diagnostic machine-code byte count overflow",
                )
            })?;
            let next = self.diagnostic_bytes.checked_add(bytes).ok_or_else(|| {
                EngineError::new(
                    FailureCode::InstallLimit,
                    Some(root),
                    "diagnostic machine-code byte count overflow",
                )
            })?;
            if next > self.config.max_diagnostic_bytes {
                return Err(EngineError::new(
                    FailureCode::InstallLimit,
                    Some(root),
                    "diagnostic machine-code byte budget exceeded",
                ));
            }
            self.diagnostic_bytes = next;
            Some(lowered.image.bytes().to_vec())
        } else {
            None
        };
        let entries = lowered.image.entries().to_vec();
        let relocations = lowered.image.relocations().to_vec();
        let runtime_calls = lowered.image.runtime_calls().to_vec();
        let numeric_conversion_sites = numeric_conversion_sites(&lowered.image);
        let frames = lowered.image.frames().to_vec();
        let safepoints = lowered.image.safepoints().to_vec();
        let source_map = lowered.image.source_map().to_vec();
        let trap_map = lowered.image.trap_map().to_vec();
        let outcome_map = lowered.image.outcome_map().to_vec();
        let install_started = Instant::now();
        let installed = self
            .installer
            .install(lowered.image)
            .map_err(|error| install_error(root, error))?;
        let installation = install_started.elapsed();
        let accounted_allocation_bytes = installed.accounted_allocation_bytes();
        let total = lowering_and_encoding.saturating_add(installation);
        if self.optimization_time.saturating_add(total) > self.config.max_object_compile_time
            || self.total_compile_time.saturating_add(total) > self.config.max_total_compile_time
        {
            return Err(EngineError::new(
                FailureCode::CompileWallTime,
                Some(root),
                "native compile/install wall-time budget exceeded",
            ));
        }
        self.total_compile_time = self.total_compile_time.saturating_add(total);
        let identity = self.next_object;
        self.next_object = self.next_object.saturating_add(1);
        let (optimization_certificate, optimization_stats) = match &self.program {
            ProgramAuthority::Baseline(_) => (None, None),
            ProgramAuthority::Optimizing(program) => {
                (Some(program.certificate().clone()), Some(*program.stats()))
            }
        };
        let object = CodeObject {
            identity,
            functions: lowered.functions.clone(),
            tier,
            contracts,
            entries,
            accounting,
            accounted_allocation_bytes,
            relocations,
            runtime_calls,
            numeric_conversion_sites,
            frames,
            safepoints,
            source_map,
            trap_map,
            outcome_map,
            compile_stats: CompileStats {
                optimization: self.optimization_time,
                lowering_and_encoding,
                installation,
                work_units: accounting
                    .work_units()
                    .saturating_add(optimization_stats.map_or(0, |stats| stats.work_units)),
            },
            optimization_certificate,
            optimization_stats,
            invalidated: false,
            explicit_traps: lowered.explicit_traps,
            diagnostic_machine_code,
            native_entry_count: 0,
            installed,
        };
        self.objects.push(object);
        for function in lowered.functions {
            if let Some(index) = function.index() {
                if let Some(record) = self.functions.get_mut(index) {
                    record.code_object = Some(identity);
                    record.epoch = self.config.epoch;
                    if self.links.is_none() || record.auto_entry_eligible {
                        record.attempts = record.attempts.max(1);
                        record.state = match tier {
                            Tier::Baseline => TierState::BaselineNative,
                            Tier::Optimizing => TierState::OptimizedNative,
                        };
                        record.last_failure = None;
                    }
                }
            }
        }
        // This deterministic mapping assertion catches a backend metadata
        // mismatch before any generated entry can be selected.
        for (source, native) in lowered.native_functions {
            if !self.objects.last().is_some_and(|object| {
                object.entries.iter().any(|entry| {
                    entry.source_function().get() == source.raw() && entry.function() == native
                })
            }) {
                return Err(EngineError::new(
                    FailureCode::BackendVerification,
                    Some(source),
                    "encoded entry metadata does not match deterministic function mapping",
                ));
            }
        }
        Ok(identity)
    }
}

fn numeric_conversion_sites(
    image: &lkjscript_native::InstallableImage,
) -> NumericConversionSiteCounts {
    let mut counts = NumericConversionSiteCounts::default();
    for site in image.heap_runtime_sites() {
        match site.descriptor().operation() {
            HeapOperation::F64FromI64Exact { .. } => counts.f64_from_i64_exact += 1,
            HeapOperation::I64FromF64Exact { .. } => counts.i64_from_f64_exact += 1,
            HeapOperation::I64FromF64Trunc { .. } => counts.i64_from_f64_trunc += 1,
            _ => {}
        }
    }
    counts
}
