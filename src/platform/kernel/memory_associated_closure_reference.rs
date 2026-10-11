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
struct Scopes {
    parents: BTreeMap<ExpressionId, Vec<Parent>>,
    bindings: BTreeMap<BindingId, Vec<ExpressionId>>,
}

impl Scopes {
    fn build(resolver: &mut Resolver<'_>) -> Result<Self> {
        let mut scopes = Self::default();
        for (key, owner) in &resolver.source.owners {
            resolver.step(0)?;
            for root in owner.expression_roots() {
                resolver.reserve_entries(1)?;
                scopes
                    .parents
                    .entry(root)
                    .or_default()
                    .push(Parent::Owner(*key));
            }
            if let OwnerRecord::Expression(expression) = owner {
                for child in expression.children() {
                    resolver.reserve_entries(1)?;
                    scopes
                        .parents
                        .entry(child.expression)
                        .or_default()
                        .push(Parent::Expression(expression.id));
                }
                let mut bindings = Vec::new();
                match &expression.operation {
                    ExpressionOperation::Let { bindings: ids, .. } => bindings.extend(ids),
                    ExpressionOperation::BorrowOwnedField { binding, .. }
                    | ExpressionOperation::BorrowOwnedRecordField { binding, .. }
                    | ExpressionOperation::BorrowOwnedItem { binding, .. }
                    | ExpressionOperation::BorrowCall { binding, .. }
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
                    resolver.reserve_entries(1)?;
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

    fn expression(
        &self,
        source: &KernelSnapshot,
        id: ExpressionId,
        depth: usize,
    ) -> Result<Option<DeclarationReference>> {
        if depth > contract::MAXIMUM_EXPRESSION_DEPTH {
            return Err(Failure::Capacity);
        }
        let [parent] = self.parents.get(&id).ok_or(Failure::Authority)?.as_slice() else {
            return Err(Failure::Authority);
        };
        match parent {
            Parent::Expression(parent) => self.expression(source, *parent, depth + 1),
            Parent::Owner(OwnerKey::Binding(binding)) => self.binding(source, *binding, depth + 1),
            Parent::Owner(OwnerKey::Declaration(id)) => {
                match source.owners.get(&OwnerKey::Declaration(*id)) {
                    Some(OwnerRecord::Declaration(_)) => Ok(Some(DeclarationReference {
                        package: source.root.package_id,
                        declaration: *id,
                    })),
                    _ => Err(Failure::Authority),
                }
            }
            Parent::Owner(OwnerKey::Port(port)) => {
                match source.owners.get(&OwnerKey::Port(*port)) {
                    Some(OwnerRecord::Port(p)) => Ok(Some(DeclarationReference {
                        package: source.root.package_id,
                        declaration: p.declaration,
                    })),
                    _ => Err(Failure::Authority),
                }
            }
            _ => Err(Failure::Authority),
        }
    }

    fn binding(
        &self,
        source: &KernelSnapshot,
        id: BindingId,
        depth: usize,
    ) -> Result<Option<DeclarationReference>> {
        let [container] = self.bindings.get(&id).ok_or(Failure::Authority)?.as_slice() else {
            return Err(Failure::Authority);
        };
        self.expression(source, *container, depth + 1)
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
        &self,
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
            OwnerRecord::Expression(e) => return scopes.expression(self.source, e.id, 0),
            OwnerRecord::Binding(_) => {
                let OwnerKey::Binding(id) = key else {
                    return Err(Failure::Authority);
                };
                return scopes.binding(self.source, id, 0);
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
                f.implementation_parameters.iter().any(|p| {
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
                    self.member(scope, member.id)?;
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
            }
        } else if oracle.function(scope).is_some() {
            self.lexical_inventory(scope)?;
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
                    let ty = self.substitute(p.ty, &environment, 0)?;
                    self.admit_roots(&[ty], occurrence, false)?;
                }
                let result = self.substitute(f.result, &environment, 0)?;
                self.admit_roots(&[result], occurrence, false)?;
                Ok(Some(result))
            }
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
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
                self.reserve_entries(1 + c.type_parameters.len() + witness.walk().count())?;
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
                for p in &m.parameters {
                    let ty = self.substitute(p.ty, &environment, 0)?;
                    self.admit_roots(&[ty], occurrence, false)?;
                }
                let result = self.substitute(m.result, &environment, 0)?;
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
            let needs_scope = metadata || self.roots_need_scope(&roots);
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
                self.admit_roots(
                    &[*result],
                    self.occurrence_for(package, *key, scope, false),
                    false,
                )?;
                self.admit_roots(
                    &[*result],
                    self.occurrence_for(
                        package,
                        OwnerKey::Binding(binding),
                        scopes.binding(self.source, binding, 0)?,
                        false,
                    ),
                    false,
                )?;
            }
        }
        // Authored and representation selections are separate. A property-only
        // selection never changes a declaration into a member-authority source.
        let selections: BTreeSet<_> = self
            .source
            .dependencies
            .iter()
            .chain(&self.source.representation_dependencies)
            .map(|(p, d)| (*p, d.package_revision))
            .collect();
        self.reserve_entries(selections.len())?;
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
