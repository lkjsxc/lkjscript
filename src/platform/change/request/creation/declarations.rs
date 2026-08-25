//! Authored builders for Graph 5 declarations, members, targets, and retained attachments.

use super::*;
use crate::platform::kernel::{
    AnnotationClass, AnnotationRecord, AnnotationValue, CaseRecord, DocumentContent,
    DocumentationClass, DocumentationRecord, ExternalDeclaration, ExternalVisibility, FieldRecord,
    Idempotency, OperationRecord, PortImplementation, PortRecord, RequirementRecord, ResourceLimit,
    ResourceUnit, TargetRecord,
};
use crate::platform::package::RunnerKind;
use schemars::JsonSchema;

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.Graph5AuthoredFieldV1")]
#[serde(deny_unknown_fields)]
pub struct AuthoredField {
    #[serde(rename = "as")]
    pub symbol: String,
    pub name: Name,
    pub ty: AuthoredType,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.Graph5AuthoredCaseV1")]
#[serde(deny_unknown_fields)]
pub struct AuthoredCase {
    #[serde(rename = "as")]
    pub symbol: String,
    pub name: Name,
    #[serde(default)]
    pub payload: Option<AuthoredType>,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.Graph5AuthoredOperationV1")]
#[serde(deny_unknown_fields)]
pub struct AuthoredOperation {
    #[serde(rename = "as")]
    pub symbol: String,
    pub name: Name,
    #[serde(default)]
    pub parameters: Vec<AuthoredParameter>,
    pub result: AuthoredType,
    pub idempotency: Idempotency,
    pub external_visibility: ExternalVisibility,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.Graph5AuthoredResourceLimitV1")]
#[serde(deny_unknown_fields)]
pub struct AuthoredResourceLimit {
    pub name: Name,
    pub maximum: u64,
    pub unit: ResourceUnit,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.Graph5AuthoredRequirementV1")]
#[serde(deny_unknown_fields)]
pub struct AuthoredRequirement {
    #[serde(rename = "as")]
    pub symbol: String,
    pub name: Name,
    pub interface: AuthoredDeclarationReference,
    #[serde(default)]
    pub operations: Vec<AuthoredOperationReference>,
    #[serde(default)]
    pub limits: Vec<AuthoredResourceLimit>,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.Graph5AuthoredPortImplementationV1")]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthoredPortImplementation {
    Expression {
        expression: AuthoredExpression,
    },
    Function {
        function: AuthoredDeclarationReference,
    },
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.Graph5AuthoredPortV1")]
#[serde(deny_unknown_fields)]
pub struct AuthoredPort {
    #[serde(rename = "as")]
    pub symbol: String,
    pub name: Name,
    pub function_type: AuthoredType,
    pub implementation: AuthoredPortImplementation,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.Graph5AuthoredAnnotationValueV1")]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AuthoredAnnotationValue {
    Bool(bool),
    I64(i64),
    Text(String),
    Name(Name),
}

pub(in crate::platform::change::request) fn collect_record_symbols(
    symbol: &str,
    fields: &[AuthoredField],
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    define_symbol(definitions, symbol, SymbolKind::Declaration)?;
    for field in fields {
        define_symbol(definitions, &field.symbol, SymbolKind::Field)?;
    }
    Ok(())
}

pub(in crate::platform::change::request) fn collect_variant_symbols(
    symbol: &str,
    cases: &[AuthoredCase],
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    define_symbol(definitions, symbol, SymbolKind::Declaration)?;
    for case in cases {
        define_symbol(definitions, &case.symbol, SymbolKind::Case)?;
    }
    Ok(())
}

pub(in crate::platform::change::request) fn collect_interface_symbols(
    symbol: &str,
    operations: &[AuthoredOperation],
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    define_symbol(definitions, symbol, SymbolKind::Declaration)?;
    for operation in operations {
        define_symbol(definitions, &operation.symbol, SymbolKind::Operation)?;
        for parameter in &operation.parameters {
            define_symbol(
                definitions,
                &parameter.symbol,
                SymbolKind::OperationParameter,
            )?;
        }
    }
    Ok(())
}

pub(in crate::platform::change::request) fn collect_external_symbols(
    symbol: &str,
    type_parameters: &[AuthoredTypeParameter],
    parameters: &[AuthoredParameter],
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    define_symbol(definitions, symbol, SymbolKind::Declaration)?;
    for parameter in type_parameters {
        define_symbol(definitions, &parameter.symbol, SymbolKind::TypeParameter)?;
    }
    for parameter in parameters {
        define_symbol(
            definitions,
            &parameter.symbol,
            SymbolKind::FunctionParameter,
        )?;
    }
    Ok(())
}

pub(in crate::platform::change::request) fn collect_constant_symbols(
    symbol: &str,
    value: &AuthoredExpression,
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    define_symbol(definitions, symbol, SymbolKind::Declaration)?;
    collect_expression_symbols(value, definitions)
}

pub(in crate::platform::change::request) fn collect_component_symbols(
    symbol: &str,
    requirements: &[AuthoredRequirement],
    ports: &[AuthoredPort],
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    define_symbol(definitions, symbol, SymbolKind::Declaration)?;
    for requirement in requirements {
        define_symbol(definitions, &requirement.symbol, SymbolKind::Requirement)?;
    }
    for port in ports {
        define_symbol(definitions, &port.symbol, SymbolKind::Port)?;
        if let AuthoredPortImplementation::Expression { expression } = &port.implementation {
            collect_expression_symbols(expression, definitions)?;
        }
    }
    Ok(())
}

pub(in crate::platform::change::request) fn collect_target_symbols(
    symbol: &str,
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    define_symbol(definitions, symbol, SymbolKind::Target)
}

pub(in crate::platform::change::request) fn collect_documentation_symbols(
    symbol: &str,
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    define_symbol(definitions, symbol, SymbolKind::Documentation)
}

pub(in crate::platform::change::request) fn collect_annotation_symbols(
    symbol: &str,
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    define_symbol(definitions, symbol, SymbolKind::Annotation)
}

#[allow(
    clippy::too_many_arguments,
    reason = "one declaration creation owns these closed fields"
)]
pub(in crate::platform::change::request) fn lower_record<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    symbol: &str,
    module: &ModuleSelector,
    name: &Name,
    visibility: DeclarationVisibility,
    fields: &[AuthoredField],
) -> Result<(), Diagnostic> {
    let declaration = lowerer.declaration_symbol(symbol)?;
    let module = lowerer.resolve_module(module)?;
    let mut field_ids = Vec::with_capacity(fields.len());
    for field in fields {
        let id = lowerer.field_symbol(&field.symbol)?;
        field_ids.push(id);
        let ty = lowerer.lower_type(&field.ty)?;
        lowerer.insert_created(OwnerRecord::Field(FieldRecord {
            header: OwnerHeader::new(OwnerKey::Field(id), OwnerKind::Field),
            declaration,
            name: field.name.clone(),
            ty,
        }))?;
    }
    field_ids.sort_unstable();
    lowerer.insert_created(OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(OwnerKey::Declaration(declaration), OwnerKind::Record),
        module,
        name: name.clone(),
        visibility,
        payload: DeclarationPayload::Record { fields: field_ids },
    }))
}

