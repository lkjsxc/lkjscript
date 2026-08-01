use super::*;

impl Evaluator<'_> {
    pub(crate) fn call(
        &mut self,
        function_id: FunctionId,
        arguments: Vec<EvalValue>,
        memory_witnesses: Vec<crate::MemoryWitnessBinding>,
        depth: usize,
    ) -> std::result::Result<EvalValue, Flow> {
        let function = match self
            .program
            .program()
            .functions
            .get(function_id.index().unwrap_or(usize::MAX))
            .filter(|function| function.id == function_id)
            .cloned()
        {
            Some(function) => function,
            None => {
                self.execute_unentered_argument_cleanup(arguments);
                return Err(Flow::Trap("evaluator missing verified function".into()));
            }
        };
        if depth >= self.config.max_frames {
            self.execute_unentered_argument_cleanup(arguments);
            return Err(Flow::Resource("frames".into()));
        }
        let entry = match function
            .blocks
            .iter()
            .find(|block| block.id == function.entry)
        {
            Some(entry) => entry,
            None => {
                self.execute_unentered_argument_cleanup(arguments);
                return Err(Flow::Trap("evaluator missing verified entry block".into()));
            }
        };
        if arguments.len() != entry.parameters.len() {
            self.execute_unentered_argument_cleanup(arguments);
            return Err(Flow::Trap("evaluator function arity mismatch".into()));
        }
        if let Err(flow) =
            self.validate_call_memory_witnesses(&function, &arguments, &memory_witnesses)
        {
            self.execute_unentered_argument_cleanup(arguments);
            return Err(flow);
        }
        let value_count = function
            .blocks
            .iter()
            .flat_map(|block| {
                block
                    .parameters
                    .iter()
                    .map(|parameter| parameter.id)
                    .chain(block.instructions.iter().map(|instruction| instruction.id))
            })
            .filter_map(ValueId::index)
            .max()
            .map_or(Some(0), |maximum| maximum.checked_add(1));
        let Some(value_count) = value_count else {
            self.execute_unentered_argument_cleanup(arguments);
            return Err(Flow::Trap("evaluator function value count overflow".into()));
        };
        let mut values = Vec::new();
        values.resize_with(value_count, || None);
        if let Err(flow) = assign_parameters(&mut values, &entry.parameters, arguments) {
            self.cleanup_frame_values(&mut values);
            return Err(flow);
        }
        let result = self.run_function(&function, &mut values, depth);
        if let Ok(value) = &result {
            if let Err(flow) = self.validate_call_memory_result(&function, value, &memory_witnesses)
            {
                self.cleanup_frame_values(&mut values);
                return Err(flow);
            }
        }
        match result {
            Ok(EvalValue::StructuralView(view) | EvalValue::StructuralUtf8View(view)) => {
                if let Err(error) = self.structural.runtime.end_view(view.key) {
                    self.note_structural_cleanup_failure(error.to_string());
                }
                self.cleanup_frame_structural_values(&mut values);
                Err(Flow::Trap(
                    "borrowed structural result escaped a call".into(),
                ))
            }
            Ok(EvalValue::StructuralDestination(destination)) => {
                self.abort_structural_destination(destination.key);
                self.cleanup_frame_structural_values(&mut values);
                Err(Flow::Trap("private destination escaped a call".into()))
            }
            other => {
                self.cleanup_frame_structural_values(&mut values);
                other
            }
        }
    }

    fn run_function(
        &mut self,
        function: &crate::Function,
        values: &mut [Option<EvalValue>],
        depth: usize,
    ) -> Result<EvalValue, Flow> {
        let mut current = function.entry;
        loop {
            let block = function
                .blocks
                .iter()
                .find(|block| block.id == current)
                .cloned()
                .ok_or_else(|| Flow::Trap("evaluator missing verified block".into()))?;
            for instruction in &block.instructions {
                if let Err(flow) = self.consume_fuel() {
                    self.execute_unentered_instruction_cleanup(function, instruction, values);
                    self.execute_failure_cleanup(
                        function,
                        instruction.metadata.failure_cleanup,
                        values,
                    );
                    return Err(flow);
                }
                let value = match self.instruction(function, instruction, values, depth) {
                    Ok(value) => value,
                    Err(flow) => {
                        self.execute_failure_cleanup(
                            function,
                            instruction.metadata.failure_cleanup,
                            values,
                        );
                        return Err(flow);
                    }
                };
                set_value(values, instruction.id, value)?;
            }
            if let Err(flow) = self.consume_fuel() {
                self.execute_failure_cleanup(function, block.metadata.failure_cleanup, values);
                return Err(flow);
            }
            match block.terminator {
                Terminator::Branch { target, arguments } => {
                    let arguments = self.edge_values(values, &arguments)?;
                    let target_block = function
                        .blocks
                        .iter()
                        .find(|block| block.id == target)
                        .ok_or_else(|| Flow::Trap("evaluator branch target is missing".into()))?;
                    assign_parameters(values, &target_block.parameters, arguments)?;
                    current = target;
                }
                Terminator::ConditionalBranch {
                    condition,
                    true_target,
                    true_arguments,
                    false_target,
                    false_arguments,
                } => {
                    let condition = as_bool(value(values, condition)?)?;
                    let (target, arguments) = if condition {
                        (true_target, true_arguments)
                    } else {
                        (false_target, false_arguments)
                    };
                    let arguments = self.edge_values(values, &arguments)?;
                    let target_block = function
                        .blocks
                        .iter()
                        .find(|block| block.id == target)
                        .ok_or_else(|| Flow::Trap("evaluator branch target is missing".into()))?;
                    assign_parameters(values, &target_block.parameters, arguments)?;
                    current = target;
                }
                Terminator::Return(result) => return take_value(values, result),
                Terminator::Trap { value: trap } => {
                    let detail = self.string_text_copy(value(values, trap)?)?;
                    return Err(Flow::Trap(detail));
                }
                Terminator::Exit { code } => return Err(Flow::Exit(as_i64(value(values, code)?)?)),
                Terminator::Outcome { outcome, detail } => {
                    let detail = detail
                        .map(|value_id| self.string_text_copy(value(values, value_id)?))
                        .transpose()?
                        .unwrap_or_default();
                    return Err(match outcome {
                        StructuredOutcome::DeadlineExceeded => Flow::Deadline,
                        StructuredOutcome::ResourceLimitExceeded => Flow::Resource(detail),
                        StructuredOutcome::HostFailure => Flow::HostFailure(detail),
                    });
                }
            }
        }
    }
}
