//! Independent serial meaning of a joined pair, with the same sealed custody boundary.
use super::super::{
    parallel,
    shared_budget::SharedBudget,
    vm::transfer::{
        ScopedArgument, ScopedReadCustody, ScopedReadLease, ScopedReadScope, TaskApplication,
        TransferArguments, TransferResult,
    },
};
use super::*;
use crate::platform::kernel::RegionMode;

struct ChildOutcome {
    result: Result<TransferResult, ExecutionError>,
    observation: NormalizedReferenceObservation,
}

struct CanonicalTask {
    associated: Option<CanonicalApplication>,
    declaration: DeclarationReference,
    types: Arc<[TypeObjectDigest]>,
    implementations: Vec<AppliedReferenceImplementation>,
    read_scope_arguments: Vec<ReadScopeArgument>,
    argument_sources: Vec<ParallelArgumentSource>,
}

struct ParallelArgumentSource {
    binding: Option<Arc<super::checked::SourceBinding>>,
}
struct CanonicalParallelCall {
    task: CanonicalTask,
    arguments: Vec<ExpressionId>,
    signature: ReferenceSignature,
}

fn parallel_reference_parameter(
    position: u32,
    parameters: &[ParameterRecord],
) -> Result<ParameterId, ExecutionError> {
    let parameter = parameters
        .get(position as usize)
        .ok_or_else(|| reference_type_error("parallel source position is foreign"))?;
    let OwnerKey::Parameter(parameter) = parameter.header.owner else {
        return Err(reference_type_error("parallel source owner is foreign"));
    };
    Ok(parameter)
}

fn parallel_reference_result(
    result: CallableResult<u32>,
    parameters: &[ParameterRecord],
) -> Result<CallableResult<ParameterId>, ExecutionError> {
    Ok(CallableResult {
        ty: result.ty,
        relation: match result.relation {
            ResultRelation::Value => ResultRelation::Value,
            ResultRelation::ReadFrom { source: position } => ResultRelation::ReadFrom {
                source: parallel_reference_parameter(position, parameters)?,
            },
            ResultRelation::ViewFrom {
                source: position,
                scope,
            } => ResultRelation::ViewFrom {
                source: parallel_reference_parameter(position, parameters)?,
                scope,
            },
        },
    })
}

fn parallel_applied_scope(
    scope: ReadScopeArgument,
    declaration: DeclarationReference,
    parameters: &[ReadScopeParameter],
    actuals: &[ReadScopeArgument],
) -> Result<ReadScopeArgument, ExecutionError> {
    if parameters.len() != actuals.len() {
        return Err(reference_type_error(
            "parallel scope application arity changed",
        ));
    }
    let ReadScopeArgument::Parameter(reference) = scope else {
        return Err(reference_type_error(
            "parallel signature scope is not an exact formal",
        ));
    };
    if reference.binder != crate::platform::kernel::ReadScopeBinder::Declaration(declaration) {
        return Err(reference_type_error(
            "parallel signature scope has a foreign binder",
        ));
    }
    parameters
        .iter()
        .position(|parameter| parameter.id == reference.parameter)
        .and_then(|position| actuals.get(position).copied())
        .ok_or_else(|| reference_type_error("parallel signature scope has a foreign parameter"))
}

fn physical_catalog_binding(
    catalog: &[super::super::prepare::NormalizedImplementationArgument],
    prepared: &super::super::prepare::NormalizedImplementationArgument,
) -> bool {
    let Some(stored) = catalog.get(prepared.identity as usize) else {
        return false;
    };
    stored.identity == prepared.identity
        && stored.depth == prepared.depth
        && stored.implementation == prepared.implementation
        && stored.implementation_type_arguments == prepared.implementation_type_arguments
        && stored.implementation_type_parameter_constraints
            == prepared.implementation_type_parameter_constraints
        && stored.contract == prepared.contract
        && stored.self_type == prepared.self_type
        && stored.type_arguments == prepared.type_arguments
        && stored.contract_type_parameter_constraints
            == prepared.contract_type_parameter_constraints
        && stored.prerequisites == prepared.prerequisites
        && stored.implementation_generation == prepared.implementation_generation
        && stored.contract_generation == prepared.contract_generation
        && stored.implementation_type_parameters == prepared.implementation_type_parameters
        && stored.implementation_parameter_ids == prepared.implementation_parameter_ids
        && stored.associated_types == prepared.associated_types
        && stored.implementations.len() == prepared.implementations.len()
        && stored
            .implementations
            .iter()
            .zip(prepared.implementations.iter())
            .all(|(stored, supplied)| stored.identity == supplied.identity)
}

