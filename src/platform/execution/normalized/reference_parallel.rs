//! Independent serial meaning of a joined pair, with the same sealed custody boundary.
use super::super::{parallel, shared_budget::SharedBudget, vm::transfer::TransferArguments};
use super::*;

struct ChildOutcome {
    result: Result<CheckedValue, ExecutionError>,
    observation: NormalizedReferenceObservation,
}

impl ReferenceState<'_> {
    fn parallel_call(
        &mut self,
        expression: ExpressionId,
    ) -> Result<(DeclarationReference, Vec<ExpressionId>, ReferenceSignature), ExecutionError> {
        let ExpressionOperation::Call {
            function,
            type_arguments,
            effect_arguments,
            requirement_arguments,
            arguments,
        } = self.read_expression(expression)?
        else {
            return Err(reference_type_error(
                "parallel branches require direct named calls",
            ));
        };
        if !type_arguments.is_empty()
            || !effect_arguments.is_empty()
            || !requirement_arguments.is_empty()
        {
            return Err(reference_type_error(
                "parallel branches require monomorphic calls",
            ));
        }
        if !matches!(
            self.declaration(function)?.payload,
            DeclarationPayload::Function(_)
        ) {
            return Err(reference_type_error(
                "parallel branch must be a canonical graph function",
            ));
        }
        let signature = self.function_signature(function)?;
        if !matches!(&signature.effect, FunctionEffect::Task { requirements, effect_parameters } if requirements.is_empty() && effect_parameters.is_empty())
            || !signature.type_parameters.is_empty()
            || !signature.effect_parameters.is_empty()
            || !signature.requirement_parameters.is_empty()
            || signature.has_implementations
            || !self.schema.comparable_types.contains(&signature.result)
            || signature.parameters.len() != arguments.len()
        {
            return Err(reference_type_error(
                "parallel child requires an exact empty-row task returning closed data",
            ));
        }
        let mut seen_owned = false;
        for parameter in &signature.parameters {
            let memory =
                direct_memory_type(&self.schema, parameter.ty, &BTreeMap::new())?.is_some();
            if seen_owned && !memory {
                return Err(reference_type_error(
                    "parallel owned parameters must form the final suffix",
                ));
            }
            seen_owned |= memory;
            if parameter.resource_requirement.is_some()
                || if memory {
                    parameter.use_mode != crate::platform::kernel::ParameterUse::Consume
                } else {
                    parameter.use_mode != crate::platform::kernel::ParameterUse::Unrestricted
                        || !self.schema.comparable_types.contains(&parameter.ty)
                }
            {
                return Err(reference_type_error(
                    "parallel child parameters require closed data or consumed owners",
                ));
            }
        }
        Ok((function, arguments, signature))
    }

    fn inspect_transfer_data(
        &mut self,
        raw: &NormalizedValue,
        ty: TypeObjectDigest,
    ) -> Result<(), ExecutionError> {
        if !self.schema.comparable_types.contains(&ty)
            || self.inspect_raw(raw, ty, &BTreeMap::new(), None, true)? != Ownership::Ordinary
        {
            return Err(reference_type_error(
                "structured child metadata is not exact canonical closed data",
            ));
        }
        Ok(())
    }

    fn parallel_destination(
        &self,
        declaration: DeclarationReference,
        signature: &ReferenceSignature,
    ) -> Result<super::super::value::FunctionIndex, ExecutionError> {
        let offset = self
            .program
            .functions
            .iter()
            .position(|f| {
                f.declaration == declaration
                    && f.type_parameters.is_empty()
                    && f.implementation_parameters.is_empty()
            })
            .ok_or_else(|| {
                reference_type_error("canonical child has no exact prepared identity")
            })?;
        let index = super::super::value::FunctionIndex(
            u32::try_from(offset)
                .map_err(|_| reference_type_error("prepared child index overflow"))?,
            self.program.value_origin,
        );
        let selected = self
            .program
            .functions
            .get(index.0 as usize)
            .ok_or_else(|| reference_type_error("missing prepared child"))?;
        if selected.result != signature.result
            || selected.parameters.len() != signature.parameters.len()
            || selected
                .parameters
                .iter()
                .zip(&signature.parameters)
                .any(|(prepared, canonical)| {
                    prepared.ty != canonical.ty
                        || prepared.use_mode != canonical.use_mode
                        || prepared.resource_requirement.is_some()
                })
        {
            return Err(reference_type_error(
                "physical child admission disagrees with its canonical signature",
            ));
        }
        Ok(index)
    }

    pub(super) fn parallel(
        &mut self,
        left: ExpressionId,
        right: ExpressionId,
        locals: &mut BTreeMap<LocalValueReference, CheckedValue>,
    ) -> Result<CheckedValue, ExecutionError> {
        if self
            .allowances
            .last()
            .unwrap_or(&self.root_allowance)
            .is_none()
        {
            return Err(reference_type_error(
                "parallel requires a task calling context",
            ));
        }
        if self.structured_depth >= parallel::MAXIMUM_STRUCTURED_DEPTH {
            return Err(reference_resource(
                "normalized_parallel_depth",
                "structured evaluator nesting exceeds its live-stack capacity",
            ));
        }
        let (left_function, left_args, left_signature) = self.parallel_call(left)?;
        let (right_function, right_args, right_signature) = self.parallel_call(right)?;
        self.charge_allocation(super::super::value::collection_storage_bytes(
            left_signature
                .parameters
                .len()
                .saturating_add(right_signature.parameters.len()) as u64,
            std::mem::size_of::<crate::platform::kernel::ParameterUse>() as u64,
            "normalized_parallel_arguments",
        )?)?;
        let left_uses: Vec<_> = left_signature
            .parameters
            .iter()
            .map(|p| p.use_mode)
            .collect();
        let right_uses: Vec<_> = right_signature
            .parameters
            .iter()
            .map(|p| p.use_mode)
            .collect();
        let count = left_args
            .len()
            .checked_add(right_args.len())
            .ok_or_else(|| reference_type_error("structured argument count overflow"))?;
        self.charge_allocation(super::super::value::collection_storage_bytes(
            count as u64,
            std::mem::size_of::<CheckedValue>() as u64,
            "normalized_parallel_arguments",
        )?)?;
        let left_values = self.evaluate_many_with_uses(&left_args, &left_uses, locals)?;
        let right_values = self.evaluate_many_with_uses(&right_args, &right_uses, locals)?;
        if self.shared_budget.is_none() {
            self.charge_allocation(
                (std::mem::size_of::<SharedBudget>() + 2 * std::mem::size_of::<usize>()) as u64,
            )?;
            self.shared_budget = Some(Arc::new(SharedBudget::new(
                self.policy,
                self.remaining_expressions,
                self.observation.allocated_bytes,
                self.observation.collection_items,
            )));
        }
        let budget = self
            .shared_budget
            .as_ref()
            .cloned()
            .ok_or_else(|| reference_type_error("missing structured budget"))?;
        let count = left_values
            .len()
            .checked_add(right_values.len())
            .ok_or_else(|| reference_type_error("structured argument count overflow"))?;
        self.charge_allocation(super::super::value::collection_storage_bytes(
            count as u64,
            std::mem::size_of::<NormalizedValue>() as u64,
            "normalized_parallel_arguments",
        )?)?;
        let left_domain = super::super::value::ValueOrigin::fresh().ok_or_else(|| {
            reference_resource(
                "normalized_parallel_origin",
                "structured destination identity exhausted",
            )
        })?;
        let right_domain = super::super::value::ValueOrigin::fresh().ok_or_else(|| {
            reference_resource(
                "normalized_parallel_origin",
                "structured destination identity exhausted",
            )
        })?;
        let program = self.program;
        let control = self.control;
        let source = self.memory_domain;
        let left_index = self.parallel_destination(left_function, &left_signature)?;
        let right_index = self.parallel_destination(right_function, &right_signature)?;
        let left = TransferArguments::seal(
            program,
            source,
            left_domain,
            left_index,
            left_values.into_iter().map(CheckedValue::release).collect(),
            control,
            &mut |raw, ty| self.inspect_transfer_data(raw, ty),
        )?;
        let right = TransferArguments::seal(
            program,
            source,
            right_domain,
            right_index,
            right_values
                .into_iter()
                .map(CheckedValue::release)
                .collect(),
            control,
            &mut |raw, ty| self.inspect_transfer_data(raw, ty),
        )?;
        let left = self.run_child(Arc::clone(&budget), left_domain, left_function, left);
        let right = self.run_child(budget, right_domain, right_function, right);
        self.observation.include_child(&left.observation);
        self.observation.include_child(&right.observation);
        let (left, right) = parallel::results(left.result, right.result)?;
        self.control.check()?;
        self.charge_items(2, std::mem::size_of::<(Name, NormalizedValue)>())?;
        self.charge_allocation(9)?;
        let left_name =
            Name::new("left").map_err(|_| reference_type_error("invalid structured field"))?;
        let right_name =
            Name::new("right").map_err(|_| reference_type_error("invalid structured field"))?;
        self.record_value(None, vec![(left_name, left), (right_name, right)])
    }

    fn run_child(
        &self,
        budget: Arc<SharedBudget>,
        memory_domain: super::super::value::ValueOrigin,
        declaration: DeclarationReference,
        envelope: TransferArguments,
    ) -> ChildOutcome {
        let resources = match NormalizedResourceScope::new() {
            Ok(resources) => resources,
            Err(error) => {
                self.control.cancel();
                return ChildOutcome {
                    result: Err(error),
                    observation: NormalizedReferenceObservation::default(),
                };
            }
        };
        let mut state = ReferenceState {
            shared_budget: Some(budget),
            structured_depth: self.structured_depth + 1,
            ancestor_depth: self.ancestor_depth.saturating_add(self.call_depth),
            memory_domain,
            authority: self.authority,
            binding: self.binding,
            active_package: declaration.package,
            program: self.program,
            schema: Arc::clone(&self.schema),
            policy: self.policy,
            host: None,
            capabilities: None,
            resources: &resources,
            control: self.control,
            remaining_expressions: self.policy.instruction_steps,
            call_depth: 0,
            control_frames: 0,
            local_counts: Vec::new(),
            next_transaction: 0,
            transactions: BTreeMap::new(),
            calls_by_requirement: BTreeMap::new(),
            implementation_scopes: Vec::new(),
            type_scopes: Vec::new(),
            effect_scopes: Vec::new(),
            requirement_scopes: Vec::new(),
            allowances: Vec::new(),
            root_allowance: Some(EffectRow::default()),
            observation: NormalizedReferenceObservation {
                production_tier: "graph14_reference_records_9",
                ..NormalizedReferenceObservation::default()
            },
        };
        let result = (|| {
            let (index, values) = envelope.adopt(self.program, memory_domain, self.control)?;
            if self
                .program
                .functions
                .get(index.0 as usize)
                .map(|f| f.declaration)
                != Some(declaration)
            {
                return Err(reference_type_error(
                    "structured adoption names another canonical child",
                ));
            }
            let signature = state.function_signature(declaration)?;
            state.charge_allocation(super::super::value::collection_storage_bytes(
                values.len() as u64,
                std::mem::size_of::<CheckedValue>() as u64,
                "normalized_parallel_arguments",
            )?)?;
            let arguments = values
                .into_iter()
                .zip(signature.parameters)
                .map(|(value, parameter)| {
                    if value.memory_form().is_some() {
                        CheckedValue::memory(&state.schema, value)
                    } else {
                        state.admit_raw(value, parameter.ty, &BTreeMap::new(), None, true)
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            let value = state.call_declaration(declaration, &[], &[], &[], arguments)?;
            if value.ownership(&state.schema, &mut state.observation.value_work)?
                != Ownership::Ordinary
            {
                return Err(reference_type_error(
                    "structured child returned nonordinary ownership",
                ));
            }
            Ok(value)
        })();
        if result.is_err() {
            self.control.cancel();
        }
        resources.release_all();
        ChildOutcome {
            result,
            observation: state.observation,
        }
    }
}

impl NormalizedReferenceObservation {
    fn include_child(&mut self, child: &Self) {
        macro_rules! add { ($($field:ident),* $(,)?) => { $(self.$field = self.$field.saturating_add(child.$field);)* }; }
        macro_rules! maximum { ($($field:ident),* $(,)?) => { $(self.$field = self.$field.max(child.$field);)* }; }
        add!(
            expressions,
            calls,
            external_calls,
            capability_calls,
            allocated_bytes,
            allocation_charges,
            collection_items,
            canonical_owner_reads,
            canonical_map_pages_read,
            canonical_objects_read,
            canonical_bytes_read,
            tail_transfers
        );
        maximum!(
            maximum_call_depth,
            maximum_control_frames,
            maximum_live_locals,
            maximum_live_type_bindings,
            maximum_live_effect_bindings,
            maximum_live_allowances
        );
        parallel::include_value_work(&mut self.value_work, &child.value_work);
    }
}
