//! Scoped canonical signatures. Caller actuals are inserted once and remain opaque.
use super::*;

impl<'a> Derivation<'a> {
    pub(super) fn read_view_operation(
        &mut self,
        operation: &ExpressionOperation,
        function: DeclarationReference,
    ) -> Result<bool, ExecutionError> {
        let mut view =
            self.read_view_owner(function)? || !operation.read_scope_arguments().is_empty();
        let types = match operation {
            ExpressionOperation::Call { type_arguments, .. }
            | ExpressionOperation::FunctionValue { type_arguments, .. }
            | ExpressionOperation::ImplementationCall { type_arguments, .. } => &type_arguments[..],
            ExpressionOperation::MethodCall { contract, .. } => {
                view |= self.read_view_owner(*contract)?;
                &[][..]
            }
            _ => return Err(reject()),
        };
        for ty in types {
            view |= self.read_view_type(*ty, 0)?;
        }
        Ok(view)
    }

    pub(super) fn associated_operation(
        &mut self,
        operation: &ExpressionOperation,
        function: DeclarationReference,
    ) -> Result<bool, ExecutionError> {
        let mut associated = self.associated_function(function)?;
        let types = match operation {
            ExpressionOperation::Call { type_arguments, .. }
            | ExpressionOperation::FunctionValue { type_arguments, .. }
            | ExpressionOperation::ImplementationCall { type_arguments, .. } => &type_arguments[..],
            ExpressionOperation::MethodCall { contract, .. } => {
                let DeclarationPayload::OwnedContract(contract) =
                    &self.declaration(*contract, None)?.payload
                else {
                    return Err(reject());
                };
                associated |= !contract.associated_types.is_empty();
                &[][..]
            }
            _ => return Err(reject()),
        };
        for ty in types {
            associated |= self.associated_type(*ty, 0)?;
        }
        Ok(associated)
    }
    pub(super) fn symbolic_member(
        &mut self,
        selected: &SymbolicSelection,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        self.budget.tick()?;
        if selected.contract != contract {
            return Err(reject());
        }
        match &selected.source {
            SymbolicSelectionSource::Parameter(scope, parameter) => {
                let ty = self.intern_type(TypeForm::AssociatedWitnessProjection {
                    scope: *scope,
                    parameter: *parameter,
                    contract,
                    member,
                })?;
                self.associated_bound(ty)?;
                Ok(ty)
            }
            SymbolicSelectionSource::Concrete { members, .. } => {
                let mut found = None;
                for candidate in members {
                    self.budget.tick()?;
                    if candidate.member == member && found.replace(candidate.ty).is_some() {
                        return Err(reject());
                    }
                }
                found.ok_or_else(reject)
            }
        }
    }

    pub(super) fn symbolic_view_member(
        &mut self,
        selected: &SymbolicSelection,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
        read_scope: ReadScopeArgument,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        self.budget.tick()?;
        if selected.contract != contract {
            return Err(reject());
        }
        match &selected.source {
            SymbolicSelectionSource::Parameter(scope, parameter) => {
                let ty = self.intern_type(TypeForm::AssociatedReadViewProjection {
                    scope: *scope,
                    parameter: *parameter,
                    contract,
                    member,
                    read_scope,
                })?;
                self.associated_bound(ty)?;
                Ok(ty)
            }
            SymbolicSelectionSource::Concrete { members, .. } => self.apply_view_member(
                members
                    .iter()
                    .find(|candidate| candidate.member == member)
                    .ok_or_else(reject)?,
                read_scope,
            ),
        }
    }

    fn authored_row(
        &mut self,
        effect: &crate::platform::kernel::FunctionEffect,
    ) -> Result<EffectRow, ExecutionError> {
        if let crate::platform::kernel::FunctionEffect::Task {
            requirements,
            effect_parameters,
        } = effect
        {
            self.budget
                .reserve::<RequirementOperand>(requirements.len())?;
            self.budget
                .reserve::<EffectParameterReference>(effect_parameters.len())?;
        }
        Ok(effect.row())
    }

