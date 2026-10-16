//! Complete associated occurrence closure from raw canonical ownership. This
//! pass establishes authority, not affine flow or callable compatibility. Those
//! are checked separately by the existing symbolic memory/witness oracles.
use super::*;
use crate::platform::semantic_id::BindingId;

#[derive(Clone, Copy)]
enum Parent {
    Owner(OwnerKey),
    Expression(ExpressionId),
}

#[derive(Default)]
pub(super) struct Scopes {
    parents: BTreeMap<ExpressionId, Vec<Parent>>,
    bindings: BTreeMap<BindingId, Vec<ExpressionId>>,
}

impl Scopes {
    pub(super) fn loan_generation(
        &self,
        resolver: &mut Resolver<'_>,
        binding: BindingId,
    ) -> Result<()> {
        resolver.step(0)?;
        let [container] = self
            .bindings
            .get(&binding)
            .ok_or(Failure::Authority)?
            .as_slice()
        else {
            return Err(Failure::Authority);
        };
        let record = match resolver
            .source
            .owners
            .get(&OwnerKey::Expression(*container))
        {
            Some(OwnerRecord::Expression(e)) => e,
            _ => return Err(Failure::Missing),
        };
        match &record.operation {
            ExpressionOperation::BorrowOwnedField { .. }
            | ExpressionOperation::MatchBorrowedOwned { .. }
            | ExpressionOperation::BorrowOwnedItem { .. }
            | ExpressionOperation::BorrowCall { .. }
            | ExpressionOperation::BorrowOwnedRecordField { .. }
            | ExpressionOperation::BorrowReadView { .. }
            | ExpressionOperation::BorrowReadViewField { .. }
            | ExpressionOperation::Let { .. } => {}
            _ => return Err(Failure::Authority),
        };
        if !contract::supported_graph_contract(record.contract_version) {
            return Err(Failure::Generation);
        }
        Ok(())
    }
    pub(super) fn build(resolver: &mut Resolver<'_>) -> Result<Self> {
        let mut scopes = Self::default();
        for (key, owner) in &resolver.source.owners {
            resolver.step(0)?;
            for root in owner.expression_roots() {
                resolver.reserve_entries(1 + usize::from(!scopes.parents.contains_key(&root)))?;
                scopes
                    .parents
                    .entry(root)
                    .or_default()
                    .push(Parent::Owner(*key));
            }
            if let OwnerRecord::Expression(expression) = owner {
                for child in expression.children() {
                    resolver.reserve_entries(
                        1 + usize::from(!scopes.parents.contains_key(&child.expression)),
                    )?;
                    scopes
                        .parents
                        .entry(child.expression)
                        .or_default()
                        .push(Parent::Expression(expression.id));
                }
                let count = match &expression.operation {
                    ExpressionOperation::Let { bindings, .. } => bindings.len(),
                    ExpressionOperation::BorrowOwnedField { .. }
                    | ExpressionOperation::BorrowOwnedRecordField { .. }
                    | ExpressionOperation::BorrowOwnedItem { .. }
                    | ExpressionOperation::BorrowCall { .. }
                    | ExpressionOperation::BorrowReadView { .. }
                    | ExpressionOperation::BorrowReadViewField { .. }
                    | ExpressionOperation::Transaction { .. }
                    | ExpressionOperation::TransactionOutcome { .. } => 1,
                    ExpressionOperation::UnpackOwned { fields, .. } => fields.len(),
                    ExpressionOperation::UnpackOwnedRecord { fields, .. } => fields.len(),
                    ExpressionOperation::MatchOwned { arms, .. }
                    | ExpressionOperation::MatchBorrowedOwned { arms, .. } => arms.len(),
                    ExpressionOperation::Match { arms, .. } => arms
                        .iter()
                        .filter(|arm| arm.payload_binding.is_some())
                        .count(),
                    _ => 0,
                };
                resolver.reserve_entries(count)?;
                let mut bindings = Vec::with_capacity(count);
                match &expression.operation {
                    ExpressionOperation::Let { bindings: ids, .. } => bindings.extend(ids),
                    ExpressionOperation::BorrowOwnedField { binding, .. }
                    | ExpressionOperation::BorrowOwnedRecordField { binding, .. }
                    | ExpressionOperation::BorrowOwnedItem { binding, .. }
                    | ExpressionOperation::BorrowCall { binding, .. }
                    | ExpressionOperation::BorrowReadView { binding, .. }
                    | ExpressionOperation::BorrowReadViewField { binding, .. }
                    | ExpressionOperation::Transaction { binding, .. }
                    | ExpressionOperation::TransactionOutcome { binding, .. } => {
                        bindings.push(binding)
                    }
                    ExpressionOperation::UnpackOwned { fields, .. } => {
                        bindings.extend(fields.iter().map(|f| &f.binding))
                    }
                    ExpressionOperation::UnpackOwnedRecord { fields, .. } => {
                        bindings.extend(fields.iter().map(|f| &f.binding))
                    }
                    ExpressionOperation::MatchOwned { arms, .. }
                    | ExpressionOperation::MatchBorrowedOwned { arms, .. } => {
                        bindings.extend(arms.iter().map(|a| &a.binding))
                    }
                    ExpressionOperation::Match { arms, .. } => {
                        bindings.extend(arms.iter().filter_map(|a| a.payload_binding.as_ref()))
                    }
                    _ => {}
                }
                for binding in bindings {
                    resolver
                        .reserve_entries(1 + usize::from(!scopes.bindings.contains_key(binding)))?;
                    scopes
                        .bindings
                        .entry(*binding)
                        .or_default()
                        .push(expression.id);
                }
            }
        }
        Ok(scopes)
    }

