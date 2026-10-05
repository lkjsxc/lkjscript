//! Independent serial meaning of a joined pair, with the same sealed custody boundary.
use super::super::{
    parallel,
    shared_budget::SharedBudget,
    vm::transfer::{TaskApplication, TransferArguments, TransferResult},
};
use super::*;

struct ChildOutcome {
    result: Result<TransferResult, ExecutionError>,
    observation: NormalizedReferenceObservation,
}

struct CanonicalTask {
    declaration: DeclarationReference,
    types: Arc<[TypeObjectDigest]>,
    implementations: Vec<DeclarationReference>,
}
struct CanonicalParallelCall {
    task: CanonicalTask,
    arguments: Vec<ExpressionId>,
    signature: ReferenceSignature,
}

impl ReferenceState<'_> {
    fn resolve_parallel_types(
        &self,
        types: &[TypeObjectDigest],
    ) -> Result<Vec<TypeObjectDigest>, ExecutionError> {
        let empty = BTreeMap::new();
        let bindings = self.type_scopes.last().unwrap_or(&empty);
        types
            .iter()
            .map(|ty| {
                self.schema
                    .transfer_type_identity(*ty, bindings, self.control)
            })
            .collect()
    }
    fn parallel_transfer_type(
        &self,
        ty: TypeObjectDigest,
        depth: usize,
        nodes: &mut u64,
    ) -> Result<bool, ExecutionError> {
        self.control.check()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(reference_type_error(
                "structured result type exceeds finite depth",
            ));
        }
        *nodes = nodes
            .checked_add(1)
            .filter(|n| *n <= super::super::value::MAXIMUM_ADMISSION_ITEMS)
            .ok_or_else(|| {
                reference_resource(
                    "normalized_parallel_transfer_items",
                    "structured result type exceeds finite admission",
                )
            })?;
        match &self
            .schema
            .types
            .get(&ty)
            .ok_or_else(|| reference_type_error("missing structured result type"))?
            .form
        {
            TypeForm::ByteBuffer | TypeForm::OwnedI64Cell => Ok(true),
            TypeForm::OwnedSequence { item } => {
                if direct_memory_type(&self.schema, *item, &BTreeMap::new(), self.control)?
                    .is_none()
                {
                    return Err(reference_type_error("sequence element must be owned"));
                }
                self.parallel_transfer_type(*item, depth + 1, nodes)?;
                Ok(true)
            }
            TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields } => {
                for field in fields {
                    self.parallel_transfer_type(field.ty, depth + 1, nodes)?;
                }
                Ok(true)
            }
            _ if self.schema.transferable_types.contains(&ty) => Ok(false),
            _ => Err(reference_type_error(
                "structured result requires closed data or owned memory",
            )),
        }
    }

    fn parallel_pair_type(
        &self,
        left: TypeObjectDigest,
        right: TypeObjectDigest,
        owned: bool,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        // Derive from canonical child signatures and canonical structural types;
        // the compiled Parallel instruction is not a reference type oracle.
        for (ty, object) in &self.schema.types {
            self.control.check()?;
            let fields = match &object.form {
                TypeForm::OwnedProduct { fields } if owned => fields,
                TypeForm::StructuralRecord { fields } if !owned => fields,
                _ => continue,
            };
            if fields.len() == 2
                && fields[0].name.as_str() == "left"
                && fields[0].ty == left
                && fields[1].name.as_str() == "right"
                && fields[1].ty == right
            {
                return Ok(*ty);
            }
        }
        Err(reference_type_error(
            "canonical structured pair type is absent",
        ))
    }

    fn parallel_call(
        &mut self,
        expression: ExpressionId,
    ) -> Result<CanonicalParallelCall, ExecutionError> {
        let (function, types, implementations, arguments) =
            match self.read_expression(expression)? {
                ExpressionOperation::Call {
                    function,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    arguments,
                } if effect_arguments.is_empty() && requirement_arguments.is_empty() => {
                    (function, type_arguments, Vec::new(), arguments)
                }
                ExpressionOperation::ImplementationCall {
                    function,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    implementations,
                    arguments,
                } if effect_arguments.is_empty() && requirement_arguments.is_empty() => {
                    (function, type_arguments, implementations, arguments)
                }
                _ => {
                    return Err(reference_type_error(
                        "parallel branches require closed direct named calls",
                    ));
                }
            };
        let DeclarationPayload::Function(declaration) = self.declaration(function)?.payload else {
            return Err(reference_type_error(
                "parallel branch must be a canonical graph function",
            ));
        };
        self.charge_allocation((types.len() * std::mem::size_of::<TypeObjectDigest>()) as u64)?;
        let types = self.resolve_parallel_types(&types)?;
        let mut signature = self.function_signature(function)?;
        if !matches!(&signature.effect, FunctionEffect::Task { requirements, effect_parameters } if requirements.is_empty() && effect_parameters.is_empty())
            || !signature.effect_parameters.is_empty()
            || !signature.requirement_parameters.is_empty()
            || signature.parameters.len() != arguments.len()
            || signature.type_parameters.len() != types.len()
            || declaration.implementation_parameters.len() != implementations.len()
        {
            return Err(reference_type_error(
                "parallel child requires an exact closed empty-row task",
            ));
        }
        self.charge_allocation(super::super::value::collection_storage_bytes(
            types.len() as u64,
            (std::mem::size_of::<(TypeParameterId, TypeObjectDigest)>()
                + 3 * std::mem::size_of::<usize>()) as u64,
            "normalized_parallel_application",
        )?)?;
        let mut bindings = BTreeMap::new();
        for ((parameter, ty), constraint) in signature
            .type_parameters
            .iter()
            .zip(&types)
            .zip(&signature.type_parameter_constraints)
        {
            let memory = self.parallel_transfer_type(*ty, 0, &mut 0)?;
            if memory != constraint.has_owned()
                || (constraint.requires_capture_safe()
                    && !self.schema.capture_safe_types.contains(ty))
                || bindings.insert(*parameter, *ty).is_some()
            {
                return Err(reference_type_error(
                    "parallel child type arguments violate exact canonical constraints",
                ));
            }
        }
        self.charge_allocation(
            (implementations.len() * std::mem::size_of::<DeclarationReference>()) as u64,
        )?;
        let mut selected = Vec::with_capacity(implementations.len());
        for operand in implementations {
            selected.push(self.resolve_implementation(operand)?);
        }
        self.implementation_bindings(&declaration, &bindings, &selected)?;
        if !bindings.is_empty() {
            signature.result =
                self.schema
                    .transfer_type_identity(signature.result, &bindings, self.control)?;
        }
        self.parallel_transfer_type(signature.result, 0, &mut 0)?;
        let mut seen_owned = false;
        for parameter in &mut signature.parameters {
            self.control.check()?;
            if !bindings.is_empty() {
                parameter.ty =
                    self.schema
                        .transfer_type_identity(parameter.ty, &bindings, self.control)?;
            }
            let memory = self.parallel_transfer_type(parameter.ty, 0, &mut 0)?;
            if seen_owned && !memory {
                return Err(reference_type_error(
                    "parallel owned parameters must form the final suffix",
                ));
            }
            seen_owned |= memory;
            if parameter.parent
                != crate::platform::kernel::ParameterParent::Function(function.declaration)
                || parameter.resource_requirement.is_some()
                || parameter.use_mode
                    != if memory {
                        crate::platform::kernel::ParameterUse::Consume
                    } else {
                        crate::platform::kernel::ParameterUse::Unrestricted
                    }
            {
                return Err(reference_type_error(
                    "parallel child parameters require closed data or consumed owners",
                ));
            }
        }
        // Vec-to-Arc conversion has its own metadata storage; payloads stay untouched.
        self.charge_allocation((types.len() * std::mem::size_of::<TypeObjectDigest>()) as u64)?;
        Ok(CanonicalParallelCall {
            task: CanonicalTask {
                declaration: function,
                types: types.into(),
                implementations: selected,
            },
            arguments,
            signature,
        })
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
        &mut self,
        task: &CanonicalTask,
        signature: &ReferenceSignature,
    ) -> Result<super::super::value::FunctionIndex, ExecutionError> {
        // Selection is by the exact canonical implementation references, never by
        // an incidental numeric instance order or by compatible Self alone.
        let offset = self
            .program
            .functions
            .iter()
            .position(|f| {
                f.declaration == task.declaration
                    && f.type_parameters.as_ref() == signature.type_parameters.as_slice()
                    && f.implementation_arguments.len() == task.implementations.len()
                    && f.implementation_arguments
                        .iter()
                        .map(|argument| &argument.implementation)
                        .eq(task.implementations.iter())
            })
            .ok_or_else(|| {
                reference_type_error("canonical child has no exact prepared application")
            })?;
        let index = super::super::value::FunctionIndex(
            u32::try_from(offset)
                .map_err(|_| reference_type_error("prepared child index overflow"))?,
            self.program.value_origin,
        );
        self.charge_allocation(super::super::value::collection_storage_bytes(
            task.types.len() as u64,
            (std::mem::size_of::<(TypeParameterId, TypeObjectDigest)>()
                + 3 * std::mem::size_of::<usize>()) as u64,
            "normalized_parallel_application",
        )?)?;
        let bindings: BTreeMap<_, _> = signature
            .type_parameters
            .iter()
            .copied()
            .zip(task.types.iter().copied())
            .collect();
        let substitute = |ty| {
            if bindings.is_empty() {
                Ok(ty)
            } else {
                self.schema
                    .transfer_type_identity(ty, &bindings, self.control)
            }
        };
        let selected = &self.program.functions[offset];
        let mut matches = substitute(selected.result)? == signature.result
            && selected.parameters.len() == signature.parameters.len();
        for (prepared, canonical) in selected.parameters.iter().zip(&signature.parameters) {
            matches &= substitute(prepared.ty)? == canonical.ty
                && prepared.use_mode == canonical.use_mode
                && prepared.resource_requirement.is_none();
        }
        let implementation_arguments = Arc::clone(&selected.implementation_arguments);
        for (prepared, reference) in implementation_arguments.iter().zip(&task.implementations) {
            let canonical = self.checked_implementation(*reference)?;
            matches &= prepared.self_type == canonical.self_type
                && prepared.type_arguments.as_ref() == canonical.type_arguments.as_slice();
        }
        if !matches {
            return Err(reference_type_error(
                "physical child admission disagrees with its canonical instantiated signature",
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
        let CanonicalParallelCall {
            task: left_task,
            arguments: left_args,
            signature: left_signature,
        } = self.parallel_call(left)?;
        let CanonicalParallelCall {
            task: right_task,
            arguments: right_args,
            signature: right_signature,
        } = self.parallel_call(right)?;
        let left_types = Arc::clone(&left_task.types);
        let right_types = Arc::clone(&right_task.types);
        let left_type = left_signature.result;
        let right_type = right_signature.result;
        let owned = self.parallel_transfer_type(left_type, 0, &mut 0)?
            | self.parallel_transfer_type(right_type, 0, &mut 0)?;
        let result_type = self.parallel_pair_type(left_type, right_type, owned)?;
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
        let left_index = self.parallel_destination(&left_task, &left_signature)?;
        let right_index = self.parallel_destination(&right_task, &right_signature)?;
        let left_application = TaskApplication::bind(
            program,
            left_index,
            Arc::clone(&left_types),
            control,
            &mut |bytes| self.charge_allocation(bytes),
        )?;
        let right_application = TaskApplication::bind(
            program,
            right_index,
            Arc::clone(&right_types),
            control,
            &mut |bytes| self.charge_allocation(bytes),
        )?;
        let left = TransferArguments::seal_applied(
            program,
            source,
            left_domain,
            left_application,
            left_values.into_iter().map(CheckedValue::release).collect(),
            control,
            &mut |raw, ty| self.inspect_transfer_data(raw, ty),
        )?;
        let right = TransferArguments::seal_applied(
            program,
            source,
            right_domain,
            right_application,
            right_values
                .into_iter()
                .map(CheckedValue::release)
                .collect(),
            control,
            &mut |raw, ty| self.inspect_transfer_data(raw, ty),
        )?;
        let left = self.run_child(Arc::clone(&budget), left_domain, left_task, left);
        let right = self.run_child(budget, right_domain, right_task, right);
        self.observation.include_child(&left.observation);
        self.observation.include_child(&right.observation);
        let (left, right) = parallel::results(left.result, right.result)?;
        self.control.check()?;
        if owned {
            self.charge_items(2, std::mem::size_of::<NormalizedValue>())?;
            self.charge_allocation(super::super::owned_product::OwnedProduct::ALLOCATION_BYTES)?;
            let left =
                left.adopt_applied(program, source, left_index, &left_types, left_type, control)?;
            let left = self.product_child(left, left_type)?;
            let right = right.adopt_applied(
                program,
                source,
                right_index,
                &right_types,
                right_type,
                control,
            )?;
            let right = self.product_child(right, right_type)?;
            // The complete wrapper storage was reserved before either adoption.
            let token = super::super::owned_product::OwnedProduct::create(
                source,
                result_type,
                vec![left.release(), right.release()],
                control,
                &mut |_| Ok(()),
            )?;
            return CheckedValue::memory(&self.schema, NormalizedValue::OwnedProduct(token));
        }
        self.charge_items(2, std::mem::size_of::<(Name, NormalizedValue)>())?;
        self.charge_allocation(9)?;
        let left_name =
            Name::new("left").map_err(|_| reference_type_error("invalid structured field"))?;
        let right_name =
            Name::new("right").map_err(|_| reference_type_error("invalid structured field"))?;
        let left =
            left.adopt_applied(program, source, left_index, &left_types, left_type, control)?;
        let left = self.admit_raw(left, left_type, &BTreeMap::new(), None, true)?;
        let right = right.adopt_applied(
            program,
            source,
            right_index,
            &right_types,
            right_type,
            control,
        )?;
        let right = self.admit_raw(right, right_type, &BTreeMap::new(), None, true)?;
        self.record_value(None, vec![(left_name, left), (right_name, right)])
    }

    fn run_child(
        &self,
        budget: Arc<SharedBudget>,
        memory_domain: super::super::value::ValueOrigin,
        task: CanonicalTask,
        envelope: TransferArguments,
    ) -> ChildOutcome {
        let declaration = task.declaration;
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
            lexical_loan_scopes: 0,
            borrow_result_sources: Vec::new(),
            borrow_result_demand: false,
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
                production_tier: "graph14_reference_records_10",
                ..NormalizedReferenceObservation::default()
            },
        };
        let result = (|| {
            let (application, values) =
                envelope.adopt_applied(self.program, memory_domain, self.control)?;
            let index = application.function();
            if application.types().as_ref() != task.types.as_ref() {
                return Err(reference_type_error(
                    "structured child type application changed in transit",
                ));
            }
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
                task.types.len() as u64,
                (std::mem::size_of::<(TypeParameterId, TypeObjectDigest)>()
                    + 3 * std::mem::size_of::<usize>()) as u64,
                "normalized_parallel_application",
            )?)?;
            let bindings: BTreeMap<_, _> = signature
                .type_parameters
                .iter()
                .copied()
                .zip(task.types.iter().copied())
                .collect();
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
                        let ty = state.schema.transfer_type_identity(
                            parameter.ty,
                            &bindings,
                            state.control,
                        )?;
                        state.admit_raw(value, ty, &BTreeMap::new(), None, true)
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            let DeclarationPayload::Function(function) = state.declaration(declaration)?.payload
            else {
                return Err(reference_type_error("missing canonical child task"));
            };
            let admitted = state.admit_graph_call_with_implementations(
                declaration,
                function,
                arguments,
                ReferenceApplication {
                    types: &task.types,
                    implementations: &task.implementations,
                    ..Default::default()
                },
            )?;
            let value = state.execute_witness_call(admitted)?;
            TransferResult::seal_applied(
                self.program,
                memory_domain,
                self.memory_domain,
                application,
                value.release(),
                self.control,
                &mut |raw, ty| state.inspect_transfer_data(raw, ty),
            )
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
