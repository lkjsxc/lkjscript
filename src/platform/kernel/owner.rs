//! Normalized Graph 7 semantic owner records.

use super::contract::{
    GRAPH_CONTRACT_VERSION, MAXIMUM_CHILDREN, MAXIMUM_DOCUMENTATION_BYTES,
    MAXIMUM_INLINE_TEXT_BYTES, MAXIMUM_RESOURCE_LIMITS,
};
use super::digest::{BlobObjectDigest, OwnerObjectDigest, TypeObjectDigest};
use super::expression::{ExpressionOperation, ExpressionRecord, TextValue};
use super::id::{OwnerHeader, OwnerKey, OwnerKind};
use super::implementation::ImplementationName;
use super::name::Name;
use super::reference::{
    DeclarationReference, OperationReference, PortReference, RequirementReference,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::package::RunnerKind;
use crate::platform::semantic_id::{
    CaseId, DeclarationId, ExpressionId, FieldId, ModuleId, OperationId, ParameterId, PortId,
    RequirementId, TypeParameterId,
};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "owner_kind", content = "record", rename_all = "snake_case")]
pub enum OwnerRecord {
    Module(ModuleRecord),
    Declaration(DeclarationRecord),
    TypeParameter(TypeParameterRecord),
    Field(FieldRecord),
    Case(CaseRecord),
    Operation(OperationRecord),
    Parameter(ParameterRecord),
    Binding(BindingRecord),
    Expression(ExpressionRecord),
    Requirement(RequirementRecord),
    Port(PortRecord),
    Target(TargetRecord),
    Documentation(DocumentationRecord),
    Annotation(AnnotationRecord),
}

impl OwnerRecord {
    pub fn header(&self) -> OwnerHeader {
        match self {
            Self::Module(record) => record.header,
            Self::Declaration(record) => record.header,
            Self::TypeParameter(record) => record.header,
            Self::Field(record) => record.header,
            Self::Case(record) => record.header,
            Self::Operation(record) => record.header,
            Self::Parameter(record) => record.header,
            Self::Binding(record) => record.header,
            Self::Expression(record) => {
                OwnerHeader::new(OwnerKey::Expression(record.id), OwnerKind::Expression)
            }
            Self::Requirement(record) => record.header,
            Self::Port(record) => record.header,
            Self::Target(record) => record.header,
            Self::Documentation(record) => record.header,
            Self::Annotation(record) => record.header,
        }
    }

    pub fn owner(&self) -> OwnerKey {
        self.header().owner
    }

    pub fn kind(&self) -> OwnerKind {
        self.header().kind
    }

    pub fn name(&self) -> Option<&Name> {
        match self {
            Self::Module(value) => Some(&value.name),
            Self::Declaration(value) => Some(&value.name),
            Self::TypeParameter(value) => Some(&value.name),
            Self::Field(value) => Some(&value.name),
            Self::Case(value) => Some(&value.name),
            Self::Operation(value) => Some(&value.name),
            Self::Parameter(value) => Some(&value.name),
            Self::Binding(value) => Some(&value.name),
            Self::Requirement(value) => Some(&value.name),
            Self::Port(value) => Some(&value.name),
            Self::Target(value) => Some(&value.name),
            Self::Expression(_) | Self::Documentation(_) | Self::Annotation(_) => None,
        }
    }