    pub(super) fn expression(
        &self,
        resolver: &mut Resolver<'_>,
        id: ExpressionId,
        depth: usize,
    ) -> Result<Option<DeclarationReference>> {
        if depth > contract::MAXIMUM_EXPRESSION_DEPTH {
            return Err(Failure::Capacity);
        }
        resolver.step(0)?;
        let [parent] = self.parents.get(&id).ok_or(Failure::Authority)?.as_slice() else {
            return Err(Failure::Authority);
        };
        match parent {
            Parent::Expression(parent) => self.expression(resolver, *parent, depth + 1),
            Parent::Owner(OwnerKey::Binding(binding)) => {
                self.binding(resolver, *binding, depth + 1)
            }
            Parent::Owner(OwnerKey::Declaration(id)) => {
                match resolver.source.owners.get(&OwnerKey::Declaration(*id)) {
                    Some(OwnerRecord::Declaration(_)) => Ok(Some(DeclarationReference {
                        package: resolver.source.root.package_id,
                        declaration: *id,
                    })),
                    _ => Err(Failure::Authority),
                }
            }
            Parent::Owner(OwnerKey::Port(port)) => {
                match resolver.source.owners.get(&OwnerKey::Port(*port)) {
                    Some(OwnerRecord::Port(p)) => Ok(Some(DeclarationReference {
                        package: resolver.source.root.package_id,
                        declaration: p.declaration,
                    })),
                    _ => Err(Failure::Authority),
                }
            }
            _ => Err(Failure::Authority),
        }
    }

    pub(super) fn binding(
        &self,
        resolver: &mut Resolver<'_>,
        id: BindingId,
        depth: usize,
    ) -> Result<Option<DeclarationReference>> {
        if depth > contract::MAXIMUM_EXPRESSION_DEPTH {
            return Err(Failure::Capacity);
        }
        resolver.step(0)?;
        let [container] = self.bindings.get(&id).ok_or(Failure::Authority)?.as_slice() else {
            return Err(Failure::Authority);
        };
        self.expression(resolver, *container, depth + 1)
    }
}

