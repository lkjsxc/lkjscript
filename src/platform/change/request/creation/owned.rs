//! Public typed intent for first-order owned contracts. Canonical admission remains mandatory.
use super::*;
use crate::platform::kernel::{
    AssociatedTypeBinding, AssociatedTypeBound, AssociatedTypeMember, ImplementationOperand,
    ImplementationParameter, OwnedContract, OwnedImplementation, OwnedMethod,
    OwnedMethodImplementation, OwnedMethodParameter,
};
use crate::platform::semantic_id::{AssociatedTypeMemberId, ImplementationParameterId, MethodId};

#[cfg(test)]
#[path = "associated_authoring_tests.rs"]
mod associated_authoring_tests;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredAssociatedTypeMember {
    pub id: AssociatedTypeMemberId,
    pub name: Name,
    pub bound: AssociatedTypeBound,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredAssociatedTypeBinding {
    pub member: AssociatedTypeMemberId,
    pub ty: AuthoredType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredOwnedMethod {
    pub id: MethodId,
    pub name: Name,
    pub parameters: Vec<(AuthoredType, ParameterUse)>,
    pub result: AuthoredType,
    pub result_borrow: Option<u32>,
    pub effect: AuthoredFunctionEffect,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredImplementationParameter {
    pub id: ImplementationParameterId,
    pub name: Name,
    pub contract: AuthoredDeclarationReference,
    pub self_type: AuthoredType,
    pub type_arguments: Vec<AuthoredType>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredOwnedMethodImplementation {
    pub method: MethodId,
    pub function: AuthoredDeclarationReference,
    pub type_arguments: Vec<AuthoredType>,
    pub implementations: Vec<AuthoredImplementationOperand>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredImplementationOperand {
    Concrete {
        implementation: AuthoredDeclarationReference,
        type_arguments: Vec<AuthoredType>,
        implementations: Vec<AuthoredImplementationOperand>,
    },
    Parameter {
        scope: AuthoredDeclarationReference,
        parameter: ImplementationParameterId,
    },
}

impl<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized> AuthoredLowerer<'_, B, W> {
    fn admit_associated_inventory(&mut self, count: usize) -> Result<(), Diagnostic> {
        if count == 0 {
            return Ok(());
        }
        self.base.validation_checkpoint()?;
        if count > crate::platform::kernel::contract::MAXIMUM_CHILDREN {
            return Err(request_error(
                DiagnosticClass::Resource,
                "change_authored_associated_capacity",
                "associated inventory exceeds the canonical child limit",
            ));
        }
        let count = u64::try_from(count).map_err(|_| associated_capacity())?;
        let steps = count
            .checked_mul(u64::from(count.checked_ilog2().unwrap_or(0)) + 2)
            .ok_or_else(associated_capacity)?;
        self.work.associated_inventory_steps = self
            .work
            .associated_inventory_steps
            .checked_add(steps)
            .ok_or_else(associated_capacity)?;
        self.work
            .associated_inventory_steps
            .checked_add(self.work.effect_normalization_steps)
            .ok_or_else(associated_capacity)?;
        self.check_budget("associated inventory before allocation")
    }

    fn owned_contract_value(
        &mut self,
        self_type: &AuthoredType,
        type_parameters: &[AuthoredTypeParameterReference],
        methods: &[AuthoredOwnedMethod],
        associated_types: &[AuthoredAssociatedTypeMember],
    ) -> Result<OwnedContract, Diagnostic> {
        self.admit_associated_inventory(associated_types.len())?;
        let mut members = Vec::new();
        members
            .try_reserve_exact(associated_types.len())
            .map_err(|_| associated_capacity())?;
        let mut ids = std::collections::BTreeSet::new();
        let mut names = std::collections::BTreeSet::new();
        for member in associated_types {
            self.base.validation_checkpoint()?;
            if !ids.insert(member.id) || !names.insert(&member.name) {
                return Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_authored_associated_member_duplicate",
                    "associated member identities and names must be distinct",
                ));
            }
            members.push(AssociatedTypeMember {
                id: member.id,
                name: member.name.clone(),
                bound: member.bound,
            });
        }
        let AuthoredType::TypeParameter { parameter } = self_type else {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_owned_self",
                "contract Self must name its owned type parameter",
            ));
        };
        let self_parameter = self.lower_type_parameter_reference(parameter)?;
        let type_parameters = type_parameters
            .iter()
            .map(|parameter| self.lower_type_parameter_reference(parameter))
            .collect::<Result<Vec<_>, _>>()?;
        let mut lowered = Vec::new();
        for method in methods {
            let mut parameters = Vec::new();
            for (ty, use_mode) in &method.parameters {
                parameters.push(OwnedMethodParameter {
                    ty: self.lower_type(ty)?,
                    use_mode: *use_mode,
                });
            }
            lowered.push(OwnedMethod {
                id: method.id,
                name: method.name.clone(),
                parameters,
                result: self.lower_type(&method.result)?,
                result_borrow: method.result_borrow,
                effect: self.lower_effect(&method.effect)?,
            });
        }
        Ok(OwnedContract {
            self_parameter,
            type_parameters,
            methods: lowered,
            associated_types: members,
        })
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "one complete implementation proposal owns these closed fields"
    )]
    fn owned_implementation_value(
        &mut self,
        contract: &AuthoredDeclarationReference,
        type_parameters: &[AuthoredTypeParameterReference],
        implementation_parameters: &[AuthoredImplementationParameter],
        self_type: &AuthoredType,
        type_arguments: &[AuthoredType],
        methods: &[AuthoredOwnedMethodImplementation],
        associated_types: &[AuthoredAssociatedTypeBinding],
    ) -> Result<OwnedImplementation, Diagnostic> {
        self.admit_associated_inventory(associated_types.len())?;
        let mut bindings = Vec::new();
        bindings
            .try_reserve_exact(associated_types.len())
            .map_err(|_| associated_capacity())?;
        for binding in associated_types {
            self.base.validation_checkpoint()?;
            bindings.push(AssociatedTypeBinding {
                member: binding.member,
                ty: self.lower_type(&binding.ty)?,
            });
        }
        if !bindings.is_empty() {
            self.base.validation_checkpoint()?;
            bindings.sort_unstable_by_key(|binding| binding.member);
            self.base.validation_checkpoint()?;
        }
        if bindings
            .windows(2)
            .any(|pair| pair[0].member == pair[1].member)
        {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_authored_associated_binding_duplicate",
                "associated binding identities must be distinct",
            ));
        }
        let contract = self.lower_declaration_reference(contract)?;
        let type_parameters = type_parameters
            .iter()
            .map(|parameter| self.lower_type_parameter_reference(parameter))
            .collect::<Result<Vec<_>, _>>()?;
        let self_type = self.lower_type(self_type)?;
        let implementation_parameters =
            self.implementation_parameters(implementation_parameters)?;
        let type_arguments = type_arguments
            .iter()
            .map(|ty| self.lower_type(ty))
            .collect::<Result<Vec<_>, _>>()?;
        let mut lowered = Vec::new();
        for mapping in methods {
            lowered.push(OwnedMethodImplementation {
                method: mapping.method,
                function: self.lower_declaration_reference(&mapping.function)?,
                type_arguments: mapping
                    .type_arguments
                    .iter()
                    .map(|ty| self.lower_type(ty))
                    .collect::<Result<Vec<_>, _>>()?,
                implementations: mapping
                    .implementations
                    .iter()
                    .map(|operand| self.lower_implementation_operand(operand))
                    .collect::<Result<Vec<_>, _>>()?,
            });
        }
        lowered.sort_by_key(|m| m.method);
        Ok(OwnedImplementation {
            contract,
            type_parameters,
            implementation_parameters,
            self_type,
            type_arguments,
            methods: lowered,
            associated_types: bindings,
        })
    }
    pub(super) fn lower_implementation_operand(
        &mut self,
        operand: &AuthoredImplementationOperand,
    ) -> Result<ImplementationOperand, Diagnostic> {
        Ok(match operand {
            AuthoredImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => ImplementationOperand::Concrete {
                implementation: self.lower_declaration_reference(implementation)?,
                type_arguments: type_arguments
                    .iter()
                    .map(|ty| self.lower_type(ty))
                    .collect::<Result<Vec<_>, _>>()?,
                implementations: implementations
                    .iter()
                    .map(|operand| self.lower_implementation_operand(operand))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            AuthoredImplementationOperand::Parameter { scope, parameter } => {
                ImplementationOperand::Parameter {
                    scope: self.lower_declaration_reference(scope)?,
                    parameter: *parameter,
                }
            }
        })
    }

    fn implementation_parameters(
        &mut self,
        parameters: &[AuthoredImplementationParameter],
    ) -> Result<Vec<ImplementationParameter>, Diagnostic> {
        parameters
            .iter()
            .map(|p| {
                Ok(ImplementationParameter {
                    id: p.id,
                    name: p.name.clone(),
                    contract: self.lower_declaration_reference(&p.contract)?,
                    self_type: self.lower_type(&p.self_type)?,
                    type_arguments: p
                        .type_arguments
                        .iter()
                        .map(|ty| self.lower_type(ty))
                        .collect::<Result<Vec<_>, _>>()?,
                })
            })
            .collect()
    }
}

