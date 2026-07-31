use crate::eval::{clone_plain_eval_value, take_value, value, EvalValue, Evaluator, Flow};

impl Evaluator<'_> {
    pub(crate) fn call_arguments(
        &mut self,
        function: &crate::Function,
        target: crate::FunctionId,
        values: &mut [Option<EvalValue>],
        ids: &[crate::ValueId],
    ) -> Result<Vec<EvalValue>, Flow> {
        let mut arguments = Vec::new();
        arguments
            .try_reserve_exact(ids.len())
            .map_err(|_| Flow::Resource("evaluator call arguments".into()))?;
        for (index, id) in ids.iter().enumerate() {
            let returned = parameter_flows_to_return(self.program.program(), target, index);
            let argument = self.call_argument(function, values, *id, returned);
            match argument {
                Ok(argument) => arguments.push(argument),
                Err(primary) => {
                    if let Err(cleanup) = self.cleanup_legacy_values_reverse(arguments) {
                        self.note_structural_cleanup_failure(cleanup.detail());
                    }
                    return Err(primary);
                }
            }
        }
        Ok(arguments)
    }

    pub(crate) fn edge_values(
        &mut self,
        values: &mut [Option<EvalValue>],
        ids: &[crate::ValueId],
    ) -> Result<Vec<EvalValue>, Flow> {
        let mut output = Vec::new();
        output
            .try_reserve_exact(ids.len())
            .map_err(|_| Flow::Resource("evaluator edge arguments".into()))?;
        for id in ids {
            let next = match value(values, *id) {
                Ok(
                    EvalValue::StructuralOwner(_)
                    | EvalValue::StructuralView(_)
                    | EvalValue::StructuralUtf8View(_)
                    | EvalValue::StructuralDestination(_)
                    | EvalValue::Bytes(_)
                    | EvalValue::ByteVector(_)
                    | EvalValue::Path(_)
                    | EvalValue::Resource(_),
                ) => take_value(values, *id),
                Ok(other) => clone_plain_eval_value(other),
                Err(primary) => Err(primary),
            };
            match next {
                Ok(next) => output.push(next),
                Err(primary) => {
                    if let Err(cleanup) = self.cleanup_legacy_values_reverse(output) {
                        self.note_structural_cleanup_failure(cleanup.detail());
                    }
                    return Err(primary);
                }
            }
        }
        Ok(output)
    }

    fn call_argument(
        &mut self,
        function: &crate::Function,
        values: &mut [Option<EvalValue>],
        id: crate::ValueId,
        returned: bool,
    ) -> Result<EvalValue, Flow> {
        let transferred = function.blocks.iter().any(|block| {
            matches!(block.terminator, crate::Terminator::Return(result) if result == id)
                || block.instructions.iter().any(|instruction| {
                    matches!(
                        instruction.kind,
                        crate::InstructionKind::Move { value, .. } if value == id
                    )
                })
        });
        match value(values, id)? {
            EvalValue::StructuralOwner(_) | EvalValue::StructuralView(_)
                if !transferred && returned =>
            {
                self.copy_eval_value(value(values, id)?)
            }
            EvalValue::StructuralOwner(_) | EvalValue::StructuralView(_) if !transferred => {
                self.borrow_eval_value(value(values, id)?, false)
            }
            EvalValue::StructuralOwner(_)
            | EvalValue::StructuralView(_)
            | EvalValue::StructuralUtf8View(_)
            | EvalValue::StructuralDestination(_)
            | EvalValue::Bytes(_)
            | EvalValue::ByteVector(_)
            | EvalValue::Path(_)
            | EvalValue::Resource(_) => take_value(values, id),
            other => clone_plain_eval_value(other),
        }
    }
}

fn parameter_flows_to_return(
    program: &crate::Program,
    target: crate::FunctionId,
    index: usize,
) -> bool {
    let Some(function) = program
        .functions
        .iter()
        .find(|function| function.id == target)
    else {
        return false;
    };
    let Some(entry) = function
        .blocks
        .iter()
        .find(|block| block.id == function.entry)
    else {
        return false;
    };
    let Some(parameter) = entry.parameters.get(index).map(|parameter| parameter.id) else {
        return false;
    };
    let mut flows = vec![parameter];
    for block in &function.blocks {
        for instruction in &block.instructions {
            let source = match instruction.kind {
                crate::InstructionKind::Copy(source)
                | crate::InstructionKind::Move { value: source, .. } => Some(source),
                _ => None,
            };
            if source.is_some_and(|source| flows.contains(&source)) {
                flows.push(instruction.id);
            }
        }
    }
    function.blocks.iter().any(|block| {
        matches!(block.terminator, crate::Terminator::Return(result) if flows.contains(&result))
    })
}