#[allow(
    clippy::too_many_arguments,
    reason = "one declaration creation owns these closed fields"
)]
pub(in crate::platform::change::request) fn lower_variant<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    symbol: &str,
    module: &ModuleSelector,
    name: &Name,
    visibility: DeclarationVisibility,
    cases: &[AuthoredCase],
) -> Result<(), Diagnostic> {
    let declaration = lowerer.declaration_symbol(symbol)?;
    let module = lowerer.resolve_module(module)?;
    let mut case_ids = Vec::with_capacity(cases.len());
    for case in cases {
        let id = lowerer.case_symbol(&case.symbol)?;
        case_ids.push(id);
        let payload = case
            .payload
            .as_ref()
            .map(|value| lowerer.lower_type(value))
            .transpose()?;
        lowerer.insert_created(OwnerRecord::Case(CaseRecord {
            header: OwnerHeader::new(OwnerKey::Case(id), OwnerKind::Case),
            declaration,
            name: case.name.clone(),
            payload,
        }))?;
    }
    case_ids.sort_unstable();
    lowerer.insert_created(OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(OwnerKey::Declaration(declaration), OwnerKind::Variant),
        module,
        name: name.clone(),
        visibility,
        payload: DeclarationPayload::Variant { cases: case_ids },
    }))
}

