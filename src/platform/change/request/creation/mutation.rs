//! Typed additions and contract updates for existing Graph 5 owners.

use super::*;
use crate::platform::kernel::{
    CaseRecord, FieldRecord, ImplementationName, OperationRecord, PortImplementation, PortRecord,
    RequirementRecord, ResourceLimit, TypeParameterRecord,
};

pub(in crate::platform::change::request) fn collect_mutation_symbols(
    change: &super::super::AuthoredChange,
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    use super::super::AuthoredChange;
    match change {
        AuthoredChange::AddField { field, .. } => {
            define_symbol(definitions, &field.symbol, SymbolKind::Field)
        }
        AuthoredChange::AddCase { case, .. } => {
            define_symbol(definitions, &case.symbol, SymbolKind::Case)
        }
        AuthoredChange::AddOperation { operation, .. } => {
            define_symbol(definitions, &operation.symbol, SymbolKind::Operation)?;
            for parameter in &operation.parameters {
                define_symbol(
                    definitions,
                    &parameter.symbol,
                    SymbolKind::OperationParameter,
                )?;
            }
            Ok(())
        }
        AuthoredChange::AddTypeParameter { parameter, .. } => {
            define_symbol(definitions, &parameter.symbol, SymbolKind::TypeParameter)
        }
        AuthoredChange::AddParameter {
            parent, parameter, ..
        } => define_symbol(
            definitions,
            &parameter.symbol,
            match parent {
                ParameterParentSelector::Declaration { .. } => SymbolKind::FunctionParameter,
                ParameterParentSelector::Operation { .. } => SymbolKind::OperationParameter,
            },
        ),
        AuthoredChange::AddRequirement { requirement, .. } => {
            define_symbol(definitions, &requirement.symbol, SymbolKind::Requirement)
        }
        AuthoredChange::AddPort { port, .. } => {
            define_symbol(definitions, &port.symbol, SymbolKind::Port)?;
            if let AuthoredPortImplementation::Expression { expression } = &port.implementation {
                collect_expression_symbols(expression, definitions)?;
            }
            Ok(())
        }
        AuthoredChange::SetDeclarationVisibility { .. }
        | AuthoredChange::SetFunctionContract { .. }
        | AuthoredChange::SetExternalContract { .. }
        | AuthoredChange::SetFieldType { .. }
        | AuthoredChange::SetCasePayload { .. }
        | AuthoredChange::SetParameterType { .. }
        | AuthoredChange::SetOperationContract { .. }
        | AuthoredChange::SetRequirementContract { .. }
        | AuthoredChange::SetTarget { .. } => Ok(()),
        _ => Err(mutation_corrupt(
            "change_mutation_symbol_dispatch",
            "non-mutation change reached mutation symbol collection",
        )),
    }
}

