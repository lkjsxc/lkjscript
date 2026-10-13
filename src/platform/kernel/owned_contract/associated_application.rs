//! Complete application inputs and disposable signature derivation.
//!
//! Function templates remain raw so their owning inference operation can apply
//! effect/requirement substitutions before ordinary and witness replacement.
//! Effect, requirement and capture-safety proofs remain caller obligations.
use super::*;
use crate::platform::kernel::owned_associated::AssociatedTypeContext;
use crate::platform::kernel::parallel_types::{
    AssociatedContractSelection, AssociatedTypeEnvironment, WitnessAppliedTypes,
};

/// Complete written application inputs, before any admission or substitution.
/// Grouping the inventories does not grant scope, category or witness authority.
#[derive(Clone, Copy)]
pub(crate) struct FunctionArguments<'a> {
    pub(crate) types: &'a [TypeObjectDigest],
    pub(crate) implementations: &'a [ImplementationOperand],
    pub(crate) effects: &'a [EffectRow],
    pub(crate) requirements: &'a [RequirementOperand],
}

/// Written read-scope actuals and the caller's existing exact authority.
/// Grouping these borrowed inputs does not perform or bypass admission.
#[derive(Clone, Copy)]
pub(crate) struct ScopeApplication<'a> {
    pub(crate) arguments: &'a [ReadScopeArgument],
    pub(crate) authority:
        crate::platform::kernel::owned_associated::read_view_admission::ScopeAuthority<'a>,
}

pub(crate) struct AdmittedFunctionApplication {
    pub(crate) signature: PackageFunctionSignature,
    pub(crate) parameters: Vec<ParameterRecord>,
    pub(crate) types: BTreeMap<TypeObjectDigest, TypeObject>,
    function: DeclarationReference,
    ordinary: BTreeMap<TypeParameterId, TypeObjectDigest>,
    scopes: BTreeMap<ReadScopeReference, ReadScopeArgument>,
    implementations: BTreeMap<ImplementationParameterId, ImplementationOperand>,
    caller: OwnedTypeOccurrence,
}

impl AdmittedFunctionApplication {
    pub(crate) fn environment(&self) -> AssociatedTypeEnvironment<'_> {
        AssociatedTypeEnvironment {
            ordinary: &self.ordinary,
            scopes: &self.scopes,
            source: AssociatedTypeContext::Lexical(self.function),
            caller: self.caller.associated,
            consumer_generation: self.caller.consumer_generation,
            contract_selection: None,
            implementations: &self.implementations,
        }
    }
}

pub(crate) struct AdmittedMethodApplication {
    pub(crate) method: OwnedMethod,
    pub(crate) types: BTreeMap<TypeObjectDigest, TypeObject>,
}