#[allow(
    clippy::too_many_arguments,
    reason = "one declaration creation owns these closed fields"
)]
pub(in crate::platform::change::request) fn lower_interface<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    symbol: &str,
    module: &ModuleSelector,
    name: &Name,
    visibility: DeclarationVisibility,
    operations: &[AuthoredOperation],
) -> Result<(), Diagnostic> {
    let declaration = lowerer.declaration_symbol(symbol)?;
    let module = lowerer.resolve_module(module)?;
    let mut operation_ids = Vec::with_capacity(operations.len());
    for operation in operations {
        let id = lowerer.operation_symbol(&operation.symbol)?;
        operation_ids.push(id);
        let mut parameters = Vec::with_capacity(operation.parameters.len());
        for parameter in &operation.parameters {
            let parameter_id = lowerer.operation_parameter_symbol(&parameter.symbol)?;
            parameters.push(parameter_id);
            let ty = lowerer.lower_type(&parameter.ty)?;
            lowerer.insert_created(OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(parameter_id), OwnerKind::Parameter),
                parent: ParameterParent::Operation(id),
                name: parameter.name.clone(),
                ty,
            }))?;
        }
        let result = lowerer.lower_type(&operation.result)?;
        lowerer.insert_created(OwnerRecord::Operation(OperationRecord {
            header: OwnerHeader::new(OwnerKey::Operation(id), OwnerKind::Operation),
            declaration,
            name: operation.name.clone(),
            parameters,
            result,
            idempotency: operation.idempotency,
            external_visibility: operation.external_visibility,
        }))?;
    }
    operation_ids.sort_unstable();
    lowerer.insert_created(OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(OwnerKey::Declaration(declaration), OwnerKind::Interface),
        module,
        name: name.clone(),
        visibility,
        payload: DeclarationPayload::Interface {
            operations: operation_ids,
        },
    }))
}

#[allow(
    clippy::too_many_arguments,
    reason = "one declaration creation owns these closed fields"
)]
pub(in crate::platform::change::request) fn lower_external<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    symbol: &str,
    module: &ModuleSelector,
    name: &Name,
    visibility: DeclarationVisibility,
    type_parameters: &[AuthoredTypeParameter],
    parameters: &[AuthoredParameter],
    result: &AuthoredType,
    implementation: &Name,
) -> Result<(), Diagnostic> {
    let declaration = lowerer.declaration_symbol(symbol)?;
    let module = lowerer.resolve_module(module)?;
    let mut type_parameter_ids = Vec::with_capacity(type_parameters.len());
    for parameter in type_parameters {
        let id = lowerer.type_parameter_symbol(&parameter.symbol)?;
        type_parameter_ids.push(id);
        lowerer.insert_created(OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(id), OwnerKind::TypeParameter),
            declaration,
            name: parameter.name.clone(),
        }))?;
    }
    let mut parameter_ids = Vec::with_capacity(parameters.len());
    for parameter in parameters {
        let id = lowerer.function_parameter_symbol(&parameter.symbol)?;
        parameter_ids.push(id);
        let ty = lowerer.lower_type(&parameter.ty)?;
        lowerer.insert_created(OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter),
            parent: ParameterParent::Function(declaration),
            name: parameter.name.clone(),
            ty,
        }))?;
    }
    let result = lowerer.lower_type(result)?;
    lowerer.insert_created(OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(OwnerKey::Declaration(declaration), OwnerKind::External),
        module,
        name: name.clone(),
        visibility,
        payload: DeclarationPayload::External(ExternalDeclaration {
            type_parameters: type_parameter_ids,
            parameters: parameter_ids,
            result,
            implementation: implementation.clone(),
        }),
    }))
}

#[allow(
    clippy::too_many_arguments,
    reason = "one declaration creation owns these closed fields"
)]
pub(in crate::platform::change::request) fn lower_constant<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    symbol: &str,
    module: &ModuleSelector,
    name: &Name,
    visibility: DeclarationVisibility,
    ty: &AuthoredType,
    value: &AuthoredExpression,
) -> Result<(), Diagnostic> {
    let declaration = lowerer.declaration_symbol(symbol)?;
    let module = lowerer.resolve_module(module)?;
    let ty = lowerer.lower_type(ty)?;
    let value = lowerer.lower_expression(value)?;
    lowerer.insert_created(OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(OwnerKey::Declaration(declaration), OwnerKind::Constant),
        module,
        name: name.clone(),
        visibility,
        payload: DeclarationPayload::Constant { ty, value },
    }))
}

