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
        let prospective = self.begin_borrow_call_context(binding)?;
        let previous = self
            .type_occurrence
            .replace((self.active_package, OwnerKey::Expression(call)));
        let opened = self.open_borrow_call_scope(call, binding, locals);
        let result = match opened {
            Ok(opened) => {
                let result = self.evaluate_borrow_call_at(call, binding, body, locals);
                if let Some((key, owner, local, anchor, scope)) = opened {
                    if let Some(value) = locals.get_mut(&local) {
                        value.restore_source_binding(anchor, scope);
                    }
                    if let Some(owner) = owner {
                        if let Some(frame) = self.read_scope_frames.last_mut() {
                            frame.remove_scope(key);
                        }
                        drop(owner);
                    }
                }
                result
            }
            Err(error) => Err(error),
        };
        if let Some(key) = prospective
            && let Some(frame) = self.read_scope_frames.last_mut()
        {
            frame.remove_scope(key);
        }
        self.type_occurrence = previous;
        result
    }

    fn evaluate_borrow_call_at(
        &mut self,
        call: ExpressionId,
        binding: BindingId,
        body: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        let (function, types, effects, requirements, implementations, scopes, arguments) =
            match self.read_expression(call)? {
                ExpressionOperation::Call {
                    read_scope_arguments,
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
                    self.resolve_read_scope_arguments(&read_scope_arguments)?,
                    arguments,
                ),
                ExpressionOperation::ImplementationCall {
                    read_scope_arguments,
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
                    self.resolve_read_scope_arguments(&read_scope_arguments)?,
                    arguments,
                ),
                ExpressionOperation::MethodCall {
                    read_scope_arguments,
                    witness,
                    contract,
                    method,
                    arguments,
                } => {
                    let (function, types, implementations, scopes) = self.method_target_scoped(
                        witness,
                        contract,
                        method,
                        &read_scope_arguments,
                    )?;
                    (
                        function,
                        types,
                        vec![],
                        vec![],
                        implementations,
                        scopes,
                        arguments,
                    )
                }
                _ => {
                    return Err(reference_type_error(
                        "borrow-call requires one exact named invocation",
                    ));
                }
            };
        let signature = self.function_signature(function)?;
        let source = signature
            .result
            .relation
            .source()
            .copied()
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
        let is_view = matches!(signature.result.relation, ResultRelation::ViewFrom { .. });
        let record = self.binding(
            binding,
            if is_view {
                BindingKind::ReadView
            } else {
                BindingKind::OwnedBorrow
            },
        )?;
        let declared = record.declared_type.ok_or_else(|| {
            reference_type_error("borrow-call binding lacks an exact result type")
        })?;
        let binding_type = self.resolve_occurrence_types(
            (self.active_package, OwnerKey::Binding(binding)),
            &[declared],
        )?[0];
        let uses: Vec<_> = signature.parameters.iter().map(|p| p.use_mode).collect();
        // Invocation arguments are evaluated once in canonical authored order.
        // Their sealed loans protect earlier arguments while later ones execute.
        let arguments = self.evaluate_many_with_uses(&arguments, &uses, locals)?;
        let parent = self.borrow_parent(source_expression, locals)?;
        let types = self.resolve_type_arguments(&types)?;
        let effects = self.resolve_effect_arguments(&effects)?;
        let requirements = self.resolve_requirement_arguments(&requirements)?;
        let target = self.admit_resolved_witness_call_scoped(
            function,
            &types,
            &effects,
            &requirements,
            &implementations,
            &scopes,
            arguments,
        )?;
        let expected = self.schema.transfer_type_identity(
            target.function.result.ty,
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
        if !is_view && !value.raw().memory_is_borrowed() {
            return Err(reference_type_error(
                "borrow-call produced an owning result",
            ));
        }
        if !is_view {
            value.raw().memory_validate(self.memory_domain, false)?;
        }
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
        if let Some(parent_anchor) = parent.source_anchor() {
            let returned = value.source_anchor().ok_or_else(|| {
                reference_type_error("source-tied result lacks its admitted source anchor")
            })?;
            let anchor = super::super::read_view_storage::SourceAnchor {
                path: Arc::clone(&returned.path),
                ..parent_anchor.clone()
            };
            if let NormalizedValue::ReadView(view) = value.raw() {
                let schema = Arc::clone(&self.schema);
                let scope = view.scope()?.key();
                let original = view.source().clone();
                let admission = super::read_views::RuntimeAdmission::rebase(
                    &schema,
                    &original,
                    &anchor,
                    scope,
                    self.control,
                )?;
                value.rebind_view_source(anchor.clone(), &admission, self.control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(self.control, &mut |bytes| self.charge_allocation(bytes))?)?;
            }
            let lease = value
                .source_scope()
                .map(|scope| scope.reborrow(self.control))
                .transpose()?;
            value.bind_source(anchor, lease);
        }
        self.evaluate_borrowed_body(parent, binding, value, body, locals)
    }
}