pub(in crate::platform::change::request) fn lower_mutation<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    change: &super::super::AuthoredChange,
) -> Result<(), Diagnostic> {
    use super::super::AuthoredChange;
    match change {
        AuthoredChange::AddField { record, field } => lower_add_field(lowerer, record, field),
        AuthoredChange::AddCase { variant, case } => lower_add_case(lowerer, variant, case),
        AuthoredChange::AddOperation {
            interface,
            operation,
        } => lower_add_operation(lowerer, interface, operation),
        AuthoredChange::AddTypeParameter {
            declaration,
            parameter,
        } => lower_add_type_parameter(lowerer, declaration, parameter),
        AuthoredChange::AddParameter { parent, parameter } => {
            lower_add_parameter(lowerer, parent, parameter)
        }
        AuthoredChange::AddRequirement {
            component,
            requirement,
        } => lower_add_requirement(lowerer, component, requirement),
        AuthoredChange::AddPort { component, port } => lower_add_port(lowerer, component, port),
        AuthoredChange::SetDeclarationVisibility {
            declaration,
            visibility,
        } => lower_set_visibility(lowerer, declaration, *visibility),
        AuthoredChange::SetFunctionContract {
            function,
            result,
            effect,
        } => lower_set_function_contract(lowerer, function, result, effect),
        AuthoredChange::SetExternalContract {
            external,
            result,
            implementation,
        } => lower_set_external_contract(lowerer, external, result, implementation),
        AuthoredChange::SetFieldType { field, ty } => {
            let ty = lowerer.lower_type(ty)?;
            let owner = lowerer.resolve_owner(field)?;
            let OwnerRecord::Field(record) = lowerer.candidate_mut(owner)? else {
                return Err(mutation_kind("field", owner));
            };
            record.ty = ty;
            Ok(())
        }
        AuthoredChange::SetCasePayload { case, payload } => {
            let payload = payload
                .as_ref()
                .map(|payload| lowerer.lower_type(payload))
                .transpose()?;
            let owner = lowerer.resolve_owner(case)?;
            let OwnerRecord::Case(record) = lowerer.candidate_mut(owner)? else {
                return Err(mutation_kind("variant case", owner));
            };
            record.payload = payload;
            Ok(())
        }
        AuthoredChange::SetParameterType { parameter, ty } => {
            let ty = lowerer.lower_type(ty)?;
            let owner = lowerer.resolve_owner(parameter)?;
            let OwnerRecord::Parameter(record) = lowerer.candidate_mut(owner)? else {
                return Err(mutation_kind("parameter", owner));
            };
            record.ty = ty;
            Ok(())
        }
        AuthoredChange::SetOperationContract {
            operation,
            result,
            idempotency,
            external_visibility,
        } => {
            let result = lowerer.lower_type(result)?;
            let owner = lowerer.resolve_owner(operation)?;
            let OwnerRecord::Operation(record) = lowerer.candidate_mut(owner)? else {
                return Err(mutation_kind("interface operation", owner));
            };
            record.result = result;
            record.idempotency = *idempotency;
            record.external_visibility = *external_visibility;
            Ok(())
        }
        AuthoredChange::SetRequirementContract {
            requirement,
            interface,
            operations,
            limits,
        } => lower_set_requirement_contract(lowerer, requirement, interface, operations, limits),
        AuthoredChange::SetTarget {
            target,
            component,
            port,
            runner,
        } => lower_set_target(lowerer, target, component, port, *runner),
        _ => Err(mutation_corrupt(
            "change_mutation_dispatch",
            "non-mutation change reached mutation lowering",
        )),
    }
}

fn lower_add_field<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    selector: &DeclarationSelector,
    field: &AuthoredField,
) -> Result<(), Diagnostic> {
    let declaration = lowerer.resolve_declaration(selector)?;
    let field_id = lowerer.field_symbol(&field.symbol)?;
    let ty = lowerer.lower_type(&field.ty)?;
    let OwnerRecord::Declaration(parent) =
        lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
    else {
        return Err(mutation_kind(
            "record declaration",
            OwnerKey::Declaration(declaration),
        ));
    };
    let DeclarationPayload::Record { fields } = &mut parent.payload else {
        return Err(mutation_kind("record declaration", parent.header.owner));
    };
    fields.push(field_id);
    fields.sort_unstable();
    lowerer.insert_created(OwnerRecord::Field(FieldRecord {
        header: OwnerHeader::new(OwnerKey::Field(field_id), OwnerKind::Field),
        declaration,
        name: field.name.clone(),
        ty,
    }))
}

fn lower_add_case<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    selector: &DeclarationSelector,
    case: &AuthoredCase,
) -> Result<(), Diagnostic> {
    let declaration = lowerer.resolve_declaration(selector)?;
    let case_id = lowerer.case_symbol(&case.symbol)?;
    let payload = case
        .payload
        .as_ref()
        .map(|payload| lowerer.lower_type(payload))
        .transpose()?;
    let OwnerRecord::Declaration(parent) =
        lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
    else {
        return Err(mutation_kind(
            "variant declaration",
            OwnerKey::Declaration(declaration),
        ));
    };
    let DeclarationPayload::Variant { cases } = &mut parent.payload else {
        return Err(mutation_kind("variant declaration", parent.header.owner));
    };
    cases.push(case_id);
    cases.sort_unstable();
    lowerer.insert_created(OwnerRecord::Case(CaseRecord {
        header: OwnerHeader::new(OwnerKey::Case(case_id), OwnerKind::Case),
        declaration,
        name: case.name.clone(),
        payload,
    }))
}