#[allow(
    clippy::too_many_arguments,
    reason = "one declaration creation owns these closed fields"
)]
pub(in crate::platform::change::request) fn lower_component<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    symbol: &str,
    module: &ModuleSelector,
    name: &Name,
    visibility: DeclarationVisibility,
    requirements: &[AuthoredRequirement],
    ports: &[AuthoredPort],
) -> Result<(), Diagnostic> {
    let declaration = lowerer.declaration_symbol(symbol)?;
    let module = lowerer.resolve_module(module)?;
    let mut requirement_ids = Vec::with_capacity(requirements.len());
    for requirement in requirements {
        let id = lowerer.requirement_symbol(&requirement.symbol)?;
        requirement_ids.push(id);
        let interface = lowerer.lower_declaration_reference(&requirement.interface)?;
        let mut operations = requirement
            .operations
            .iter()
            .map(|operation| lowerer.lower_operation_reference(operation))
            .collect::<Result<Vec<_>, _>>()?;
        operations.sort_unstable();
        let mut limits = requirement
            .limits
            .iter()
            .map(|limit| ResourceLimit {
                name: limit.name.clone(),
                maximum: limit.maximum,
                unit: limit.unit,
            })
            .collect::<Vec<_>>();
        limits.sort_by(|left, right| left.name.cmp(&right.name));
        lowerer.insert_created(OwnerRecord::Requirement(RequirementRecord {
            header: OwnerHeader::new(OwnerKey::Requirement(id), OwnerKind::Requirement),
            declaration,
            name: requirement.name.clone(),
            interface,
            operations,
            limits,
        }))?;
    }
    let mut port_ids = Vec::with_capacity(ports.len());
    for port in ports {
        let id = lowerer.port_symbol(&port.symbol)?;
        port_ids.push(id);
        let function_type = lowerer.lower_type(&port.function_type)?;
        let implementation = match &port.implementation {
            AuthoredPortImplementation::Expression { expression } => {
                PortImplementation::Expression(lowerer.lower_expression(expression)?)
            }
            AuthoredPortImplementation::Function { function } => {
                PortImplementation::Function(lowerer.lower_declaration_reference(function)?)
            }
        };
        lowerer.insert_created(OwnerRecord::Port(PortRecord {
            header: OwnerHeader::new(OwnerKey::Port(id), OwnerKind::Port),
            declaration,
            name: port.name.clone(),
            function_type,
            implementation,
        }))?;
    }
    requirement_ids.sort_unstable();
    port_ids.sort_unstable();
    lowerer.insert_created(OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(OwnerKey::Declaration(declaration), OwnerKind::Component),
        module,
        name: name.clone(),
        visibility,
        payload: DeclarationPayload::Component {
            requirements: requirement_ids,
            ports: port_ids,
        },
    }))
}

#[allow(
    clippy::too_many_arguments,
    reason = "one target creation owns these closed fields"
)]
pub(in crate::platform::change::request) fn lower_target<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    symbol: &str,
    name: &Name,
    component: &AuthoredDeclarationReference,
    port: &AuthoredPortReference,
    runner: RunnerKind,
) -> Result<(), Diagnostic> {
    let target = lowerer.target_symbol(symbol)?;
    let component = lowerer.lower_declaration_reference(component)?;
    let port = lowerer.lower_port_reference(port)?;
    lowerer.insert_created(OwnerRecord::Target(TargetRecord {
        header: OwnerHeader::new(OwnerKey::Target(target), OwnerKind::Target),
        name: name.clone(),
        component,
        port,
        runner,
    }))
}

pub(in crate::platform::change::request) fn lower_documentation<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    symbol: &str,
    owner: &super::super::OwnerSelector,
    class: DocumentationClass,
    text: &str,
) -> Result<(), Diagnostic> {
    let documentation = lowerer.documentation_symbol(symbol)?;
    let owner = lowerer.resolve_creation_owner(owner)?;
    lowerer.insert_created(OwnerRecord::Documentation(DocumentationRecord {
        header: OwnerHeader::new(
            OwnerKey::Documentation(documentation),
            OwnerKind::Documentation,
        ),
        owner,
        class,
        content: DocumentContent::Inline(text.to_owned()),
    }))
}

pub(in crate::platform::change::request) fn lower_annotation<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    symbol: &str,
    owner: &super::super::OwnerSelector,
    class: AnnotationClass,
    key: &Name,
    value: &AuthoredAnnotationValue,
) -> Result<(), Diagnostic> {
    let annotation = lowerer.annotation_symbol(symbol)?;
    let owner = lowerer.resolve_creation_owner(owner)?;
    let value = match value {
        AuthoredAnnotationValue::Bool(value) => AnnotationValue::Bool(*value),
        AuthoredAnnotationValue::I64(value) => AnnotationValue::I64(*value),
        AuthoredAnnotationValue::Text(value) => AnnotationValue::Text(value.clone()),
        AuthoredAnnotationValue::Name(value) => AnnotationValue::Name(value.clone()),
    };
    lowerer.insert_created(OwnerRecord::Annotation(AnnotationRecord {
        header: OwnerHeader::new(OwnerKey::Annotation(annotation), OwnerKind::Annotation),
        owner,
        class,
        key: key.clone(),
        value,
    }))
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.Graph5AuthoredPortReferenceV1")]
#[serde(tag = "by", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthoredPortReference {
    Exact {
        package: crate::platform::kernel::PackageId,
        port: crate::platform::semantic_id::PortId,
    },
    Symbol {
        symbol: String,
    },
}
