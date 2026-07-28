use super::*;

impl Evaluator<'_> {
    pub(crate) fn call(
        &mut self,
        function_id: FunctionId,
        arguments: Vec<EvalValue>,
        depth: usize,
    ) -> std::result::Result<EvalValue, Flow> {
        if depth >= self.config.max_frames {
            return Err(Flow::Resource("frames".into()));
        }
        let function = self
            .program
            .program()
            .functions
            .get(function_id.index().unwrap_or(usize::MAX))
            .filter(|function| function.id == function_id)
            .cloned()
            .ok_or_else(|| Flow::Trap("evaluator missing verified function".into()))?;
        let entry = function
            .blocks
            .iter()
            .find(|block| block.id == function.entry)
            .ok_or_else(|| Flow::Trap("evaluator missing verified entry block".into()))?;
        if arguments.len() != entry.parameters.len() {
            return Err(Flow::Trap("evaluator function arity mismatch".into()));
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
            .map_or(Some(0), |maximum| maximum.checked_add(1))
            .ok_or_else(|| Flow::Trap("evaluator function value count overflow".into()))?;
        let mut values = vec![None; value_count];
        assign_parameters(&mut values, &entry.parameters, arguments)?;
        let mut current = function.entry;
        loop {
            self.consume_fuel()?;
            let block = function
                .blocks
                .iter()
                .find(|block| block.id == current)
                .cloned()
                .ok_or_else(|| Flow::Trap("evaluator missing verified block".into()))?;
            for instruction in &block.instructions {
                self.consume_fuel()?;
                let value = self.instruction(instruction, &mut values, depth)?;
                set_value(&mut values, instruction.id, value)?;
            }
            self.consume_fuel()?;
            match block.terminator {
                Terminator::Branch { target, arguments } => {
                    let arguments = values_for_edge(&mut values, &arguments)?;
                    let target_block = function
                        .blocks
                        .iter()
                        .find(|block| block.id == target)
                        .ok_or_else(|| Flow::Trap("evaluator branch target is missing".into()))?;
                    assign_parameters(&mut values, &target_block.parameters, arguments)?;
                    current = target;
                }
                Terminator::ConditionalBranch {
                    condition,
                    true_target,
                    true_arguments,
                    false_target,
                    false_arguments,
                } => {
                    let condition = as_bool(value(&values, condition)?)?;
                    let (target, arguments) = if condition {
                        (true_target, true_arguments)
                    } else {
                        (false_target, false_arguments)
                    };
                    let arguments = values_for_edge(&mut values, &arguments)?;
                    let target_block = function
                        .blocks
                        .iter()
                        .find(|block| block.id == target)
                        .ok_or_else(|| Flow::Trap("evaluator branch target is missing".into()))?;
                    assign_parameters(&mut values, &target_block.parameters, arguments)?;
                    current = target;
                }
                Terminator::Return(result) => {
                    return if matches!(
                        value(&values, result)?,
                        EvalValue::Bytes(_) | EvalValue::ByteVector(_)
                    ) {
                        take_value(&mut values, result)
                    } else {
                        value(&values, result).cloned()
                    };
                }
                Terminator::Trap { value: trap } => {
                    return Err(Flow::Trap(as_str(value(&values, trap)?)?.to_owned()))
                }
                Terminator::Exit { code } => {
                    return Err(Flow::Exit(as_i64(value(&values, code)?)?))
                }
                Terminator::Outcome { outcome, detail } => {
                    let detail = detail
                        .map(|value_id| as_str(value(&values, value_id)?).map(str::to_owned))
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
