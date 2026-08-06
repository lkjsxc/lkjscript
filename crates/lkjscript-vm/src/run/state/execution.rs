use super::*;

impl<'a, J: RuntimeTier> Vm<'a, J> {
    pub(super) fn run_loop(&mut self) -> Result<Stop> {
        if let Some(error) = self.structural_initialization_error.take() {
            return Err(error);
        }
        if let Some(error) = self.region_product_initialization_error.take() {
            return Err(error);
        }
        if self.inputs.capabilities != self.chunk.required_capabilities() {
            return Err(Error::msg(format!(
                "execution capability mismatch: required {:?}, received {:?}",
                self.chunk.required_capabilities(),
                self.inputs.capabilities
            )));
        }
        if self.chunk.main().locals > self.config.max_stack_values {
            return Err(Error::resource(
                ResourceLimitKind::StackValues,
                "VM entry frame exceeds the stack value limit",
            ));
        }
        self.frames
            .try_reserve(1)
            .map_err(|_| Error::host("VM entry frame reservation failed"))?;
        let mut unique_places = Vec::new();
        unique_places
            .try_reserve_exact(self.chunk.main().unique_places)
            .map_err(|_| Error::host("VM entry unique-place reservation failed"))?;
        unique_places.resize(
            self.chunk.main().unique_places,
            unique::RuntimePlace::Inactive,
        );
        self.frames.push(Frame {
            proto: u32::MAX,
            ip: 0,
            instruction_offset: 0,
            stack_base: 0,
            locals_base: 0,
            unique_places,
            borrowed_resources: Vec::new(),
            memory_witnesses: Vec::new(),
        });
        self.stack
            .try_reserve(self.chunk.main().locals)
            .map_err(|_| Error::host("VM entry locals reservation failed"))?;
        for kind in &self.inputs.capabilities {
            self.stack.push(Value::from_capability(*kind));
        }
        for _ in self.inputs.capabilities.len()..self.chunk.main().locals {
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
            let site = self.current_failure_offset();
            if self.is_failure_boundary(site) {
                if let Err(error) = self.check_deadline() {
                    self.restore_structural_handoffs();
                    self.execute_failure_unwind(site, true);
                    return Err(error);
                }
                if self.fuel_remaining == 0 {
                    let error = Error::resource(
                        ResourceLimitKind::InstructionFuel,
                        "instruction fuel exhausted",
                    );
                    self.restore_structural_handoffs();
                    self.execute_failure_unwind(site, true);
                    return Err(error);
                }
                self.fuel_remaining -= 1;
            }
            if let Err(error) = self.step() {
                self.restore_structural_handoffs();
                self.execute_failure_unwind(site, false);
                return Err(error);
            }
            if let Some(error) = self.allocation_error.take() {
                let failure_site = self
                    .frames
                    .last()
                    .map_or(0, |frame| frame.instruction_offset);
                self.restore_structural_handoffs();
                self.execute_failure_unwind(failure_site, false);
                return Err(error);
            }
            let next_site = self.current_failure_offset();
            if self.is_failure_boundary(next_site) {
                if let Err(error) = self.check_runtime_limits() {
                    let failure_site = self
                        .frames
                        .last()
                        .map_or(0, |frame| frame.instruction_offset);
                    self.restore_structural_handoffs();
                    self.execute_failure_unwind(failure_site, false);
                    return Err(error);
                }
            }
        }
    }
}

#[cfg(feature = "jit")]
impl<'a> Vm<'a, JitSession> {
    pub fn run_auto(mut self) -> (ExecutionOutcome, JitStats) {
        let outcome = self.run_inner();
        let stats = self.jit.stats();
        (outcome, stats)
    }
}
