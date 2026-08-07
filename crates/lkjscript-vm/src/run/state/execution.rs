use super::*;

impl<'a> Vm<'a> {
    pub(super) fn run_loop(&mut self) -> Result<Stop> {
        if let Some(error) = self.structural_initialization_error.take() {
            return Err(error);
        }
        if let Some(error) = self.global_initialization_error.take() {
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
        if self
            .config
            .max_stack_values()
            .is_some_and(|maximum| self.chunk.main().locals > maximum)
        {
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
            proto: None,
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
                if let Some(fuel) = &mut self.fuel_remaining {
                    if *fuel == 0 {
                        let error = Error::resource(
                            ResourceLimitKind::InstructionFuel,
                            "instruction fuel exhausted",
                        );
                        self.restore_structural_handoffs();
                        self.execute_failure_unwind(site, true);
                        return Err(error);
                    }
                    *fuel -= 1;
                }
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