fn lower_add_operation<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    selector: &DeclarationSelector,
    operation: &AuthoredOperation,
) -> Result<(), Diagnostic> {
    let declaration = lowerer.resolve_declaration(selector)?;
    let operation_id = lowerer.operation_symbol(&operation.symbol)?;
    let mut parameters = Vec::with_capacity(operation.parameters.len());
    for parameter in &operation.parameters {
        let parameter_id = lowerer.operation_parameter_symbol(&parameter.symbol)?;
        let ty = lowerer.lower_type(&parameter.ty)?;
        parameters.push(parameter_id);
        lowerer.insert_created(OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(parameter_id), OwnerKind::Parameter),
            parent: ParameterParent::Operation(operation_id),
            name: parameter.name.clone(),
            ty,
        }))?;
    }
    let result = lowerer.lower_type(&operation.result)?;
    let OwnerRecord::Declaration(parent) =
        lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
    else {
        return Err(mutation_kind(
            "interface declaration",
            OwnerKey::Declaration(declaration),
        ));
    };
    let DeclarationPayload::Interface { operations } = &mut parent.payload else {
        return Err(mutation_kind("interface declaration", parent.header.owner));
    };
    operations.push(operation_id);
    operations.sort_unstable();
    lowerer.insert_created(OwnerRecord::Operation(OperationRecord {
        header: OwnerHeader::new(OwnerKey::Operation(operation_id), OwnerKind::Operation),
        declaration,
        name: operation.name.clone(),
        parameters,
        result,
        idempotency: operation.idempotency,
        external_visibility: operation.external_visibility,
    }))
}

fn lower_add_type_parameter<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    selector: &DeclarationSelector,
    parameter: &AuthoredTypeParameter,
) -> Result<(), Diagnostic> {
    let declaration = lowerer.resolve_declaration(selector)?;
    let parameter_id = lowerer.type_parameter_symbol(&parameter.symbol)?;
    let OwnerRecord::Declaration(parent) =
        lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
    else {
        return Err(mutation_kind(
            "function or external declaration",
            OwnerKey::Declaration(declaration),
        ));
    };
    match &mut parent.payload {
        DeclarationPayload::Function(function) => function.type_parameters.push(parameter_id),
        DeclarationPayload::External(function) => function.type_parameters.push(parameter_id),
        _ => {
            return Err(mutation_kind(
                "function or external declaration",
                parent.header.owner,
            ));
        }
    }
    lowerer.insert_created(OwnerRecord::TypeParameter(TypeParameterRecord {
        header: OwnerHeader::new(
            OwnerKey::TypeParameter(parameter_id),
            OwnerKind::TypeParameter,
        ),
        declaration,
        name: parameter.name.clone(),
    }))
}