    pub(super) fn scope_parameters(
        &mut self,
        binder: ReadScopeBinder,
    ) -> Result<&'a [ReadScopeParameter], ExecutionError> {
        let owner = self.declaration(binder.declaration(), None)?;
        let parameters = match (binder, &owner.payload) {
            (ReadScopeBinder::Declaration(_), DeclarationPayload::Function(function)) => {
                &function.read_scope_parameters[..]
            }
            // Ordinary external applications have an exact empty scope inventory.
            // They never acquire a lexical binder or accept a supplied read scope.
            (ReadScopeBinder::Declaration(_), DeclarationPayload::External(_)) => &[],
            (ReadScopeBinder::Declaration(_), DeclarationPayload::ReadView(view)) => {
                std::slice::from_ref(&view.scope_parameter)
            }
            (
                ReadScopeBinder::ContractMethod { method, .. },
                DeclarationPayload::OwnedContract(contract),
            ) => {
                &contract
                    .methods
                    .iter()
                    .find(|candidate| candidate.id == method)
                    .ok_or_else(reject)?
                    .read_scope_parameters[..]
            }
            (
                ReadScopeBinder::AssociatedMember { member, .. },
                DeclarationPayload::OwnedContract(contract),
            ) => match &contract
                .associated_types
                .iter()
                .find(|candidate| candidate.id == member)
                .ok_or_else(reject)?
                .bound
            {
                AssociatedTypeBound::ReadViewFamily { scope_parameter } => {
                    std::slice::from_ref(scope_parameter)
                }
                AssociatedTypeBound::Owned => return Err(reject()),
            },
            (
                ReadScopeBinder::AssociatedBinding { member, .. },
                DeclarationPayload::OwnedImplementation(scheme),
            ) => match &scheme
                .associated_types
                .iter()
                .find(|candidate| candidate.member == member)
                .ok_or_else(reject)?
                .value
            {
                AssociatedTypeBindingValue::ReadViewFamily {
                    scope_parameter, ..
                } => std::slice::from_ref(scope_parameter),
                AssociatedTypeBindingValue::Owned { .. } => return Err(reject()),
            },
            _ => return Err(reject()),
        };
        if !parameters.is_empty() {
            self.current(binder.declaration(), owner.header.contract_version)?;
        }
        Ok(parameters)
    }

    pub(super) fn read_scope(
        &mut self,
        scope: ReadScopeArgument,
        env: &Environment,
    ) -> Result<ReadScopeArgument, ExecutionError> {
        self.budget.tick()?;
        match scope {
            ReadScopeArgument::Parameter(reference) => {
                let parameters = self.scope_parameters(reference.binder)?;
                let mut found = 0;
                for parameter in parameters {
                    self.budget.tick()?;
                    found += usize::from(parameter.id == reference.parameter);
                }
                if found != 1 {
                    return Err(reject());
                }
                if let Some(actual) = env.read_scopes.get(&reference) {
                    return Ok(*actual);
                }
                if !env.symbolic && env.scope != Some(reference.binder.declaration()) {
                    return Err(reject());
                }
            }
            ReadScopeArgument::Lexical { callable, binding } => {
                if env.scope != Some(callable) {
                    return Err(reject());
                }
                let OwnerRecord::Binding(owner) =
                    self.owner(callable.package, OwnerKey::Binding(binding))?
                else {
                    return Err(reject());
                };
                if !matches!(
                    owner.kind,
                    crate::platform::kernel::BindingKind::OwnedBorrow
                        | crate::platform::kernel::BindingKind::ReadView
                        | crate::platform::kernel::BindingKind::ReadViewBorrow
                ) {
                    return Err(reject());
                }
            }
        }
        Ok(scope)
    }

    pub(super) fn copy_scope_parameters(
        &mut self,
        parameters: &[ReadScopeParameter],
    ) -> Result<Vec<ReadScopeParameter>, ExecutionError> {
        self.budget
            .reserve::<ReadScopeParameter>(parameters.len())?;
        for parameter in parameters {
            self.budget.tick()?;
            self.budget.reserve::<u8>(parameter.name.as_str().len())?;
        }
        Ok(parameters.to_vec())
    }

    pub(super) fn scope_bindings(
        &mut self,
        binder: ReadScopeBinder,
        parameters: &[ReadScopeParameter],
        arguments: &[ReadScopeArgument],
        env: &mut Environment,
    ) -> Result<(), ExecutionError> {
        if self.scope_parameters(binder)? != parameters {
            return Err(reject());
        }
        if parameters.len() != arguments.len() {
            return Err(reject());
        }
        let mut names = BTreeSet::new();
        for (formal, actual) in parameters.iter().zip(arguments) {
            self.budget.tick()?;
            self.budget
                .node::<(ReadScopeReference, ReadScopeArgument)>()?;
            self.budget.node::<crate::platform::kernel::Name>()?;
            self.budget.reserve::<u8>(formal.name.as_str().len())?;
            if !names.insert(formal.name.clone())
                || env
                    .read_scopes
                    .insert(
                        ReadScopeReference {
                            binder,
                            parameter: formal.id,
                        },
                        *actual,
                    )
                    .is_some()
            {
                return Err(reject());
            }
        }
        Ok(())
    }

    pub(super) fn closed_result<S: Copy>(
        &mut self,
        result: CallableResult<S>,
        ty: TypeObjectDigest,
        env: &Environment,
        mut source: impl FnMut(S) -> Result<u32, ExecutionError>,
    ) -> Result<CallableResult<u32>, ExecutionError> {
        let relation = match result.relation {
            ResultRelation::Value => ResultRelation::Value,
            ResultRelation::ReadFrom { source: value } => ResultRelation::ReadFrom {
                source: source(value)?,
            },
            ResultRelation::ViewFrom {
                source: value,
                scope,
            } => ResultRelation::ViewFrom {
                source: source(value)?,
                scope: self.read_scope(scope, env)?,
            },
        };
        Ok(CallableResult { ty, relation })
    }

    pub(super) fn closed_sources<S: Copy>(
        &mut self,
        sources: &[ReadScopeSource<S>],
        env: &Environment,
        mut source: impl FnMut(S) -> Result<u32, ExecutionError>,
    ) -> Result<Vec<ReadScopeSource<u32>>, ExecutionError> {
        self.budget.reserve::<ReadScopeSource<u32>>(sources.len())?;
        sources
            .iter()
            .map(|entry| {
                Ok(ReadScopeSource {
                    source: source(entry.source)?,
                    scope: self.read_scope(entry.scope, env)?,
                })
            })
            .collect()
    }

    pub(super) fn scoped_signature(
        &mut self,
        scope: DeclarationReference,
        env: &mut Environment,
    ) -> Result<ScopedSignature, ExecutionError> {
        let owner = self.declaration(scope, None)?;
        let (parameters, result, scope_parameters, sources, is_task, row) = match &owner.payload {
            DeclarationPayload::Function(function) => (
                &function.parameters[..],
                function.result,
                &function.read_scope_parameters[..],
                &function.read_scope_sources[..],
                matches!(
                    function.effect,
                    crate::platform::kernel::FunctionEffect::Task { .. }
                ),
                self.authored_row(&function.effect)?,
            ),
            DeclarationPayload::External(function) => (
                &function.parameters[..],
                CallableResult {
                    ty: function.result,
                    relation: ResultRelation::Value,
                },
                &[][..],
                &[][..],
                false,
                EffectRow::default(),
            ),
            _ => return Err(reject()),
        };
        let effect = self.row(&row, env)?;
        env.generation = owner.header.contract_version;
        let result_ty = self.ty(result.ty, env, 0)?;
        let ordinal = |source| {
            parameters
                .iter()
                .position(|candidate| *candidate == source)
                .and_then(|index| u32::try_from(index).ok())
                .ok_or_else(reject)
        };
        let result = self.closed_result(result, result_ty, env, ordinal)?;
        let read_scope_sources = self.closed_sources(sources, env, ordinal)?;
        self.budget
            .reserve::<ReadScopeParameter>(scope_parameters.len())?;
        let read_scope_parameters = self.copy_scope_parameters(scope_parameters)?;
        self.budget
            .reserve::<(TypeObjectDigest, ParameterUse)>(parameters.len())?;
        let mut closed = Vec::new();
        let mut ids = BTreeSet::new();
        for id in parameters {
            self.budget.tick()?;
            self.budget
                .node::<crate::platform::semantic_id::ParameterId>()?;
            if !ids.insert(*id) {
                return Err(reject());
            }
            let OwnerRecord::Parameter(parameter) =
                self.owner(scope.package, OwnerKey::Parameter(*id))?
            else {
                return Err(reject());
            };
            if parameter.parent != ParameterParent::Function(scope.declaration) {
                return Err(reject());
            }
            env.generation = parameter.header.contract_version;
            let ty = self.ty(parameter.ty, env, 0)?;
            env.generation = owner.header.contract_version;
            if self.ty(parameter.ty, env, 0)? != ty {
                return Err(reject());
            }
            closed.push((ty, parameter.use_mode));
        }
        if let Some(position) = result.relation.source() {
            let parameter = closed.get(*position as usize).ok_or_else(reject)?;
            if is_task || parameter.1 != ParameterUse::Borrow {
                return Err(reject());
            }
        }
        let mut requires_associated_generation = !read_scope_parameters.is_empty()
            || self.associated_function(scope)?
            || self.associated_type(result.ty, 0)?;
        let mut requires_read_view_generation =
            self.read_view_owner(scope)? || self.read_view_type(result.ty, 0)?;
        for (ty, _) in &closed {
            requires_associated_generation |= self.associated_type(*ty, 0)?;
            requires_read_view_generation |= self.read_view_type(*ty, 0)?;
        }
        if requires_associated_generation || requires_read_view_generation {
            self.associated_generation(
                scope,
                owner.header.contract_version,
                requires_read_view_generation,
            )?;
        }
        Ok(ScopedSignature {
            parameters: closed,
            result,
            read_scope_parameters,
            read_scope_sources,
            effect,
            is_task,
            requires_associated_generation,
            requires_read_view_generation,
        })
    }

    pub(super) fn scoped_method_signature(
        &mut self,
        selected: &Arc<SymbolicSelection>,
        method: MethodId,
        read_scope_arguments: &[ReadScopeArgument],
    ) -> Result<ScopedSignature, ExecutionError> {
        let owner = self.declaration(selected.contract, None)?;
        let DeclarationPayload::OwnedContract(contract) = &owner.payload else {
            return Err(reject());
        };
        let mut found = None;
        for candidate in &contract.methods {
            self.budget.tick()?;
            if candidate.id == method && found.replace(candidate).is_some() {
                return Err(reject());
            }
        }
        let method = found.ok_or_else(reject)?;
        if selected.arguments.len() != contract.type_parameters.len() {
            return Err(reject());
        }
        let mut env = Environment {
            scope: Some(selected.contract),
            generation: owner.header.contract_version,
            symbolic: true,
            symbolic_contract: Some(Arc::clone(selected)),
            ..Environment::default()
        };
        self.scope_bindings(
            ReadScopeBinder::ContractMethod {
                contract: selected.contract,
                method: method.id,
            },
            &method.read_scope_parameters,
            read_scope_arguments,
            &mut env,
        )?;
        self.budget.node::<(TypeParameterId, TypeObjectDigest)>()?;
        env.ordinary
            .insert(contract.self_parameter, selected.self_type);
        for (formal, actual) in contract.type_parameters.iter().zip(&selected.arguments) {
            self.budget.tick()?;
            self.budget.node::<(TypeParameterId, TypeObjectDigest)>()?;
            if env.ordinary.insert(*formal, *actual).is_some() {
                return Err(reject());
            }
        }
        let row = self.authored_row(&method.effect)?;
        let effect = self.row(&row, &env)?;
        let mut requires_associated_generation = !method.read_scope_parameters.is_empty()
            || !contract.associated_types.is_empty()
            || self.associated_type(method.result.ty, 0)?;
        let mut requires_read_view_generation = self.read_view_contract(selected.contract)?
            || !read_scope_arguments.is_empty()
            || self.read_view_type(method.result.ty, 0)?;
        let result_ty = self.ty(method.result.ty, &env, 0)?;
        let result = self.closed_result(method.result, result_ty, &env, Ok)?;
        let read_scope_sources = self.closed_sources(&method.read_scope_sources, &env, Ok)?;
        self.budget
            .reserve::<(TypeObjectDigest, ParameterUse)>(method.parameters.len())?;
        let mut parameters = Vec::new();
        for parameter in &method.parameters {
            self.budget.tick()?;
            requires_associated_generation |= self.associated_type(parameter.ty, 0)?;
            requires_read_view_generation |= self.read_view_type(parameter.ty, 0)?;
            let ty = self.ty(parameter.ty, &env, 0)?;
            requires_associated_generation |= self.associated_type(ty, 0)?;
            requires_read_view_generation |= self.read_view_type(ty, 0)?;
            parameters.push((ty, parameter.use_mode));
        }
        requires_associated_generation |= self.associated_type(result.ty, 0)?;
        requires_read_view_generation |= self.read_view_type(result.ty, 0)?;
        if requires_associated_generation || requires_read_view_generation {
            self.associated_generation(
                selected.contract,
                owner.header.contract_version,
                requires_read_view_generation,
            )?;
        }
        Ok(ScopedSignature {
            parameters,
            result,
            read_scope_parameters: self.copy_scope_parameters(&method.read_scope_parameters)?,
            read_scope_sources,
            effect,
            is_task: matches!(
                method.effect,
                crate::platform::kernel::FunctionEffect::Task { .. }
            ),
            requires_associated_generation,
            requires_read_view_generation,
        })
    }

    pub(super) fn compare_scoped_method(
        &mut self,
        expected: &ScopedSignature,
        target: &SymbolicTarget,
    ) -> Result<(), ExecutionError> {
        self.budget.tick()?;
        let actual = &target.signature;
        if expected.result != actual.result
            || expected.read_scope_sources != actual.read_scope_sources
            || expected.is_task != actual.is_task
            || expected.parameters.len() != actual.parameters.len()
        {
            return Err(reject());
        }
        for (expected, actual) in expected.parameters.iter().zip(&actual.parameters) {
            self.budget.tick()?;
            if expected != actual {
                return Err(reject());
            }
        }
        for _ in expected
            .effect
            .requirements
            .iter()
            .chain(&actual.effect.requirements)
        {
            self.budget.tick()?;
        }
        for _ in expected
            .effect
            .parameters
            .iter()
            .chain(&actual.effect.parameters)
        {
            self.budget.tick()?;
        }
        if expected.effect != actual.effect {
            return Err(reject());
        }
        let scope = target.function.ok_or_else(reject)?;
        let DeclarationPayload::Function(function) = &self.declaration(scope, None)?.payload else {
            return Err(reject());
        };
        for id in &function.parameters {
            let OwnerRecord::Parameter(parameter) =
                self.owner(scope.package, OwnerKey::Parameter(*id))?
            else {
                return Err(reject());
            };
            if parameter.resource_requirement.is_some() {
                return Err(reject());
            }
        }
        Ok(())
    }
}
