//! Positive view category and complete nominal/family closure from canonical
//! records. This does not consume the production property or admission helper.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in super::super) enum Category {
    Ordinary,
    Owned,
    ReadView,
    Capability,
    Callable,
}

fn view_form(form: &TypeForm) -> bool {
    matches!(
        form,
        TypeForm::ReadViewApplied { .. }
            | TypeForm::AssociatedReadViewMember { .. }
            | TypeForm::AssociatedReadViewProjection { .. }
            | TypeForm::RegionCap {
                mode: RegionMode::Shared { .. },
                ..
            }
    )
}

impl Resolver<'_> {
    pub(super) fn read_view_declaration(
        &mut self,
        declaration: DeclarationReference,
    ) -> Result<ReadViewDeclaration> {
        self.step(0)?;
        let key = OwnerKey::Declaration(declaration.declaration);
        let header = self.header(declaration.package, key, true)?;
        if header.kind != OwnerKind::ReadView
            || header.contract_version != contract::GRAPH_CONTRACT_VERSION
            || self.source.root.graph_contract_version != contract::GRAPH_CONTRACT_VERSION
        {
            return Err(Failure::Authority);
        }
        let view = if declaration.package == self.source.root.package_id {
            match self.source.owners.get(&key) {
                Some(OwnerRecord::Declaration(d)) => match &d.payload {
                    DeclarationPayload::ReadView(view) => view,
                    _ => return Err(Failure::Authority),
                },
                _ => return Err(Failure::Missing),
            }
        } else {
            match self.interface(declaration.package, key, true)? {
                PackageInterfaceRecord::Declaration(d)
                | PackageInterfaceRecord::RepresentationDeclaration(d) => match &d.payload {
                    PackageInterfaceDeclarationPayload::ReadView(view) => view,
                    _ => return Err(Failure::Authority),
                },
                _ => return Err(Failure::Missing),
            }
        };
        let count = view
            .fields
            .len()
            .checked_add(view.type_parameters.len())
            .and_then(|n| n.checked_add(1))
            .ok_or(Failure::Capacity)?;
        self.reserve_entries(count)?;
        // Reborrow after the reserve so no reference across mutable accounting
        // is retained. Inline inventories remain part of their parent owner.
        let view = if declaration.package == self.source.root.package_id {
            match self.source.owners.get(&key) {
                Some(OwnerRecord::Declaration(d)) => match &d.payload {
                    DeclarationPayload::ReadView(v) => v.clone(),
                    _ => return Err(Failure::Authority),
                },
                _ => return Err(Failure::Missing),
            }
        } else {
            match self.interface(declaration.package, key, true)? {
                PackageInterfaceRecord::Declaration(d)
                | PackageInterfaceRecord::RepresentationDeclaration(d) => match &d.payload {
                    PackageInterfaceDeclarationPayload::ReadView(v) => v.clone(),
                    _ => return Err(Failure::Authority),
                },
                _ => return Err(Failure::Missing),
            }
        };
        if view.fields.len() > contract::MAXIMUM_CHILDREN
            || view.type_parameters.len() > contract::MAXIMUM_CHILDREN
        {
            return Err(Failure::Capacity);
        }
        self.reserve_entries(
            view.fields
                .len()
                .checked_mul(2)
                .and_then(|n| n.checked_add(view.type_parameters.len()))
                .ok_or(Failure::Capacity)?,
        )?;
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for field in &view.fields {
            self.step(0)?;
            if !ids.insert(field.id) || !names.insert(field.name.as_str()) {
                return Err(Failure::Inventory);
            }
        }
        let mut parameters = BTreeSet::new();
        let oracle = Oracle(self.source, Some(declaration));
        for parameter in &view.type_parameters {
            if !parameters.insert(*parameter) {
                return Err(Failure::Inventory);
            }
            let p = oracle
                .type_parameter(declaration.package, *parameter)
                .ok_or(Failure::Authority)?;
            if p.header.owner != OwnerKey::TypeParameter(*parameter)
                || p.header.kind != OwnerKind::TypeParameter
                || p.declaration != declaration.declaration
                || p.header.contract_version != contract::GRAPH_CONTRACT_VERSION
                || (!p.constraints.has_owned() && !p.constraints.requires_transfer())
            {
                return Err(Failure::Authority);
            }
        }
        Ok(view)
    }

    pub(super) fn view_scope_binder(
        &self,
        occurrence: Occurrence,
        scope: &ReadScopeArgument,
    ) -> Result<ReadScopeBinder> {
        match occurrence.context {
            Context::Lexical(callable) => match scope {
                ReadScopeArgument::Parameter(ReadScopeReference {
                    binder: ReadScopeBinder::ContractMethod { contract, method },
                    ..
                }) if self.implementation(callable).is_ok_and(|i| {
                    i.contract == *contract
                        && i.methods.iter().filter(|m| m.method == *method).count() == 1
                }) =>
                {
                    Ok(ReadScopeBinder::ContractMethod {
                        contract: *contract,
                        method: *method,
                    })
                }
                ReadScopeArgument::Parameter(ReadScopeReference {
                    binder:
                        ReadScopeBinder::AssociatedBinding {
                            implementation,
                            member,
                        },
                    ..
                }) if *implementation == callable
                    && self
                        .implementation(callable)?
                        .associated_types
                        .iter()
                        .any(|b| b.member == *member) =>
                {
                    Ok(ReadScopeBinder::AssociatedBinding {
                        implementation: callable,
                        member: *member,
                    })
                }
                _ => Ok(ReadScopeBinder::Declaration(callable)),
            },
            Context::Contract(contract) => match scope {
                ReadScopeArgument::Parameter(ReadScopeReference {
                    binder:
                        ReadScopeBinder::ContractMethod {
                            contract: actual,
                            method,
                        },
                    ..
                }) if *actual == contract => Ok(ReadScopeBinder::ContractMethod {
                    contract,
                    method: *method,
                }),
                _ => Err(Failure::Authority),
            },
            Context::Forbidden => {
                let declaration = occurrence.parameter_scope.ok_or(Failure::Authority)?;
                let key = OwnerKey::Declaration(declaration.declaration);
                if self.header(declaration.package, key, true)?.kind == OwnerKind::ReadView {
                    Ok(ReadScopeBinder::Declaration(declaration))
                } else {
                    Err(Failure::Authority)
                }
            }
        }
    }

    pub(super) fn admit_read_view_form(
        &mut self,
        form: &TypeForm,
        occurrence: Occurrence,
    ) -> Result<()> {
        self.step(0)?;
        self.current_view(occurrence)?;
        let read_scope = match form {
            TypeForm::RegionCap {
                region,
                mode: RegionMode::Shared { scope },
            } => {
                let TypeForm::TypeParameter { parameter } = self.object(*region)?.form else {
                    return Err(Failure::Authority);
                };
                self.parameter_authority(parameter, occurrence)?;
                let owner = occurrence.parameter_scope.ok_or(Failure::Authority)?;
                let parameter = Oracle(self.source, Some(owner))
                    .type_parameter(owner.package, parameter)
                    .map(|p| p.constraints)
                    .ok_or(Failure::Authority)?;
                if !parameter.is_region() {
                    return Err(Failure::Category);
                }
                scope
            }
            TypeForm::ReadViewApplied {
                declaration,
                arguments,
                read_scope,
            } => {
                let view = self.read_view_declaration(*declaration)?;
                if arguments.len() != view.type_parameters.len() {
                    return Err(Failure::Arity);
                }
                for (parameter, actual) in view.type_parameters.iter().zip(arguments) {
                    let constraints = Oracle(self.source, Some(*declaration))
                        .type_parameter(declaration.package, *parameter)
                        .map(|p| p.constraints)
                        .ok_or(Failure::Authority)?;
                    self.constraints(*actual, constraints, occurrence, 0)?;
                    if self.contains_read_view(*actual)? {
                        return Err(Failure::Category);
                    }
                }
                read_scope
            }
            TypeForm::AssociatedReadViewMember {
                contract,
                member,
                read_scope,
            } => {
                if occurrence.context != Context::Contract(*contract) {
                    return Err(Failure::Authority);
                }
                self.read_view_member(*contract, *member)?;
                read_scope
            }
            TypeForm::AssociatedReadViewProjection {
                scope,
                parameter,
                contract,
                member,
                read_scope,
            } => {
                if occurrence.context != Context::Lexical(*scope) {
                    return Err(Failure::Authority);
                }
                let witnesses = self.lexical_inventory(*scope)?;
                if !witnesses
                    .iter()
                    .any(|w| w.id == *parameter && w.contract == *contract)
                {
                    return Err(Failure::Authority);
                }
                self.read_view_member(*contract, *member)?;
                read_scope
            }
            _ => return Err(Failure::Category),
        };
        let binder = self.view_scope_binder(occurrence, read_scope)?;
        self.admit_scope_argument(read_scope, &binder, occurrence)
    }

    pub(super) fn read_view_member(
        &mut self,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
    ) -> Result<()> {
        self.step(0)?;
        self.current_view(self.declaration_occurrence(contract, Context::Contract(contract)))?;
        let inventory = self.contract(contract)?;
        let count = inventory.associated_types.len();
        self.reserve_entries(count.checked_mul(2).ok_or(Failure::Capacity)?)?;
        let inventory = self.contract(contract)?;
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut found = false;
        for item in &inventory.associated_types {
            if !ids.insert(item.id) || !names.insert(item.name.as_str()) {
                return Err(Failure::Inventory);
            }
            if item.id == member {
                found = matches!(item.bound, AssociatedTypeBound::ReadViewFamily { .. });
            }
        }
        if !found {
            return Err(Failure::Authority);
        }
        self.admit_scope_inventory(&ReadScopeBinder::AssociatedMember { contract, member })
    }

    /// Used before the legacy resolver/cache: an enclosing old-form type cannot
    /// hide a scope-dependent child and accidentally take the scope-blind path.
    pub(super) fn contains_read_view(&mut self, ty: TypeObjectDigest) -> Result<bool> {
        self.reserve_entries(1)?;
        let mut pending = vec![ty];
        let mut seen = BTreeSet::new();
        while let Some(ty) = pending.pop() {
            self.step(0)?;
            if seen.contains(&ty) {
                continue;
            }
            self.reserve_entries(1)?;
            seen.insert(ty);
            let object = self.object(ty)?;
            if view_form(&object.form) {
                return Ok(true);
            }
            let count = object.child_type_count();
            self.reserve_entries(count)?;
            pending.extend(self.object(ty)?.child_types());
            if let TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } =
                self.object(ty)?.form
            {
                let oracle = Oracle(self.source, Some(declaration));
                let fields = if let Some((_, fields)) = oracle.owned_record_inventory(declaration) {
                    fields.into_iter().map(|(_, t)| t).collect::<Vec<_>>()
                } else {
                    oracle.nominal(declaration).ok_or(Failure::Authority)?.1
                };
                self.reserve_entries(fields.len())?;
                pending.extend(fields);
            }
        }
        Ok(false)
    }

    pub(super) fn read_view_category(
        &mut self,
        ty: TypeObjectDigest,
        occurrence: Occurrence,
    ) -> Result<Category> {
        self.step(0)?;
        let form = self.object(ty)?.form.clone();
        if view_form(&form) {
            self.admit_read_view_form(&form, occurrence)?;
            if let TypeForm::ReadViewApplied { declaration, .. } = form {
                self.admit_view_layout(declaration)?;
            }
            return Ok(Category::ReadView);
        }
        if self.contains_read_view(ty)? {
            return Err(Failure::Category);
        }
        if matches!(form, TypeForm::CapabilityResource { .. }) {
            return Ok(Category::Capability);
        }
        if matches!(
            form,
            TypeForm::Function { .. } | TypeForm::TaskFunction { .. }
        ) {
            return Ok(Category::Callable);
        }
        if self.owned_type(ty, occurrence, 0)? {
            return Ok(Category::Owned);
        }
        if Oracle(self.source, occurrence.parameter_scope).ordinary(ty) {
            Ok(Category::Ordinary)
        } else {
            Err(Failure::Unproved)
        }
    }

    pub(super) fn admit_view_layout(&mut self, declaration: DeclarationReference) -> Result<()> {
        self.view_layout(declaration, &mut BTreeSet::new(), 0)
    }
    fn view_layout(
        &mut self,
        declaration: DeclarationReference,
        active: &mut BTreeSet<DeclarationReference>,
        depth: usize,
    ) -> Result<()> {
        self.step(depth)?;
        if active.contains(&declaration) {
            return Err(Failure::Category);
        }
        self.reserve_entries(1)?;
        active.insert(declaration);
        let view = self.read_view_declaration(declaration)?;
        let binder = ReadScopeBinder::Declaration(declaration);
        self.admit_scope_inventory(&binder)?;
        let expected_scope = ReadScopeArgument::Parameter(ReadScopeReference {
            binder,
            parameter: view.scope_parameter.id,
        });
        let occurrence = self.declaration_occurrence(declaration, Context::Forbidden);
        for field in &view.fields {
            self.step(depth)?;
            match &field.kind {
                ReadViewFieldKind::Ordinary => {
                    if self.contains_read_view(field.ty)?
                        || self.owned_type(field.ty, occurrence, depth + 1)?
                        || !Oracle(self.source, Some(declaration)).ordinary(field.ty)
                        || !Oracle(self.source, Some(declaration))
                            .satisfies_constraints(field.ty, TypeParameterConstraints::Transferable)
                    {
                        return Err(Failure::Category);
                    }
                    self.admit_roots(&[field.ty], occurrence, true)?;
                }
                ReadViewFieldKind::ReadReference { read_scope } => {
                    if read_scope != &expected_scope {
                        return Err(Failure::Authority);
                    }
                    match self.object(field.ty)?.form.clone() {
                        TypeForm::ReadViewApplied {
                            declaration: child,
                            read_scope: actual,
                            ..
                        } => {
                            if actual != expected_scope {
                                return Err(Failure::Authority);
                            }
                            self.admit_read_view_form(
                                &self.object(field.ty)?.form.clone(),
                                occurrence,
                            )?;
                            self.view_layout(child, active, depth + 1)?;
                        }
                        TypeForm::AssociatedReadViewMember { .. }
                        | TypeForm::AssociatedReadViewProjection { .. } => {
                            return Err(Failure::Authority);
                        }
                        _ => {
                            self.admit_roots(&[field.ty], occurrence, true)?;
                            if !self.owned_type(field.ty, occurrence, depth + 1)? {
                                return Err(Failure::Category);
                            }
                        }
                    }
                }
            }
        }
        active.remove(&declaration);
        Ok(())
    }
}