fn lower_add_parameter<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    selector: &ParameterParentSelector,
    parameter: &AuthoredParameter,
) -> Result<(), Diagnostic> {
    let ty = lowerer.lower_type(&parameter.ty)?;
    let (parameter_id, parent) = match selector {
        ParameterParentSelector::Declaration { declaration } => {
            let declaration = lowerer.resolve_declaration(declaration)?;
            let parameter_id = lowerer.function_parameter_symbol(&parameter.symbol)?;
            let OwnerRecord::Declaration(parent) =
                lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
            else {
                return Err(mutation_kind(
                    "function or external declaration",
                    OwnerKey::Declaration(declaration),
                ));
            };
            match &mut parent.payload {
                DeclarationPayload::Function(function) => function.parameters.push(parameter_id),
                DeclarationPayload::External(function) => function.parameters.push(parameter_id),
                _ => {
                    return Err(mutation_kind(
                        "function or external declaration",
                        parent.header.owner,
                    ));
                }
            }
            (parameter_id, ParameterParent::Function(declaration))
        }
        ParameterParentSelector::Operation { operation } => {
            let owner = lowerer.resolve_owner(operation)?;
            let parameter_id = lowerer.operation_parameter_symbol(&parameter.symbol)?;
            let OwnerKey::Operation(operation_id) = owner else {
                return Err(mutation_kind("interface operation", owner));
            };
            let OwnerRecord::Operation(parent) = lowerer.candidate_mut(owner)? else {
                return Err(mutation_kind("interface operation", owner));
            };
            parent.parameters.push(parameter_id);
            (parameter_id, ParameterParent::Operation(operation_id))
        }
    };
    lowerer.insert_created(OwnerRecord::Parameter(ParameterRecord {
        header: OwnerHeader::new(OwnerKey::Parameter(parameter_id), OwnerKind::Parameter),
        parent,
        name: parameter.name.clone(),
        ty,
    }))
}

fn lower_add_requirement<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    selector: &DeclarationSelector,
    requirement: &AuthoredRequirement,
) -> Result<(), Diagnostic> {
    let declaration = lowerer.resolve_declaration(selector)?;
    let requirement_id = lowerer.requirement_symbol(&requirement.symbol)?;
    let interface = lowerer.lower_declaration_reference(&requirement.interface)?;
    let mut operations = requirement
        .operations
        .iter()
        .map(|operation| lowerer.lower_operation_reference(operation))
        .collect::<Result<Vec<_>, _>>()?;
    operations.sort_unstable();
    let limits = lower_limits(&requirement.limits);
    let OwnerRecord::Declaration(parent) =
        lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
    else {
        return Err(mutation_kind(
            "component declaration",
            OwnerKey::Declaration(declaration),
        ));
    };
    let DeclarationPayload::Component { requirements, .. } = &mut parent.payload else {
        return Err(mutation_kind("component declaration", parent.header.owner));
    };
    requirements.push(requirement_id);
    requirements.sort_unstable();
    lowerer.insert_created(OwnerRecord::Requirement(RequirementRecord {
        header: OwnerHeader::new(
            OwnerKey::Requirement(requirement_id),
            OwnerKind::Requirement,
        ),
        declaration,
        name: requirement.name.clone(),
        interface,
        operations,
        limits,
    }))
}

fn lower_add_port<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    selector: &DeclarationSelector,
    port: &AuthoredPort,
) -> Result<(), Diagnostic> {
    let declaration = lowerer.resolve_declaration(selector)?;
    let port_id = lowerer.port_symbol(&port.symbol)?;
    let function_type = lowerer.lower_type(&port.function_type)?;
    let implementation = match &port.implementation {
        AuthoredPortImplementation::Expression { expression } => {
            PortImplementation::Expression(lowerer.lower_expression(expression)?)
        }
        AuthoredPortImplementation::Function { function } => {
            PortImplementation::Function(lowerer.lower_declaration_reference(function)?)
        }
    };
    let OwnerRecord::Declaration(parent) =
        lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
    else {
        return Err(mutation_kind(
            "component declaration",
            OwnerKey::Declaration(declaration),
        ));
    };
    let DeclarationPayload::Component { ports, .. } = &mut parent.payload else {
        return Err(mutation_kind("component declaration", parent.header.owner));
    };
    ports.push(port_id);
    ports.sort_unstable();
    lowerer.insert_created(OwnerRecord::Port(PortRecord {
        header: OwnerHeader::new(OwnerKey::Port(port_id), OwnerKind::Port),
        declaration,
        name: port.name.clone(),
        function_type,
        implementation,
    }))
}

fn lower_set_visibility<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    selector: &DeclarationSelector,
    visibility: DeclarationVisibility,
) -> Result<(), Diagnostic> {
    let declaration = lowerer.resolve_declaration(selector)?;
    let OwnerRecord::Declaration(record) =
        lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
    else {
        return Err(mutation_kind(
            "declaration",
            OwnerKey::Declaration(declaration),
        ));
    };
    record.visibility = visibility;
    Ok(())
}

