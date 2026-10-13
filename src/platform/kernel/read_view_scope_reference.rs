//! Independent symbolic scope authority over raw canonical inventories. Scope
//! equality never establishes which ordered argument supplied a read.
use super::*;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in super::super) struct ReadViewEnvironment {
    pub types: Environment,
    pub source_binder: ReadScopeBinder,
    pub caller_binder: ReadScopeBinder,
    pub scopes: BTreeMap<ReadScopeReference, ReadScopeArgument>,
}

impl Resolver<'_> {
    pub(super) fn scope_parameters(
        &mut self,
        binder: &ReadScopeBinder,
    ) -> Result<Vec<ReadScopeParameter>> {
        self.step(0)?;
        let parameters = match binder {
            ReadScopeBinder::Declaration(declaration) => {
                let oracle = Oracle(self.source, Some(*declaration));
                if let Some(function) = oracle.function(*declaration) {
                    if !function.read_scope_parameters.is_empty() {
                        self.current_view(
                            self.declaration_occurrence(
                                *declaration,
                                Context::Lexical(*declaration),
                            ),
                        )?;
                    }
                    function.read_scope_parameters
                } else {
                    let view = self.read_view_declaration(*declaration)?;
                    vec![view.scope_parameter]
                }
            }
            ReadScopeBinder::ContractMethod { contract, method } => {
                let count = {
                    let contract = self.contract(*contract)?;
                    let mut found = contract.methods.iter().filter(|m| m.id == *method);
                    let selected = found.next().ok_or(Failure::Authority)?;
                    if found.next().is_some() {
                        return Err(Failure::Inventory);
                    }
                    selected.read_scope_parameters.len()
                };
                if count != 0 {
                    self.current_view(
                        self.declaration_occurrence(*contract, Context::Contract(*contract)),
                    )?;
                }
                self.reserve_entries(count)?;
                self.contract(*contract)?
                    .methods
                    .iter()
                    .find(|m| m.id == *method)
                    .ok_or(Failure::Authority)?
                    .read_scope_parameters
                    .clone()
            }
            ReadScopeBinder::AssociatedMember { contract, member } => {
                self.current_view(
                    self.declaration_occurrence(*contract, Context::Contract(*contract)),
                )?;
                let contract = self.contract(*contract)?;
                let mut found = contract.associated_types.iter().filter(|m| m.id == *member);
                let selected = found.next().ok_or(Failure::Authority)?;
                if found.next().is_some() {
                    return Err(Failure::Inventory);
                }
                match &selected.bound {
                    AssociatedTypeBound::ReadViewFamily { scope_parameter } => {
                        vec![scope_parameter.clone()]
                    }
                    AssociatedTypeBound::Owned => return Err(Failure::Category),
                }
            }
            ReadScopeBinder::AssociatedBinding {
                implementation,
                member,
            } => {
                self.current_view(
                    self.declaration_occurrence(*implementation, Context::Lexical(*implementation)),
                )?;
                let implementation = self.implementation(*implementation)?;
                let mut found = implementation
                    .associated_types
                    .iter()
                    .filter(|m| m.member == *member);
                let selected = found.next().ok_or(Failure::Authority)?;
                if found.next().is_some() {
                    return Err(Failure::Inventory);
                }
                match &selected.value {
                    AssociatedTypeBindingValue::ReadViewFamily {
                        scope_parameter, ..
                    } => vec![scope_parameter.clone()],
                    AssociatedTypeBindingValue::Owned { .. } => return Err(Failure::Category),
                }
            }
        };
        if parameters.len() > contract::MAXIMUM_CHILDREN {
            return Err(Failure::Capacity);
        }
        self.reserve_entries(parameters.len().checked_mul(3).ok_or(Failure::Capacity)?)?;
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for parameter in &parameters {
            self.step(0)?;
            if !ids.insert(parameter.id) || !names.insert(parameter.name.as_str()) {
                return Err(Failure::Inventory);
            }
        }
        Ok(parameters)
    }

    pub(super) fn admit_scope_inventory(&mut self, binder: &ReadScopeBinder) -> Result<()> {
        self.scope_parameters(binder).map(|_| ())
    }

    pub(super) fn admit_scope_argument(
        &mut self,
        argument: &ReadScopeArgument,
        binder: &ReadScopeBinder,
        occurrence: Occurrence,
    ) -> Result<()> {
        self.step(0)?;
        self.current_view(occurrence)?;
        match argument {
            ReadScopeArgument::Parameter(reference) => {
                if &reference.binder != binder {
                    return Err(Failure::Authority);
                }
                if !self
                    .scope_parameters(binder)?
                    .iter()
                    .any(|p| p.id == reference.parameter)
                {
                    return Err(Failure::Authority);
                }
            }
            ReadScopeArgument::Lexical { callable, binding } => {
                if occurrence.context != Context::Lexical(*callable)
                    || binder != &ReadScopeBinder::Declaration(*callable)
                    || callable.package != self.source.root.package_id
                {
                    return Err(Failure::Authority);
                }
                self.current_view(
                    self.declaration_occurrence(*callable, Context::Lexical(*callable)),
                )?;
                let record = match self.source.owners.get(&OwnerKey::Binding(*binding)) {
                    Some(OwnerRecord::Binding(record)) => record,
                    _ => return Err(Failure::Missing),
                };
                if record.header.owner != OwnerKey::Binding(*binding)
                    || record.header.kind != OwnerKind::Binding
                    || !matches!(
                        record.kind,
                        BindingKind::OwnedBorrow
                            | BindingKind::ReadView
                            | BindingKind::ReadViewBorrow
                    )
                    || match record.kind {
                        BindingKind::OwnedBorrow => {
                            !matches!(record.header.contract_version, 24..=34)
                        }
                        _ => record.header.contract_version != 34,
                    }
                {
                    return Err(Failure::Authority);
                }
                let scopes = closure::Scopes::build(self)?;
                scopes.loan_generation(self, *binding)?;
                if scopes.binding(self, *binding, 0)? != Some(*callable) {
                    return Err(Failure::Authority);
                }
            }
        }
        Ok(())
    }

    pub(super) fn substitute_scope(
        &mut self,
        argument: &ReadScopeArgument,
        environment: &ReadViewEnvironment,
        source_binder: &ReadScopeBinder,
        caller_binder: &ReadScopeBinder,
    ) -> Result<ReadScopeArgument> {
        self.step(0)?;
        let source = match source_binder {
            ReadScopeBinder::Declaration(d)
            | ReadScopeBinder::ContractMethod { contract: d, .. }
            | ReadScopeBinder::AssociatedMember { contract: d, .. }
            | ReadScopeBinder::AssociatedBinding {
                implementation: d, ..
            } => *d,
        };
        let occurrence = self.declaration_occurrence(source, environment.types.source);
        self.admit_scope_argument(argument, source_binder, occurrence)?;
        // Check every formal and actual before selecting one. Unused maps remain
        // semantic inputs; no result cache may erase them.
        let formals = self.scope_parameters(source_binder)?;
        if environment.scopes.len() != formals.len() {
            return Err(Failure::Arity);
        }
        for parameter in &formals {
            let reference = ReadScopeReference {
                binder: *source_binder,
                parameter: parameter.id,
            };
            let actual = environment
                .scopes
                .get(&reference)
                .ok_or(Failure::Authority)?;
            self.admit_scope_argument(actual, caller_binder, environment.types.caller)?;
        }
        let selected = match argument {
            ReadScopeArgument::Parameter(reference) => environment
                .scopes
                .get(reference)
                .ok_or(Failure::Authority)?,
            // A callee's lexical loan is not an exported formal. It cannot be
            // promoted into the caller merely by naming its declaration.
            ReadScopeArgument::Lexical { .. } => return Err(Failure::Authority),
        };
        self.reserve_entries(1)?;
        Ok(*selected)
    }
}

