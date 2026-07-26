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
            resources: ResourceTable::new(config.max_handles),
            fuel_remaining: config.instruction_fuel,
            output_bytes: 0,
            allocation_error: None,
            logical_aggregate_constructions: 0,
            started: Instant::now(),
            config,
        }
    }

    pub fn run(mut self) -> ExecutionOutcome {
        self.run_inner()
    }

    fn run_inner(&mut self) -> ExecutionOutcome {
        let stopped = self.run_loop();
        let mut outcome = match stopped {
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

        let resources = std::mem::replace(&mut self.resources, ResourceTable::new(0));
        drop(resources);
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
        if restore_error.is_some() || flush_error.is_some() {
            let prior = outcome.summary();
            let message = match (restore_error, flush_error) {
                (Some(restore), Some(flush)) => {
                    format!("{restore}; stdout cleanup {flush}")
                }
                (Some(restore), None) => restore.to_string(),
                (None, Some(flush)) => format!("stdout cleanup {flush}"),
                (None, None) => String::new(),
            };
            outcome = ExecutionOutcome::HostFailure(HostError::during_cleanup(message, prior));
        }
        outcome
    }

    fn run_loop(&mut self) -> Result<Stop> {
        if self.inputs.capabilities != self.chunk.required_capabilities() {
            return Err(Error::msg(format!(
                "execution capability mismatch: required {:?}, received {:?}",
                self.chunk.required_capabilities(),
                self.inputs.capabilities
            )));
        }
        self.frames.push(Frame {
            proto: u32::MAX,
            ip: 0,
            stack_base: 0,
            locals_base: 0,
        });
        for kind in &self.inputs.capabilities {
            self.stack.push(Value::from_capability(*kind));
        }
        for _ in self.inputs.capabilities.len()..usize::from(self.chunk.main().locals) {
            self.stack.push(Value::INVALID);
        }
        self.check_runtime_limits()?;
        loop {
            if let Some(code) = self.exit_code {
                return Ok(Stop::Exited(code));
            }
            if self.frames.is_empty() {
                return self.pop().map(Stop::Returned);
            }
            self.check_deadline()?;
            if self.fuel_remaining == 0 {
                return Err(Error::resource(
                    ResourceLimitKind::InstructionFuel,
                    "instruction fuel exhausted",
                ));
            }
            self.fuel_remaining -= 1;
            if self.arena.needs_collect() {
                self.collect();
            }
            self.step()?;
            if let Some(error) = self.allocation_error.take() {
                return Err(error);
            }
            self.check_runtime_limits()?;
        }
    }
}

impl<'a> Vm<'a, JitSession> {
    pub fn run_auto(mut self) -> (ExecutionOutcome, JitStats) {
        let outcome = self.run_inner();
        let stats = self.jit.stats();
        (outcome, stats)
    }
}
