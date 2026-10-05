//! Canonical borrowed invocation, independent of compiled instructions and VM frames.
use super::*;

impl ReferenceState<'_> {
    pub(super) fn evaluate_borrow_call(
        &mut self,
        call: ExpressionId,
        binding: BindingId,
        body: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        let (function, types, effects, requirements, implementations, arguments) =
            match self.read_expression(call)? {
                ExpressionOperation::Call {
                    function,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    arguments,
                } => (
                    function,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    vec![],
                    arguments,
                ),
                ExpressionOperation::ImplementationCall {
                    function,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    implementations,
                    arguments,
                } => (
                    function,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    implementations
                        .into_iter()
                        .map(|operand| self.resolve_implementation(operand))
                        .collect::<Result<Vec<_>, _>>()?,
                    arguments,
                ),
                ExpressionOperation::MethodCall {
                    witness,
                    contract,
                    method,
                    arguments,
                } => {
                    let (function, types, implementations) =
                        self.method_target(witness, contract, method)?;
                    (function, types, vec![], vec![], implementations, arguments)
                }
                _ => {
                    return Err(reference_type_error(
                        "borrow-call requires one exact named invocation",
                    ));
                }
            };
        let signature = self.function_signature(function)?;
        let source = signature
            .result_borrow
            .ok_or_else(|| reference_type_error("borrow-call target has an owning result"))?;
        let position = signature
            .parameters
            .iter()
            .position(|parameter| parameter.header.owner == OwnerKey::Parameter(source))
            .ok_or_else(|| reference_type_error("borrowed result selects a foreign parameter"))?;
        if !signature.pure || signature.parameters[position].use_mode != ParameterUse::Borrow {
            return Err(reference_type_error(
                "borrowed result requires a pure borrowed source parameter",
            ));
        }
        let source_expression = *arguments.get(position).ok_or_else(|| {
            reference_type_error("borrow-call lacks its selected source argument")
        })?;
        let source_local = match self.read_expression(source_expression)? {
            ExpressionOperation::Local { value } if locals.contains_key(&value) => value,
            _ => {
                return Err(reference_type_error(
                    "borrow-call source requires an exact live local",
                ));
            }
        };
        let record = self.binding(binding, BindingKind::OwnedBorrow)?;
        let declared = record.declared_type.ok_or_else(|| {
            reference_type_error("borrow-call binding lacks an exact result type")
        })?;
        let binding_type = self.resolve_type_arguments(&[declared])?[0];
        let uses: Vec<_> = signature.parameters.iter().map(|p| p.use_mode).collect();
        // Invocation arguments are evaluated once in canonical authored order.
        // Their sealed loans protect earlier arguments while later ones execute.
        let arguments = self.evaluate_many_with_uses(&arguments, &uses, locals)?;
        let parent = self.borrow_parent(source_expression, locals)?;
        let types = self.resolve_type_arguments(&types)?;
        let effects = self.resolve_effect_arguments(&effects)?;
        let requirements = self.resolve_requirement_arguments(&requirements)?;
        let target = self.admit_resolved_witness_call(
            function,
            &types,
            &effects,
            &requirements,
            &implementations,
            arguments,
        )?;
        let expected = self.schema.transfer_type_identity(
            target.function.result,
            &target.types,
            self.control,
        )?;
        if expected != binding_type {
            return Err(reference_type_error(
                "borrow-call binding disagrees with its exact result type",
            ));
        }
        let mut value = self.execute_witness_call(target)?;
        #[cfg(test)]
        super::borrowed_result_tests::fault(
            super::borrowed_result_tests::FailureStage::AfterExtraction,
        )?;
        if !value.raw().memory_is_borrowed() {
            return Err(reference_type_error(
                "borrow-call produced an owning result",
            ));
        }
        value.raw().memory_validate(self.memory_domain, false)?;
        #[cfg(test)]
        super::borrowed_result_tests::fault(
            super::borrowed_result_tests::FailureStage::AdoptionReservation,
        )?;
        // A failed caller-slot reservation leaves the sealed return packet in
        // this local, after the departed callee has relinquished custody.
        self.charge_allocation(
            (std::mem::size_of::<LocalValueReference>() + std::mem::size_of::<CheckedValue>())
                as u64,
        )?;
        // Function entry reset each argument to its exact parameter identity;
        // only this lexical caller can rebind that admitted return to its source.
        value.set_provenance(parent.provenance().unwrap_or(source_local));
        self.evaluate_borrowed_body(parent, binding, value, body, locals)
    }
}