    pub fn name_mut(&mut self) -> Option<&mut Name> {
        match self {
            Self::Module(value) => Some(&mut value.name),
            Self::Declaration(value) => Some(&mut value.name),
            Self::TypeParameter(value) => Some(&mut value.name),
            Self::Field(value) => Some(&mut value.name),
            Self::Case(value) => Some(&mut value.name),
            Self::Operation(value) => Some(&mut value.name),
            Self::Parameter(value) => Some(&mut value.name),
            Self::Binding(value) => Some(&mut value.name),
            Self::Requirement(value) => Some(&mut value.name),
            Self::Port(value) => Some(&mut value.name),
            Self::Target(value) => Some(&mut value.name),
            Self::Expression(_) | Self::Documentation(_) | Self::Annotation(_) => None,
        }
    }

    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
        match self {
            Self::Module(record) => {
                validate_header_domain(record.header, OwnerKind::Module)?;
                validate_names([&record.name])
            }
            Self::Declaration(record) => record.validate_local(),
            Self::TypeParameter(record) => {
                validate_header_domain(record.header, OwnerKind::TypeParameter)?;
                validate_names([&record.name])
            }
            Self::Field(record) => {
                validate_header_domain(record.header, OwnerKind::Field)?;
                validate_names([&record.name])
            }
            Self::Case(record) => {
                validate_header_domain(record.header, OwnerKind::Case)?;
                validate_names([&record.name])
            }
            Self::Operation(record) => record.validate_local(),
            Self::Parameter(record) => {
                validate_header_domain(record.header, OwnerKind::Parameter)?;
                validate_names([&record.name])
            }
            Self::Binding(record) => record.validate_local(),
            Self::Expression(record) => record.validate_local(),
            Self::Requirement(record) => record.validate_local(),
            Self::Port(record) => {
                validate_header_domain(record.header, OwnerKind::Port)?;
                validate_names([&record.name])
            }
            Self::Target(record) => {
                validate_header_domain(record.header, OwnerKind::Target)?;
                validate_names([&record.name])
            }
            Self::Documentation(record) => record.validate_local(),
            Self::Annotation(record) => record.validate_local(),
        }
    }

    pub fn type_roots(&self) -> Vec<TypeObjectDigest> {
        match self {
            Self::Declaration(record) => record.type_roots(),
            Self::Field(record) => vec![record.ty],
            Self::Case(record) => record.payload.into_iter().collect(),
            Self::Operation(record) => vec![record.result],
            Self::Parameter(record) => vec![record.ty],
            Self::Binding(record) => record.declared_type.into_iter().collect(),
            Self::Port(record) => vec![record.function_type],
            Self::Expression(record) => record.type_roots(),
            Self::Module(_)
            | Self::TypeParameter(_)
            | Self::Requirement(_)
            | Self::Target(_)
            | Self::Documentation(_)
            | Self::Annotation(_) => Vec::new(),
        }
    }

    pub fn blob_roots(&self) -> Vec<(BlobObjectDigest, u64)> {
        match self {
            Self::Expression(record) => match &record.operation {
                ExpressionOperation::Text {
                    value: TextValue::Blob { digest, bytes },
                }
                | ExpressionOperation::StaticText {
                    value: TextValue::Blob { digest, bytes },
                } => vec![(*digest, *bytes)],
                _ => Vec::new(),
            },
            Self::Documentation(record) => match &record.content {
                DocumentContent::Blob { digest, bytes } => vec![(*digest, *bytes)],
                DocumentContent::Inline(_) => Vec::new(),
            },
            Self::Module(_)
            | Self::Declaration(_)
            | Self::TypeParameter(_)
            | Self::Field(_)
            | Self::Case(_)
            | Self::Operation(_)
            | Self::Parameter(_)
            | Self::Binding(_)
            | Self::Requirement(_)
            | Self::Port(_)
            | Self::Target(_)
            | Self::Annotation(_) => Vec::new(),
        }
    }

    pub fn expression_roots(&self) -> Vec<ExpressionId> {
        match self {
            Self::Declaration(record) => record.expression_roots(),
            Self::Binding(record) => record.value.into_iter().collect(),
            Self::Port(record) => match &record.implementation {
                PortImplementation::Expression(expression) => vec![*expression],
                PortImplementation::Function(_) => Vec::new(),
            },
            _ => Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerBinding {
    pub kind: OwnerKind,
    pub object: OwnerObjectDigest,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleRecord {
    pub header: OwnerHeader,
    pub name: Name,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeclarationRecord {
    pub header: OwnerHeader,
    pub module: ModuleId,
    pub name: Name,
    pub visibility: DeclarationVisibility,
    pub payload: DeclarationPayload,
}

impl DeclarationRecord {
    fn validate_local(&self) -> Result<(), Diagnostic> {
        validate_header_domain(self.header, self.expected_kind())?;
        validate_names([&self.name])?;
        match &self.payload {
            DeclarationPayload::Record { fields } => {
                validate_sorted_ids("record fields", fields, false)
            }
            DeclarationPayload::Variant { cases } => {
                validate_sorted_ids("variant cases", cases, false)
            }
            DeclarationPayload::Interface { operations } => {
                validate_sorted_ids("interface operations", operations, false)
            }
            DeclarationPayload::External(function) => function.validate_local(),
            DeclarationPayload::Function(function) => function.validate_local(),
            DeclarationPayload::Constant { .. } => Ok(()),
            DeclarationPayload::Component {
                requirements,
                ports,
            } => {
                validate_sorted_ids("component requirements", requirements, true)?;
                validate_sorted_ids("component ports", ports, false)
            }
            DeclarationPayload::Test { .. } => Ok(()),
        }
    }

    pub fn expected_kind(&self) -> OwnerKind {
        match &self.payload {
            DeclarationPayload::Record { .. } => OwnerKind::Record,
            DeclarationPayload::Variant { .. } => OwnerKind::Variant,
            DeclarationPayload::Interface { .. } => OwnerKind::Interface,
            DeclarationPayload::External(_) => OwnerKind::External,
            DeclarationPayload::Function(function) => match &function.effect {
                FunctionEffect::Pure => OwnerKind::PureFunction,
                FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
            },
            DeclarationPayload::Constant { .. } => OwnerKind::Constant,
            DeclarationPayload::Component { .. } => OwnerKind::Component,
            DeclarationPayload::Test { .. } => OwnerKind::Test,
        }
    }

    fn type_roots(&self) -> Vec<TypeObjectDigest> {
        match &self.payload {
            DeclarationPayload::External(function) => {
                vec![function.result]
            }
            DeclarationPayload::Function(function) => vec![function.result],
            DeclarationPayload::Constant { ty, .. } => vec![*ty],
            DeclarationPayload::Record { .. }
            | DeclarationPayload::Variant { .. }
            | DeclarationPayload::Interface { .. }
            | DeclarationPayload::Component { .. }
            | DeclarationPayload::Test { .. } => Vec::new(),
        }
    }

    fn expression_roots(&self) -> Vec<ExpressionId> {
        match &self.payload {
            DeclarationPayload::Function(function) => vec![function.body],
            DeclarationPayload::Constant { value, .. } => vec![*value],
            DeclarationPayload::Test {
                actual, expected, ..
            } => vec![*actual, *expected],
            _ => Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclarationVisibility {
    Private,
    Package,
    Public,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeclarationPayload {
    Record {
        fields: Vec<FieldId>,
    },
    Variant {
        cases: Vec<CaseId>,
    },
    Interface {
        operations: Vec<OperationId>,
    },
    External(ExternalDeclaration),
    Function(FunctionDeclaration),
    Constant {
        ty: TypeObjectDigest,
        value: ExpressionId,
    },
    Component {
        requirements: Vec<RequirementId>,
        ports: Vec<PortId>,
    },
    Test {
        actual: ExpressionId,
        expected: ExpressionId,
        comparison: ComparisonPolicy,
    },
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalDeclaration {
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    pub implementation: ImplementationName,
}

impl ExternalDeclaration {
    fn validate_local(&self) -> Result<(), Diagnostic> {
        validate_ordered_unique("external type parameters", &self.type_parameters, true)?;
        validate_ordered_unique("external parameters", &self.parameters, true)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionDeclaration {
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    pub effect: FunctionEffect,
    pub body: ExpressionId,
}

impl FunctionDeclaration {
    fn validate_local(&self) -> Result<(), Diagnostic> {
        validate_ordered_unique("function type parameters", &self.type_parameters, true)?;
        validate_ordered_unique("function parameters", &self.parameters, true)?;
        if let FunctionEffect::Task { requirements } = &self.effect {
            if !self.type_parameters.is_empty() {
                return Err(owner_error(
                    "kernel_owner_generic_task",
                    "task functions cannot declare type parameters",
                ));
            }
            validate_sorted_ids("task requirements", requirements, true)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FunctionEffect {
    Pure,
    Task {
        requirements: Vec<RequirementReference>,
    },
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonPolicy {
    Exact,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypeParameterRecord {
    pub header: OwnerHeader,
    pub declaration: DeclarationId,
    pub name: Name,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldRecord {
    pub header: OwnerHeader,
    pub declaration: DeclarationId,
    pub name: Name,
    pub ty: TypeObjectDigest,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaseRecord {
    pub header: OwnerHeader,
    pub declaration: DeclarationId,
    pub name: Name,
    pub payload: Option<TypeObjectDigest>,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationRecord {
    pub header: OwnerHeader,
    pub declaration: DeclarationId,
    pub name: Name,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    pub idempotency: Idempotency,
    pub external_visibility: ExternalVisibility,
}

impl OperationRecord {
    fn validate_local(&self) -> Result<(), Diagnostic> {
        validate_header_domain(self.header, OwnerKind::Operation)?;
        validate_names([&self.name])?;
        validate_ordered_unique("operation parameters", &self.parameters, true)
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Idempotency {
    Idempotent,
    IdempotentWithKey,
    NonIdempotent,
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalVisibility {
    None,
    Possible,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterRecord {
    pub header: OwnerHeader,
    pub parent: ParameterParent,
    pub name: Name,
    pub ty: TypeObjectDigest,
    pub use_mode: ParameterUse,
    pub resource_requirement: Option<RequirementReference>,
}

#[derive(Clone, Copy, Debug, Default, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterUse {
    #[default]
    Unrestricted,
    Borrow,
    Consume,
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum ParameterParent {
    Function(DeclarationId),
    Operation(OperationId),
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BindingRecord {
    pub header: OwnerHeader,
    pub name: Name,
    pub kind: BindingKind,
    pub value: Option<ExpressionId>,
    pub declared_type: Option<TypeObjectDigest>,
}

impl BindingRecord {
    fn validate_local(&self) -> Result<(), Diagnostic> {
        validate_header_domain(self.header, OwnerKind::Binding)?;
        validate_names([&self.name])?;
        if matches!(self.kind, BindingKind::Let) != self.value.is_some() {
            return Err(owner_error(
                "kernel_binding_value",
                "only a let binding owns a value expression",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingKind {
    Let,
    MatchPayload,
    Transaction,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementRecord {
    pub header: OwnerHeader,
    pub declaration: DeclarationId,
    pub name: Name,
    pub interface: DeclarationReference,
    pub operations: Vec<OperationReference>,
    pub limits: Vec<ResourceLimit>,
}

impl RequirementRecord {
    fn validate_local(&self) -> Result<(), Diagnostic> {
        validate_header_domain(self.header, OwnerKind::Requirement)?;
        validate_names([&self.name])?;
        validate_sorted_ids("requirement operations", &self.operations, true)?;
        if self.limits.len() > MAXIMUM_RESOURCE_LIMITS
            || self
                .limits
                .windows(2)
                .any(|pair| pair[0].name >= pair[1].name)
        {
            return Err(owner_error(
                "kernel_requirement_limits",
                "resource limits must be within bounds and strictly ordered by name",
            ));
        }
        if self.limits.iter().any(|limit| limit.maximum == 0) {
            return Err(owner_error(
                "kernel_requirement_limit_zero",
                "resource limit maximum must be nonzero",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimit {
    pub name: Name,
    pub maximum: u64,
    pub unit: ResourceUnit,
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceUnit {
    Bytes,
    Items,
    Calls,
    Tasks,
    Milliseconds,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PortRecord {
    pub header: OwnerHeader,
    pub declaration: DeclarationId,
    pub name: Name,
    pub function_type: TypeObjectDigest,
    pub implementation: PortImplementation,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PortImplementation {
    Expression(ExpressionId),
    Function(DeclarationReference),
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetRecord {
    pub header: OwnerHeader,
    pub name: Name,
    pub component: DeclarationReference,
    pub port: PortReference,
    pub runner: RunnerKind,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentationRecord {
    pub header: OwnerHeader,
    pub owner: OwnerKey,
    pub class: DocumentationClass,
    pub content: DocumentContent,
}

impl DocumentationRecord {
    fn validate_local(&self) -> Result<(), Diagnostic> {
        validate_header_domain(self.header, OwnerKind::Documentation)?;
        match &self.content {
            DocumentContent::Inline(text)
                if text.len() <= MAXIMUM_INLINE_TEXT_BYTES
                    && text.len() <= MAXIMUM_DOCUMENTATION_BYTES =>
            {
                Ok(())
            }
            DocumentContent::Inline(text) => Err(owner_error(
                "kernel_documentation_inline_limit",
                format!("inline documentation has {} bytes", text.len()),
            )),
            DocumentContent::Blob { bytes: 0, .. } => Err(owner_error(
                "kernel_documentation_blob_length",
                "documentation blob must contain at least one byte",
            )),
            DocumentContent::Blob { bytes, .. } if *bytes <= MAXIMUM_DOCUMENTATION_BYTES as u64 => {
                Ok(())
            }
            DocumentContent::Blob { .. } => Err(owner_error(
                "kernel_documentation_limit",
                "documentation exceeds the hostile-decoder bound",
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentationClass {
    Semantic,
    Nonsemantic,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "storage", content = "value", rename_all = "snake_case")]
pub enum DocumentContent {
    Inline(String),
    Blob {
        digest: BlobObjectDigest,
        bytes: u64,
    },
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnnotationRecord {
    pub header: OwnerHeader,
    pub owner: OwnerKey,
    pub class: AnnotationClass,
    pub key: Name,
    pub value: AnnotationValue,
}

impl AnnotationRecord {
    fn validate_local(&self) -> Result<(), Diagnostic> {
        validate_header_domain(self.header, OwnerKind::Annotation)?;
        validate_names([&self.key])?;
        if let AnnotationValue::Text(value) = &self.value
            && value.len() > MAXIMUM_INLINE_TEXT_BYTES
        {
            return Err(owner_error(
                "kernel_annotation_text_limit",
                "annotation text exceeds the inline value bound",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationClass {
    Semantic,
    Nonsemantic,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum AnnotationValue {
    Bool(bool),
    I64(i64),
    Text(String),
    Name(Name),
}

fn validate_header_domain(header: OwnerHeader, kind: OwnerKind) -> Result<(), Diagnostic> {
    if header.contract_version != GRAPH_CONTRACT_VERSION || header.kind != kind {
        return Err(owner_error(
            "kernel_owner_header",
            "owner header contract or kind does not match its record",
        ));
    }
    let valid_domain = matches!(
        (header.owner, kind),
        (OwnerKey::Module(_), OwnerKind::Module)
            | (OwnerKey::Declaration(_), OwnerKind::Record)
            | (OwnerKey::Declaration(_), OwnerKind::Variant)
            | (OwnerKey::Declaration(_), OwnerKind::Interface)
            | (OwnerKey::Declaration(_), OwnerKind::External)
            | (OwnerKey::Declaration(_), OwnerKind::PureFunction)
            | (OwnerKey::Declaration(_), OwnerKind::TaskFunction)
            | (OwnerKey::Declaration(_), OwnerKind::Constant)
            | (OwnerKey::Declaration(_), OwnerKind::Component)
            | (OwnerKey::Declaration(_), OwnerKind::Test)
            | (OwnerKey::TypeParameter(_), OwnerKind::TypeParameter)
            | (OwnerKey::Field(_), OwnerKind::Field)
            | (OwnerKey::Case(_), OwnerKind::Case)
            | (OwnerKey::Operation(_), OwnerKind::Operation)
            | (OwnerKey::Parameter(_), OwnerKind::Parameter)
            | (OwnerKey::Binding(_), OwnerKind::Binding)
            | (OwnerKey::Expression(_), OwnerKind::Expression)
            | (OwnerKey::Requirement(_), OwnerKind::Requirement)
            | (OwnerKey::Port(_), OwnerKind::Port)
            | (OwnerKey::Target(_), OwnerKind::Target)
            | (OwnerKey::Documentation(_), OwnerKind::Documentation)
            | (OwnerKey::Annotation(_), OwnerKind::Annotation)
    );
    if !valid_domain {
        return Err(owner_error(
            "kernel_owner_identity_domain",
            "owner identity domain does not match its semantic owner kind",
        ));
    }
    Ok(())
}

fn validate_names<'a>(names: impl IntoIterator<Item = &'a Name>) -> Result<(), Diagnostic> {
    for name in names {
        if name.as_str().is_empty() {
            return Err(owner_error("kernel_owner_name", "owner name is empty"));
        }
    }
    Ok(())
}

fn validate_sorted_ids<T: Ord>(
    label: &str,
    values: &[T],
    allow_zero: bool,
) -> Result<(), Diagnostic> {
    if (!allow_zero && values.is_empty())
        || values.len() > MAXIMUM_CHILDREN
        || values.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(owner_error(
            "kernel_owner_child_order",
            format!("{label} must be within bounds and strictly ordered"),
        ));
    }
    Ok(())
}

fn validate_ordered_unique<T: Ord + Copy>(
    label: &str,
    values: &[T],
    allow_zero: bool,
) -> Result<(), Diagnostic> {
    if (!allow_zero && values.is_empty()) || values.len() > MAXIMUM_CHILDREN {
        return Err(owner_error(
            "kernel_owner_child_count",
            format!("{label} count is outside the Graph 7 bound"),
        ));
    }
    let unique = values.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != values.len() {
        return Err(owner_error(
            "kernel_owner_duplicate_child",
            format!("{label} contains a duplicate identity"),
        ));
    }
    Ok(())
}

fn owner_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}
