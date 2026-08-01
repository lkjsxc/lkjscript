use super::*;

impl Evaluator<'_> {
    pub(in crate::eval) fn validate_call_memory_witnesses(
        &self,
        function: &crate::Function,
        arguments: &[EvalValue],
        bindings: &[crate::MemoryWitnessBinding],
    ) -> std::result::Result<(), Flow> {
        if function.signature.memory_witness_parameters.len() != bindings.len() {
            return Err(Flow::Trap(
                "evaluator generic call witness count mismatch".into(),
            ));
        }
        for (requirement, binding) in function
            .signature
            .memory_witness_parameters
            .iter()
            .zip(bindings)
        {
            let witness = self
                .program
                .program()
                .memory
                .witness(binding.witness)
                .ok_or_else(|| Flow::Trap("evaluator memory witness is not installed".into()))?;
            if requirement.parameter != binding.parameter {
                return Err(Flow::Trap(
                    "evaluator memory witness parameter mismatch".into(),
                ));
            }
            for (index, ty) in function.signature.parameters.iter().enumerate() {
                if matches!(ty, crate::SsaType::TypeParameter(name) if name == &requirement.parameter)
                    && arguments
                        .get(index)
                        .is_none_or(|value| !self.eval_value_matches_witness(value, witness))
                {
                    return Err(Flow::Trap(
                        "evaluator memory witness argument mismatch".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    pub(in crate::eval) fn validate_call_memory_result(
        &self,
        function: &crate::Function,
        result: &EvalValue,
        bindings: &[crate::MemoryWitnessBinding],
    ) -> std::result::Result<(), Flow> {
        if function.signature.memory_witness_parameters.is_empty() {
            return Ok(());
        }
        let crate::SsaType::TypeParameter(parameter) = function.signature.result.as_ref() else {
            return Ok(());
        };
        let binding = bindings
            .iter()
            .find(|binding| &binding.parameter == parameter)
            .ok_or_else(|| Flow::Trap("evaluator generic return witness is missing".into()))?;
        let witness = self
            .program
            .program()
            .memory
            .witness(binding.witness)
            .ok_or_else(|| {
                Flow::Trap("evaluator generic return witness is not installed".into())
            })?;
        if self.eval_value_matches_witness(result, witness) {
            Ok(())
        } else {
            Err(Flow::Trap(
                "evaluator generic return does not match its memory witness".into(),
            ))
        }
    }

    fn eval_value_matches_witness(
        &self,
        value: &EvalValue,
        witness: &crate::MemoryWitnessDescriptor,
    ) -> bool {
        match (&witness.ty, value) {
            (crate::SsaType::Unit, EvalValue::Unit)
            | (crate::SsaType::Bool, EvalValue::Bool(_))
            | (crate::SsaType::I64, EvalValue::I64(_))
            | (crate::SsaType::F64, EvalValue::F64(_))
            | (crate::SsaType::List(_), EvalValue::SegmentedList(_) | EvalValue::List(_)) => true,
            (_, EvalValue::StructuralOwner(owner)) if witness.representation.is_some() => {
                structural_type(self.program.program(), &witness.ty).ok() == Some(owner.value_type)
            }
            (_, EvalValue::StructuralView(view) | EvalValue::StructuralUtf8View(view))
                if witness.representation.is_some() =>
            {
                structural_type(self.program.program(), &witness.ty).ok() == Some(view.value_type)
            }
            _ => false,
        }
    }
}