/// Finite explicit containment, without allocation identities or runtime leases.
/// Different source parameters may have equal scopes while retaining distinct
/// anchors in the provenance proof.
#[derive(Default)]
pub(in super::super) struct ScopeRelations {
    edges: BTreeSet<(ReadScopeArgument, ReadScopeArgument)>,
}
impl ScopeRelations {
    pub(super) fn declare(
        &mut self,
        resolver: &mut Resolver<'_>,
        child: ReadScopeArgument,
        parent: ReadScopeArgument,
    ) -> Result<()> {
        resolver.step(0)?;
        resolver.reserve_entries(1)?;
        self.edges.insert((child, parent));
        Ok(())
    }
    pub(super) fn contains(
        &self,
        resolver: &mut Resolver<'_>,
        child: &ReadScopeArgument,
        parent: &ReadScopeArgument,
    ) -> Result<bool> {
        resolver.step(0)?;
        if child == parent {
            return Ok(true);
        }
        resolver.reserve_entries(1)?;
        let mut pending = vec![*child];
        let mut seen = BTreeSet::new();
        while let Some(current) = pending.pop() {
            resolver.step(0)?;
            if !seen.contains(&current) {
                resolver.reserve_entries(1)?;
                seen.insert(current);
                for (from, to) in &self.edges {
                    resolver.step(0)?;
                    if from == &current {
                        if to == parent {
                            return Ok(true);
                        }
                        resolver.reserve_entries(1)?;
                        pending.push(*to);
                    }
                }
            }
        }
        Ok(false)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in super::super) struct SourceAnchor {
    pub callable: DeclarationReference,
    pub source: LocalValueReference,
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in super::super) enum ReadPathStep {
    ViewReference {
        declaration: DeclarationReference,
        field: crate::platform::semantic_id::FieldId,
    },
    OwnedField(FieldReference),
    OwnedProductField(Name),
    OwnedCase(Name),
    OwnedItem(TypeObjectDigest),
    MethodRead {
        contract: DeclarationReference,
        method: crate::platform::semantic_id::MethodId,
        ty: TypeObjectDigest,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in super::super) struct ReadProvenance {
    pub anchor: SourceAnchor,
    pub path: Vec<ReadPathStep>,
}
impl ReadProvenance {
    pub(super) fn extend(&self, resolver: &mut Resolver<'_>, step: ReadPathStep) -> Result<Self> {
        resolver.step(0)?;
        resolver.reserve_entries(self.path.len().checked_add(2).ok_or(Failure::Capacity)?)?;
        let mut path = self.path.clone();
        path.push(step);
        Ok(Self {
            anchor: self.anchor.clone(),
            path,
        })
    }
    pub(super) fn derives_from(&self, selected: &Self) -> bool {
        self.anchor == selected.anchor && self.path.starts_with(&selected.path)
    }
}
