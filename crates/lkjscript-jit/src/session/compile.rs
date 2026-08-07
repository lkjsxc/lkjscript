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
        let automatic_stack_requirements = if self.links.is_some() {
            automatic_stack_requirements(
                self.program.program(),
                &lowered.functions,
                &lowered.image,
                root,
            )?
        } else {
            Vec::new()
        };
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
            automatic_stack_requirements,
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

fn automatic_stack_requirements(
    program: &lkjscript_ir::Program,
    functions: &[FunctionId],
    image: &lkjscript_native::InstallableImage,
    root: FunctionId,
) -> Result<Vec<(FunctionId, usize)>, EngineError> {
    let mut included = vec![false; program.functions.len()];
    let mut frame_bytes: Vec<Option<usize>> = vec![None; program.functions.len()];
    for function in functions {
        let Some(index) = function.index() else {
            return Err(EngineError::new(
                FailureCode::NativeStackBoundary,
                Some(root),
                "automatic native stack planning cannot index a source function",
            ));
        };
        let Some(entry) = image
            .entries()
            .iter()
            .find(|entry| entry.source_function().get() == function.raw())
        else {
            return Err(EngineError::new(
                FailureCode::BackendVerification,
                Some(*function),
                "native stack planning cannot find the installed entry",
            ));
        };
        let Some(frame) = image
            .frames()
            .iter()
            .find(|frame| frame.function() == entry.function())
        else {
            return Err(EngineError::new(
                FailureCode::BackendVerification,
                Some(*function),
                "native stack planning cannot find frame facts",
            ));
        };
        let bytes = usize::try_from(frame.frame_bytes()).map_err(|_| {
            EngineError::new(
                FailureCode::NativeStackBoundary,
                Some(*function),
                "native frame size exceeds host stack representation",
            )
        })?;
        let Some(slot) = included.get_mut(index) else {
            return Err(EngineError::new(
                FailureCode::BackendVerification,
                Some(*function),
                "native stack planning source index is out of range",
            ));
        };
        *slot = true;
        frame_bytes[index] = Some(bytes);
    }

    let mut edges = vec![Vec::new(); program.functions.len()];
    for function in functions {
        let index = function.index().ok_or_else(|| {
            EngineError::new(
                FailureCode::BackendVerification,
                Some(*function),
                "native stack planning source identity is invalid",
            )
        })?;
        let item = program.functions.get(index).ok_or_else(|| {
            EngineError::new(
                FailureCode::BackendVerification,
                Some(*function),
                "native stack planning source function is absent",
            )
        })?;
        for callee in item
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .filter_map(|instruction| match &instruction.kind {
                lkjscript_ir::InstructionKind::Call {
                    target: lkjscript_ir::CallTarget::Direct(callee),
                    ..
                } => Some(*callee),
                _ => None,
            })
        {
            let callee_index = callee.index().ok_or_else(|| {
                EngineError::new(
                    FailureCode::BackendVerification,
                    Some(*function),
                    "native stack planning cannot index a direct callee",
                )
            })?;
            if included.get(callee_index).copied() == Some(true) {
                edges[index].push(callee_index);
            }
        }
    }

    let mut requirements: Vec<Option<usize>> = vec![None; program.functions.len()];
    let mut visiting = vec![false; program.functions.len()];
    for function in functions {
        let root_index = function.index().ok_or_else(|| {
            EngineError::new(
                FailureCode::BackendVerification,
                Some(*function),
                "native stack planning source identity is invalid",
            )
        })?;
        if requirements[root_index].is_some() {
            continue;
        }
        let mut work = vec![(root_index, false)];
        while let Some((index, expanded)) = work.pop() {
            if requirements[index].is_some() {
                continue;
            }
            if expanded {
                let mut deepest_callee = 0_usize;
                for callee in &edges[index] {
                    let callee_bytes = requirements[*callee].ok_or_else(|| {
                        EngineError::new(
                            FailureCode::BackendVerification,
                            program.functions.get(index).map(|item| item.id),
                            "native stack planning postorder is incomplete",
                        )
                    })?;
                    let with_call_frame = callee_bytes
                        .checked_add(2 * std::mem::size_of::<usize>())
                        .ok_or_else(|| {
                            EngineError::new(
                                FailureCode::NativeStackBoundary,
                                program.functions.get(index).map(|item| item.id),
                                "native call-stack requirement exceeds host representation",
                            )
                        })?;
                    deepest_callee = deepest_callee.max(with_call_frame);
                }
                requirements[index] = Some(
                    frame_bytes[index]
                        .ok_or_else(|| {
                            EngineError::new(
                                FailureCode::BackendVerification,
                                program.functions.get(index).map(|item| item.id),
                                "native stack planning frame size is absent",
                            )
                        })?
                        .checked_add(deepest_callee)
                        .ok_or_else(|| {
                            EngineError::new(
                                FailureCode::NativeStackBoundary,
                                program.functions.get(index).map(|item| item.id),
                                "native stack requirement exceeds host representation",
                            )
                        })?,
                );
                visiting[index] = false;
                continue;
            }
            if visiting[index] {
                return Err(EngineError::new(
                    FailureCode::NativeStackBoundary,
                    program.functions.get(index).map(|item| item.id),
                    "automatic native entry declines a recursive call graph",
                ));
            }
            visiting[index] = true;
            work.push((index, true));
            for callee in edges[index].iter().rev() {
                if requirements[*callee].is_none() {
                    work.push((*callee, false));
                }
            }
        }
    }

    functions
        .iter()
        .map(|function| {
            let index = function.index().ok_or_else(|| {
                EngineError::new(
                    FailureCode::BackendVerification,
                    Some(*function),
                    "native stack requirement source identity is invalid",
                )
            })?;
            Ok((
                *function,
                requirements[index].ok_or_else(|| {
                    EngineError::new(
                        FailureCode::BackendVerification,
                        Some(*function),
                        "native stack requirement was not computed",
                    )
                })?,
            ))
        })
        .collect()
}

fn numeric_conversion_sites(
    image: &lkjscript_native::InstallableImage,
) -> NumericConversionSiteCounts {
    let mut counts = NumericConversionSiteCounts::default();
    for site in image.structural_runtime_sites() {
        let lkjscript_native::StructuralOperation::NumericConversion {
            kind: conversion, ..
        } = site.descriptor().operation()
        else {
            continue;
        };
        match conversion {
            lkjscript_native::StructuralNumericConversion::F64FromI64Exact => {
                counts.f64_from_i64_exact += 1;
            }
            lkjscript_native::StructuralNumericConversion::I64FromF64Exact => {
                counts.i64_from_f64_exact += 1;
            }
            lkjscript_native::StructuralNumericConversion::I64FromF64Truncating => {
                counts.i64_from_f64_trunc += 1;
            }
        }
    }
    counts
}
