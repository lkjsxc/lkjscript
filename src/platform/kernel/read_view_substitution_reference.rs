//! Capture-free type/witness/scope substitution with a complete symbolic cache
//! key. Fresh invocation IDs and live leases cannot be represented by this API.
use super::*;

impl Resolver<'_> {
    pub(super) fn substitute_view(
        &mut self,
        ty: TypeObjectDigest,
        environment: &ReadViewEnvironment,
        depth: usize,
    ) -> Result<TypeObjectDigest> {
        let source_declaration = environment.source_binder.declaration();
        if self
            .header(
                source_declaration.package,
                OwnerKey::Declaration(source_declaration.declaration),
                true,
            )?
            .kind
            == OwnerKind::ReadView
        {
            let view = self.read_view_declaration(source_declaration)?;
            if !environment.types.witnesses.is_empty()
                || environment.types.contract_operand.is_some()
                || view.type_parameters.len() != environment.types.ordinary.len()
                || !view
                    .type_parameters
                    .iter()
                    .all(|p| environment.types.ordinary.contains_key(p))
            {
                return Err(Failure::Arity);
            }
            for parameter in &view.type_parameters {
                let constraints = Oracle(self.source, Some(source_declaration))
                    .type_parameter(source_declaration.package, *parameter)
                    .map(|p| p.constraints)
                    .ok_or(Failure::Authority)?;
                self.constraints(
                    environment.types.ordinary[parameter],
                    constraints,
                    environment.types.caller,
                    depth + 1,
                )?;
            }
        } else {
            self.admit_environment(&environment.types, depth, true)?;
        }
        let parameters = self.scope_parameters(&environment.source_binder)?;
        if parameters.len() != environment.scopes.len() {
            return Err(Failure::Arity);
        }
        for parameter in &parameters {
            self.step(depth)?;
            let formal = ReadScopeReference {
                binder: environment.source_binder,
                parameter: parameter.id,
            };
            let actual = environment.scopes.get(&formal).ok_or(Failure::Authority)?;
            self.admit_scope_argument(
                actual,
                &environment.caller_binder,
                environment.types.caller,
            )?;
        }
        self.substitute_view_inner(ty, environment, depth)
    }

    fn substitute_view_inner(
        &mut self,
        ty: TypeObjectDigest,
        environment: &ReadViewEnvironment,
        depth: usize,
    ) -> Result<TypeObjectDigest> {
        self.step(depth)?;
        let source_occurrence = self.declaration_occurrence(
            environment.source_binder.declaration(),
            environment.types.source,
        );
        self.admit_roots(&[ty], source_occurrence, false)?;
        for actual in environment.types.ordinary.values() {
            if self.contains_read_view(*actual)? {
                return Err(Failure::Category);
            }
            self.admit_roots(&[*actual], environment.types.caller, false)?;
        }
        let children = self.object(ty)?.child_type_count();
        self.reserve_entries(1 + children)?;
        let object = self.object(ty)?.clone();
        match &object.form {
            TypeForm::ReadViewApplied { read_scope, .. }
            | TypeForm::AssociatedReadViewMember { read_scope, .. }
            | TypeForm::AssociatedReadViewProjection { read_scope, .. } => {
                self.admit_scope_argument(
                    read_scope,
                    &environment.source_binder,
                    source_occurrence,
                )?;
            }
            _ => {}
        }
        // Complete source/caller occurrence and unused maps were checked first.
        if let Some(result) = self.view_resolutions.get(&ty).and_then(|entries| {
            entries
                .iter()
                .find_map(|(saved, result)| (saved == environment).then_some(*result))
        }) {
            return Ok(result);
        }
        let result = match object.form {
            TypeForm::TypeParameter { parameter } => environment
                .types
                .ordinary
                .get(&parameter)
                .copied()
                .ok_or(Failure::Authority)?,
            TypeForm::ReadViewApplied {
                declaration,
                arguments,
                read_scope,
            } => {
                let read_scope = self.substitute_scope(
                    &read_scope,
                    environment,
                    &environment.source_binder,
                    &environment.caller_binder,
                )?;
                self.reserve_entries(arguments.len())?;
                let mut actuals = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    actuals.push(self.substitute_view_inner(argument, environment, depth + 1)?);
                }
                self.intern(TypeForm::ReadViewApplied {
                    declaration,
                    arguments: actuals,
                    read_scope,
                })?
            }
            TypeForm::AssociatedReadViewMember {
                contract,
                member,
                read_scope,
            } => {
                let read_scope = self.substitute_scope(
                    &read_scope,
                    environment,
                    &environment.source_binder,
                    &environment.caller_binder,
                )?;
                let operand = environment
                    .types
                    .contract_operand
                    .as_ref()
                    .ok_or(Failure::Authority)?;
                self.select_view(
                    contract,
                    member,
                    operand,
                    read_scope,
                    environment,
                    depth + 1,
                )?
            }
            TypeForm::AssociatedReadViewProjection {
                parameter,
                contract,
                member,
                read_scope,
                ..
            } => {
                let read_scope = self.substitute_scope(
                    &read_scope,
                    environment,
                    &environment.source_binder,
                    &environment.caller_binder,
                )?;
                let operand = environment
                    .types
                    .witnesses
                    .get(&parameter)
                    .ok_or(Failure::Authority)?;
                self.select_view(
                    contract,
                    member,
                    operand,
                    read_scope,
                    environment,
                    depth + 1,
                )?
            }
            TypeForm::AssociatedContractMember { contract, member } => self.select(
                contract,
                member,
                environment
                    .types
                    .contract_operand
                    .as_ref()
                    .ok_or(Failure::Authority)?,
                environment.types.caller,
                depth + 1,
            )?,
            TypeForm::AssociatedWitnessProjection {
                contract,
                member,
                parameter,
                ..
            } => self.select(
                contract,
                member,
                environment
                    .types
                    .witnesses
                    .get(&parameter)
                    .ok_or(Failure::Authority)?,
                environment.types.caller,
                depth + 1,
            )?,
            TypeForm::RegionCap {
                region,
                mode: RegionMode::Shared { scope },
            } => {
                let scope = self.substitute_scope(
                    &scope,
                    environment,
                    &environment.source_binder,
                    &environment.caller_binder,
                )?;
                let region = self.substitute_view_inner(region, environment, depth + 1)?;
                self.intern(TypeForm::RegionCap {
                    region,
                    mode: RegionMode::Shared { scope },
                })?
            }
            form => {
                let form = self.children(form, |resolver, child| {
                    resolver.substitute_view_inner(child, environment, depth + 1)
                })?;
                self.intern(form)?
            }
        };
        self.admit_roots(&[result], environment.types.caller, false)?;
        let mut count = 1usize
            .checked_add(environment.types.ordinary.len())
            .and_then(|n| n.checked_add(environment.types.witnesses.len()))
            .and_then(|n| n.checked_add(environment.scopes.len().checked_mul(2)?))
            .ok_or(Failure::Capacity)?;
        for operand in environment
            .types
            .witnesses
            .values()
            .chain(environment.types.contract_operand.iter())
        {
            count = count
                .checked_add(self.operand_entries(operand, 0)?)
                .ok_or(Failure::Capacity)?;
        }
        self.reserve_entries(count)?;
        self.view_resolutions
            .entry(ty)
            .or_default()
            .push((environment.clone(), result));
        Ok(result)
    }

    fn select_view(
        &mut self,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
        operand: &ImplementationOperand,
        read_scope: ReadScopeArgument,
        environment: &ReadViewEnvironment,
        depth: usize,
    ) -> Result<TypeObjectDigest> {
        self.step(depth)?;
        self.read_view_member(contract, member)?;
        if self
            .admit_operand(operand, environment.types.caller, depth + 1)?
            .0
            != contract
        {
            return Err(Failure::Authority);
        }
        match operand {
            ImplementationOperand::Parameter { scope, parameter } => {
                if environment.types.caller.context != Context::Lexical(*scope) {
                    return Err(Failure::Authority);
                }
                self.intern(TypeForm::AssociatedReadViewProjection {
                    scope: *scope,
                    parameter: *parameter,
                    contract,
                    member,
                    read_scope,
                })
            }
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                self.admit_bindings(*implementation)?;
                let i = self.implementation(*implementation)?.clone();
                if i.contract != contract
                    || i.type_parameters.len() != type_arguments.len()
                    || i.implementation_parameters.len() != implementations.len()
                {
                    return Err(Failure::Arity);
                }
                let selected = i
                    .associated_types
                    .iter()
                    .find(|b| b.member == member)
                    .ok_or(Failure::Inventory)?;
                let AssociatedTypeBindingValue::ReadViewFamily {
                    scope_parameter,
                    ty,
                } = &selected.value
                else {
                    return Err(Failure::Category);
                };
                let binder = ReadScopeBinder::AssociatedBinding {
                    implementation: *implementation,
                    member,
                };
                self.reserve_entries(
                    1 + i.type_parameters.len() + i.implementation_parameters.len(),
                )?;
                let binding_environment = ReadViewEnvironment {
                    types: Environment {
                        ordinary: i
                            .type_parameters
                            .iter()
                            .copied()
                            .zip(type_arguments.iter().copied())
                            .collect(),
                        source: Context::Lexical(*implementation),
                        caller: environment.types.caller,
                        contract_operand: None,
                        witnesses: i
                            .implementation_parameters
                            .iter()
                            .map(|p| p.id)
                            .zip(implementations.iter().cloned())
                            .collect(),
                    },
                    source_binder: binder,
                    caller_binder: environment.caller_binder,
                    scopes: BTreeMap::from([(
                        ReadScopeReference {
                            binder,
                            parameter: scope_parameter.id,
                        },
                        read_scope,
                    )]),
                };
                self.substitute_view(*ty, &binding_environment, depth + 1)
            }
        }
    }
}