fn associated_capacity() -> Diagnostic {
    request_error(
        DiagnosticClass::Resource,
        "change_authored_associated_capacity",
        "associated inventory accounting or allocation exhausted",
    )
}

pub(in crate::platform::change::request) fn collect(
    change: &super::super::AuthoredChange,
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    match change {
        super::super::AuthoredChange::CreateOwnedContract { symbol, .. }
        | super::super::AuthoredChange::CreateOwnedImplementation { symbol, .. } => {
            define_symbol(definitions, symbol, SymbolKind::Declaration)
        }
        super::super::AuthoredChange::SetOwnedContract { .. }
        | super::super::AuthoredChange::SetOwnedImplementation { .. }
        | super::super::AuthoredChange::SetImplementationParameters { .. } => Ok(()),
        _ => Err(request_error(
            DiagnosticClass::Corrupt,
            "change_owned_kind",
            "foreign owned intent",
        )),
    }
}

pub(in crate::platform::change::request) fn lower<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    change: &super::super::AuthoredChange,
) -> Result<(), Diagnostic> {
    use super::super::AuthoredChange as C;
    match change {
        C::CreateOwnedContract {
            symbol,
            module,
            name,
            visibility,
            self_type,
            type_parameters,
            methods,
            associated_types,
        } => {
            let declaration = lowerer.declaration_symbol(symbol)?;
            let module = lowerer.resolve_module(module)?;
            let value = lowerer.owned_contract_value(
                self_type,
                type_parameters,
                methods,
                associated_types,
            )?;
            lowerer.insert_created(OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(
                    OwnerKey::Declaration(declaration),
                    OwnerKind::OwnedContract,
                ),
                module,
                name: name.clone(),
                visibility: *visibility,
                payload: DeclarationPayload::OwnedContract(value),
            }))
        }
        C::CreateOwnedImplementation {
            symbol,
            module,
            name,
            visibility,
            contract,
            type_parameters,
            implementation_parameters,
            self_type,
            type_arguments,
            methods,
            associated_types,
        } => {
            let declaration = lowerer.declaration_symbol(symbol)?;
            let module = lowerer.resolve_module(module)?;
            let value = lowerer.owned_implementation_value(
                contract,
                type_parameters,
                implementation_parameters,
                self_type,
                type_arguments,
                methods,
                associated_types,
            )?;
            lowerer.insert_created(OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(
                    OwnerKey::Declaration(declaration),
                    OwnerKind::OwnedImplementation,
                ),
                module,
                name: name.clone(),
                visibility: *visibility,
                payload: DeclarationPayload::OwnedImplementation(value),
            }))
        }
        C::SetOwnedContract {
            declaration,
            self_type,
            type_parameters,
            methods,
            associated_types,
        } => {
            let declaration = lowerer.resolve_declaration(declaration)?;
            let value = lowerer.owned_contract_value(
                self_type,
                type_parameters,
                methods,
                associated_types,
            )?;
            let OwnerRecord::Declaration(d) =
                lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
            else {
                return Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_owned_kind",
                    "contract mutation requires a declaration",
                ));
            };
            if !matches!(d.payload, DeclarationPayload::OwnedContract(_)) {
                return Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_owned_kind",
                    "contract mutation cannot change declaration kind",
                ));
            }
            d.payload = DeclarationPayload::OwnedContract(value);
            Ok(())
        }
        C::SetOwnedImplementation {
            declaration,
            contract,
            type_parameters,
            implementation_parameters,
            self_type,
            type_arguments,
            methods,
            associated_types,
        } => {
            let declaration = lowerer.resolve_declaration(declaration)?;
            let value = lowerer.owned_implementation_value(
                contract,
                type_parameters,
                implementation_parameters,
                self_type,
                type_arguments,
                methods,
                associated_types,
            )?;
            let OwnerRecord::Declaration(d) =
                lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
            else {
                return Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_owned_kind",
                    "implementation mutation requires a declaration",
                ));
            };
            if !matches!(d.payload, DeclarationPayload::OwnedImplementation(_)) {
                return Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_owned_kind",
                    "implementation mutation cannot change declaration kind",
                ));
            }
            if value.requires_prerequisite_generation() {
                d.header.contract_version =
                    crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
            }
            d.payload = DeclarationPayload::OwnedImplementation(value);
            Ok(())
        }
        C::SetImplementationParameters {
            declaration,
            parameters,
        } => {
            let declaration = lowerer.resolve_declaration(declaration)?;
            let lowered = lowerer.implementation_parameters(parameters)?;
            let OwnerRecord::Declaration(d) =
                lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
            else {
                return Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_owned_function",
                    "implementation parameters require a graph function or owned implementation",
                ));
            };
            let target = match &mut d.payload {
                DeclarationPayload::Function(f) => &mut f.implementation_parameters,
                DeclarationPayload::OwnedImplementation(i) => &mut i.implementation_parameters,
                _ => {
                    return Err(request_error(
                        DiagnosticClass::Semantic,
                        "change_owned_function",
                        "implementation parameters require a graph function or owned implementation",
                    ));
                }
            };
            *target = lowered;
            if !target.is_empty() {
                d.header.contract_version =
                    crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
            }
            Ok(())
        }
        _ => Err(request_error(
            DiagnosticClass::Corrupt,
            "change_owned_kind",
            "foreign owned intent",
        )),
    }
}