fn lower_set_function_contract<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    selector: &DeclarationSelector,
    result: &AuthoredType,
    effect: &AuthoredFunctionEffect,
) -> Result<(), Diagnostic> {
    let declaration = lowerer.resolve_declaration(selector)?;
    let result = lowerer.lower_type(result)?;
    let effect = lowerer.lower_effect(effect)?;
    let OwnerRecord::Declaration(record) =
        lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
    else {
        return Err(mutation_kind(
            "function declaration",
            OwnerKey::Declaration(declaration),
        ));
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        return Err(mutation_kind("function declaration", record.header.owner));
    };
    record.header.kind = match effect {
        FunctionEffect::Pure => OwnerKind::PureFunction,
        FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
    };
    function.result = result;
    function.effect = effect;
    Ok(())
}

fn lower_set_external_contract<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    selector: &DeclarationSelector,
    result: &AuthoredType,
    implementation: &ImplementationName,
) -> Result<(), Diagnostic> {
    let declaration = lowerer.resolve_declaration(selector)?;
    let result = lowerer.lower_type(result)?;
    let OwnerRecord::Declaration(record) =
        lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
    else {
        return Err(mutation_kind(
            "external declaration",
            OwnerKey::Declaration(declaration),
        ));
    };
    let DeclarationPayload::External(external) = &mut record.payload else {
        return Err(mutation_kind("external declaration", record.header.owner));
    };
    external.result = result;
    external.implementation = implementation.clone();
    Ok(())
}

fn lower_set_requirement_contract<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    selector: &OwnerSelector,
    interface: &AuthoredDeclarationReference,
    operations: &[AuthoredOperationReference],
    limits: &[AuthoredResourceLimit],
) -> Result<(), Diagnostic> {
    let interface = lowerer.lower_declaration_reference(interface)?;
    let mut operations = operations
        .iter()
        .map(|operation| lowerer.lower_operation_reference(operation))
        .collect::<Result<Vec<_>, _>>()?;
    operations.sort_unstable();
    let limits = lower_limits(limits);
    let owner = lowerer.resolve_owner(selector)?;
    let OwnerRecord::Requirement(record) = lowerer.candidate_mut(owner)? else {
        return Err(mutation_kind("component requirement", owner));
    };
    record.interface = interface;
    record.operations = operations;
    record.limits = limits;
    Ok(())
}

fn lower_set_target<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    selector: &OwnerSelector,
    component: &AuthoredDeclarationReference,
    port: &AuthoredPortReference,
    runner: crate::platform::package::RunnerKind,
) -> Result<(), Diagnostic> {
    let component = lowerer.lower_declaration_reference(component)?;
    let port = lowerer.lower_port_reference(port)?;
    let owner = lowerer.resolve_owner(selector)?;
    let OwnerRecord::Target(record) = lowerer.candidate_mut(owner)? else {
        return Err(mutation_kind("target", owner));
    };
    record.component = component;
    record.port = port;
    record.runner = runner;
    Ok(())
}

fn lower_limits(limits: &[AuthoredResourceLimit]) -> Vec<ResourceLimit> {
    let mut lowered = limits
        .iter()
        .map(|limit| ResourceLimit {
            name: limit.name.clone(),
            maximum: limit.maximum,
            unit: limit.unit,
        })
        .collect::<Vec<_>>();
    lowered.sort_by(|left, right| left.name.cmp(&right.name));
    lowered
}

fn mutation_kind(expected: &str, owner: OwnerKey) -> Diagnostic {
    request_error(
        DiagnosticClass::Semantic,
        "change_mutation_owner_kind",
        format!("selected owner {owner:?} is not a {expected}"),
    )
}

fn mutation_corrupt(code: &'static str, message: &'static str) -> Diagnostic {
    request_error(DiagnosticClass::Corrupt, code, message)
}