fn admit_actual_tree(
    read: &(impl ExpressionRead + ?Sized),
    operand: &ImplementationOperand,
    caller: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<(), Diagnostic> {
    admit_operand_metadata(read, operand, associated)?;
    for node in operand.walk() {
        associated.validate_occurrence_at(read, node.type_arguments(), caller)?;
    }
    Ok(())
}

pub(crate) fn admit_function_application(
    read: &(impl ExpressionRead + ?Sized),
    function: DeclarationReference,
    type_actuals: &[TypeObjectDigest],
    operands: &[ImplementationOperand],
    caller: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<AdmittedFunctionApplication, Diagnostic> {
    let loans = BTreeSet::new();
    admit_function_application_with_scopes(
        read,
        function,
        type_actuals,
        operands,
        ScopeApplication {
            arguments: &[],
            authority:
                crate::platform::kernel::owned_associated::read_view_admission::ScopeAuthority {
                    binder: crate::platform::kernel::owned_associated::read_view_admission::binder(
                        caller.associated,
                    ),
                    lexical:
                        crate::platform::kernel::owned_associated::read_view_admission::lexical(
                            caller.associated,
                        ),
                    live_loans: &loans,
                },
        },
        caller,
        associated,
    )
}

pub(crate) fn admit_function_application_with_scopes(
    read: &(impl ExpressionRead + ?Sized),
    function: DeclarationReference,
    type_actuals: &[TypeObjectDigest],
    operands: &[ImplementationOperand],
    scope_application: ScopeApplication<'_>,
    caller: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<AdmittedFunctionApplication, Diagnostic> {
    let ScopeApplication {
        arguments: scope_actuals,
        authority: scope_authority,
    } = scope_application;
    // A sized forwarding view preserves the original reader's authority while
    // permitting the shared witness checker to consume a trait-object reader.
    let empty_types = BTreeMap::new();
    let source_read = AppliedTypeRead {
        read,
        types: &empty_types,
    };
    let read: &dyn ExpressionRead = &source_read;
    caller.validate_context()?;
    let signature = function_contract(read, function)?;
    let scopes = if signature.read_scope_parameters.is_empty() && scope_actuals.is_empty() {
        BTreeMap::new()
    } else {
        crate::platform::kernel::owned_associated::read_view_admission::scope_bindings(
            read,
            ReadScopeBinder::Declaration(function),
            scope_actuals,
            scope_authority,
            associated,
        )?
    };
    let source = defining_occurrence(read, Some(function))?;
    associated.reserve::<PackageFunctionSignature>(1)?;
    associated.reserve::<TypeParameterId>(signature.type_parameters.len())?;
    associated.reserve::<crate::platform::semantic_id::ParameterId>(signature.parameters.len())?;
    associated.reserve::<crate::platform::semantic_id::EffectParameterId>(
        signature.effect_parameters.len(),
    )?;
    associated.reserve::<crate::platform::semantic_id::RequirementParameterId>(
        signature.requirement_parameters.len(),
    )?;
    associated.reserve::<ImplementationParameter>(signature.implementation_parameters.len())?;
    for formal in &signature.implementation_parameters {
        associated.reserve_bytes(formal.name.as_str().len())?;
        associated.reserve::<TypeObjectDigest>(formal.type_arguments.len())?;
    }
    if signature.type_parameters.len() != type_actuals.len()
        || signature.implementation_parameters.len() != operands.len()
    {
        return Err(reject(
            "function application requires its complete ordinary and witness inventories",
        ));
    }
    associated.reserve::<ParameterRecord>(signature.parameters.len())?;
    let mut parameters = Vec::with_capacity(signature.parameters.len());
    let mut parameter_ids = BTreeSet::new();
    associated.reserve::<crate::platform::semantic_id::ParameterId>(signature.parameters.len())?;
    for id in &signature.parameters {
        read.validation_work()?;
        let parameter = parameter(read, function.package, *id)?;
        if !parameter_ids.insert(*id)
            || parameter.header.owner != OwnerKey::Parameter(*id)
            || parameter.header.kind != OwnerKind::Parameter
            || parameter.parent != ParameterParent::Function(function.declaration)
        {
            return Err(reject(
                "function parameter is outside its exact ordered source inventory",
            ));
        }
        // Both owners consume the raw signature before any application can erase
        // an associated leaf or turn its result into a concrete older type.
        associated.validate_occurrence_at(read, &[parameter.ty], source)?;
        associated.validate_occurrence_at(
            read,
            &[parameter.ty],
            OwnedTypeOccurrence {
                consumer_generation: parameter.header.contract_version,
                ..source
            },
        )?;
        associated.reserve_bytes(parameter.name.as_str().len())?;
        parameters.push(parameter);
    }
    associated.validate_occurrence_at(read, &[signature.result.ty], source)?;
    validate_witness_parameters(
        read,
        &signature.implementation_parameters,
        function,
        associated,
    )?;
    associated.validate_occurrence_at(read, type_actuals, caller)?;
    associated.reserve::<(TypeParameterId, TypeObjectDigest)>(type_actuals.len())?;
    let mut ordinary = BTreeMap::new();
    for (id, actual) in signature.type_parameters.iter().zip(type_actuals) {
        read.validation_work()?;
        let parameter = type_parameter_at(read, function, *id)?;
        if ordinary.insert(*id, *actual).is_some() {
            return Err(reject("function has a duplicate exact ordinary formal"));
        }
        validate_type_argument_constraints_at(
            read,
            parameter.constraints,
            *actual,
            caller,
            associated,
        )?;
    }
    for operand in operands {
        admit_actual_tree(read, operand, caller, associated)?;
    }
    let mut admission = WitnessAdmission::new(associated);
    let types = validate_witness_arguments_with_scopes(
        read,
        function,
        &signature.implementation_parameters,
        &ordinary,
        &scopes,
        operands,
        caller,
        &mut admission,
    )?;
    associated.reserve::<(ImplementationParameterId, ImplementationOperand)>(operands.len())?;
    let implementations = signature
        .implementation_parameters
        .iter()
        .zip(operands)
        .map(|(formal, operand)| (formal.id, operand.clone()))
        .collect();
    Ok(AdmittedFunctionApplication {
        signature,
        parameters,
        types,
        function,
        ordinary,
        scopes,
        implementations,
        caller,
    })
}

pub(crate) fn admit_method_application(
    read: &(impl ExpressionRead + ?Sized),
    operand: &ImplementationOperand,
    reference: DeclarationReference,
    id: MethodId,
    caller: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<AdmittedMethodApplication, Diagnostic> {
    let loans = BTreeSet::new();
    admit_method_application_with_scopes(
        read,
        operand,
        reference,
        id,
        ScopeApplication {
            arguments: &[],
            authority:
                crate::platform::kernel::owned_associated::read_view_admission::ScopeAuthority {
                    binder: crate::platform::kernel::owned_associated::read_view_admission::binder(
                        caller.associated,
                    ),
                    lexical: caller.associated.scope(),
                    live_loans: &loans,
                },
        },
        caller,
        associated,
    )
}

pub(crate) fn admit_method_application_with_scopes(
    read: &(impl ExpressionRead + ?Sized),
    operand: &ImplementationOperand,
    reference: DeclarationReference,
    id: MethodId,
    scope_application: ScopeApplication<'_>,
    caller: OwnedTypeOccurrence,
    associated: &OwnedAssociatedAdmission,
) -> Result<AdmittedMethodApplication, Diagnostic> {
    let ScopeApplication {
        arguments: scope_actuals,
        authority: scope_authority,
    } = scope_application;
    let empty_types = BTreeMap::new();
    let source_read = AppliedTypeRead {
        read,
        types: &empty_types,
    };
    let read: &dyn ExpressionRead = &source_read;
    caller.validate_context()?;
    admit_actual_tree(read, operand, caller, associated)?;
    let mut admission = WitnessAdmission::new(associated);
    let application =
        witness_contract_scoped(read, operand, caller.parameters.scope(), &mut admission)?;
    if application.contract != reference {
        return Err(reject(
            "selected method witness names a different exact contract",
        ));
    }
    let contract = contract_record(read, reference)?;
    validate_contract_at_with(read, reference, &contract, associated)?;
    let mut method = contract
        .methods
        .into_iter()
        .find(|method| method.id == id)
        .ok_or_else(|| reject("method is absent from the exact selected contract"))?;
    associated.reserve::<OwnedMethod>(1)?;
    associated.reserve::<OwnedMethodParameter>(method.parameters.len())?;
    associated.reserve_bytes(method.name.as_str().len())?;
    let contract = contract_record(read, reference)?;
    associated.reserve::<(TypeParameterId, TypeObjectDigest)>(1)?;
    associated.reserve::<(TypeParameterId, TypeObjectDigest)>(contract.type_parameters.len())?;
    let ordinary = contract_bindings(
        &contract,
        application.self_type,
        &application.type_arguments,
    );
    let implementations = BTreeMap::new();
    let overlay = AppliedTypeRead {
        read,
        types: &application.types,
    };
    let scopes = if method.read_scope_parameters.is_empty() && scope_actuals.is_empty() {
        BTreeMap::new()
    } else {
        crate::platform::kernel::owned_associated::read_view_admission::scope_bindings(
            read,
            ReadScopeBinder::ContractMethod {
                contract: reference,
                method: id,
            },
            scope_actuals,
            scope_authority,
            associated,
        )?
    };
    let environment = AssociatedTypeEnvironment {
        ordinary: &ordinary,
        scopes: &scopes,
        source: AssociatedTypeContext::ContractMethod {
            contract: reference,
            method: id,
        },
        caller: caller.associated,
        consumer_generation: caller.consumer_generation,
        contract_selection: Some(AssociatedContractSelection::Witness(operand)),
        implementations: &implementations,
    };
    let mut derived = WitnessAppliedTypes::with_admission(&overlay, environment, associated);
    for parameter in &mut method.parameters {
        parameter.ty = derived.substitute(parameter.ty, 0)?;
    }
    method.result.ty = derived.substitute(method.result.ty, 0)?;
    if let ResultRelation::ViewFrom { scope, .. } = &mut method.result.relation {
        *scope = derived.substitute_scope(*scope)?;
    }
    for source in &mut method.read_scope_sources {
        source.scope = derived.substitute_scope(source.scope)?;
    }
    for parameter in &method.parameters {
        associated.validate_occurrence_at(&derived, &[parameter.ty], caller)?;
    }
    associated.validate_occurrence_at(&derived, &[method.result.ty], caller)?;
    let derived_types = derived.into_types();
    let mut types = application.types;
    extend_application_types(read, &mut types, derived_types, associated)?;
    Ok(AdmittedMethodApplication { method, types })
}