fn exact_witness_identity(
    catalog: &[super::super::prepare::NormalizedImplementationArgument],
    prepared: &super::super::prepare::NormalizedImplementationArgument,
    canonical: &AppliedReferenceImplementation,
    depth: usize,
    seen: &mut std::collections::BTreeSet<(u32, u64)>,
) -> bool {
    if depth.saturating_add(canonical.depth) > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
        || seen.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
        || !physical_catalog_binding(catalog, prepared)
        || prepared.implementation != canonical.implementation
        || prepared.depth != canonical.depth
        || prepared.implementation_type_arguments.as_ref() != canonical.type_arguments.as_ref()
        || prepared.implementations.len() != canonical.implementations.len()
    {
        return false;
    }
    if !seen.insert((prepared.identity, canonical.identity)) {
        return true;
    }
    prepared
        .implementations
        .iter()
        .zip(canonical.implementations.iter())
        .all(|(prepared, canonical)| {
            exact_witness_identity(catalog, prepared, canonical, depth + 1, seen)
        })
}

impl ReferenceState<'_> {
    fn checked_physical_witness(
        &mut self,
        prepared: &super::super::prepare::NormalizedImplementationArgument,
        reference: &AppliedReferenceImplementation,
        depth: usize,
        seen: &mut std::collections::BTreeSet<(u32, u64)>,
    ) -> Result<bool, ExecutionError> {
        self.control.check()?;
        if reference.canonical.is_some() {
            let prefix = prepared
                .implementation_type_arguments
                .len()
                .saturating_add(prepared.implementation_type_parameters.len())
                .saturating_add(prepared.implementation_parameter_ids.len())
                .saturating_add(prepared.type_arguments.len())
                .saturating_add(prepared.associated_types.len())
                .saturating_add(prepared.implementations.len())
                .saturating_add(prepared.prerequisites.len())
                .saturating_add(1);
            self.associated_step(prefix)?;
            for prerequisite in prepared.prerequisites.iter() {
                self.associated_step(prerequisite.type_arguments.len().saturating_add(1))?;
            }
        }
        if depth.saturating_add(reference.depth)
            > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
            || seen.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN
        {
            return Err(reference_type_error(
                "physical witness exceeds canonical bounds",
            ));
        }
        if !physical_catalog_binding(&self.program.implementation_applications, prepared) {
            return Ok(false);
        }
        // Check each occurrence's flat catalog binding before reusing a pair.
        if !seen.insert((prepared.identity, reference.identity)) {
            return Ok(true);
        }
        let canonical = self.checked_implementation(reference)?;
        if let Some(selected) = &reference.canonical {
            let mut matches = prepared.implementation == selected.implementation
                && prepared.contract == selected.contract
                && prepared.depth == selected.depth
                && prepared.implementation_generation == selected.implementation_generation
                && prepared.contract_generation == selected.contract_generation
                && prepared.implementation_type_parameters.as_ref()
                    == selected.parameters.as_slice()
                && prepared.implementation_parameter_ids.as_ref()
                    == selected.prerequisite_parameters.as_slice()
                && prepared.implementation_type_arguments.as_ref() == selected.actuals.as_slice()
                && prepared.implementation_type_parameter_constraints.as_ref()
                    == selected.constraints.as_slice()
                && prepared.self_type == selected.self_type
                && prepared.type_arguments.as_ref() == selected.contract_actuals.as_slice()
                && prepared.contract_type_parameter_constraints.as_ref()
                    == selected.contract_constraints.as_slice()
                && prepared.associated_types.len() == selected.members.len()
                && prepared.implementations.len() == reference.implementations.len();
            for (prepared, expected) in prepared.associated_types.iter().zip(&selected.members) {
                self.control.check()?;
                matches &= prepared.member == expected.member
                    && prepared.template == expected.template
                    && prepared.ty == expected.ty;
            }
            for (prepared, reference) in prepared
                .implementations
                .iter()
                .zip(reference.implementations.iter())
            {
                matches &= self.checked_physical_witness(prepared, reference, depth + 1, seen)?;
            }
            return Ok(matches);
        }
        let scheme_constraints =
            self.type_parameter_constraints(reference.implementation, &canonical.type_parameters)?;
        let DeclarationPayload::OwnedContract(contract) =
            self.declaration(canonical.contract)?.payload
        else {
            return Err(reference_type_error(
                "physical witness selects a foreign contract kind",
            ));
        };
        let contract_constraints =
            self.type_parameter_constraints(canonical.contract, &contract.type_parameters)?;
        let mut matches = prepared.implementation == reference.implementation
            && prepared.depth == reference.depth
            && prepared.implementation_type_arguments.as_ref() == reference.type_arguments.as_ref()
            && prepared.implementation_type_parameter_constraints.as_ref()
                == scheme_constraints.as_slice()
            && prepared.contract == canonical.contract
            && prepared.self_type == canonical.self_type
            && prepared.type_arguments.as_ref() == canonical.type_arguments.as_slice()
            && prepared.contract_type_parameter_constraints.as_ref()
                == contract_constraints.as_slice()
            && prepared.implementations.len() == reference.implementations.len();
        for (prepared, reference) in prepared
            .implementations
            .iter()
            .zip(reference.implementations.iter())
        {
            matches &= self.checked_physical_witness(prepared, reference, depth + 1, seen)?;
        }
        Ok(matches)
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
            TypeForm::RegionCap {
                mode: RegionMode::Unique | RegionMode::Frozen,
                ..
            } => Ok(true),
            TypeForm::OwnedSequence { item } => {
                self.parallel_transfer_type(*item, depth + 1, nodes)?;
                Ok(true)
            }
            TypeForm::OwnedProduct { fields } | TypeForm::OwnedChoice { cases: fields } => {
                for field in fields {
                    self.parallel_transfer_type(field.ty, depth + 1, nodes)?;
                }
                Ok(true)
            }
            TypeForm::Named { .. } | TypeForm::Applied { .. } if self.schema.is_owned_type(ty) => {
                let record = self.schema.owned_record_layout(ty).ok_or_else(|| {
                    reference_type_error("structured result has no exact owned nominal layout")
                })?;
                for argument in record.arguments.iter() {
                    self.parallel_transfer_type(*argument, depth + 1, nodes)?;
                }
                for field in record.fields.iter() {
                    self.parallel_transfer_type(field.ty, depth + 1, nodes)?;
                }
                Ok(true)
            }
            TypeForm::ReadViewApplied { .. }
            | TypeForm::AssociatedReadViewMember { .. }
            | TypeForm::AssociatedReadViewProjection { .. } => Err(reference_type_error(
                "structured transfer cannot own, capture or return a read view",
            )),
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
        let previous = self
            .type_occurrence
            .replace((self.active_package, OwnerKey::Expression(expression)));
        let result = self.parallel_call_at(expression);
        self.type_occurrence = previous;
        result
    }

    fn parallel_call_at(
        &mut self,
        expression: ExpressionId,
    ) -> Result<CanonicalParallelCall, ExecutionError> {
        let (function, types, implementations, read_scope_arguments, arguments) =
            match self.read_expression(expression)? {
                ExpressionOperation::Call {
                    function,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    read_scope_arguments,
                    arguments,
                } if effect_arguments.is_empty() && requirement_arguments.is_empty() => (
                    function,
                    type_arguments,
                    Vec::new(),
                    read_scope_arguments,
                    arguments,
                ),
                ExpressionOperation::ImplementationCall {
                    function,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    implementations,
                    read_scope_arguments,
                    arguments,
                } if effect_arguments.is_empty() && requirement_arguments.is_empty() => (
                    function,
                    type_arguments,
                    implementations,
                    read_scope_arguments,
                    arguments,
                ),
                _ => {
                    return Err(reference_type_error(
                        "parallel branches require closed direct named calls",
                    ));
                }
            };
        let owner = self.declaration(function)?;
        let contract_version = owner.header.contract_version;
        let DeclarationPayload::Function(declaration) = owner.payload else {
            return Err(reference_type_error(
                "parallel branch must be a canonical graph function",
            ));
        };
        self.charge_allocation((types.len() * std::mem::size_of::<TypeObjectDigest>()) as u64)?;
        let types = self.resolve_type_arguments(&types)?;
        let read_scope_arguments = self.resolve_read_scope_arguments(&read_scope_arguments)?;
        let mut signature = self.function_signature(function)?;
        if !matches!(&signature.effect, FunctionEffect::Task { requirements, effect_parameters } if requirements.is_empty() && effect_parameters.is_empty())
            || !signature.effect_parameters.is_empty()
            || !signature.requirement_parameters.is_empty()
            || signature.parameters.len() != arguments.len()
            || signature.type_parameters.len() != types.len()
            || declaration.implementation_parameters.len() != implementations.len()
            || signature.read_scope_parameters.len() != read_scope_arguments.len()
            || signature.result.relation != ResultRelation::Value
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
            let region = matches!(
                self.schema.types.get(ty).map(|t| &t.form),
                Some(TypeForm::RegionBrand { .. })
            );
            if region || constraint.is_region() {
                if region != constraint.is_region() || bindings.insert(*parameter, *ty).is_some() {
                    return Err(reference_type_error(
                        "parallel child changes its canonical region kind",
                    ));
                }
                continue;
            }
            let memory =
                direct_memory_type(&self.schema, *ty, &BTreeMap::new(), self.control)?.is_some();
            if memory != constraint.has_owned()
                || (!memory && !self.schema.buffer_free_types.contains(ty))
                || (constraint.requires_transfer() && !self.schema.transferable_types.contains(ty))
                || (constraint.requires_share() && !self.schema.shareable_types.contains(ty))
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
            (implementations.len() * std::mem::size_of::<AppliedReferenceImplementation>()) as u64,
        )?;
        let mut selected = Vec::with_capacity(implementations.len());
        for operand in implementations {
            selected.push(self.resolve_implementation(operand)?);
        }
        let associated = self.canonical_application(
            function,
            ReferenceApplication {
                types: &types,
                implementations: &selected,
                scopes: &read_scope_arguments,
                ..Default::default()
            },
        )?;
        if let Some(application) = &associated {
            self.canonical_implementation_bindings(
                application,
                &declaration.implementation_parameters,
                &selected,
            )?;
            let proof = self
                .schema
                .associated_proof()
                .cloned()
                .ok_or_else(|| reference_type_error("canonical parallel proof is missing"))?;
            let closed = proof.application(application, self.control)?;
            if closed.parameters.len() != signature.parameters.len()
                || closed.result.relation != ResultRelation::Value
                || !closed.effect.requirements.is_empty()
                || !closed.effect.parameters.is_empty()
                || closed.read_scope_parameters != signature.read_scope_parameters
            {
                return Err(reference_type_error(
                    "canonical parallel signature is not an empty-row owning task",
                ));
            }
            signature.result = parallel_reference_result(closed.result, &signature.parameters)?;
            self.charge_items(
                closed.read_scope_sources.len(),
                std::mem::size_of::<ReadScopeSource<ParameterId>>(),
            )?;
            signature.read_scope_sources = closed
                .read_scope_sources
                .iter()
                .map(|source| {
                    Ok(ReadScopeSource {
                        source: parallel_reference_parameter(source.source, &signature.parameters)?,
                        scope: source.scope,
                    })
                })
                .collect::<Result<_, ExecutionError>>()?;
            for (parameter, (ty, use_mode)) in
                signature.parameters.iter_mut().zip(&closed.parameters)
            {
                if parameter.use_mode != *use_mode {
                    return Err(reference_type_error(
                        "canonical parallel parameter mode changed",
                    ));
                }
                parameter.ty = *ty;
            }
        } else {
            if !signature.read_scope_parameters.is_empty()
                || !signature.read_scope_sources.is_empty()
                || !read_scope_arguments.is_empty()
            {
                return Err(reference_type_error(
                    "parallel read scopes require exact canonical application admission",
                ));
            }
            self.implementation_bindings(
                function,
                &declaration.implementation_parameters,
                &bindings,
                &selected,
            )?;
        }
        if associated.is_none() && !bindings.is_empty() {
            signature.result = CallableResult {
                ty: self.schema.transfer_type_identity(
                    signature.result.ty,
                    &bindings,
                    self.control,
                )?,
                relation: signature.result.relation,
            };
        }
        self.parallel_transfer_type(signature.result.ty, 0, &mut 0)?;
        let mut seen_owned = false;
        for parameter in &mut signature.parameters {
            self.control.check()?;
            if associated.is_none() && !bindings.is_empty() {
                parameter.ty =
                    self.schema
                        .transfer_type_identity(parameter.ty, &bindings, self.control)?;
            }
            let memory =
                direct_memory_type(&self.schema, parameter.ty, &BTreeMap::new(), self.control)?
                    .is_some();
            let view = self
                .schema
                .types
                .get(&parameter.ty)
                .is_some_and(|object| object.form.is_read_view());
            if view {
                let proof = self.schema.read_view_proof.as_ref().ok_or_else(|| {
                    reference_type_error("parallel view input lacks independent category admission")
                })?;
                if proof.prepared_category(parameter.ty)?
                    != crate::platform::kernel::ValueCategory::ReadView
                    || !proof.prepared_read_shareable(parameter.ty)?
                    || parameter.use_mode != ParameterUse::Borrow
                    || contract_version != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
                    || parameter.header.contract_version
                        != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
                {
                    return Err(reference_type_error(
                        "parallel view input requires an independently ReadShareable exact descriptor",
                    ));
                }
            }
            let region = matches!(
                self.schema.types.get(&parameter.ty).map(|t| &t.form),
                Some(TypeForm::RegionCap { .. })
            );
            let scoped = memory || view || region;
            if seen_owned && !scoped {
                return Err(reference_type_error(
                    "parallel owned parameters must form the final suffix",
                ));
            }
            seen_owned |= scoped;
            if parameter.parent
                != crate::platform::kernel::ParameterParent::Function(function.declaration)
                || parameter.resource_requirement.is_some()
                || if view {
                    parameter.use_mode != ParameterUse::Borrow
                } else if memory || region {
                    !matches!(
                        parameter.use_mode,
                        ParameterUse::Borrow | ParameterUse::Consume
                    )
                } else {
                    parameter.use_mode != ParameterUse::Unrestricted
                }
            {
                return Err(reference_type_error(
                    "parallel child parameters require closed data or scoped memory",
                ));
            }
            if view {
                // The positive category and accessible reference closure above
                // grant only joined read input, never transfer or capture.
            } else if parameter.use_mode == ParameterUse::Borrow {
                if contract_version != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
                    || parameter.header.contract_version
                        != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
                    || !self.schema.shareable_types.contains(&parameter.ty)
                {
                    return Err(reference_type_error(
                        "parallel read input requires an independently shareable complete carrier",
                    ));
                }
            } else {
                self.parallel_transfer_type(parameter.ty, 0, &mut 0)?;
            }
        }
        // Vec-to-Arc conversion has its own metadata storage; payloads stay untouched.
        self.charge_allocation((types.len() * std::mem::size_of::<TypeObjectDigest>()) as u64)?;
        Ok(CanonicalParallelCall {
            task: CanonicalTask {
                associated,
                declaration: function,
                types: types.into(),
                implementations: selected,
                read_scope_arguments,
                argument_sources: Vec::new(),
            },
            arguments,
            signature,
        })
    }

    fn scoped_arguments(
        &mut self,
        values: Vec<CheckedValue>,
        application: &TaskApplication,
        signature: &ReferenceSignature,
        destination: super::super::value::ValueOrigin,
    ) -> Result<(Vec<ScopedArgument>, Vec<ParallelArgumentSource>), ExecutionError> {
        if values.len() != signature.parameters.len() {
            return Err(reference_type_error(
                "structured scope custody requires the complete ordered canonical arguments",
            ));
        }
        self.charge_allocation(super::super::value::collection_storage_bytes(
            values.len() as u64,
            std::mem::size_of::<ScopedArgument>() as u64,
            "normalized_parallel_arguments",
        )?)?;
        self.charge_items(values.len(), std::mem::size_of::<ParallelArgumentSource>())?;
        let mut captured = Vec::with_capacity(values.len());
        let mut sources = Vec::with_capacity(values.len());
        for (position, value) in values.into_iter().enumerate() {
            // Preserve each argument's exact parent placement before the shared
            // raw envelope changes invocation coordinates.
            // Preserve the prior liveness validation before sharing immutable placement metadata.
            if let Some(scope) = value.source_scope() {
                scope.reborrow(self.control)?;
            }
            sources.push(ParallelArgumentSource {
                binding: value.source_binding(),
            });
            let parameter = parallel_reference_parameter(
                u32::try_from(position).map_err(|_| {
                    reference_type_error("parallel source argument position overflow")
                })?,
                &signature.parameters,
            )?;
            let scope_count = signature
                .read_scope_sources
                .iter()
                .filter(|source| source.source == parameter)
                .count();
            if application.read_scope_source_count(position) != scope_count {
                return Err(reference_type_error(
                    "structured source scope custody differs from the canonical ordered inventory",
                ));
            }
            self.charge_items(scope_count, std::mem::size_of::<ScopedReadScope>())?;
            let mut scope_custody = Vec::with_capacity(scope_count);
            let mut expected = signature
                .read_scope_sources
                .iter()
                .filter(|source| source.source == parameter);
            application.visit_read_scope_keys(position, &mut |key| {
                self.control.check()?;
                let expected = expected.next().ok_or_else(|| reference_type_error(
                    "structured source scope custody contains an extra relationship",
                ))?;
                let source = sources.last().ok_or_else(|| reference_type_error(
                    "structured source scope custody lost its ordered argument",
                ))?;
                let lease = source.binding.as_ref().and_then(|binding| binding.scope()).ok_or_else(|| reference_type_error(
                    "structured scoped source lacks an independently admitted runtime lease",
                ))?;
                let anchor = source.binding.as_ref().map(|binding| binding.anchor()).ok_or_else(|| reference_type_error(
                    "structured scoped source lacks an exact checked placement",
                ))?;
                let borrowed = value.raw().memory_is_borrowed()
                    || (matches!(value.raw(), NormalizedValue::ReadView(_)) && value.is_view_borrowed());
                if key != expected.scope || key != lease.key()
                    || anchor.program != self.schema.value_origin
                    || lease.source().program != self.schema.value_origin
                    || signature.parameters[position].use_mode != ParameterUse::Borrow
                    || !borrowed
                {
                    return Err(reference_type_error(
                        "structured source relationship changes its exact canonical key or checked placement",
                    ));
                }
                scope_custody.push(ScopedReadScope::new(
                    key,
                    lease.reborrow(self.control)?,
                    anchor.clone(),
                    lease.source().clone(),
                )?);
                Ok(())
            })?;
            if expected.next().is_some() || scope_custody.len() != scope_count {
                return Err(reference_type_error(
                    "structured source scope custody omits a canonical relationship",
                ));
            }
            if let NormalizedValue::ReadView(view) = value.raw() {
                if !value.is_view_borrowed() {
                    return Err(reference_type_error(
                        "parallel view custody requires an admitted read borrow",
                    ));
                }
                self.charge_allocation(
                    ScopedReadCustody::ALLOCATION_BYTES
                        + std::mem::size_of::<CheckedValue>() as u64,
                )?;
                let source = view.source().clone();
                let anchor = super::super::read_view_storage::SourceAnchor {
                    program: self.schema.value_origin,
                    application: application.application_identity(),
                    invocation: destination,
                    coordinate: super::super::read_view_storage::SourceCoordinate::Parameter(
                        u32::try_from(position).map_err(|_| {
                            reference_type_error("parallel view argument position overflow")
                        })?,
                    ),
                    origin: destination,
                    path: Arc::from([]),
                };
                let shared = self.share_parallel_view(&value, anchor)?;
                captured.push(ScopedArgument::View {
                    value: shared.release(),
                    source,
                    custody: ScopedReadCustody::new(value),
                });
            } else if value.raw().memory_is_borrowed() {
                self.charge_allocation(
                    ScopedReadCustody::ALLOCATION_BYTES
                        + std::mem::size_of::<CheckedValue>() as u64,
                )?;
                let raw = value.duplicate(ParameterUse::Borrow)?.release();
                let custody = ScopedReadCustody::new(value);
                if scope_count == 0 {
                    captured.push(ScopedArgument::Read {
                        value: raw,
                        custody,
                    });
                } else {
                    captured.push(ScopedArgument::ReadScoped {
                        value: raw,
                        custody,
                        scopes: scope_custody,
                    });
                }
            } else {
                captured.push(ScopedArgument::Value(value.release()));
            }
        }
        Ok((captured, sources))
    }

    fn inspect_transfer_data(
        &mut self,
        raw: &NormalizedValue,
        ty: TypeObjectDigest,
    ) -> Result<(), ExecutionError> {
        if matches!(raw, NormalizedValue::RegionPointer { .. }) {
            return super::super::region_value::check_plain(
                &self.schema.types,
                raw,
                ty,
                self.control,
            );
        }
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
        let mut offset = None;
        let program = self.program;
        for (candidate, f) in program.functions.iter().enumerate() {
            if task.associated.is_some() {
                self.control.check()?;
                self.associated_step(
                    f.type_parameters
                        .len()
                        .saturating_add(f.type_arguments.len())
                        .saturating_add(f.implementation_arguments.len())
                        .saturating_add(1),
                )?;
            }
            let compatible = {
                f.declaration == task.declaration
                    && f.type_parameters.as_ref() == signature.type_parameters.as_slice()
                    && f.type_arguments.as_ref() == task.types.as_ref()
                    && f.implementation_arguments.len() == task.implementations.len()
                    && f.implementation_arguments
                        .iter()
                        .zip(&task.implementations)
                        .all(|(argument, canonical)| {
                            exact_witness_identity(
                                &self.program.implementation_applications,
                                argument,
                                canonical,
                                0,
                                &mut std::collections::BTreeSet::new(),
                            )
                        })
            };
            if !compatible {
                continue;
            }
            if task.associated.is_some() {
                let mut matches = true;
                for (prepared, canonical) in
                    f.implementation_arguments.iter().zip(&task.implementations)
                {
                    matches &= self.checked_physical_witness(
                        prepared,
                        canonical,
                        0,
                        &mut std::collections::BTreeSet::new(),
                    )?;
                }
                if !matches {
                    continue;
                }
            }
            offset = Some(candidate);
            break;
        }
        let offset = offset.ok_or_else(|| {
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
        let physical_context = if task.associated.is_some() {
            Some(
                super::super::prepare::PreparedApplicationContext::new(
                    self.program,
                    index,
                    &bindings,
                    self.control,
                )
                .map_err(super::super::vm::transfer::context_error)?,
            )
        } else {
            None
        };
        let substitute = |ty| {
            if let Some(context) = &physical_context {
                return super::super::vm::transfer::resolve_application(
                    self.program,
                    ty,
                    context,
                    self.control,
                );
            }
            if bindings.is_empty() {
                Ok(ty)
            } else {
                self.schema
                    .transfer_type_identity(ty, &bindings, self.control)
            }
        };
        let selected = &self.program.functions[offset];
        let mut matches = parallel_reference_result(
            CallableResult {
                ty: substitute(selected.result.ty)?,
                relation: selected.result.relation,
            },
            &signature.parameters,
        )? == signature.result
            && selected.parameters.len() == signature.parameters.len()
            && selected.read_scope_parameters.as_ref()
                == signature.read_scope_parameters.as_slice()
            && selected.read_scope_arguments.as_ref() == task.read_scope_arguments.as_slice()
            && selected.read_scope_sources.len() == signature.read_scope_sources.len();
        for (prepared, canonical) in selected
            .read_scope_sources
            .iter()
            .zip(&signature.read_scope_sources)
        {
            self.control.check()?;
            matches &= parallel_reference_parameter(prepared.source, &signature.parameters)?
                == canonical.source
                && parallel_applied_scope(
                    prepared.scope,
                    task.declaration,
                    &selected.read_scope_parameters,
                    &task.read_scope_arguments,
                )? == canonical.scope;
        }
        if let Some(application) = &task.associated {
            let proof = self
                .schema
                .associated_proof()
                .ok_or_else(|| reference_type_error("canonical physical child proof is missing"))?;
            matches &= selected.source_generation
                == proof.context(application, self.control)?.occurrence(
                    (
                        task.declaration.package,
                        OwnerKey::Declaration(task.declaration.declaration),
                    ),
                    self.control,
                )?;
        }
        for (prepared, canonical) in selected.parameters.iter().zip(&signature.parameters) {
            matches &= substitute(prepared.ty)? == canonical.ty
                && prepared.use_mode == canonical.use_mode
                && prepared.resource_requirement.is_none()
                && canonical.header.owner == OwnerKey::Parameter(prepared.parameter);
        }
        let implementation_arguments = Arc::clone(&selected.implementation_arguments);
        for (prepared, reference) in implementation_arguments.iter().zip(&task.implementations) {
            matches &= self.checked_physical_witness(
                prepared,
                reference,
                0,
                &mut std::collections::BTreeSet::new(),
            )?;
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
            task: mut left_task,
            arguments: left_args,
            signature: left_signature,
        } = self.parallel_call(left)?;
        let CanonicalParallelCall {
            task: mut right_task,
            arguments: right_args,
            signature: right_signature,
        } = self.parallel_call(right)?;
        let left_types = Arc::clone(&left_task.types);
        let right_types = Arc::clone(&right_task.types);
        let left_type = left_signature.result.ty;
        let right_type = right_signature.result.ty;
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
            std::mem::size_of::<NormalizedValue>() as u64 + ScopedReadLease::ALLOCATION_BYTES,
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
        let (left_values, left_sources) =
            self.scoped_arguments(left_values, &left_application, &left_signature, left_domain)?;
        let (right_values, right_sources) = self.scoped_arguments(
            right_values,
            &right_application,
            &right_signature,
            right_domain,
        )?;
        left_task.argument_sources = left_sources;
        right_task.argument_sources = right_sources;
        let left = TransferArguments::seal_scoped_applied(
            program,
            self.regions,
            source,
            left_domain,
            left_application,
            left_values,
            control,
            &mut |raw, ty| self.inspect_transfer_data(raw, ty),
        )?;
        let right = TransferArguments::seal_scoped_applied(
            program,
            self.regions,
            source,
            right_domain,
            right_application,
            right_values,
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
            token.establish_admission(self.schema.value_origin)?;
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
        let mut capture_bytes = (std::mem::size_of::<read_views::ScopeFrame>()
            + task.read_scope_arguments.len()
                * (std::mem::size_of::<ReadScopeArgument>()
                    + std::mem::size_of::<super::super::read_view_storage::RuntimeScopeLease>()
                    + 6 * std::mem::size_of::<usize>())) as u64;
        if let Err(error) = budget.reserve(capture_bytes, 0) {
            self.control.cancel();
            return ChildOutcome {
                result: Err(error),
                observation: NormalizedReferenceObservation::default(),
            };
        }
        let mut capture_charges = 1;
        let capture = self.capture_read_scope_frame(&task.read_scope_arguments, &mut |bytes| {
            let next_bytes = capture_bytes.checked_add(bytes).ok_or_else(|| {
                reference_resource(
                    "normalized_reference_allocation",
                    "scope capture allocation size overflowed",
                )
            })?;
            budget.reserve(bytes, 0)?;
            capture_bytes = next_bytes;
            capture_charges += 1;
            Ok(())
        });
        let observation = NormalizedReferenceObservation {
            allocated_bytes: capture_bytes,
            allocation_charges: capture_charges,
            production_tier: "graph14_reference_records_10",
            ..NormalizedReferenceObservation::default()
        };
        let read_scope_frame = match capture {
            Ok(frame) => frame,
            Err(error) => {
                self.control.cancel();
                return ChildOutcome {
                    result: Err(error),
                    observation,
                };
            }
        };
        let resources = match NormalizedResourceScope::new() {
            Ok(resources) => resources,
            Err(error) => {
                self.control.cancel();
                return ChildOutcome {
                    result: Err(error),
                    observation,
                };
            }
        };
        let mut state = ReferenceState {
            regions: self.regions,
            reference_witnesses: Arc::clone(&self.reference_witnesses),
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
            read_source_demand: Vec::new(),
            local_counts: Vec::new(),
            next_transaction: 0,
            transactions: BTreeMap::new(),
            calls_by_requirement: BTreeMap::new(),
            implementation_scopes: Vec::new(),
            associated_applications: Vec::new(),
            associated_work: Arc::clone(&self.associated_work),
            type_occurrence: None,
            type_scopes: Vec::new(),
            effect_scopes: Vec::new(),
            requirement_scopes: Vec::new(),
            read_scope_frames: vec![read_scope_frame],
            allowances: Vec::new(),
            root_allowance: Some(EffectRow::default()),
            observation,
        };
        let result = (|| {
            let (application, values, leases) = envelope.adopt_scoped_applied(
                self.program,
                self.regions,
                memory_domain,
                self.control,
            )?;
            // These anchors belong to the complete child invocation, outside
            // all replaceable call frames and nested structured groups.
            let read_leases = leases;
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
            let mut signature = state.function_signature(declaration)?;
            if let Some(application) = &task.associated {
                let proof = state
                    .schema
                    .associated_proof()
                    .cloned()
                    .ok_or_else(|| reference_type_error("canonical child proof is missing"))?;
                let closed = proof.application(application, state.control)?;
                if closed.parameters.len() != signature.parameters.len() {
                    return Err(reference_type_error(
                        "canonical child signature changed in transit",
                    ));
                }
                signature.result = parallel_reference_result(closed.result, &signature.parameters)?;
                if closed.read_scope_parameters != signature.read_scope_parameters {
                    return Err(reference_type_error(
                        "canonical child scope inventory changed in transit",
                    ));
                }
                state.charge_items(
                    closed.read_scope_sources.len(),
                    std::mem::size_of::<ReadScopeSource<ParameterId>>(),
                )?;
                signature.read_scope_sources = closed
                    .read_scope_sources
                    .iter()
                    .map(|source| {
                        Ok(ReadScopeSource {
                            source: parallel_reference_parameter(
                                source.source,
                                &signature.parameters,
                            )?,
                            scope: source.scope,
                        })
                    })
                    .collect::<Result<_, ExecutionError>>()?;
                for (parameter, (ty, use_mode)) in
                    signature.parameters.iter_mut().zip(&closed.parameters)
                {
                    if parameter.use_mode != *use_mode {
                        return Err(reference_type_error(
                            "canonical child mode changed in transit",
                        ));
                    }
                    parameter.ty = *ty;
                }
            }
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
            if values.len() != signature.parameters.len()
                || values.len() != task.argument_sources.len()
            {
                return Err(reference_type_error(
                    "structured ordered argument source inventory changed in transit",
                ));
            }
            let mut imported_sources = 0usize;
            for (position, source) in task.argument_sources.iter().enumerate() {
                state.control.check()?;
                let parameter = parallel_reference_parameter(
                    u32::try_from(position)
                        .map_err(|_| reference_type_error("structured source position overflow"))?,
                    &signature.parameters,
                )?;
                let scope_count = signature
                    .read_scope_sources
                    .iter()
                    .filter(|source| source.source == parameter)
                    .count();
                let lease = read_leases
                    .iter()
                    .find(|lease| lease.position() == position);
                let Some(lease) = lease else {
                    if scope_count != 0 {
                        return Err(reference_type_error(
                            "structured child omitted ordered source scope custody",
                        ));
                    }
                    continue;
                };
                if lease.imported_scope_count(&application) != scope_count {
                    return Err(reference_type_error(
                        "structured child source scope custody changed its complete inventory",
                    ));
                }
                let mut expected = signature
                    .read_scope_sources
                    .iter()
                    .filter(|source| source.source == parameter);
                lease.visit_imported_scopes(&application, state.control, &mut |actual_position, key, imported| {
                    let expected = expected.next().ok_or_else(|| reference_type_error(
                        "structured child imported an extra source scope relationship",
                    ))?;
                    let captured = source.binding.as_ref().and_then(|binding| binding.scope()).ok_or_else(|| reference_type_error(
                        "structured child source scope lacks independent checked custody",
                    ))?;
                    let anchor = source.binding.as_ref().map(|binding| binding.anchor()).ok_or_else(|| reference_type_error(
                        "structured child source scope lacks its ordered checked placement",
                    ))?;
                    if actual_position != position || key != expected.scope
                        || key != captured.key() || key != imported.key()
                        || !captured.same_live_scope(&imported)
                        || captured.source() != imported.source()
                        || anchor.program != state.schema.value_origin
                    {
                        return Err(reference_type_error(
                            "structured child scope is detached from its exact ordered argument lifetime",
                        ));
                    }
                    imported_sources = imported_sources.checked_add(1).ok_or_else(|| {
                        reference_type_error("structured child source scope count overflow")
                    })?;
                    Ok(())
                })?;
                if expected.next().is_some() {
                    return Err(reference_type_error(
                        "structured child omitted a canonical source scope relationship",
                    ));
                }
            }
            if imported_sources != signature.read_scope_sources.len() {
                return Err(reference_type_error(
                    "structured child source scope custody is not the complete canonical inventory",
                ));
            }
            state.charge_allocation(super::super::value::collection_storage_bytes(
                values.len() as u64,
                std::mem::size_of::<CheckedValue>() as u64,
                "normalized_parallel_arguments",
            )?)?;
            let arguments = values
                .into_iter()
                .zip(signature.parameters)
                .zip(task.argument_sources)
                .map(|((value, parameter), source)| {
                    let schema = Arc::clone(&state.schema);
                    let control = state.control;
                    let mut checked = if matches!(
                        value,
                        NormalizedValue::RegionCap { .. } | NormalizedValue::RegionPointer { .. }
                    ) {
                        let ty = schema.transfer_type_identity(parameter.ty, &bindings, control)?;
                        CheckedValue::region(
                            &schema,
                            state.regions,
                            memory_domain,
                            ty,
                            value,
                            control,
                        )?
                    } else if let NormalizedValue::ReadView(view) = value {
                        CheckedValue::view(&schema, view, true, control, &mut |bytes| {
                            state.charge_allocation(bytes)
                        })?
                    } else if value.memory_form().is_some() {
                        CheckedValue::memory(&state.schema, value)?
                    } else {
                        let ty = state.schema.transfer_type_identity(
                            parameter.ty,
                            &bindings,
                            state.control,
                        )?;
                        state.admit_raw(value, ty, &BTreeMap::new(), None, true)?
                    };
                    if let Some(binding) = source.binding {
                        checked.bind_source(binding);
                    }
                    Ok(checked)
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
                    scopes: &task.read_scope_arguments,
                    ..Default::default()
                },
            )?;
            if admitted.associated != task.associated {
                return Err(reference_type_error(
                    "structured child changed canonical application in transit",
                ));
            }
            // The forwarding frame grants only the parent actuals needed for
            // admission. The admitted call now owns its exact callee frame.
            state.read_scope_frames.clear();
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
