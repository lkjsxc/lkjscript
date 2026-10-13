//! Complete mapped view-method compatibility from the raw definition templates.
//! A scheme's own binding is opaque data, never an invented concrete witness.
use super::*;
use crate::platform::semantic_id::MethodId;

impl Resolver<'_> {
    pub(super) fn definition_view_type(
        &mut self,
        ty: TypeObjectDigest,
        implementation: DeclarationReference,
        method: MethodId,
        depth: usize,
    ) -> Result<TypeObjectDigest> {
        self.step(depth)?;
        self.admit_bindings(implementation)?;
        let i = self.implementation(implementation)?.clone();
        let c = self.contract(i.contract)?.clone();
        if i.methods.iter().filter(|m| m.method == method).count() != 1
            || c.methods.iter().filter(|m| m.id == method).count() != 1
        {
            return Err(Failure::Inventory);
        }
        self.reserve_entries(
            c.type_parameters
                .len()
                .checked_add(1)
                .ok_or(Failure::Capacity)?,
        )?;
        let ordinary = std::iter::once((c.self_parameter, i.self_type))
            .chain(
                c.type_parameters
                    .iter()
                    .copied()
                    .zip(i.type_arguments.iter().copied()),
            )
            .collect();
        self.definition_view_inner(ty, implementation, method, &ordinary, depth + 1)
    }

    fn definition_view_inner(
        &mut self,
        ty: TypeObjectDigest,
        implementation: DeclarationReference,
        method: MethodId,
        ordinary: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        depth: usize,
    ) -> Result<TypeObjectDigest> {
        self.step(depth)?;
        let i = self.implementation(implementation)?.clone();
        let caller = self.declaration_occurrence(implementation, Context::Lexical(implementation));
        let caller_binder = ReadScopeBinder::ContractMethod {
            contract: i.contract,
            method,
        };
        let result = match self.object(ty)?.form.clone() {
            TypeForm::TypeParameter { parameter } => ordinary
                .get(&parameter)
                .copied()
                .ok_or(Failure::Authority)?,
            TypeForm::AssociatedReadViewMember {
                contract,
                member,
                read_scope,
            } => {
                if contract != i.contract {
                    return Err(Failure::Authority);
                }
                self.read_view_member(contract, member)?;
                self.admit_scope_argument(&read_scope, &caller_binder, caller)?;
                let binding = i
                    .associated_types
                    .iter()
                    .find(|b| b.member == member)
                    .ok_or(Failure::Inventory)?;
                let AssociatedTypeBindingValue::ReadViewFamily {
                    scope_parameter,
                    ty,
                } = &binding.value
                else {
                    return Err(Failure::Category);
                };
                let binder = ReadScopeBinder::AssociatedBinding {
                    implementation,
                    member,
                };
                self.reserve_entries(
                    i.type_parameters.len() + i.implementation_parameters.len() + 1,
                )?;
                let mut types = BTreeMap::new();
                for parameter in &i.type_parameters {
                    types.insert(
                        *parameter,
                        self.intern(TypeForm::TypeParameter {
                            parameter: *parameter,
                        })?,
                    );
                }
                let environment = ReadViewEnvironment {
                    types: Environment {
                        ordinary: types,
                        source: Context::Lexical(implementation),
                        caller,
                        contract_operand: None,
                        witnesses: i
                            .implementation_parameters
                            .iter()
                            .map(|p| {
                                (
                                    p.id,
                                    ImplementationOperand::Parameter {
                                        scope: implementation,
                                        parameter: p.id,
                                    },
                                )
                            })
                            .collect(),
                    },
                    source_binder: binder,
                    caller_binder,
                    scopes: BTreeMap::from([(
                        ReadScopeReference {
                            binder,
                            parameter: scope_parameter.id,
                        },
                        read_scope,
                    )]),
                };
                self.substitute_view(*ty, &environment, depth + 1)?
            }
            TypeForm::AssociatedContractMember { contract, member } => {
                if contract != i.contract {
                    return Err(Failure::Authority);
                }
                self.member(contract, member)?;
                match &i
                    .associated_types
                    .iter()
                    .find(|b| b.member == member)
                    .ok_or(Failure::Inventory)?
                    .value
                {
                    AssociatedTypeBindingValue::Owned { ty } => *ty,
                    _ => return Err(Failure::Category),
                }
            }
            TypeForm::AssociatedWitnessProjection { .. }
            | TypeForm::AssociatedReadViewProjection { .. } => return Err(Failure::Authority),
            TypeForm::ReadViewApplied {
                declaration,
                arguments,
                read_scope,
            } => {
                self.admit_scope_argument(&read_scope, &caller_binder, caller)?;
                self.reserve_entries(arguments.len())?;
                let mut actuals = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    actuals.push(self.definition_view_inner(
                        argument,
                        implementation,
                        method,
                        ordinary,
                        depth + 1,
                    )?);
                }
                self.intern(TypeForm::ReadViewApplied {
                    declaration,
                    arguments: actuals,
                    read_scope,
                })?
            }
            form => {
                let form = self.children(form, |r, child| {
                    r.definition_view_inner(child, implementation, method, ordinary, depth + 1)
                })?;
                self.intern(form)?
            }
        };
        self.admit_roots(&[result], caller, false)?;
        Ok(result)
    }

    pub(in super::super) fn admit_view_method_mapping(
        &mut self,
        implementation: DeclarationReference,
        method: &OwnedMethod,
        mapping: &OwnedMethodImplementation,
    ) -> Result<()> {
        self.step(0)?;
        let i = self.implementation(implementation)?.clone();
        if mapping.method != method.id
            || i.methods.iter().filter(|m| m.method == method.id).count() != 1
            || !i.methods.iter().any(|m| m == mapping)
        {
            return Err(Failure::Inventory);
        }
        let expected = self.read_view_method_signature(i.contract, method)?;
        let actual = self.read_view_function_signature(mapping.function)?;
        let f = Oracle(self.source, Some(mapping.function))
            .function(mapping.function)
            .ok_or(Failure::Authority)?;
        if method.effect != f.effect
            || !f.effect_parameters.is_empty()
            || !f.requirement_parameters.is_empty()
            || mapping.type_arguments.len() != f.type_parameters.len()
            || mapping.implementations.len() != f.implementation_parameters.len()
            || mapping.read_scope_arguments.len() != f.read_scope_parameters.len()
            || expected.parameters.len() != actual.parameters.len()
        {
            return Err(Failure::Arity);
        }
        self.reserve_entries(
            f.type_parameters.len()
                + f.implementation_parameters.len()
                + f.read_scope_parameters.len(),
        )?;
        let caller = self.declaration_occurrence(implementation, Context::Lexical(implementation));
        let caller_binder = ReadScopeBinder::ContractMethod {
            contract: i.contract,
            method: method.id,
        };
        let environment = ReadViewEnvironment {
            types: Environment {
                ordinary: f
                    .type_parameters
                    .iter()
                    .copied()
                    .zip(mapping.type_arguments.iter().copied())
                    .collect(),
                source: Context::Lexical(mapping.function),
                caller,
                contract_operand: None,
                witnesses: f
                    .implementation_parameters
                    .iter()
                    .map(|p| p.id)
                    .zip(mapping.implementations.iter().cloned())
                    .collect(),
            },
            source_binder: actual.binder,
            caller_binder,
            scopes: f
                .read_scope_parameters
                .iter()
                .zip(&mapping.read_scope_arguments)
                .map(|(p, scope)| {
                    (
                        ReadScopeReference {
                            binder: actual.binder,
                            parameter: p.id,
                        },
                        *scope,
                    )
                })
                .collect(),
        };
        for ((expected_ty, expected_mode), (actual_ty, actual_mode)) in
            expected.parameters.iter().zip(&actual.parameters)
        {
            self.step(0)?;
            if expected_mode != actual_mode
                || self.definition_view_type(*expected_ty, implementation, method.id, 0)?
                    != self.substitute_view(*actual_ty, &environment, 0)?
            {
                return Err(Failure::Authority);
            }
        }
        if self.definition_view_type(expected.result.ty, implementation, method.id, 0)?
            != self.substitute_view(actual.result.ty, &environment, 0)?
        {
            return Err(Failure::Authority);
        }
        let actual_relation = match actual.result.relation {
            ResultRelation::Value => ResultRelation::Value,
            ResultRelation::ReadFrom { source } => ResultRelation::ReadFrom { source },
            ResultRelation::ViewFrom { source, scope } => ResultRelation::ViewFrom {
                source,
                scope: self.substitute_scope(
                    &scope,
                    &environment,
                    &actual.binder,
                    &caller_binder,
                )?,
            },
        };
        if expected.result.relation != actual_relation
            || expected.source_scopes.len() != actual.source_scopes.len()
        {
            return Err(Failure::Authority);
        }
        for ((expected_source, expected_scope), (actual_source, actual_scope)) in
            expected.source_scopes.iter().zip(&actual.source_scopes)
        {
            if expected_source != actual_source
                || *expected_scope
                    != self.substitute_scope(
                        actual_scope,
                        &environment,
                        &actual.binder,
                        &caller_binder,
                    )?
            {
                return Err(Failure::Authority);
            }
        }
        Ok(())
    }
}
