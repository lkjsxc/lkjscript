//! Public typed intent for first-order owned contracts. Canonical admission remains mandatory.
use super::*;
use crate::platform::kernel::{
    ImplementationOperand, ImplementationParameter, OwnedContract, OwnedImplementation,
    OwnedMethod, OwnedMethodImplementation, OwnedMethodParameter,
};
use crate::platform::semantic_id::{ImplementationParameterId, MethodId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredOwnedMethod {
    pub id: MethodId,
    pub name: Name,
    pub parameters: Vec<(AuthoredType, ParameterUse)>,
    pub result: AuthoredType,
    pub effect: AuthoredFunctionEffect,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredImplementationParameter {
    pub id: ImplementationParameterId,
    pub name: Name,
    pub contract: AuthoredDeclarationReference,
    pub self_type: AuthoredType,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredImplementationOperand {
    Concrete {
        implementation: AuthoredDeclarationReference,
    },
    Parameter {
        function: AuthoredDeclarationReference,
        parameter: ImplementationParameterId,
    },
}

impl<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized> AuthoredLowerer<'_, B, W> {
    fn owned_contract_value(
        &mut self,
        self_type: &AuthoredType,
        methods: &[AuthoredOwnedMethod],
    ) -> Result<OwnedContract, Diagnostic> {
        let AuthoredType::TypeParameter { parameter } = self_type else {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_owned_self",
                "contract Self must name its owned type parameter",
            ));
        };
        let self_parameter = self.lower_type_parameter_reference(parameter)?;
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
                effect: self.lower_effect(&method.effect)?,
            });
        }
        Ok(OwnedContract {
            self_parameter,
            methods: lowered,
        })
    }
    fn owned_implementation_value(
        &mut self,
        contract: &AuthoredDeclarationReference,
        self_type: &AuthoredType,
        methods: &[(MethodId, AuthoredDeclarationReference)],
    ) -> Result<OwnedImplementation, Diagnostic> {
        let contract = self.lower_declaration_reference(contract)?;
        let self_type = self.lower_type(self_type)?;
        let mut lowered = Vec::new();
        for (method, function) in methods {
            lowered.push(OwnedMethodImplementation {
                method: *method,
                function: self.lower_declaration_reference(function)?,
            });
        }
        lowered.sort_by_key(|m| m.method);
        Ok(OwnedImplementation {
            contract,
            self_type,
            methods: lowered,
        })
    }
    pub(super) fn lower_implementation_operand(
        &mut self,
        operand: &AuthoredImplementationOperand,
    ) -> Result<ImplementationOperand, Diagnostic> {
        Ok(match operand {
            AuthoredImplementationOperand::Concrete { implementation } => {
                ImplementationOperand::Concrete {
                    implementation: self.lower_declaration_reference(implementation)?,
                }
            }
            AuthoredImplementationOperand::Parameter {
                function,
                parameter,
            } => ImplementationOperand::Parameter {
                function: self.lower_declaration_reference(function)?,
                parameter: *parameter,
            },
        })
    }
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
            methods,
        } => {
            let declaration = lowerer.declaration_symbol(symbol)?;
            let module = lowerer.resolve_module(module)?;
            let value = lowerer.owned_contract_value(self_type, methods)?;
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
            self_type,
            methods,
        } => {
            let declaration = lowerer.declaration_symbol(symbol)?;
            let module = lowerer.resolve_module(module)?;
            let value = lowerer.owned_implementation_value(contract, self_type, methods)?;
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
            methods,
        } => {
            let declaration = lowerer.resolve_declaration(declaration)?;
            let value = lowerer.owned_contract_value(self_type, methods)?;
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
            self_type,
            methods,
        } => {
            let declaration = lowerer.resolve_declaration(declaration)?;
            let value = lowerer.owned_implementation_value(contract, self_type, methods)?;
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
            d.payload = DeclarationPayload::OwnedImplementation(value);
            Ok(())
        }
        C::SetImplementationParameters {
            declaration,
            parameters,
        } => {
            let declaration = lowerer.resolve_declaration(declaration)?;
            let mut lowered = Vec::new();
            for p in parameters {
                lowered.push(ImplementationParameter {
                    id: p.id,
                    name: p.name.clone(),
                    contract: lowerer.lower_declaration_reference(&p.contract)?,
                    self_type: lowerer.lower_type(&p.self_type)?,
                });
            }
            let OwnerRecord::Declaration(d) =
                lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
            else {
                return Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_owned_function",
                    "implementation parameters require a graph function",
                ));
            };
            let DeclarationPayload::Function(f) = &mut d.payload else {
                return Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_owned_function",
                    "implementation parameters require a graph function",
                ));
            };
            f.implementation_parameters = lowered;
            if !f.implementation_parameters.is_empty() {
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