impl Resolver<'_> {
    fn context(&self, scope: Option<DeclarationReference>, representation: bool) -> Context {
        let Some(scope) = scope else {
            return Context::Forbidden;
        };
        if representation {
            return Context::Forbidden;
        }
        let oracle = Oracle(self.source, Some(scope));
        if oracle.contract(scope).is_some() {
            Context::Contract(scope)
        } else if oracle.function(scope).is_some() || oracle.implementation(scope).is_some() {
            Context::Lexical(scope)
        } else {
            Context::Forbidden
        }
    }

    fn occurrence_for(
        &self,
        package: PackageId,
        key: OwnerKey,
        scope: Option<DeclarationReference>,
        representation: bool,
    ) -> Occurrence {
        Occurrence {
            parameter_scope: scope,
            context: self.context(scope, representation),
            consumer: (package, key),
        }
    }

    fn local_scope(
        &mut self,
        scopes: &Scopes,
        key: OwnerKey,
        owner: &OwnerRecord,
    ) -> Result<Option<DeclarationReference>> {
        let declaration = match owner {
            OwnerRecord::Declaration(_) => {
                let OwnerKey::Declaration(id) = key else {
                    return Err(Failure::Authority);
                };
                Some(id)
            }
            OwnerRecord::TypeParameter(p) => Some(p.declaration),
            OwnerRecord::Field(f) => Some(f.declaration),
            OwnerRecord::Case(c) => Some(c.declaration),
            OwnerRecord::Parameter(p) => match p.parent {
                ParameterParent::Function(id) => {
                    let scope = DeclarationReference {
                        package: self.source.root.package_id,
                        declaration: id,
                    };
                    if p.header.kind != OwnerKind::Parameter
                        || Oracle(self.source, Some(scope))
                            .signature(scope)
                            .is_none_or(|s| !s.0.iter().any(|p| p.header.owner == key))
                    {
                        return Err(Failure::Authority);
                    }
                    Some(id)
                }
                ParameterParent::Operation(_) => None,
            },
            OwnerRecord::Port(p) => Some(p.declaration),
            OwnerRecord::Expression(e) => return scopes.expression(self, e.id, 0),
            OwnerRecord::Binding(_) => {
                let OwnerKey::Binding(id) = key else {
                    return Err(Failure::Authority);
                };
                return scopes.binding(self, id, 0);
            }
            _ => None,
        };
        Ok(declaration.map(|declaration| DeclarationReference {
            package: self.source.root.package_id,
            declaration,
        }))
    }

    fn associated_metadata(&self, package: PackageId, key: OwnerKey) -> bool {
        let OwnerKey::Declaration(id) = key else {
            return false;
        };
        let scope = DeclarationReference {
            package,
            declaration: id,
        };
        let oracle = Oracle(self.source, Some(scope));
        if self
            .header(package, key, true)
            .is_ok_and(|h| h.kind == OwnerKind::ReadView)
        {
            return true;
        }
        oracle
            .contract(scope)
            .is_some_and(|c| !c.associated_types.is_empty())
            || oracle.implementation(scope).is_some_and(|i| {
                !i.associated_types.is_empty()
                    || oracle
                        .contract(i.contract)
                        .is_some_and(|c| !c.associated_types.is_empty())
                    || i.methods.iter().any(|m| {
                        m.type_arguments
                            .iter()
                            .any(|t| oracle.associated_closure(*t))
                            || m.implementations
                                .iter()
                                .any(|p| oracle.associated_operand(p))
                    })
            })
            || oracle.function(scope).is_some_and(|f| {
                !f.read_scope_parameters.is_empty()
                    || !f.read_scope_sources.is_empty()
                    || matches!(f.result.relation, ResultRelation::ViewFrom { .. })
                    || f.implementation_parameters.iter().any(|p| {
                        oracle
                            .contract(p.contract)
                            .is_some_and(|c| !c.associated_types.is_empty())
                    })
            })
    }

    fn roots_need_scope(&self, roots: &[TypeObjectDigest]) -> bool {
        let oracle = Oracle(self.source, None);
        roots.iter().any(|ty| oracle.associated_closure(*ty))
    }

    fn declaration_metadata(
        &mut self,
        scope: DeclarationReference,
        occurrence: Occurrence,
    ) -> Result<()> {
        let oracle = Oracle(self.source, Some(scope));
        if let Some(c) = oracle.contract(scope) {
            if !c.associated_types.is_empty() {
                self.current(occurrence)?;
                for member in &c.associated_types {
                    match member.bound {
                        AssociatedTypeBound::Owned => self.member(scope, member.id)?,
                        AssociatedTypeBound::ReadViewFamily { .. } => {
                            self.read_view_member(scope, member.id)?
                        }
                    }
                }
            }
            for method in &c.methods {
                if !method.read_scope_parameters.is_empty()
                    || !method.read_scope_sources.is_empty()
                    || matches!(method.result.relation, ResultRelation::ViewFrom { .. })
                    || method
                        .parameters
                        .iter()
                        .any(|p| self.object(p.ty).is_ok_and(|t| t.form.is_read_view()))
                {
                    self.read_view_method_signature(scope, method)?;
                }
            }
        } else if let Some(i) = oracle.implementation(scope) {
            self.admit_bindings(scope)?;
            self.lexical_inventory(scope)?;
            for mapping in &i.methods {
                self.step(0)?;
                let f = Oracle(self.source, Some(mapping.function))
                    .function(mapping.function)
                    .ok_or(Failure::Authority)?;
                if f.type_parameters.len() != mapping.type_arguments.len()
                    || f.implementation_parameters.len() != mapping.implementations.len()
                {
                    return Err(Failure::Arity);
                }
                self.reserve_entries(f.type_parameters.len() + f.implementation_parameters.len())?;
                for operand in &mapping.implementations {
                    let entries = self.operand_entries(operand, 0)?;
                    self.reserve_entries(entries)?;
                }
                let environment = Environment {
                    ordinary: f
                        .type_parameters
                        .iter()
                        .copied()
                        .zip(mapping.type_arguments.iter().copied())
                        .collect(),
                    source: Context::Lexical(mapping.function),
                    caller: occurrence,
                    contract_operand: None,
                    witnesses: f
                        .implementation_parameters
                        .iter()
                        .map(|p| p.id)
                        .zip(mapping.implementations.iter().cloned())
                        .collect(),
                };
                self.admit_application(&environment)?;
                let method = self
                    .contract(i.contract)?
                    .methods
                    .iter()
                    .find(|m| m.id == mapping.method)
                    .ok_or(Failure::Inventory)?
                    .clone();
                if !mapping.read_scope_arguments.is_empty()
                    || !method.read_scope_parameters.is_empty()
                    || !method.read_scope_sources.is_empty()
                    || self.contains_read_view(method.result.ty)?
                    || method.parameters.iter().any(|p| {
                        Oracle(self.source, Some(i.contract))
                            .form(p.ty)
                            .is_some_and(TypeForm::is_read_view)
                    })
                {
                    self.admit_view_method_mapping(scope, &method, mapping)?;
                }
            }
        } else if let Some(function) = oracle.function(scope) {
            self.lexical_inventory(scope)?;
            if !function.read_scope_parameters.is_empty()
                || !function.read_scope_sources.is_empty()
                || matches!(function.result.relation, ResultRelation::ViewFrom { .. })
                || self.object(function.result.ty)?.form.is_read_view()
                || function.parameters.iter().any(|p| {
                    oracle
                        .parameter(scope.package, *p)
                        .is_some_and(|p| oracle.form(p.ty).is_some_and(TypeForm::is_read_view))
                })
            {
                self.read_view_function_signature(scope)?;
            }
        } else if self
            .header(
                scope.package,
                OwnerKey::Declaration(scope.declaration),
                true,
            )?
            .kind
            == OwnerKind::ReadView
        {
            self.admit_view_layout(scope)?;
        }
        Ok(())
    }

    fn call_roots(
        &mut self,
        id: ExpressionId,
        occurrence: Occurrence,
    ) -> Result<Option<TypeObjectDigest>> {
        let scope = occurrence.parameter_scope;
        let oracle = Oracle(self.source, scope);
        if !oracle.associated_invocation_required(id) {
            // FunctionValue has no implementation operands, but its phantom
            // ordinary actuals and complete target signature still need scope.
            if !matches!(oracle.expression(id), Some(ExpressionOperation::FunctionValue { type_arguments, .. }) if type_arguments.iter().any(|t| oracle.associated_closure(*t)))
            {
                return Ok(None);
            }
        }
        let operation = match self.source.owners.get(&OwnerKey::Expression(id)) {
            Some(OwnerRecord::Expression(e)) => &e.operation,
            _ => return Err(Failure::Missing),
        };
        match operation {
            ExpressionOperation::Call {
                function,
                type_arguments,
                ..
            }
            | ExpressionOperation::FunctionValue {
                function,
                type_arguments,
                ..
            }
            | ExpressionOperation::ImplementationCall {
                function,
                type_arguments,
                ..
            } => {
                let operands = match operation {
                    ExpressionOperation::ImplementationCall {
                        implementations, ..
                    } => implementations.as_slice(),
                    _ => &[],
                };
                let f = Oracle(self.source, Some(*function))
                    .function(*function)
                    .ok_or(Failure::Authority)?;
                if f.type_parameters.len() != type_arguments.len()
                    || f.implementation_parameters.len() != operands.len()
                {
                    return Err(Failure::Arity);
                }
                self.reserve_entries(f.type_parameters.len() + f.implementation_parameters.len())?;
                for operand in operands {
                    let entries = self.operand_entries(operand, 0)?;
                    self.reserve_entries(entries)?;
                }
                let environment = Environment {
                    ordinary: f
                        .type_parameters
                        .iter()
                        .copied()
                        .zip(type_arguments.iter().copied())
                        .collect(),
                    source: Context::Lexical(*function),
                    caller: occurrence,
                    contract_operand: None,
                    witnesses: f
                        .implementation_parameters
                        .iter()
                        .map(|p| p.id)
                        .zip(operands.iter().cloned())
                        .collect(),
                };
                self.admit_application(&environment)?;
                let scopes = match operation {
                    ExpressionOperation::Call {
                        read_scope_arguments,
                        ..
                    }
                    | ExpressionOperation::ImplementationCall {
                        read_scope_arguments,
                        ..
                    }
                    | ExpressionOperation::FunctionValue {
                        read_scope_arguments,
                        ..
                    } => read_scope_arguments,
                    _ => unreachable!(),
                };
                let view_dependent = !f.read_scope_parameters.is_empty()
                    || !f.read_scope_sources.is_empty()
                    || !scopes.is_empty()
                    || self.contains_read_view(f.result.ty)?
                    || f.parameters.iter().any(|p| {
                        Oracle(self.source, Some(*function))
                            .parameter(function.package, *p)
                            .is_some_and(|p| {
                                Oracle(self.source, Some(*function))
                                    .form(p.ty)
                                    .is_some_and(TypeForm::is_read_view)
                            })
                    });
                let view_environment = if view_dependent {
                    if scopes.len() != f.read_scope_parameters.len()
                        || matches!(operation, ExpressionOperation::FunctionValue { .. })
                    {
                        return Err(Failure::Authority);
                    }
                    self.read_view_function_signature(*function)?;
                    self.reserve_entries(scopes.len())?;
                    Some(ReadViewEnvironment {
                        types: environment.clone(),
                        source_binder: ReadScopeBinder::Declaration(*function),
                        caller_binder: ReadScopeBinder::Declaration(
                            occurrence.parameter_scope.ok_or(Failure::Authority)?,
                        ),
                        scopes: f
                            .read_scope_parameters
                            .iter()
                            .zip(scopes)
                            .map(|(p, actual)| {
                                (
                                    ReadScopeReference {
                                        binder: ReadScopeBinder::Declaration(*function),
                                        parameter: p.id,
                                    },
                                    *actual,
                                )
                            })
                            .collect(),
                    })
                } else {
                    None
                };
                for id in &f.parameters {
                    let p = Oracle(self.source, Some(*function))
                        .parameter(function.package, *id)
                        .ok_or(Failure::Missing)?;
                    if p.parent != ParameterParent::Function(function.declaration)
                        || p.header.owner != OwnerKey::Parameter(*id)
                    {
                        return Err(Failure::Authority);
                    }
                    self.admit_roots(
                        &[p.ty],
                        self.occurrence_for(
                            function.package,
                            p.header.owner,
                            Some(*function),
                            false,
                        ),
                        false,
                    )?;
                    let ty = if let Some(view_environment) = &view_environment {
                        self.substitute_view(p.ty, view_environment, 0)?
                    } else {
                        self.substitute(p.ty, &environment, 0)?
                    };
                    self.admit_roots(&[ty], occurrence, false)?;
                }
                let result = if let Some(view_environment) = &view_environment {
                    self.substitute_view(f.result.ty, view_environment, 0)?
                } else {
                    self.substitute(f.result.ty, &environment, 0)?
                };
                self.admit_roots(&[result], occurrence, false)?;
                Ok(Some(result))
            }
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                read_scope_arguments,
                ..
            } => {
                let (selected, self_type, actuals) = self.admit_operand(witness, occurrence, 0)?;
                if selected != *contract {
                    return Err(Failure::Authority);
                }
                let c = self.contract(*contract)?.clone();
                let m = c
                    .methods
                    .iter()
                    .find(|m| m.id == *method)
                    .ok_or(Failure::Authority)?;
                let entries = self.operand_entries(witness, 0)?;
                self.reserve_entries(1 + c.type_parameters.len() + entries)?;
                let environment = Environment {
                    ordinary: std::iter::once((c.self_parameter, self_type))
                        .chain(c.type_parameters.iter().copied().zip(actuals))
                        .collect(),
                    source: Context::Contract(*contract),
                    caller: occurrence,
                    contract_operand: Some(witness.clone()),
                    witnesses: BTreeMap::new(),
                };
                self.admit_application(&environment)?;
                let view_dependent = !m.read_scope_parameters.is_empty()
                    || !m.read_scope_sources.is_empty()
                    || !read_scope_arguments.is_empty()
                    || self.contains_read_view(m.result.ty)?
                    || m.parameters.iter().any(|p| {
                        Oracle(self.source, Some(*contract))
                            .form(p.ty)
                            .is_some_and(TypeForm::is_read_view)
                    });
                let view_environment = if view_dependent {
                    if read_scope_arguments.len() != m.read_scope_parameters.len() {
                        return Err(Failure::Arity);
                    }
                    self.read_view_method_signature(*contract, m)?;
                    self.reserve_entries(read_scope_arguments.len())?;
                    let binder = ReadScopeBinder::ContractMethod {
                        contract: *contract,
                        method: *method,
                    };
                    Some(ReadViewEnvironment {
                        types: environment.clone(),
                        source_binder: binder,
                        caller_binder: ReadScopeBinder::Declaration(
                            occurrence.parameter_scope.ok_or(Failure::Authority)?,
                        ),
                        scopes: m
                            .read_scope_parameters
                            .iter()
                            .zip(read_scope_arguments)
                            .map(|(p, actual)| {
                                (
                                    ReadScopeReference {
                                        binder,
                                        parameter: p.id,
                                    },
                                    *actual,
                                )
                            })
                            .collect(),
                    })
                } else {
                    None
                };
                for p in &m.parameters {
                    let ty = if let Some(view_environment) = &view_environment {
                        self.substitute_view(p.ty, view_environment, 0)?
                    } else {
                        self.substitute(p.ty, &environment, 0)?
                    };
                    self.admit_roots(&[ty], occurrence, false)?;
                }
                let result = if let Some(view_environment) = &view_environment {
                    self.substitute_view(m.result.ty, view_environment, 0)?
                } else {
                    self.substitute(m.result.ty, &environment, 0)?
                };
                self.admit_roots(&[result], occurrence, false)?;
                Ok(Some(result))
            }
            _ => Ok(None),
        }
    }

    /// All canonical associated occurrences, including untaken syntax and
    /// unused maps. Pure ordinary unreachable records remain the responsibility
    /// of structural admission; they cannot supply a lexical associated scope.
    pub fn admit_canonical_closure(&mut self) -> Result<()> {
        let scopes = Scopes::build(self)?;
        let package = self.source.root.package_id;
        let mut results = BTreeMap::new();
        for (key, owner) in &self.source.owners {
            self.step(0)?;
            if owner.header().owner != *key {
                return Err(Failure::Authority);
            }
            let mut roots = owner.type_roots();
            if let OwnerRecord::Expression(e) = owner
                && let ExpressionOperation::Local { value } = e.operation
            {
                let local_key = match value {
                    LocalValueReference::FunctionParameter(id)
                    | LocalValueReference::OperationParameter(id) => OwnerKey::Parameter(id),
                    LocalValueReference::LexicalBinding(id)
                    | LocalValueReference::MatchPayload(id)
                    | LocalValueReference::TransactionBinding(id) => OwnerKey::Binding(id),
                };
                let local = self.source.owners.get(&local_key).ok_or(Failure::Missing)?;
                roots.extend(local.type_roots());
                if self.roots_need_scope(&roots)
                    && self.local_scope(&scopes, *key, owner)?
                        != self.local_scope(&scopes, local_key, local)?
                {
                    return Err(Failure::Authority);
                }
            }
            let metadata = self.associated_metadata(package, *key);
            let view_operation = matches!(owner, OwnerRecord::Expression(e) if matches!(e.operation, ExpressionOperation::CreateReadView { .. } | ExpressionOperation::BorrowReadView { .. } | ExpressionOperation::ReborrowReadView { .. } | ExpressionOperation::MoveReadView { .. } | ExpressionOperation::DropReadView { .. } | ExpressionOperation::ReadViewDataField { .. } | ExpressionOperation::BorrowReadViewField { .. }));
            let needs_scope = metadata || self.roots_need_scope(&roots) || view_operation;
            let invocation = matches!(owner, OwnerRecord::Expression(e) if matches!(e.operation, ExpressionOperation::Call { .. } | ExpressionOperation::ImplementationCall { .. } | ExpressionOperation::MethodCall { .. } | ExpressionOperation::FunctionValue { .. }));
            // An ordinary call whose signature is projected needs scope even
            // when its expression has no explicit type roots.
            let call_needs_scope = invocation
                && match owner {
                    OwnerRecord::Expression(e) => {
                        Oracle(self.source, None).associated_invocation_required(e.id)
                    }
                    _ => false,
                };
            if !needs_scope && !call_needs_scope {
                continue;
            }
            let scope = self.local_scope(&scopes, *key, owner)?;
            let representation = matches!(owner, OwnerRecord::Field(_) | OwnerRecord::Case(_));
            let occurrence = self.occurrence_for(package, *key, scope, representation);
            if view_operation {
                self.current_view(occurrence)?;
            }
            self.admit_roots(&roots, occurrence, false)?;
            if metadata {
                self.declaration_metadata(scope.ok_or(Failure::Authority)?, occurrence)?;
            }
            if let OwnerRecord::Expression(e) = owner
                && let Some(result) = self.call_roots(e.id, occurrence)?
            {
                self.reserve_entries(1)?;
                results.insert(e.id, result);
            }
        }
        for (key, owner) in &self.source.owners {
            if let OwnerRecord::Expression(e) = owner
                && let ExpressionOperation::BorrowCall { call, binding, .. } = e.operation
                && let Some(result) = results.get(&call)
            {
                let scope = self.local_scope(&scopes, *key, owner)?;
                let binding_scope = scopes.binding(self, binding, 0)?;
                self.admit_roots(
                    &[*result],
                    self.occurrence_for(package, *key, scope, false),
                    false,
                )?;
                self.admit_roots(
                    &[*result],
                    self.occurrence_for(package, OwnerKey::Binding(binding), binding_scope, false),
                    false,
                )?;
            }
        }
        // Authored and representation selections are separate. A property-only
        // selection never changes a declaration into a member-authority source.
        // Reserve the upper bound before constructing the deduplicated set.
        self.reserve_entries(
            self.source
                .dependencies
                .len()
                .checked_add(self.source.representation_dependencies.len())
                .ok_or(Failure::Capacity)?,
        )?;
        let selections: BTreeSet<_> = self
            .source
            .dependencies
            .iter()
            .chain(&self.source.representation_dependencies)
            .map(|(p, d)| (*p, d.package_revision))
            .collect();
        for (package, revision) in selections {
            let interface = self
                .source
                .dependency_interfaces
                .get(&revision)
                .ok_or(Failure::Missing)?;
            for (key, owner) in interface.iter() {
                self.step(0)?;
                if owner.header().owner != *key {
                    return Err(Failure::Authority);
                }
                let roots = owner.type_roots();
                let metadata = self.associated_metadata(package, *key)
                    || matches!(owner,
                        PackageInterfaceRecord::Declaration(d)
                        | PackageInterfaceRecord::RepresentationDeclaration(d)
                        if matches!(&d.payload,
                            PackageInterfaceDeclarationPayload::OwnedContract(c) if !c.associated_types.is_empty())
                            || matches!(&d.payload,
                                PackageInterfaceDeclarationPayload::OwnedImplementation(i) if !i.associated_types.is_empty()));
                if !metadata && !self.roots_need_scope(&roots) {
                    continue;
                }
                let (declaration, representation) = match owner {
                    PackageInterfaceRecord::Declaration(_) => {
                        let OwnerKey::Declaration(id) = key else {
                            return Err(Failure::Authority);
                        };
                        (Some(*id), false)
                    }
                    PackageInterfaceRecord::RepresentationDeclaration(_) => {
                        let OwnerKey::Declaration(id) = key else {
                            return Err(Failure::Authority);
                        };
                        (Some(*id), true)
                    }
                    PackageInterfaceRecord::Field(f) => (Some(f.declaration), true),
                    PackageInterfaceRecord::Case(c) => (Some(c.declaration), true),
                    PackageInterfaceRecord::Parameter(p) => {
                        let declaration = match p.parent {
                            ParameterParent::Function(id) => {
                                let scope = DeclarationReference {
                                    package,
                                    declaration: id,
                                };
                                if p.header.kind != OwnerKind::Parameter
                                    || !matches!(key, OwnerKey::Parameter(_))
                                    || Oracle(self.source, Some(scope))
                                        .signature(scope)
                                        .is_none_or(|s| {
                                            s.0.iter().filter(|p| p.header.owner == *key).count()
                                                != 1
                                        })
                                {
                                    return Err(Failure::Authority);
                                }
                                Some(id)
                            }
                            ParameterParent::Operation(_) => None,
                        };
                        (declaration, false)
                    }
                    _ => (None, false),
                };
                let scope = declaration.map(|declaration| DeclarationReference {
                    package,
                    declaration,
                });
                let occurrence = self.occurrence_for(package, *key, scope, representation);
                self.admit_roots(&roots, occurrence, false)?;
                if metadata {
                    if matches!(owner, PackageInterfaceRecord::RepresentationDeclaration(d) if matches!(d.payload, PackageInterfaceDeclarationPayload::ReadView(_)))
                    {
                        self.admit_view_layout(scope.ok_or(Failure::Authority)?)?;
                        continue;
                    }
                    if representation || !self.source.dependencies.contains_key(&package) {
                        return Err(Failure::Authority);
                    }
                    self.declaration_metadata(scope.ok_or(Failure::Authority)?, occurrence)?;
                }
            }
        }
        Ok(())
    }
}
