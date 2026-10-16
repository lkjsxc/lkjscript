//! Complete raw canonical inventories, separate from concrete runtime properties.
use super::*;

impl Derivation<'_> {
    pub(super) fn associated_bound(&mut self, ty: TypeObjectDigest) -> Result<(), ExecutionError> {
        self.budget.tick()?;
        let view = matches!(
            self.types.get(&ty).ok_or_else(reject)?.form,
            TypeForm::AssociatedReadViewMember { .. }
                | TypeForm::AssociatedReadViewProjection { .. }
        );
        let (contract, member) = match self.types.get(&ty).ok_or_else(reject)?.form {
            TypeForm::AssociatedContractMember { contract, member }
            | TypeForm::AssociatedReadViewMember {
                contract, member, ..
            } => (contract, member),
            TypeForm::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            }
            | TypeForm::AssociatedReadViewProjection {
                scope,
                parameter,
                contract,
                member,
                ..
            } => {
                let owner = self.declaration(scope, None)?;
                let source_view = view || self.read_view_owner(scope)?;
                self.associated_generation(scope, owner.header.contract_version, source_view)?;
                let parameters = match &owner.payload {
                    DeclarationPayload::Function(function) => &function.implementation_parameters,
                    DeclarationPayload::OwnedImplementation(scheme) => {
                        &scheme.implementation_parameters
                    }
                    _ => return Err(reject()),
                };
                let mut found = None;
                for formal in parameters {
                    self.budget.tick()?;
                    if formal.id == parameter
                        && formal.contract == contract
                        && found.replace(formal).is_some()
                    {
                        return Err(reject());
                    }
                }
                let formal = found.ok_or_else(reject)?;
                let TypeForm::TypeParameter { parameter } =
                    self.types.get(&formal.self_type).ok_or_else(reject)?.form
                else {
                    return Err(reject());
                };
                let OwnerRecord::TypeParameter(parameter) =
                    self.owner(scope.package, OwnerKey::TypeParameter(parameter))?
                else {
                    return Err(reject());
                };
                if parameter.declaration != scope.declaration
                    || !parameter.constraints.has_owned()
                    || parameter.header.contract_version
                        != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
                {
                    return Err(reject());
                }
                (contract, member)
            }
            _ => return Err(reject()),
        };
        let owner = self.declaration(contract, None)?;
        let contract_view = view || self.read_view_contract(contract)?;
        self.associated_generation(contract, owner.header.contract_version, contract_view)?;
        let DeclarationPayload::OwnedContract(contract) = &owner.payload else {
            return Err(reject());
        };
        let mut found = false;
        for candidate in &contract.associated_types {
            self.budget.tick()?;
            if candidate.id == member
                && match &candidate.bound {
                    AssociatedTypeBound::Owned => !view,
                    AssociatedTypeBound::ReadViewFamily { .. } => view,
                }
            {
                if found {
                    return Err(reject());
                }
                found = true;
            }
        }
        if found { Ok(()) } else { Err(reject()) }
    }

    pub(super) fn symbolic_row(
        &mut self,
        row: &EffectRow,
        env: &Environment,
    ) -> Result<EffectRow, ExecutionError> {
        self.budget.validate_row(row)?;
        let mut requirements = BTreeSet::new();
        let mut parameters = BTreeSet::new();
        for reference in &row.parameters {
            self.budget.tick()?;
            let scope = env.scope.ok_or_else(reject)?;
            if reference.package != scope.package {
                return Err(reject());
            }
            let OwnerRecord::EffectParameter(owner) = self.owner(
                reference.package,
                OwnerKey::EffectParameter(reference.parameter),
            )?
            else {
                return Err(reject());
            };
            let DeclarationPayload::Function(function) = &self.declaration(scope, None)?.payload
            else {
                return Err(reject());
            };
            let mut found = 0;
            for id in &function.effect_parameters {
                self.budget.tick()?;
                found += usize::from(*id == reference.parameter);
            }
            if owner.declaration != scope.declaration || found != 1 {
                return Err(reject());
            }
            if let Some(actual) = env.symbolic_effects.get(reference) {
                // Supplied rows were already closed in their original caller context.
                // Insert their atoms without interpreting them in this callee scope.
                self.budget.validate_row(actual)?;
                for requirement in &actual.requirements {
                    self.budget.tick()?;
                    if !requirements.contains(requirement) {
                        self.budget.node::<RequirementOperand>()?;
                        requirements.insert(*requirement);
                    }
                }
                for parameter in &actual.parameters {
                    self.budget.tick()?;
                    if !parameters.contains(parameter) {
                        self.budget.node::<EffectParameterReference>()?;
                        parameters.insert(*parameter);
                    }
                }
            } else if !parameters.contains(reference) {
                self.budget.node::<EffectParameterReference>()?;
                parameters.insert(*reference);
            }
        }
        for operand in &row.requirements {
            let operand = self.symbolic_requirement(*operand, env)?;
            if !requirements.contains(&operand) {
                self.budget.node::<RequirementOperand>()?;
                requirements.insert(operand);
            }
        }
        self.budget
            .reserve::<RequirementOperand>(requirements.len())?;
        self.budget
            .reserve::<EffectParameterReference>(parameters.len())?;
        Ok(EffectRow {
            requirements: requirements.into_iter().collect(),
            parameters: parameters.into_iter().collect(),
        })
    }

    fn symbolic_requirement(
        &mut self,
        operand: RequirementOperand,
        env: &Environment,
    ) -> Result<RequirementOperand, ExecutionError> {
        self.budget.tick()?;
        match operand {
            RequirementOperand::Concrete(reference) => {
                if !matches!(
                    self.owner(
                        reference.package,
                        OwnerKey::Requirement(reference.requirement)
                    )?,
                    OwnerRecord::Requirement(_)
                ) {
                    return Err(reject());
                }
            }
            RequirementOperand::Parameter(reference) => {
                let scope = env.scope.ok_or_else(reject)?;
                if reference.package != scope.package {
                    return Err(reject());
                }
                let OwnerRecord::RequirementParameter(owner) = self.owner(
                    reference.package,
                    OwnerKey::RequirementParameter(reference.parameter),
                )?
                else {
                    return Err(reject());
                };
                let DeclarationPayload::Function(function) =
                    &self.declaration(scope, None)?.payload
                else {
                    return Err(reject());
                };
                let mut found = 0;
                for id in &function.requirement_parameters {
                    self.budget.tick()?;
                    found += usize::from(*id == reference.parameter);
                }
                if owner.declaration != scope.declaration || found != 1 {
                    return Err(reject());
                }
                if let Some(actual) = env.symbolic_requirements.get(&reference) {
                    return Ok(*actual);
                }
            }
        }
        Ok(operand)
    }

    fn symbolic_environment(
        &mut self,
        scope: DeclarationReference,
    ) -> Result<Environment, ExecutionError> {
        let owner = self.declaration(scope, None)?;
        let parameters = owner.payload.type_parameters();
        self.assumptions(scope, parameters.iter())?;
        let mut env = Environment {
            scope: Some(scope),
            generation: owner.header.contract_version,
            lexical: matches!(
                owner.payload,
                DeclarationPayload::Function(_) | DeclarationPayload::OwnedImplementation(_)
            ),
            symbolic: true,
            ..Environment::default()
        };
        if let DeclarationPayload::Function(function) = &owner.payload {
            let scopes = function
                .read_scope_parameters
                .iter()
                .map(|formal| {
                    ReadScopeArgument::Parameter(ReadScopeReference {
                        binder: ReadScopeBinder::Declaration(scope),
                        parameter: formal.id,
                    })
                })
                .collect::<Vec<_>>();
            self.scope_bindings(
                ReadScopeBinder::Declaration(scope),
                &function.read_scope_parameters,
                &scopes,
                &mut env,
            )?;
        }
        for parameter in parameters.iter() {
            let ty = self.intern_type(TypeForm::TypeParameter {
                parameter: *parameter,
            })?;
            self.budget.node::<(TypeParameterId, TypeObjectDigest)>()?;
            env.ordinary.insert(*parameter, ty);
        }
        let formals = match &owner.payload {
            DeclarationPayload::Function(function) => &function.implementation_parameters[..],
            DeclarationPayload::OwnedImplementation(scheme) => {
                &scheme.implementation_parameters[..]
            }
            _ => &[],
        };
        let mut names = BTreeSet::new();
        for formal in formals {
            if env.symbolic_witnesses.contains_key(&formal.id) || names.contains(&formal.name) {
                return Err(reject());
            }
            let TypeForm::TypeParameter { parameter } =
                self.types.get(&formal.self_type).ok_or_else(reject)?.form
            else {
                return Err(reject());
            };
            let OwnerRecord::TypeParameter(parameter) =
                self.owner(scope.package, OwnerKey::TypeParameter(parameter))?
            else {
                return Err(reject());
            };
            if parameter.declaration != scope.declaration || !parameter.constraints.has_owned() {
                return Err(reject());
            }
            let contract = self.declaration(formal.contract, Some(scope.package))?;
            let DeclarationPayload::OwnedContract(contract) = &contract.payload else {
                return Err(reject());
            };
            if contract.type_parameters.len() != formal.type_arguments.len() {
                return Err(reject());
            }
            if let Some(previous) = self.symbolic_formals.get(&(scope, formal.id)) {
                self.budget.tick()?;
                for _ in &formal.type_arguments {
                    self.budget.tick()?;
                }
                for _ in formal.name.as_str().bytes() {
                    self.budget.tick()?;
                }
                if previous != formal {
                    return Err(reject());
                }
            } else {
                self.budget.node::<(
                    (DeclarationReference, ImplementationParameterId),
                    ImplementationParameter,
                )>()?;
                self.budget.reserve::<u8>(formal.name.as_str().len())?;
                self.budget
                    .reserve::<TypeObjectDigest>(formal.type_arguments.len())?;
                self.symbolic_formals
                    .insert((scope, formal.id), formal.clone());
            }
            self.budget
                .reserve::<TypeObjectDigest>(formal.type_arguments.len())?;
            self.budget.reserve::<SymbolicSelection>(1)?;
            self.budget.reserve::<usize>(2)?;
            self.budget
                .node::<(ImplementationParameterId, Arc<SymbolicSelection>)>()?;
            self.budget.node::<&crate::platform::kernel::Name>()?;
            env.symbolic_witnesses.insert(
                formal.id,
                Arc::new(SymbolicSelection {
                    source: SymbolicSelectionSource::Parameter(scope, formal.id),
                    contract: formal.contract,
                    self_type: formal.self_type,
                    arguments: formal.type_arguments.clone(),
                }),
            );
            names.insert(&formal.name);
        }
        let assumptions = self.assumptions(scope, owner.payload.type_parameters().iter())?;
        for formal in formals {
            self.ty(formal.self_type, &env, 0)?;
            let contract = self.declaration(formal.contract, Some(scope.package))?;
            let DeclarationPayload::OwnedContract(contract) = &contract.payload else {
                return Err(reject());
            };
            let constraints = self.assumptions(formal.contract, contract.type_parameters.iter())?;
            for (parameter, actual) in contract.type_parameters.iter().zip(&formal.type_arguments) {
                let actual = self.ty(*actual, &env, 0)?;
                if self.classify_scoped(actual, false, 0, &mut BTreeSet::new(), &assumptions)?
                    != constraints.get(parameter).ok_or_else(reject)?.has_owned()
                {
                    return Err(reject());
                }
            }
        }
        Ok(env)
    }

    fn reserve_operand(
        &mut self,
        operand: &ImplementationOperand,
        depth: usize,
    ) -> Result<(), ExecutionError> {
        self.budget.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        if let ImplementationOperand::Concrete {
            type_arguments,
            implementations,
            ..
        } = operand
        {
            self.budget
                .reserve::<TypeObjectDigest>(type_arguments.len())?;
            self.budget
                .reserve::<ImplementationOperand>(implementations.len())?;
            for child in implementations {
                self.reserve_operand(child, depth + 1)?;
            }
        }
        Ok(())
    }

    pub(super) fn mapping_inventory(
        &mut self,
        scope: DeclarationReference,
        scheme: &crate::platform::kernel::OwnedImplementation,
    ) -> Result<(), ExecutionError> {
        let contract = self.declaration(scheme.contract, Some(scope.package))?;
        let DeclarationPayload::OwnedContract(contract) = &contract.payload else {
            return Err(reject());
        };
        if scheme.methods.len() != contract.methods.len() {
            return Err(reject());
        }
        let env = self.symbolic_environment(scope)?;
        self.budget
            .reserve::<TypeObjectDigest>(scheme.type_parameters.len())?;
        self.budget
            .reserve::<ImplementationOperand>(scheme.implementation_parameters.len())?;
        let mut actuals = Vec::new();
        for id in &scheme.type_parameters {
            self.budget.tick()?;
            actuals.push(*env.ordinary.get(id).ok_or_else(reject)?);
        }
        let mut prerequisites = Vec::new();
        for formal in &scheme.implementation_parameters {
            self.budget.tick()?;
            prerequisites.push(ImplementationOperand::Parameter {
                scope,
                parameter: formal.id,
            });
        }
        let selected = self.symbolic_operand(
            &ImplementationOperand::Concrete {
                implementation: scope,
                type_arguments: actuals,
                implementations: prerequisites,
            },
            &env,
            0,
        )?;
        for mapping in &scheme.methods {
            let mut count = 0;
            for method in &contract.methods {
                self.budget.tick()?;
                count += usize::from(method.id == mapping.method);
            }
            if count != 1
                || self
                    .mapping_templates
                    .contains_key(&(scope, mapping.method))
            {
                return Err(reject());
            }
            // Close the complete operand inventory under the implementation's symbolic
            // scope. This validates its target and all unused actual/prerequisite roots.
            // Reserve both temporary operation and retained source template before cloning.
            self.budget.reserve::<TypeObjectDigest>(
                mapping
                    .type_arguments
                    .len()
                    .checked_mul(2)
                    .ok_or_else(capacity)?,
            )?;
            self.budget.reserve::<ImplementationOperand>(
                mapping
                    .implementations
                    .len()
                    .checked_mul(2)
                    .ok_or_else(capacity)?,
            )?;
            for operand in &mapping.implementations {
                self.reserve_operand(operand, 0)?;
                self.reserve_operand(operand, 0)?;
            }
            self.budget.reserve::<ReadScopeArgument>(
                mapping
                    .read_scope_arguments
                    .len()
                    .checked_mul(2)
                    .ok_or_else(capacity)?,
            )?;
            let operation = ExpressionOperation::ImplementationCall {
                function: mapping.function,
                type_arguments: mapping.type_arguments.clone(),
                implementations: mapping.implementations.clone(),
                effect_arguments: Vec::new(),
                requirement_arguments: Vec::new(),
                read_scope_arguments: mapping.read_scope_arguments.clone(),
                arguments: Vec::new(),
            };
            let target = self.symbolic_target(&operation, &env)?.ok_or_else(reject)?;
            let method = contract
                .methods
                .iter()
                .find(|method| method.id == mapping.method)
                .ok_or_else(reject)?;
            let expected = self.scoped_method_signature(
                &selected,
                mapping.method,
                &method
                    .read_scope_parameters
                    .iter()
                    .map(|formal| {
                        ReadScopeArgument::Parameter(ReadScopeReference {
                            binder: ReadScopeBinder::ContractMethod {
                                contract: scheme.contract,
                                method: mapping.method,
                            },
                            parameter: formal.id,
                        })
                    })
                    .collect::<Vec<_>>(),
            )?;
            self.compare_scoped_method(&expected, &target)?;
            self.budget
                .node::<((DeclarationReference, MethodId), SymbolicTarget)>()?;
            if self
                .symbolic_mappings
                .insert((scope, mapping.method), target)
                .is_some()
            {
                return Err(reject());
            }
            self.budget.node::<(
                (DeclarationReference, MethodId),
                crate::platform::kernel::OwnedMethodImplementation,
            )>()?;
            self.mapping_templates
                .insert((scope, mapping.method), mapping.clone());
        }
        Ok(())
    }

    pub(super) fn contract_inventory(
        &mut self,
        scope: DeclarationReference,
    ) -> Result<(), ExecutionError> {
        let owner = self.declaration(scope, None)?;
        let DeclarationPayload::OwnedContract(contract) = &owner.payload else {
            return Err(reject());
        };
        let contract_view = self.read_view_contract(scope)?;
        if !contract.associated_types.is_empty() || contract_view {
            self.associated_generation(scope, owner.header.contract_version, contract_view)?;
        }
        self.budget
            .node::<(DeclarationReference, crate::platform::kernel::OwnedContract)>()?;
        self.budget
            .reserve::<TypeParameterId>(contract.type_parameters.len())?;
        self.budget
            .reserve::<crate::platform::kernel::AssociatedTypeMember>(
                contract.associated_types.len(),
            )?;
        for member in &contract.associated_types {
            self.budget.tick()?;
            self.budget.reserve::<u8>(member.name.as_str().len())?;
            if let AssociatedTypeBound::ReadViewFamily { scope_parameter } = &member.bound {
                self.budget
                    .reserve::<u8>(scope_parameter.name.as_str().len())?;
            }
        }
        self.budget
            .reserve::<crate::platform::kernel::OwnedMethod>(contract.methods.len())?;
        for method in &contract.methods {
            self.budget.tick()?;
            self.budget.reserve::<u8>(method.name.as_str().len())?;
            self.budget
                .reserve::<ReadScopeParameter>(method.read_scope_parameters.len())?;
            self.budget
                .reserve::<ReadScopeSource<u32>>(method.read_scope_sources.len())?;
            for parameter in &method.read_scope_parameters {
                self.budget.reserve::<u8>(parameter.name.as_str().len())?;
            }
            self.budget
                .reserve::<crate::platform::kernel::OwnedMethodParameter>(
                    method.parameters.len(),
                )?;
            if let crate::platform::kernel::FunctionEffect::Task {
                requirements,
                effect_parameters,
            } = &method.effect
            {
                self.budget
                    .reserve::<RequirementOperand>(requirements.len())?;
                self.budget
                    .reserve::<EffectParameterReference>(effect_parameters.len())?;
            }
        }
        self.contract_templates.insert(scope, contract.clone());
        // Validate and intern every contract formal even if no method uses it.
        // Each method then gets its own environment for its qualified scope formals.
        self.symbolic_environment(scope)?;
        let assumptions = self.assumptions(scope, owner.payload.type_parameters().iter())?;
        if !assumptions
            .get(&contract.self_parameter)
            .is_some_and(|constraints| constraints.has_owned())
        {
            return Err(reject());
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for method in &contract.methods {
            let mut env = self.symbolic_environment(scope)?;
            let scopes = method
                .read_scope_parameters
                .iter()
                .map(|formal| {
                    ReadScopeArgument::Parameter(ReadScopeReference {
                        binder: ReadScopeBinder::ContractMethod {
                            contract: scope,
                            method: method.id,
                        },
                        parameter: formal.id,
                    })
                })
                .collect::<Vec<_>>();
            self.scope_bindings(
                ReadScopeBinder::ContractMethod {
                    contract: scope,
                    method: method.id,
                },
                &method.read_scope_parameters,
                &scopes,
                &mut env,
            )?;
            if ids.contains(&method.id) || names.contains(&method.name) {
                return Err(reject());
            }
            self.budget.node::<MethodId>()?;
            self.budget.node::<&crate::platform::kernel::Name>()?;
            ids.insert(method.id);
            names.insert(&method.name);
            let mut roots = BTreeMap::new();
            let mut occurrences = BTreeMap::new();
            self.occurrence(
                &mut occurrences,
                scope.package,
                OwnerKey::Declaration(scope.declaration),
            )?;
            let result = self.root(
                &mut roots,
                OwnerKey::Declaration(scope.declaration),
                method.result.ty,
                &env,
            )?;
            self.budget
                .reserve::<(TypeObjectDigest, ParameterUse)>(method.parameters.len())?;
            let mut parameters = Vec::new();
            let mut seen_owned = false;
            for parameter in &method.parameters {
                let ty = self.root(
                    &mut roots,
                    OwnerKey::Declaration(scope.declaration),
                    parameter.ty,
                    &env,
                )?;
                let view = self.view_type(ty)?;
                let owned =
                    self.classify_scoped(ty, false, 0, &mut BTreeSet::new(), &assumptions)?;
                if seen_owned && !owned
                    || if owned || view {
                        !matches!(
                            parameter.use_mode,
                            ParameterUse::Consume | ParameterUse::Borrow
                        )
                    } else {
                        parameter.use_mode != ParameterUse::Unrestricted
                    }
                {
                    return Err(reject());
                }
                seen_owned |= owned;
                parameters.push((ty, parameter.use_mode));
            }
            if let Some(position) = method.result.relation.source().copied() {
                let parameter = parameters.get(position as usize).ok_or_else(reject)?;
                if parameter.1 != ParameterUse::Borrow {
                    return Err(reject());
                }
            }
            let effect = self.row(&method.effect.row(), &env)?;
            let mut requires_read_view_generation = contract_view;
            let mut requires_associated_generation = contract_view
                || !contract.associated_types.is_empty()
                || self.associated_type(method.result.ty, 0)?;
            requires_read_view_generation |= self.read_view_type(method.result.ty, 0)?;
            for parameter in &method.parameters {
                requires_associated_generation |= self.associated_type(parameter.ty, 0)?;
                requires_read_view_generation |= self.read_view_type(parameter.ty, 0)?;
            }
            let read_scope_parameters =
                self.copy_scope_parameters(&method.read_scope_parameters)?;
            let result = self.closed_result(method.result, result, &env, Ok)?;
            let read_scope_sources = self.closed_sources(&method.read_scope_sources, &env, Ok)?;
            self.budget
                .node::<((DeclarationReference, MethodId), Function)>()?;
            self.contract_methods.insert(
                (scope, method.id),
                Function {
                    is_task: matches!(
                        method.effect,
                        crate::platform::kernel::FunctionEffect::Task { .. }
                    ),
                    requires_associated_generation,
                    requires_read_view_generation,
                    implementation_templates: Vec::new(),
                    parameters,
                    result,
                    read_scope_parameters,
                    read_scope_sources,
                    effect,
                    roots,
                    targets: BTreeMap::new(),
                    invocations: BTreeMap::new(),
                    occurrences,
                    symbolic_targets: BTreeMap::new(),
                    associated_targets: BTreeSet::new(),
                    read_view_targets: BTreeSet::new(),
                },
            );
        }
        Ok(())
    }

    pub(super) fn symbolic_inventory(
        &mut self,
        scope: DeclarationReference,
    ) -> Result<(), ExecutionError> {
        let owner = self.declaration(scope, None)?;
        let (parameters, result, scope_parameters, scope_sources, effect, body) =
            match &owner.payload {
                DeclarationPayload::Function(function) => (
                    &function.parameters[..],
                    function.result,
                    &function.read_scope_parameters[..],
                    &function.read_scope_sources[..],
                    function.effect.row(),
                    Some(function.body),
                ),
                DeclarationPayload::External(function) => (
                    &function.parameters[..],
                    CallableResult {
                        ty: function.result,
                        relation: ResultRelation::Value,
                    },
                    &[][..],
                    &[][..],
                    EffectRow::default(),
                    None,
                ),
                _ => return Err(reject()),
            };
        let mut env = self.symbolic_environment(scope)?;
        let effect = self.row(&effect, &env)?;
        let mut roots = BTreeMap::new();
        let mut occurrences = BTreeMap::new();
        self.occurrence(
            &mut occurrences,
            scope.package,
            OwnerKey::Declaration(scope.declaration),
        )?;
        let result = self.root(
            &mut roots,
            OwnerKey::Declaration(scope.declaration),
            result.ty,
            &env,
        )?;
        self.budget
            .reserve::<(TypeObjectDigest, ParameterUse)>(parameters.len())?;
        let mut actual_parameters = Vec::new();
        let mut ids = BTreeSet::new();
        for id in parameters {
            if ids.contains(id) {
                return Err(reject());
            }
            self.budget
                .node::<crate::platform::semantic_id::ParameterId>()?;
            ids.insert(*id);
            let OwnerRecord::Parameter(parameter) =
                self.owner(scope.package, OwnerKey::Parameter(*id))?
            else {
                return Err(reject());
            };
            if parameter.parent != ParameterParent::Function(scope.declaration) {
                return Err(reject());
            }
            self.occurrence(&mut occurrences, scope.package, OwnerKey::Parameter(*id))?;
            env.generation = parameter.header.contract_version;
            let ty = self.root(&mut roots, OwnerKey::Parameter(*id), parameter.ty, &env)?;
            // Signature types have both their declaration and parameter occurrences.
            env.generation = owner.header.contract_version;
            self.root(
                &mut roots,
                OwnerKey::Declaration(scope.declaration),
                parameter.ty,
                &env,
            )?;
            actual_parameters.push((ty, parameter.use_mode));
        }
        let ordinal = |source| {
            parameters
                .iter()
                .position(|id| *id == source)
                .and_then(|position| u32::try_from(position).ok())
                .ok_or_else(reject)
        };
        let result = self.closed_result(
            match &owner.payload {
                DeclarationPayload::Function(function) => function.result,
                DeclarationPayload::External(function) => CallableResult {
                    ty: function.result,
                    relation: ResultRelation::Value,
                },
                _ => return Err(reject()),
            },
            result,
            &env,
            ordinal,
        )?;
        let read_scope_sources = self.closed_sources(scope_sources, &env, ordinal)?;
        let mut targets = BTreeMap::new();
        let mut invocations = BTreeMap::new();
        let mut symbolic_targets = BTreeMap::new();
        let mut associated_targets = BTreeSet::new();
        let mut read_view_targets = BTreeSet::new();
        if let Some(body) = body {
            self.body(
                scope.package,
                body,
                &mut env,
                BodyOutputs {
                    roots: &mut roots,
                    targets: &mut targets,
                    invocations: &mut invocations,
                    occurrences: &mut occurrences,
                    symbolic_targets: &mut symbolic_targets,
                    associated_targets: &mut associated_targets,
                    read_view_targets: &mut read_view_targets,
                },
            )?;
        }
        self.budget.node::<(DeclarationReference, Function)>()?;
        let implementation_templates = self.implementation_templates(scope)?;
        let requires_read_view_generation = self.read_view_owner(scope)?;
        let requires_associated_generation =
            self.associated_function(scope)? || requires_read_view_generation;
        if requires_associated_generation || requires_read_view_generation {
            self.associated_generation(
                scope,
                owner.header.contract_version,
                requires_read_view_generation,
            )?;
        }
        let read_scope_parameters = self.copy_scope_parameters(scope_parameters)?;
        self.symbolic.insert(
            scope,
            Function {
                is_task: matches!(&owner.payload, DeclarationPayload::Function(function) if matches!(function.effect, crate::platform::kernel::FunctionEffect::Task { .. })),
                requires_associated_generation,
                requires_read_view_generation,
                implementation_templates,
                parameters: actual_parameters,
                result,
                read_scope_parameters,
                read_scope_sources,
                effect,
                roots,
                targets,
                invocations,
                occurrences,
                symbolic_targets,
                associated_targets,
                read_view_targets,
            },
        );
        Ok(())
    }

    fn symbolic_witness_bindings(
        &mut self,
        scope: DeclarationReference,
        formals: &[ImplementationParameter],
        supplied: &[Arc<SymbolicSelection>],
        env: &mut Environment,
    ) -> Result<(), ExecutionError> {
        if formals.len() != supplied.len() || !env.symbolic_witnesses.is_empty() {
            return Err(reject());
        }
        let mut names = BTreeSet::new();
        for (formal, selected) in formals.iter().zip(supplied) {
            self.budget.tick()?;
            let TypeForm::TypeParameter { parameter } =
                self.types.get(&formal.self_type).ok_or_else(reject)?.form
            else {
                return Err(reject());
            };
            let OwnerRecord::TypeParameter(owner) =
                self.owner(scope.package, OwnerKey::TypeParameter(parameter))?
            else {
                return Err(reject());
            };
            if owner.declaration != scope.declaration
                || !owner.constraints.has_owned()
                || names.contains(&formal.name)
                || env.symbolic_witnesses.contains_key(&formal.id)
            {
                return Err(reject());
            }
            self.budget
                .node::<(ImplementationParameterId, Arc<SymbolicSelection>)>()?;
            self.budget.node::<&crate::platform::kernel::Name>()?;
            names.insert(&formal.name);
            env.symbolic_witnesses
                .insert(formal.id, Arc::clone(selected));
        }
        // Insert complete caller selections before interpreting any callee formal argument.
        // Their original Parameter coordinates and closed concrete members remain opaque.
        for (formal, selected) in formals.iter().zip(supplied) {
            self.budget.tick()?;
            if selected.contract != formal.contract
                || selected.self_type != self.ty(formal.self_type, env, 0)?
                || selected.arguments.len() != formal.type_arguments.len()
            {
                return Err(reject());
            }
            for (expected, actual) in formal.type_arguments.iter().zip(&selected.arguments) {
                if self.ty(*expected, env, 0)? != *actual {
                    return Err(reject());
                }
            }
        }
        Ok(())
    }

    fn symbolic_operand(
        &mut self,
        operand: &ImplementationOperand,
        env: &Environment,
        depth: usize,
    ) -> Result<Arc<SymbolicSelection>, ExecutionError> {
        self.budget.tick()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        match operand {
            ImplementationOperand::Parameter { scope, parameter } => {
                if env.scope != Some(*scope) || !env.lexical {
                    return Err(reject());
                }
                let selected = env
                    .symbolic_witnesses
                    .get(parameter)
                    .cloned()
                    .ok_or_else(reject)?;
                let view = self.symbolic_selection_read_view(&selected, depth)?;
                self.associated_generation(*scope, env.generation, view)?;
                Ok(selected)
            }
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                let owner = self.declaration(
                    *implementation,
                    env.scope.map(|scope| scope.package).or(env.source_package),
                )?;
                let DeclarationPayload::OwnedImplementation(scheme) = &owner.payload else {
                    return Err(reject());
                };
                self.inventory(*implementation, scheme)?;
                if scheme.type_parameters.len() != type_arguments.len()
                    || scheme.implementation_parameters.len() != implementations.len()
                {
                    return Err(reject());
                }
                let source = env.scope.ok_or_else(reject)?;
                let source_parameters = self.declaration(source, None)?.payload.type_parameters();
                let assumptions = self.assumptions(source, source_parameters.iter())?;
                let mut selected_env = Environment {
                    scope: Some(*implementation),
                    generation: owner.header.contract_version,
                    lexical: true,
                    symbolic: true,
                    ..Environment::default()
                };
                self.budget
                    .reserve::<TypeObjectDigest>(type_arguments.len())?;
                let mut actuals = Vec::new();
                for (formal, actual) in scheme.type_parameters.iter().zip(type_arguments) {
                    let actual = self.ty(*actual, env, 0)?;
                    let OwnerRecord::TypeParameter(parameter) =
                        self.owner(implementation.package, OwnerKey::TypeParameter(*formal))?
                    else {
                        return Err(reject());
                    };
                    if self.classify_scoped(actual, false, 0, &mut BTreeSet::new(), &assumptions)?
                        != parameter.constraints.has_owned()
                    {
                        return Err(reject());
                    }
                    self.budget.node::<(TypeParameterId, TypeObjectDigest)>()?;
                    if selected_env.ordinary.insert(*formal, actual).is_some() {
                        return Err(reject());
                    }
                    actuals.push(actual);
                }
                self.budget
                    .reserve::<Arc<SymbolicSelection>>(implementations.len())?;
                let mut prerequisites = Vec::new();
                for operand in implementations {
                    prerequisites.push(self.symbolic_operand(operand, env, depth + 1)?);
                }
                self.symbolic_witness_bindings(
                    *implementation,
                    &scheme.implementation_parameters,
                    &prerequisites,
                    &mut selected_env,
                )?;
                let self_type = self.ty(scheme.self_type, &selected_env, 0)?;
                self.budget
                    .reserve::<TypeObjectDigest>(scheme.type_arguments.len())?;
                let arguments = scheme
                    .type_arguments
                    .iter()
                    .map(|ty| self.ty(*ty, &selected_env, 0))
                    .collect::<Result<Vec<_>, _>>()?;
                self.budget
                    .reserve::<Member>(scheme.associated_types.len())?;
                let mut members = Vec::new();
                for binding in &scheme.associated_types {
                    let ty = self.ty(binding.ty(), &selected_env, 0)?;
                    if matches!(binding.value, AssociatedTypeBindingValue::Owned { .. })
                        && !self.classify_scoped(
                            ty,
                            false,
                            0,
                            &mut BTreeSet::new(),
                            &assumptions,
                        )?
                    {
                        return Err(reject());
                    }
                    members.push(Member {
                        member: binding.member,
                        template: binding.ty(),
                        ty,
                        scope_parameter: match &binding.value {
                            AssociatedTypeBindingValue::Owned { .. } => None,
                            AssociatedTypeBindingValue::ReadViewFamily {
                                scope_parameter, ..
                            } => Some(ReadScopeReference {
                                binder: ReadScopeBinder::AssociatedBinding {
                                    implementation: *implementation,
                                    member: binding.member,
                                },
                                parameter: scope_parameter.id,
                            }),
                        },
                    });
                }
                self.budget.reserve::<SymbolicSelection>(1)?;
                self.budget.reserve::<usize>(2)?;
                Ok(Arc::new(SymbolicSelection {
                    source: SymbolicSelectionSource::Concrete {
                        implementation: *implementation,
                        actuals,
                        prerequisites,
                        members,
                    },
                    contract: scheme.contract,
                    self_type,
                    arguments,
                }))
            }
        }
    }

    pub(super) fn symbolic_target(
        &mut self,
        operation: &ExpressionOperation,
        env: &Environment,
    ) -> Result<Option<SymbolicTarget>, ExecutionError> {
        let (function, types, implementations, effects, requirements, read_scope_arguments) =
            match operation {
                ExpressionOperation::Call {
                    function,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    read_scope_arguments,
                    ..
                }
                | ExpressionOperation::FunctionValue {
                    function,
                    type_arguments,
                    effect_arguments,
                    requirement_arguments,
                    read_scope_arguments,
                } => (
                    *function,
                    &type_arguments[..],
                    &[][..],
                    &effect_arguments[..],
                    &requirement_arguments[..],
                    &read_scope_arguments[..],
                ),
                ExpressionOperation::ImplementationCall {
                    function,
                    type_arguments,
                    implementations,
                    effect_arguments,
                    requirement_arguments,
                    read_scope_arguments,
                    ..
                } => (
                    *function,
                    &type_arguments[..],
                    &implementations[..],
                    &effect_arguments[..],
                    &requirement_arguments[..],
                    &read_scope_arguments[..],
                ),
                ExpressionOperation::MethodCall {
                    witness,
                    contract,
                    method,
                    read_scope_arguments,
                    ..
                } => {
                    let selected = self.symbolic_operand(witness, env, 0)?;
                    if selected.contract != *contract {
                        return Err(reject());
                    }
                    let owner = self.declaration(
                        *contract,
                        env.scope.map(|scope| scope.package).or(env.source_package),
                    )?;
                    let DeclarationPayload::OwnedContract(_) = &owner.payload else {
                        return Err(reject());
                    };
                    self.budget
                        .reserve::<ReadScopeArgument>(read_scope_arguments.len())?;
                    let scopes = read_scope_arguments
                        .iter()
                        .map(|scope| self.read_scope(*scope, env))
                        .collect::<Result<Vec<_>, _>>()?;
                    let mut signature =
                        self.scoped_method_signature(&selected, *method, &scopes)?;
                    signature.requires_read_view_generation |=
                        self.symbolic_selection_read_view(&selected, 0)?;
                    if signature.requires_associated_generation
                        || signature.requires_read_view_generation
                    {
                        self.call_generation(env, signature.requires_read_view_generation)?;
                    }
                    self.budget.reserve::<Arc<SymbolicSelection>>(1)?;
                    return Ok(Some(SymbolicTarget {
                        signature,
                        function: None,
                        method: Some((selected.contract, *method)),
                        types: Vec::new(),
                        witnesses: vec![selected],
                        effects: Vec::new(),
                        requirements: Vec::new(),
                        read_scope_arguments: scopes,
                    }));
                }
                _ => return Ok(None),
            };
        let owner = self.declaration(
            function,
            env.scope.map(|scope| scope.package).or(env.source_package),
        )?;
        let (formals, witnesses, effect_parameters, requirement_parameters) = match &owner.payload {
            DeclarationPayload::Function(function) => (
                &function.type_parameters[..],
                &function.implementation_parameters[..],
                function.effect_parameters.len(),
                function.requirement_parameters.len(),
            ),
            DeclarationPayload::External(function) => {
                (&function.type_parameters[..], &[][..], 0, 0)
            }
            _ => return Err(reject()),
        };
        if formals.len() != types.len()
            || witnesses.len() != implementations.len()
            || effect_parameters != effects.len()
            || requirement_parameters != requirements.len()
        {
            return Err(reject());
        }
        let source = env.scope.ok_or_else(reject)?;
        let source_parameters = self.declaration(source, None)?.payload.type_parameters();
        let assumptions = self.assumptions(source, source_parameters.iter())?;
        let mut target_env = Environment {
            scope: Some(function),
            generation: owner.header.contract_version,
            lexical: matches!(owner.payload, DeclarationPayload::Function(_)),
            symbolic: true,
            ..Environment::default()
        };
        self.budget.reserve::<TypeObjectDigest>(types.len())?;
        let mut actuals = Vec::new();
        for (formal, actual) in formals.iter().zip(types) {
            let actual = self.ty(*actual, env, 0)?;
            if self.view_type(actual)? {
                return Err(reject());
            }
            let OwnerRecord::TypeParameter(parameter) =
                self.owner(function.package, OwnerKey::TypeParameter(*formal))?
            else {
                return Err(reject());
            };
            if parameter.declaration != function.declaration
                || self.classify_scoped(actual, false, 0, &mut BTreeSet::new(), &assumptions)?
                    != parameter.constraints.has_owned()
            {
                return Err(reject());
            }
            self.budget.node::<(TypeParameterId, TypeObjectDigest)>()?;
            if target_env.ordinary.insert(*formal, actual).is_some() {
                return Err(reject());
            }
            actuals.push(actual);
        }
        self.budget
            .reserve::<Arc<SymbolicSelection>>(implementations.len())?;
        let mut supplied = Vec::new();
        for operand in implementations {
            supplied.push(self.symbolic_operand(operand, env, 0)?);
        }
        self.symbolic_witness_bindings(function, witnesses, &supplied, &mut target_env)?;
        self.budget.reserve::<EffectRow>(effects.len())?;
        let effects = effects
            .iter()
            .map(|row| self.row(row, env))
            .collect::<Result<Vec<_>, _>>()?;
        if let DeclarationPayload::Function(declaration) = &owner.payload {
            for (id, row) in declaration.effect_parameters.iter().zip(&effects) {
                self.budget.tick()?;
                self.budget
                    .node::<(EffectParameterReference, EffectRow)>()?;
                self.budget.row(row)?;
                if target_env
                    .symbolic_effects
                    .insert(
                        EffectParameterReference {
                            package: function.package,
                            parameter: *id,
                        },
                        row.clone(),
                    )
                    .is_some()
                {
                    return Err(reject());
                }
            }
        }
        self.budget
            .reserve::<RequirementOperand>(requirements.len())?;
        let mut closed_requirements = Vec::new();
        for operand in requirements {
            closed_requirements.push(self.symbolic_requirement(*operand, env)?);
        }
        if let DeclarationPayload::Function(declaration) = &owner.payload {
            for (id, actual) in declaration
                .requirement_parameters
                .iter()
                .zip(&closed_requirements)
            {
                self.budget.tick()?;
                self.budget
                    .node::<(RequirementParameterReference, RequirementOperand)>()?;
                if target_env
                    .symbolic_requirements
                    .insert(
                        RequirementParameterReference {
                            package: function.package,
                            parameter: *id,
                        },
                        *actual,
                    )
                    .is_some()
                {
                    return Err(reject());
                }
            }
        }
        self.budget
            .reserve::<ReadScopeArgument>(read_scope_arguments.len())?;
        let read_scope_arguments = read_scope_arguments
            .iter()
            .map(|scope| self.read_scope(*scope, env))
            .collect::<Result<Vec<_>, _>>()?;
        let scope_parameters = match &owner.payload {
            DeclarationPayload::Function(function) => &function.read_scope_parameters[..],
            _ => &[][..],
        };
        self.scope_bindings(
            ReadScopeBinder::Declaration(function),
            scope_parameters,
            &read_scope_arguments,
            &mut target_env,
        )?;
        let mut signature = self.scoped_signature(function, &mut target_env)?;
        signature.requires_associated_generation |=
            self.associated_operation(operation, function)?;
        signature.requires_read_view_generation |= self.read_view_operation(operation, function)?;
        for selected in &supplied {
            signature.requires_read_view_generation |=
                self.symbolic_selection_read_view(selected, 0)?;
        }
        if signature.requires_associated_generation || signature.requires_read_view_generation {
            self.call_generation(env, signature.requires_read_view_generation)?;
        }
        Ok(Some(SymbolicTarget {
            signature,
            function: Some(function),
            method: None,
            types: actuals,
            witnesses: supplied,
            effects,
            requirements: closed_requirements,
            read_scope_arguments,
        }))
    }

    pub(super) fn executable_inventory(
        &mut self,
        package: PackageId,
        key: OwnerKey,
        owner: &OwnerRecord,
    ) -> Result<(), ExecutionError> {
        let mut env = match key {
            OwnerKey::Declaration(id) => self.symbolic_environment(DeclarationReference {
                package,
                declaration: id,
            })?,
            _ => Environment {
                generation: owner.header().contract_version,
                ..Environment::default()
            },
        };
        // Executable declarations have no lexical associated authority. Their bodies
        // close concrete applications; nominal templates retain scoped ordinary formals.
        env.symbolic = matches!(owner, OwnerRecord::Declaration(declaration) if matches!(declaration.payload, DeclarationPayload::Record { .. } | DeclarationPayload::OwnedRecord { .. } | DeclarationPayload::Variant { .. } | DeclarationPayload::ReadView(_)));
        let mut roots = BTreeMap::new();
        let mut occurrences = BTreeMap::new();
        self.occurrence(&mut occurrences, package, key)?;
        self.budget
            .reserve::<TypeObjectDigest>(owner.type_roots().len())?;
        for ty in owner.type_roots() {
            self.root(&mut roots, key, ty, &env)?;
        }
        if let OwnerRecord::Declaration(declaration) = owner {
            if let DeclarationPayload::ReadView(view) = &declaration.payload {
                self.current(
                    env.scope.ok_or_else(reject)?,
                    declaration.header.contract_version,
                )?;
                self.scope_parameters(ReadScopeBinder::Declaration(env.scope.ok_or_else(reject)?))?;
                for field in &view.fields {
                    self.budget.tick()?;
                    if let crate::platform::kernel::ReadViewFieldKind::ReadReference {
                        read_scope,
                    } = field.kind
                    {
                        self.read_scope(read_scope, &env)?;
                    }
                }
            }
            let (fields, cases): (&[_], &[_]) = match &declaration.payload {
                DeclarationPayload::Record { fields, .. }
                | DeclarationPayload::OwnedRecord { fields, .. } => (fields, &[]),
                DeclarationPayload::Variant { cases, .. } => (&[], cases),
                _ => (&[], &[]),
            };
            for field in fields {
                let child_key = OwnerKey::Field(*field);
                let OwnerRecord::Field(child) = self.owner(package, child_key)? else {
                    return Err(reject());
                };
                if Some(child.declaration) != env.scope.map(|scope| scope.declaration) {
                    return Err(reject());
                }
                self.occurrence(&mut occurrences, package, child_key)?;
                env.generation = child.header.contract_version;
                self.root(&mut roots, child_key, child.ty, &env)?;
            }
            for case in cases {
                let child_key = OwnerKey::Case(*case);
                let OwnerRecord::Case(child) = self.owner(package, child_key)? else {
                    return Err(reject());
                };
                if Some(child.declaration) != env.scope.map(|scope| scope.declaration) {
                    return Err(reject());
                }
                self.occurrence(&mut occurrences, package, child_key)?;
                env.generation = child.header.contract_version;
                if let Some(ty) = child.payload {
                    self.root(&mut roots, child_key, ty, &env)?;
                }
            }
            env.generation = declaration.header.contract_version;
        }
        let mut targets = BTreeMap::new();
        let mut invocations = BTreeMap::new();
        let mut symbolic_targets = BTreeMap::new();
        let mut associated_targets = BTreeSet::new();
        let mut read_view_targets = BTreeSet::new();
        self.budget
            .reserve::<ExpressionId>(owner.expression_roots().len())?;
        for root in owner.expression_roots() {
            self.body(
                package,
                root,
                &mut env,
                BodyOutputs {
                    roots: &mut roots,
                    targets: &mut targets,
                    invocations: &mut invocations,
                    occurrences: &mut occurrences,
                    symbolic_targets: &mut symbolic_targets,
                    associated_targets: &mut associated_targets,
                    read_view_targets: &mut read_view_targets,
                },
            )?;
        }
        let result = self.intern_type(TypeForm::Unit)?;
        self.budget.node::<((PackageId, OwnerKey), Function)>()?;
        self.executables.insert(
            (package, key),
            Function {
                is_task: false,
                requires_associated_generation: matches!(owner, OwnerRecord::Declaration(declaration) if declaration.payload.has_read_view_metadata()),
                requires_read_view_generation: matches!(owner, OwnerRecord::Declaration(declaration) if declaration.payload.has_read_view_metadata()),
                implementation_templates: Vec::new(),
                parameters: Vec::new(),
                result: CallableResult {
                    ty: result,
                    relation: ResultRelation::Value,
                },
                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
                effect: EffectRow::default(),
                roots,
                targets,
                invocations,
                occurrences,
                symbolic_targets,
                associated_targets,
                read_view_targets,
            },
        );
        Ok(())
    }
}
