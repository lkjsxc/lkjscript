//! Independent finite symbolic scope checks for the growth adapter. Scope
//! coordinates are validated here and erased only from constructor-depth flow;
//! fresh runtime invocation/lease coordinates are never canonical terms.
use super::*;

impl<'a, 'b> Adapter<'a, 'b> {
    pub(super) fn require_owned_member(
        &self,
        package: PackageId,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
    ) -> Result<()> {
        let DeclarationPayload::OwnedContract(c) = &self.authored(package, contract)?.payload
        else {
            return Err(model::Refusal::ForeignContract);
        };
        if c.associated_types.iter().filter(|m| m.id == member).count() != 1
            || !c
                .associated_types
                .iter()
                .any(|m| m.id == member && matches!(m.bound, AssociatedTypeBound::Owned))
        {
            return Err(model::Refusal::ForeignMember);
        }
        Ok(())
    }
    fn view_scope_parameters(&self, binder: ReadScopeBinder) -> Result<&'a [ReadScopeParameter]> {
        self.budget.tick()?;
        let declaration = binder.declaration();
        let record = self.declaration(declaration)?;
        if record.header.contract_version != 34
            || self
                .source(declaration.package)?
                .root
                .graph_contract_version
                != 34
        {
            return Err(model::Refusal::Scope);
        }
        let result = match (binder, &record.payload) {
            (ReadScopeBinder::Declaration(_), DeclarationPayload::Function(f)) => {
                f.read_scope_parameters.as_slice()
            }
            (ReadScopeBinder::Declaration(_), DeclarationPayload::ReadView(v)) => {
                std::slice::from_ref(&v.scope_parameter)
            }
            (
                ReadScopeBinder::ContractMethod { method, .. },
                DeclarationPayload::OwnedContract(c),
            ) => {
                let mut methods = c.methods.iter().filter(|m| m.id == method);
                let selected = methods.next().ok_or(model::Refusal::Scope)?;
                if methods.next().is_some() {
                    return Err(model::Refusal::Inventory);
                }
                selected.read_scope_parameters.as_slice()
            }
            (
                ReadScopeBinder::AssociatedMember { member, .. },
                DeclarationPayload::OwnedContract(c),
            ) => {
                let mut members = c.associated_types.iter().filter(|m| m.id == member);
                let selected = members.next().ok_or(model::Refusal::ForeignMember)?;
                if members.next().is_some() {
                    return Err(model::Refusal::Inventory);
                }
                let AssociatedTypeBound::ReadViewFamily { scope_parameter } = &selected.bound
                else {
                    return Err(model::Refusal::ForeignMember);
                };
                std::slice::from_ref(scope_parameter)
            }
            (
                ReadScopeBinder::AssociatedBinding { member, .. },
                DeclarationPayload::OwnedImplementation(i),
            ) => {
                let mut members = i.associated_types.iter().filter(|m| m.member == member);
                let selected = members.next().ok_or(model::Refusal::ForeignMember)?;
                if members.next().is_some() {
                    return Err(model::Refusal::Inventory);
                }
                let AssociatedTypeBindingValue::ReadViewFamily {
                    scope_parameter, ..
                } = &selected.value
                else {
                    return Err(model::Refusal::ForeignMember);
                };
                std::slice::from_ref(scope_parameter)
            }
            _ => return Err(model::Refusal::Scope),
        };
        self.budget
            .reserve::<crate::platform::semantic_id::ReadScopeParameterId>(result.len())?;
        self.budget.reserve::<String>(result.len())?;
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for parameter in result {
            self.budget.tick()?;
            self.budget.reserve_bytes(parameter.name.as_str().len())?;
            if !ids.insert(parameter.id) || !names.insert(parameter.name.as_str()) {
                return Err(model::Refusal::Inventory);
            }
        }
        Ok(result)
    }

    pub(super) fn view_scope(&self, argument: ReadScopeArgument, context: Context) -> Result<()> {
        self.budget.tick()?;
        if context.generation != 34
            || self
                .source(context.source.package)?
                .root
                .graph_contract_version
                != 34
        {
            return Err(model::Refusal::Scope);
        }
        let binder = context.scope_binder.ok_or(model::Refusal::Scope)?;
        match argument {
            ReadScopeArgument::Parameter(reference) => {
                if reference.binder != binder
                    || !self
                        .view_scope_parameters(binder)?
                        .iter()
                        .any(|p| p.id == reference.parameter)
                {
                    return Err(model::Refusal::Scope);
                }
            }
            ReadScopeArgument::Lexical { callable, binding } => {
                if callable != context.source
                    || binder != ReadScopeBinder::Declaration(callable)
                    || !matches!(context.mode, TypeMode::Callable)
                {
                    return Err(model::Refusal::Scope);
                }
                if self.declaration(callable)?.header.contract_version != 34 {
                    return Err(model::Refusal::Scope);
                }
                let source = self.source(callable.package)?;
                let record = match source.owners.get(&OwnerKey::Binding(binding)) {
                    Some(OwnerRecord::Binding(b)) => b,
                    _ => return Err(model::Refusal::Scope),
                };
                if record.header.owner != OwnerKey::Binding(binding)
                    || record.header.kind != OwnerKind::Binding
                    || match record.kind {
                        BindingKind::OwnedBorrow => {
                            !matches!(record.header.contract_version, 24..=34)
                        }
                        _ => record.header.contract_version != 34,
                    }
                    || !matches!(
                        record.kind,
                        BindingKind::OwnedBorrow
                            | BindingKind::ReadView
                            | BindingKind::ReadViewBorrow
                    )
                {
                    return Err(model::Refusal::Scope);
                }
                let DeclarationPayload::Function(function) = &self.declaration(callable)?.payload
                else {
                    return Err(model::Refusal::Scope);
                };
                self.budget.reserve::<ExpressionId>(1)?;
                let mut pending = vec![function.body];
                let mut seen = BTreeSet::new();
                let mut definitions = 0;
                while let Some(id) = pending.pop() {
                    self.budget.tick()?;
                    if seen.contains(&id) {
                        continue;
                    }
                    self.budget.reserve::<ExpressionId>(1)?;
                    seen.insert(id);
                    let expression = match source.owners.get(&OwnerKey::Expression(id)) {
                        Some(OwnerRecord::Expression(e)) => e,
                        _ => return Err(model::Refusal::MissingDefinition),
                    };
                    let binds = match &expression.operation {
                        ExpressionOperation::Let { bindings, .. } => bindings.contains(&binding),
                        ExpressionOperation::BorrowCall { binding: b, .. }
                        | ExpressionOperation::BorrowOwnedField { binding: b, .. }
                        | ExpressionOperation::BorrowOwnedRecordField { binding: b, .. }
                        | ExpressionOperation::BorrowOwnedItem { binding: b, .. }
                        | ExpressionOperation::BorrowReadView { binding: b, .. }
                        | ExpressionOperation::BorrowReadViewField { binding: b, .. } => {
                            *b == binding
                        }
                        ExpressionOperation::MatchBorrowedOwned { arms, .. } => {
                            arms.iter().any(|arm| arm.binding == binding)
                        }
                        _ => false,
                    };
                    if binds {
                        let minimum = match &expression.operation {
                            ExpressionOperation::BorrowOwnedField { .. }
                            | ExpressionOperation::MatchBorrowedOwned { .. } => 24,
                            ExpressionOperation::BorrowOwnedItem { .. } => 25,
                            ExpressionOperation::BorrowCall { .. } => 27,
                            ExpressionOperation::BorrowOwnedRecordField { .. } => 32,
                            _ => 34,
                        };
                        if !contract::supported_graph_contract(expression.contract_version)
                            || expression.contract_version < minimum
                        {
                            return Err(model::Refusal::Scope);
                        }
                    }
                    definitions += usize::from(binds);
                    let children = expression.children();
                    self.budget.reserve::<ExpressionId>(children.len())?;
                    pending.extend(children.into_iter().map(|c| c.expression));
                    if let ExpressionOperation::Let { bindings, .. } = &expression.operation {
                        for id in bindings {
                            self.budget.tick()?;
                            let record = match source.owners.get(&OwnerKey::Binding(*id)) {
                                Some(OwnerRecord::Binding(b)) => b,
                                _ => return Err(model::Refusal::MissingDefinition),
                            };
                            if let Some(value) = record.value {
                                self.budget.reserve::<ExpressionId>(1)?;
                                pending.push(value);
                            }
                        }
                    }
                }
                if definitions != 1 {
                    return Err(model::Refusal::Scope);
                }
            }
        }
        Ok(())
    }

    pub(super) fn view_member(
        &self,
        package: PackageId,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
    ) -> Result<()> {
        let record = self.authored(package, contract)?;
        if record.header.contract_version != 34 {
            return Err(model::Refusal::Scope);
        }
        let DeclarationPayload::OwnedContract(c) = &record.payload else {
            return Err(model::Refusal::ForeignContract);
        };
        let mut members = c.associated_types.iter().filter(|m| m.id == member);
        let selected = members.next().ok_or(model::Refusal::ForeignMember)?;
        if members.next().is_some()
            || !matches!(selected.bound, AssociatedTypeBound::ReadViewFamily { .. })
        {
            return Err(model::Refusal::ForeignMember);
        }
        self.view_scope_parameters(ReadScopeBinder::AssociatedMember { contract, member })?;
        Ok(())
    }

    pub(super) fn view_signature_inventory<S: Copy + Ord + std::fmt::Debug>(
        &self,
        parameters: &[ReadScopeParameter],
        sources: &[ReadScopeSource<S>],
        context: Context,
    ) -> Result<()> {
        if parameters.is_empty() && sources.is_empty() {
            return Ok(());
        }
        let binder = context.scope_binder.ok_or(model::Refusal::Scope)?;
        if self.view_scope_parameters(binder)? != parameters {
            return Err(model::Refusal::Inventory);
        }
        self.budget
            .reserve::<(S, ReadScopeArgument)>(sources.len())?;
        let mut pairs = BTreeSet::new();
        for source in sources {
            self.budget.tick()?;
            if !pairs.insert((source.source, source.scope)) {
                return Err(model::Refusal::Inventory);
            }
            self.view_scope(source.scope, context)?;
        }
        Ok(())
    }

    pub(super) fn view_call_scopes(
        &self,
        function: DeclarationReference,
        arguments: &[ReadScopeArgument],
        context: Context,
    ) -> Result<()> {
        let record = self.authored(context.source.package, function)?;
        let DeclarationPayload::Function(function) = &record.payload else {
            return if arguments.is_empty() {
                Ok(())
            } else {
                Err(model::Refusal::Scope)
            };
        };
        if function.read_scope_parameters.len() != arguments.len() {
            return Err(model::Refusal::Arity);
        }
        for argument in arguments {
            self.view_scope(*argument, context)?;
        }
        Ok(())
    }
}
