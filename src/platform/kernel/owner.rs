//! Normalized Graph 10 semantic owner records.

use super::contract::{
    GRAPH_CONTRACT_VERSION, MAXIMUM_CHILDREN, MAXIMUM_DOCUMENTATION_BYTES,
    MAXIMUM_INLINE_TEXT_BYTES, MAXIMUM_RESOURCE_LIMITS,
};
use super::digest::{BlobObjectDigest, OwnerObjectDigest, TypeObjectDigest};
use super::expression::{ExpressionOperation, ExpressionRecord, TextValue};
use super::id::{OwnerHeader, OwnerKey, OwnerKind, PackageId};
use super::implementation::ImplementationName;
use super::name::Name;
use super::reference::{
    DeclarationReference, OperationReference, PortReference, RequirementReference,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::package::RunnerKind;
use crate::platform::semantic_id::{
    CaseId, DeclarationId, EffectParameterId, ExpressionId, FieldId, ModuleId, OperationId,
    ParameterId, PortId, RequirementId, TargetId, TypeParameterId,
};
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{BorrowDecode, Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "owner_kind", content = "record", rename_all = "snake_case")]
pub enum OwnerRecord {
    Module(ModuleRecord),
    Declaration(DeclarationRecord),
    TypeParameter(TypeParameterRecord),
    EffectParameter(EffectParameterRecord),
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
    HttpRoute(HttpRouteRecord),
    RequirementParameter(RequirementParameterRecord),
}

impl OwnerRecord {
    pub(crate) fn set_encoding_for_edit(&mut self, generation: u16) {
        match self {
            Self::Module(record) => record.header.contract_version = generation,
            Self::Declaration(record) => record.header.contract_version = generation,
            Self::TypeParameter(record) => record.header.contract_version = generation,
            Self::EffectParameter(record) => record.header.contract_version = generation,
            Self::Field(record) => record.header.contract_version = generation,
            Self::Case(record) => record.header.contract_version = generation,
            Self::Operation(record) => record.header.contract_version = generation,
            Self::Parameter(record) => record.header.contract_version = generation,
            Self::Binding(record) => record.header.contract_version = generation,
            Self::Expression(record) => record.contract_version = generation,
            Self::Requirement(record) => record.header.contract_version = generation,
            Self::Port(record) => record.header.contract_version = generation,
            Self::Target(record) => record.header.contract_version = generation,
            Self::Documentation(record) => record.header.contract_version = generation,
            Self::Annotation(record) => record.header.contract_version = generation,
            Self::HttpRoute(record) => record.header.contract_version = generation,
            Self::RequirementParameter(record) => record.header.contract_version = generation,
        }
    }

    pub fn header(&self) -> OwnerHeader {
        match self {
            Self::Module(record) => record.header,
            Self::Declaration(record) => record.header,
            Self::TypeParameter(record) => record.header,
            Self::EffectParameter(record) => record.header,
            Self::RequirementParameter(record) => record.header,
            Self::Field(record) => record.header,
            Self::Case(record) => record.header,
            Self::Operation(record) => record.header,
            Self::Parameter(record) => record.header,
            Self::Binding(record) => record.header,
            Self::Expression(record) => OwnerHeader {
                contract_version: record.contract_version,
                owner: OwnerKey::Expression(record.id),
                kind: OwnerKind::Expression,
            },
            Self::Requirement(record) => record.header,
            Self::Port(record) => record.header,
            Self::Target(record) => record.header,
            Self::HttpRoute(record) => record.header,
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
            Self::EffectParameter(value) => Some(&value.name),
            Self::RequirementParameter(value) => Some(&value.name),
            Self::Field(value) => Some(&value.name),
            Self::Case(value) => Some(&value.name),
            Self::Operation(value) => Some(&value.name),
            Self::Parameter(value) => Some(&value.name),
            Self::Binding(value) => Some(&value.name),
            Self::Requirement(value) => Some(&value.name),
            Self::Port(value) => Some(&value.name),
            Self::Target(value) => Some(&value.name),
            Self::HttpRoute(_) => None,
            Self::Expression(_) | Self::Documentation(_) | Self::Annotation(_) => None,
        }
    }

    pub fn name_mut(&mut self) -> Option<&mut Name> {
        match self {
            Self::Module(value) => Some(&mut value.name),
            Self::Declaration(value) => Some(&mut value.name),
            Self::TypeParameter(value) => Some(&mut value.name),
            Self::EffectParameter(value) => Some(&mut value.name),
            Self::RequirementParameter(value) => Some(&mut value.name),
            Self::Field(value) => Some(&mut value.name),
            Self::Case(value) => Some(&mut value.name),
            Self::Operation(value) => Some(&mut value.name),
            Self::Parameter(value) => Some(&mut value.name),
            Self::Binding(value) => Some(&mut value.name),
            Self::Requirement(value) => Some(&mut value.name),
            Self::Port(value) => Some(&mut value.name),
            Self::Target(value) => Some(&mut value.name),
            Self::HttpRoute(_) => None,
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
            Self::RequirementParameter(record) => {
                validate_header_domain(record.header, OwnerKind::RequirementParameter)?;
                validate_names([&record.name])?;
                record.constraint.validate()
            }
            Self::EffectParameter(record) => {
                validate_header_domain(record.header, OwnerKind::EffectParameter)?;
                validate_names([&record.name])
            }
            Self::TypeParameter(record) => {
                validate_header_domain(record.header, OwnerKind::TypeParameter)?;
                if record.constraints.has_owned() && record.header.contract_version < 18 {
                    return Err(owner_error(
                        "kernel_owned_generation",
                        "owned constraint requires Graph 18",
                    ));
                }
                if record.constraints.requires_transfer() && record.header.contract_version < 22 {
                    return Err(owner_error(
                        "kernel_transfer_constraint_generation",
                        "transferable constraints require Graph 22",
                    ));
                }
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
            Self::HttpRoute(record) => record.validate_local(),
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
            | Self::EffectParameter(_)
            | Self::RequirementParameter(_)
            | Self::Requirement(_)
            | Self::Target(_)
            | Self::HttpRoute(_)
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
            | Self::EffectParameter(_)
            | Self::RequirementParameter(_)
            | Self::Field(_)
            | Self::Case(_)
            | Self::Operation(_)
            | Self::Parameter(_)
            | Self::Binding(_)
            | Self::Requirement(_)
            | Self::Port(_)
            | Self::Target(_)
            | Self::HttpRoute(_)
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
        if self.header.contract_version < 29
            && matches!(&self.payload, DeclarationPayload::OwnedImplementation(i)
                if !i.implementation_parameters.is_empty() || i.methods.iter().any(|m| !m.implementations.is_empty()))
        {
            return Err(super::wire28::implementation_scheme_extension());
        }
        if self.header.contract_version < 28
            && matches!(&self.payload, DeclarationPayload::OwnedImplementation(i)
                if !i.type_parameters.is_empty() || i.methods.iter().any(|m| !m.type_arguments.is_empty()))
        {
            return Err(owner_error(
                "kernel_implementation_scheme_generation",
                "generic implementation schemes and mapped applications require Graph 28",
            ));
        }
        if self.header.contract_version < 27
            && match &self.payload {
                DeclarationPayload::Function(f) => f.result_borrow.is_some(),
                DeclarationPayload::OwnedContract(c) => {
                    c.methods.iter().any(|m| m.result_borrow.is_some())
                }
                _ => false,
            }
        {
            return Err(owner_error(
                "kernel_borrow_result_generation",
                "source-tied borrowed results require Graph 27",
            ));
        }
        if self.header.contract_version < 18
            && (matches!(
                self.payload,
                DeclarationPayload::OwnedContract(_) | DeclarationPayload::OwnedImplementation(_)
            ) || matches!(&self.payload, DeclarationPayload::Function(f) if !f.implementation_parameters.is_empty()))
        {
            return Err(owner_error(
                "kernel_owned_generation",
                "owned contracts require Graph 18",
            ));
        }
        match &self.payload {
            DeclarationPayload::OwnedContract(c) => c.validate_local(),
            DeclarationPayload::OwnedImplementation(i) => i.validate_local(),
            DeclarationPayload::Record {
                fields,
                type_parameters,
            } => {
                validate_ordered_unique("record type parameters", type_parameters, true)?;
                validate_sorted_ids("record fields", fields, false)
            }
            DeclarationPayload::Variant {
                cases,
                type_parameters,
            } => {
                validate_ordered_unique("variant type parameters", type_parameters, true)?;
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
            DeclarationPayload::OwnedContract(_) => OwnerKind::OwnedContract,
            DeclarationPayload::OwnedImplementation(_) => OwnerKind::OwnedImplementation,
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
            DeclarationPayload::Function(function) => function
                .implementation_parameters
                .iter()
                .flat_map(|p| std::iter::once(p.self_type).chain(p.type_arguments.iter().copied()))
                .chain([function.result])
                .collect(),
            DeclarationPayload::OwnedContract(c) => c.type_roots(),
            DeclarationPayload::OwnedImplementation(i) => i.type_roots(),
            DeclarationPayload::Constant { ty, .. } => vec![*ty],
            DeclarationPayload::Record { .. }
            | DeclarationPayload::Variant { .. }
            | DeclarationPayload::Interface { .. }
            | DeclarationPayload::Component { .. }
            | DeclarationPayload::Test { .. } => Vec::new(),
        }
    }

    pub(crate) fn expression_roots(&self) -> Vec<ExpressionId> {
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
        type_parameters: Vec<TypeParameterId>,
        fields: Vec<FieldId>,
    },
    Variant {
        type_parameters: Vec<TypeParameterId>,
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
    OwnedContract(super::OwnedContract),
    OwnedImplementation(super::OwnedImplementation),
}

impl DeclarationPayload {
    pub fn type_parameters(&self) -> DeclarationTypeParameters<'_> {
        let (first, second) = match self {
            Self::Record {
                type_parameters, ..
            }
            | Self::Variant {
                type_parameters, ..
            } => (type_parameters.as_slice(), &[][..]),
            Self::OwnedContract(c) => (
                std::slice::from_ref(&c.self_parameter),
                c.type_parameters.as_slice(),
            ),
            Self::OwnedImplementation(i) => (i.type_parameters.as_slice(), &[][..]),
            Self::Function(function) => (function.type_parameters.as_slice(), &[][..]),
            Self::External(function) => (function.type_parameters.as_slice(), &[][..]),
            _ => (&[][..], &[][..]),
        };
        DeclarationTypeParameters { first, second }
    }
}

/// A declaration's authored parameter order, including an owned contract's
/// distinguished Self, without allocating during membership or arity checks.
#[derive(Clone, Copy, Debug)]
pub struct DeclarationTypeParameters<'a> {
    first: &'a [TypeParameterId],
    second: &'a [TypeParameterId],
}

impl DeclarationTypeParameters<'_> {
    pub fn iter(&self) -> impl Iterator<Item = &TypeParameterId> {
        self.first.iter().chain(self.second.iter())
    }

    pub fn len(&self) -> usize {
        self.first.len() + self.second.len()
    }

    pub fn is_empty(&self) -> bool {
        self.first.is_empty() && self.second.is_empty()
    }

    pub fn contains(&self, parameter: &TypeParameterId) -> bool {
        self.first.contains(parameter) || self.second.contains(parameter)
    }

    pub fn to_vec(self) -> Vec<TypeParameterId> {
        self.iter().copied().collect()
    }
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
    pub implementation_parameters: Vec<super::ImplementationParameter>,
    pub requirement_parameters: Vec<crate::platform::semantic_id::RequirementParameterId>,
    pub effect_parameters: Vec<EffectParameterId>,
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    /// Exact borrowed input whose custody protects the read-only result.
    #[serde(default)]
    pub result_borrow: Option<ParameterId>,
    pub effect: FunctionEffect,
    pub body: ExpressionId,
}

impl FunctionDeclaration {
    fn validate_local(&self) -> Result<(), Diagnostic> {
        validate_ordered_unique(
            "function requirement parameters",
            &self.requirement_parameters,
            true,
        )?;
        validate_ordered_unique("function type parameters", &self.type_parameters, true)?;
        validate_ordered_unique("function parameters", &self.parameters, true)?;
        if let Some(source) = self.result_borrow
            && (!self.parameters.contains(&source) || self.effect != FunctionEffect::Pure)
        {
            return Err(owner_error(
                "kernel_borrow_result",
                "a borrowed result requires an exact parameter of a pure function",
            ));
        }
        validate_ordered_unique("function effect parameters", &self.effect_parameters, true)?;
        self.effect.row().validate()?;
        Ok(())
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FunctionEffect {
    Pure,
    Task {
        requirements: Vec<super::RequirementOperand>,
        effect_parameters: Vec<super::EffectParameterReference>,
    },
}

impl FunctionEffect {
    pub fn row(&self) -> super::EffectRow {
        match self {
            Self::Pure => super::EffectRow::default(),
            Self::Task {
                requirements,
                effect_parameters,
            } => super::EffectRow {
                requirements: requirements.clone(),
                parameters: effect_parameters.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EffectParameterRecord {
    pub header: OwnerHeader,
    pub declaration: DeclarationId,
    pub name: Name,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementParameterRecord {
    pub header: OwnerHeader,
    pub declaration: DeclarationId,
    pub name: Name,
    pub constraint: super::RequirementConstraint,
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
    pub constraints: TypeParameterConstraints,
}

/// The closed structural constraint set. The wire tags below are explicit and do not
/// depend on Rust enum order. A set needs no heap storage; duplicate/unknown entries reject.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TypeParameterConstraints {
    #[default]
    None,
    CaptureSafe,
    Owned,
    Transferable,
    CaptureSafeTransferable,
    OwnedTransferable,
}

impl TypeParameterConstraints {
    pub const fn tag(self) -> u8 {
        match self {
            Self::None => 0,
            Self::CaptureSafe => 1,
            Self::Owned => 2,
            Self::Transferable => 3,
            Self::CaptureSafeTransferable => 4,
            Self::OwnedTransferable => 5,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::CaptureSafe => "capture-safe",
            Self::Owned => "owned",
            Self::Transferable => "transferable",
            Self::CaptureSafeTransferable => "capture-safe transferable",
            Self::OwnedTransferable => "owned transferable",
        }
    }

    pub const fn has_owned(self) -> bool {
        matches!(self, Self::Owned | Self::OwnedTransferable)
    }

    pub const fn requires_capture_safe(self) -> bool {
        matches!(self, Self::CaptureSafe | Self::CaptureSafeTransferable)
    }

    pub const fn proves_capture_safe(self) -> bool {
        matches!(
            self,
            Self::CaptureSafe | Self::Transferable | Self::CaptureSafeTransferable
        )
    }

    pub const fn requires_transfer(self) -> bool {
        matches!(
            self,
            Self::Transferable | Self::CaptureSafeTransferable | Self::OwnedTransferable
        )
    }

    pub const fn names(self) -> &'static [&'static str] {
        match self {
            Self::None => &[],
            Self::CaptureSafe => &["capture-safe"],
            Self::Owned => &["owned"],
            Self::Transferable => &["transferable"],
            Self::CaptureSafeTransferable => &["capture-safe", "transferable"],
            Self::OwnedTransferable => &["owned", "transferable"],
        }
    }

    pub fn from_names(names: &[&str]) -> Option<Self> {
        match names {
            [] => Some(Self::None),
            ["capture-safe"] => Some(Self::CaptureSafe),
            ["owned"] => Some(Self::Owned),
            ["transferable"] => Some(Self::Transferable),
            ["capture-safe", "transferable"] | ["transferable", "capture-safe"] => {
                Some(Self::CaptureSafeTransferable)
            }
            ["owned", "transferable"] | ["transferable", "owned"] => Some(Self::OwnedTransferable),
            _ => None,
        }
    }
}

impl Encode for TypeParameterConstraints {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.tag().encode(encoder)
    }
}

impl<Context> Decode<Context> for TypeParameterConstraints {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        match u8::decode(decoder)? {
            0 => Ok(Self::None),
            1 => Ok(Self::CaptureSafe),
            2 => Ok(Self::Owned),
            3 => Ok(Self::Transferable),
            4 => Ok(Self::CaptureSafeTransferable),
            5 => Ok(Self::OwnedTransferable),
            _ => Err(DecodeError::Other("unknown type-parameter constraint set")),
        }
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for TypeParameterConstraints {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Self::decode(decoder)
    }
}

impl Serialize for TypeParameterConstraints {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut sequence = serializer.serialize_seq(Some(self.names().len()))?;
        for name in self.names() {
            sequence.serialize_element(name)?;
        }
        sequence.end()
    }
}

impl<'de> Deserialize<'de> for TypeParameterConstraints {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ConstraintSet;
        impl<'de> serde::de::Visitor<'de> for ConstraintSet {
            type Value = TypeParameterConstraints;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter
                    .write_str("a closed set of capture-safe, owned, and transferable constraints")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let Some(value) = sequence.next_element::<String>()? else {
                    return Ok(TypeParameterConstraints::None);
                };
                let second = sequence.next_element::<String>()?;
                if sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
                    return Err(serde::de::Error::custom(
                        "duplicate, incompatible, or unknown type-parameter constraint",
                    ));
                }
                let constraints = match second.as_deref() {
                    Some(second) => TypeParameterConstraints::from_names(&[value.as_str(), second]),
                    None => TypeParameterConstraints::from_names(&[value.as_str()]),
                };
                constraints.ok_or_else(|| {
                    serde::de::Error::custom(
                        "duplicate, incompatible, or unknown type-parameter constraint",
                    )
                })
            }
        }
        deserializer.deserialize_seq(ConstraintSet)
    }
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
        if self.kind == BindingKind::OwnedBorrow
            && (self.header.contract_version < 24 || self.declared_type.is_none())
        {
            return Err(owner_error(
                "kernel_borrow_binding",
                "borrowed child bindings require Graph 24 and exact type annotations",
            ));
        }
        if self.kind == BindingKind::OwnedChoicePayload
            && (self.header.contract_version < 20 || self.declared_type.is_none())
        {
            return Err(owner_error(
                "kernel_choice_binding",
                "owned choice bindings require Graph 20 and exact type annotations",
            ));
        }
        if self.kind == BindingKind::OwnedUnpack
            && (self.header.contract_version < 19 || self.declared_type.is_none())
        {
            return Err(owner_error(
                "kernel_product_binding",
                "unpack bindings require Graph 19 and exact type annotations",
            ));
        }
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
    OwnedUnpack,
    OwnedChoicePayload,
    OwnedBorrow,
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

/// Returns whether one function-local requirement can be bound through one component capability
/// slot without widening the function's declared authority. Requirement identities remain local
/// to their owning declarations, so closure is structural and name-indexed rather than an identity
/// subset.
pub fn requirement_is_covered_by(
    candidate_package: PackageId,
    candidate: &RequirementRecord,
    component_package: PackageId,
    component: &RequirementRecord,
) -> bool {
    candidate_package == component_package
        && candidate.name == component.name
        && candidate.interface == component.interface
        && candidate
            .operations
            .iter()
            .all(|operation| component.operations.binary_search(operation).is_ok())
        && candidate.limits.iter().all(|required| {
            component
                .limits
                .binary_search_by(|available| available.name.cmp(&required.name))
                .ok()
                .and_then(|index| component.limits.get(index))
                .is_some_and(|available| {
                    available.unit == required.unit && available.maximum <= required.maximum
                })
        })
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
    pub port: Option<PortReference>,
    pub runner: RunnerKind,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HttpRouteRecord {
    pub header: OwnerHeader,
    pub target: TargetId,
    pub method: String,
    pub selector: HttpRouteSelector,
    pub port: PortReference,
}

impl HttpRouteRecord {
    fn validate_local(&self) -> Result<(), Diagnostic> {
        validate_header_domain(self.header, OwnerKind::HttpRoute)?;
        validate_http_route_method(&self.method)?;
        self.selector.validate_local()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HttpRouteSelector {
    Exact {
        path: String,
    },
    Pattern {
        segments: Vec<HttpRoutePatternSegment>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum HttpRoutePatternSegment {
    Literal(String),
    Capture(Name),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpRouteSetAnalysis {
    pub exact_routes: usize,
    pub pattern_routes: usize,
    pub pattern_segments: usize,
    pub maximum_specificity_chain: usize,
}

impl Encode for HttpRouteSelector {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            Self::Exact { path } => {
                0u8.encode(encoder)?;
                encode_http_selector_text(path, encoder)
            }
            Self::Pattern { segments } => {
                1u8.encode(encoder)?;
                let length = u8::try_from(segments.len()).map_err(|_| {
                    EncodeError::Other("HTTP route pattern segment count exceeds u8")
                })?;
                length.encode(encoder)?;
                for segment in segments {
                    segment.encode(encoder)?;
                }
                Ok(())
            }
        }
    }
}

impl<Context> Decode<Context> for HttpRouteSelector {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let selector = match u8::decode(decoder)? {
            0 => Self::Exact {
                path: decode_http_selector_text(
                    decoder,
                    super::contract::MAXIMUM_HTTP_ROUTE_PATH_BYTES,
                    "HTTP exact route path",
                )?,
            },
            1 => {
                let length = usize::from(u8::decode(decoder)?);
                if length == 0 || length > super::contract::MAXIMUM_HTTP_PATTERN_SEGMENTS {
                    return Err(DecodeError::OtherString(format!(
                        "HTTP route pattern segment count must be 1 through {} before allocation",
                        super::contract::MAXIMUM_HTTP_PATTERN_SEGMENTS
                    )));
                }
                decoder.claim_container_read::<HttpRoutePatternSegment>(length)?;
                let mut segments = Vec::with_capacity(length);
                for _ in 0..length {
                    decoder.unclaim_bytes_read(std::mem::size_of::<HttpRoutePatternSegment>());
                    segments.push(HttpRoutePatternSegment::decode(decoder)?);
                }
                Self::Pattern { segments }
            }
            tag => {
                return Err(DecodeError::OtherString(format!(
                    "unknown HTTP route selector tag {tag}"
                )));
            }
        };
        selector
            .validate_local()
            .map_err(|error| DecodeError::OtherString(error.message))?;
        Ok(selector)
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for HttpRouteSelector {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Self::decode(decoder)
    }
}

impl Encode for HttpRoutePatternSegment {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            Self::Literal(value) => {
                0u8.encode(encoder)?;
                encode_http_selector_text(value, encoder)
            }
            Self::Capture(name) => {
                1u8.encode(encoder)?;
                encode_http_selector_text(name.as_str(), encoder)
            }
        }
    }
}

impl<Context> Decode<Context> for HttpRoutePatternSegment {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        match u8::decode(decoder)? {
            0 => Ok(Self::Literal(decode_http_selector_text(
                decoder,
                super::contract::MAXIMUM_HTTP_ROUTE_PATH_BYTES,
                "HTTP route pattern literal",
            )?)),
            1 => {
                let value = decode_http_selector_text(
                    decoder,
                    super::contract::MAXIMUM_NAME_BYTES,
                    "HTTP route capture name",
                )?;
                Name::new(value)
                    .map(Self::Capture)
                    .map_err(|error| DecodeError::OtherString(error.message))
            }
            tag => Err(DecodeError::OtherString(format!(
                "unknown HTTP route pattern segment tag {tag}"
            ))),
        }
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for HttpRoutePatternSegment {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Self::decode(decoder)
    }
}

fn encode_http_selector_text<E: Encoder>(value: &str, encoder: &mut E) -> Result<(), EncodeError> {
    let length = u16::try_from(value.len())
        .map_err(|_| EncodeError::Other("HTTP selector text exceeds u16"))?;
    length.encode(encoder)?;
    for byte in value.bytes() {
        byte.encode(encoder)?;
    }
    Ok(())
}

fn decode_http_selector_text<Context, D: Decoder<Context = Context>>(
    decoder: &mut D,
    maximum: usize,
    label: &'static str,
) -> Result<String, DecodeError> {
    let length = usize::from(u16::decode(decoder)?);
    if length > maximum {
        return Err(DecodeError::OtherString(format!(
            "{label} exceeds {maximum} bytes before allocation"
        )));
    }
    decoder.claim_container_read::<u8>(length)?;
    let mut bytes = Vec::with_capacity(length);
    for _ in 0..length {
        decoder.unclaim_bytes_read(std::mem::size_of::<u8>());
        bytes.push(u8::decode(decoder)?);
    }
    String::from_utf8(bytes).map_err(|error| DecodeError::OtherString(error.to_string()))
}

impl HttpRouteSelector {
    pub fn exact(path: impl Into<String>) -> Result<Self, Diagnostic> {
        let selector = Self::Exact { path: path.into() };
        selector.validate_local()?;
        Ok(selector)
    }

    pub fn parse_pattern(pattern: &str) -> Result<Self, Diagnostic> {
        validate_http_route_path(pattern)?;
        let Some(remainder) = pattern.strip_prefix('/') else {
            return Err(owner_error(
                "kernel_http_route_pattern",
                "HTTP route pattern must begin with '/'",
            ));
        };
        if remainder.is_empty() || remainder.ends_with('/') {
            return Err(owner_error(
                "kernel_http_route_pattern_segment",
                "HTTP route pattern requires nonempty segments and no trailing slash",
            ));
        }
        let mut segments = Vec::new();
        for segment in remainder.split('/') {
            if segments.len() == super::contract::MAXIMUM_HTTP_PATTERN_SEGMENTS {
                return Err(owner_error(
                    "kernel_http_route_pattern_segments",
                    format!(
                        "HTTP route pattern exceeds {} segments",
                        super::contract::MAXIMUM_HTTP_PATTERN_SEGMENTS
                    ),
                ));
            }
            if segment.is_empty() {
                return Err(owner_error(
                    "kernel_http_route_pattern_segment",
                    "HTTP route pattern contains an empty segment",
                ));
            }
            let parsed = if let Some(name) = segment
                .strip_prefix('{')
                .and_then(|value| value.strip_suffix('}'))
            {
                if name.contains(['{', '}']) {
                    return Err(owner_error(
                        "kernel_http_route_pattern_capture",
                        "HTTP route capture must occupy one whole segment",
                    ));
                }
                HttpRoutePatternSegment::Capture(Name::new(name.to_owned()).map_err(|_| {
                    owner_error(
                        "kernel_http_route_pattern_capture",
                        "HTTP route capture must contain a valid graph Name",
                    )
                })?)
            } else {
                if segment.contains(['{', '}']) {
                    return Err(owner_error(
                        "kernel_http_route_pattern_segment",
                        "HTTP route braces are valid only around one whole capture segment",
                    ));
                }
                HttpRoutePatternSegment::Literal(segment.to_owned())
            };
            segments.push(parsed);
        }
        let selector = Self::Pattern { segments };
        selector.validate_local()?;
        Ok(selector)
    }

    pub fn validate_local(&self) -> Result<(), Diagnostic> {
        match self {
            Self::Exact { path } => validate_http_route_path(path),
            Self::Pattern { segments } => {
                if segments.is_empty()
                    || segments.len() > super::contract::MAXIMUM_HTTP_PATTERN_SEGMENTS
                {
                    return Err(owner_error(
                        "kernel_http_route_pattern_segments",
                        format!(
                            "HTTP route pattern must contain 1 through {} segments",
                            super::contract::MAXIMUM_HTTP_PATTERN_SEGMENTS
                        ),
                    ));
                }
                let mut captures = BTreeSet::new();
                let mut path_bytes = 1usize;
                for (index, segment) in segments.iter().enumerate() {
                    if index != 0 {
                        path_bytes = path_bytes.checked_add(1).ok_or_else(|| {
                            owner_error(
                                "kernel_http_route_pattern_bytes",
                                "HTTP route pattern byte accounting overflowed",
                            )
                        })?;
                    }
                    match segment {
                        HttpRoutePatternSegment::Literal(value) => {
                            if value.is_empty()
                                || value.contains(['/', '{', '}'])
                                || value.bytes().any(|byte| {
                                    byte == b'?' || byte == b'#' || byte.is_ascii_control()
                                })
                            {
                                return Err(owner_error(
                                    "kernel_http_route_pattern_literal",
                                    "HTTP route pattern literal must be one nonempty brace-free path segment",
                                ));
                            }
                            path_bytes = path_bytes.checked_add(value.len()).ok_or_else(|| {
                                owner_error(
                                    "kernel_http_route_pattern_bytes",
                                    "HTTP route pattern byte accounting overflowed",
                                )
                            })?;
                        }
                        HttpRoutePatternSegment::Capture(name) => {
                            if !captures.insert(name) {
                                return Err(owner_error(
                                    "kernel_http_route_pattern_capture_duplicate",
                                    "HTTP route pattern capture names must be unique",
                                ));
                            }
                            path_bytes = path_bytes
                                .checked_add(name.as_str().len())
                                .and_then(|value| value.checked_add(2))
                                .ok_or_else(|| {
                                    owner_error(
                                        "kernel_http_route_pattern_bytes",
                                        "HTTP route pattern byte accounting overflowed",
                                    )
                                })?;
                        }
                    }
                }
                if captures.is_empty()
                    || captures.len() > super::contract::MAXIMUM_HTTP_PATTERN_CAPTURES
                {
                    return Err(owner_error(
                        "kernel_http_route_pattern_captures",
                        format!(
                            "HTTP route pattern must contain 1 through {} unique captures",
                            super::contract::MAXIMUM_HTTP_PATTERN_CAPTURES
                        ),
                    ));
                }
                if path_bytes > super::contract::MAXIMUM_HTTP_ROUTE_PATH_BYTES {
                    return Err(owner_error(
                        "kernel_http_route_pattern_bytes",
                        format!(
                            "HTTP route pattern exceeds {} bytes",
                            super::contract::MAXIMUM_HTTP_ROUTE_PATH_BYTES
                        ),
                    ));
                }
                Ok(())
            }
        }
    }

    pub fn display(&self) -> String {
        match self {
            Self::Exact { path } => path.clone(),
            Self::Pattern { segments } => {
                let mut pattern = String::new();
                for segment in segments {
                    pattern.push('/');
                    match segment {
                        HttpRoutePatternSegment::Literal(value) => pattern.push_str(value),
                        HttpRoutePatternSegment::Capture(name) => {
                            pattern.push('{');
                            pattern.push_str(name.as_str());
                            pattern.push('}');
                        }
                    }
                }
                pattern
            }
        }
    }

    pub fn capture_names(&self) -> Vec<&Name> {
        match self {
            Self::Exact { .. } => Vec::new(),
            Self::Pattern { segments } => segments
                .iter()
                .filter_map(|segment| match segment {
                    HttpRoutePatternSegment::Capture(name) => Some(name),
                    HttpRoutePatternSegment::Literal(_) => None,
                })
                .collect(),
        }
    }

    pub fn capture_count(&self) -> usize {
        match self {
            Self::Exact { .. } => 0,
            Self::Pattern { segments } => segments
                .iter()
                .filter(|segment| matches!(segment, HttpRoutePatternSegment::Capture(_)))
                .count(),
        }
    }

    pub fn segment_count(&self) -> usize {
        match self {
            Self::Exact { .. } => 0,
            Self::Pattern { segments } => segments.len(),
        }
    }

    pub fn key_bytes(&self) -> usize {
        match self {
            Self::Exact { path } => path.len(),
            Self::Pattern { .. } => self.display().len(),
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Exact { .. } => "exact",
            Self::Pattern { .. } => "pattern",
        }
    }
}

pub fn validate_http_route_method(method: &str) -> Result<(), Diagnostic> {
    if method.is_empty()
        || method.len() > super::contract::MAXIMUM_HTTP_ROUTE_METHOD_BYTES
        || !method.bytes().all(is_http_token_byte)
    {
        return Err(owner_error(
            "kernel_http_route_method",
            format!(
                "HTTP route method must be a nonempty ASCII token of at most {} bytes",
                super::contract::MAXIMUM_HTTP_ROUTE_METHOD_BYTES
            ),
        ));
    }
    Ok(())
}

pub fn validate_http_route_path(path: &str) -> Result<(), Diagnostic> {
    if path.is_empty()
        || path.len() > super::contract::MAXIMUM_HTTP_ROUTE_PATH_BYTES
        || !path.starts_with('/')
        || path
            .bytes()
            .any(|byte| byte == b'?' || byte == b'#' || byte.is_ascii_control())
    {
        return Err(owner_error(
            "kernel_http_route_path",
            format!(
                "HTTP route path must begin with '/', contain no query, fragment, or control bytes, and use at most {} bytes",
                super::contract::MAXIMUM_HTTP_ROUTE_PATH_BYTES
            ),
        ));
    }
    Ok(())
}

pub fn validate_http_route_key(method: &str, path: &str) -> Result<(), Diagnostic> {
    validate_http_route_method(method)?;
    validate_http_route_path(path)
}

pub fn http_route_canonical_cmp(
    left: &HttpRouteRecord,
    right: &HttpRouteRecord,
) -> std::cmp::Ordering {
    left.method
        .as_bytes()
        .cmp(right.method.as_bytes())
        .then_with(|| http_route_selector_cmp(&left.selector, &right.selector))
        .then_with(|| left.header.owner.bytes().cmp(&right.header.owner.bytes()))
}

pub fn analyze_http_route_set(
    routes: &[HttpRouteRecord],
) -> Result<HttpRouteSetAnalysis, Diagnostic> {
    use super::contract::{
        MAXIMUM_HTTP_PATTERN_SEGMENTS_PER_TARGET, MAXIMUM_HTTP_ROUTE_KEY_BYTES_PER_TARGET,
        MAXIMUM_HTTP_ROUTES_PER_TARGET,
    };

    if routes.is_empty() || routes.len() > MAXIMUM_HTTP_ROUTES_PER_TARGET {
        return Err(owner_error(
            "kernel_http_route_count",
            "HTTP route set requires one bounded nonempty target route set",
        ));
    }
    let mut aggregate = 0usize;
    let mut pattern_segments = 0usize;
    let mut exact_routes = 0usize;
    let mut pattern_routes = 0usize;
    for route in routes {
        route.validate_local()?;
        aggregate = aggregate
            .checked_add(route.method.len())
            .and_then(|value| value.checked_add(route.selector.key_bytes()))
            .ok_or_else(|| {
                owner_error(
                    "kernel_http_route_aggregate",
                    "HTTP route-set byte accounting overflowed",
                )
            })?;
        if aggregate > MAXIMUM_HTTP_ROUTE_KEY_BYTES_PER_TARGET {
            return Err(owner_error(
                "kernel_http_route_aggregate",
                "HTTP route-set keys exceed the aggregate byte bound",
            ));
        }
        match &route.selector {
            HttpRouteSelector::Exact { .. } => exact_routes += 1,
            HttpRouteSelector::Pattern { segments } => {
                pattern_routes += 1;
                pattern_segments =
                    pattern_segments
                        .checked_add(segments.len())
                        .ok_or_else(|| {
                            owner_error(
                                "kernel_http_route_pattern_segment_aggregate",
                                "HTTP route pattern-segment accounting overflowed",
                            )
                        })?;
                if pattern_segments > MAXIMUM_HTTP_PATTERN_SEGMENTS_PER_TARGET {
                    return Err(owner_error(
                        "kernel_http_route_pattern_segment_aggregate",
                        format!(
                            "HTTP target exceeds {MAXIMUM_HTTP_PATTERN_SEGMENTS_PER_TARGET} stored pattern segments"
                        ),
                    ));
                }
            }
        }
    }

    let mut maximum_specificity_chain = 1usize;
    for (index, left) in routes.iter().enumerate() {
        let mut less_specific = 0usize;
        for right in routes.iter().skip(index + 1) {
            validate_http_route_pair(left, right)?;
        }
        for other in routes {
            if left.method == other.method && http_route_strictly_more_specific(left, other) {
                less_specific = less_specific.saturating_add(1);
            }
        }
        maximum_specificity_chain = maximum_specificity_chain.max(less_specific.saturating_add(1));
    }

    Ok(HttpRouteSetAnalysis {
        exact_routes,
        pattern_routes,
        pattern_segments,
        maximum_specificity_chain,
    })
}

pub fn http_route_set_digest(routes: &[HttpRouteRecord]) -> Result<[u8; 32], Diagnostic> {
    analyze_http_route_set(routes)?;
    let mut routes = routes.iter().collect::<Vec<_>>();
    routes.sort_by(|left, right| http_route_canonical_cmp(left, right));
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.kernel.http-route-set.v2");
    hasher.update(&(routes.len() as u64).to_be_bytes());
    for route in routes {
        let OwnerKey::HttpRoute(route_id) = route.header.owner else {
            return Err(owner_error(
                "kernel_http_route_identity",
                "HTTP route record has another owner identity domain",
            ));
        };
        hasher.update(&route_id.bytes());
        hasher.update(&(route.method.len() as u64).to_be_bytes());
        hasher.update(route.method.as_bytes());
        hash_http_route_selector(&mut hasher, &route.selector);
        hasher.update(&route.port.package.bytes());
        hasher.update(&route.port.port.bytes());
    }
    Ok(*hasher.finalize().as_bytes())
}

fn validate_http_route_pair(
    left: &HttpRouteRecord,
    right: &HttpRouteRecord,
) -> Result<(), Diagnostic> {
    if left.port == right.port && left.selector.capture_names() != right.selector.capture_names() {
        return Err(owner_error(
            "kernel_http_route_shared_port_signature",
            "routes sharing a port must use one capture-name sequence",
        ));
    }
    if left.method != right.method {
        return Ok(());
    }
    match (&left.selector, &right.selector) {
        (HttpRouteSelector::Exact { path: left }, HttpRouteSelector::Exact { path: right })
            if left == right =>
        {
            Err(owner_error(
                "kernel_http_route_duplicate_language",
                "HTTP exact routes repeat one method/path match language",
            ))
        }
        (
            HttpRouteSelector::Pattern {
                segments: left_segments,
            },
            HttpRouteSelector::Pattern {
                segments: right_segments,
            },
        ) => {
            if !http_route_patterns_overlap(left_segments, right_segments) {
                return Ok(());
            }
            if http_route_pattern_strictly_more_specific(left_segments, right_segments)
                || http_route_pattern_strictly_more_specific(right_segments, left_segments)
            {
                Ok(())
            } else if http_route_same_pattern_language(left_segments, right_segments) {
                Err(owner_error(
                    "kernel_http_route_duplicate_language",
                    "HTTP patterns repeat one match language",
                ))
            } else {
                Err(owner_error(
                    "kernel_http_route_incomparable_overlap",
                    "overlapping HTTP patterns must have a strict specificity relation",
                ))
            }
        }
        _ => Ok(()),
    }
}

pub fn http_route_strictly_more_specific(left: &HttpRouteRecord, right: &HttpRouteRecord) -> bool {
    match (&left.selector, &right.selector) {
        (HttpRouteSelector::Exact { path }, HttpRouteSelector::Pattern { segments }) => {
            pattern_matches_path(segments, path)
        }
        (
            HttpRouteSelector::Pattern {
                segments: left_segments,
            },
            HttpRouteSelector::Pattern {
                segments: right_segments,
            },
        ) => http_route_pattern_strictly_more_specific(left_segments, right_segments),
        _ => false,
    }
}

pub fn http_route_languages_overlap(left: &HttpRouteRecord, right: &HttpRouteRecord) -> bool {
    if left.method != right.method {
        return false;
    }
    match (&left.selector, &right.selector) {
        (HttpRouteSelector::Exact { path: left }, HttpRouteSelector::Exact { path: right }) => {
            left == right
        }
        (HttpRouteSelector::Exact { path }, HttpRouteSelector::Pattern { segments })
        | (HttpRouteSelector::Pattern { segments }, HttpRouteSelector::Exact { path }) => {
            pattern_matches_path(segments, path)
        }
        (
            HttpRouteSelector::Pattern {
                segments: left_segments,
            },
            HttpRouteSelector::Pattern {
                segments: right_segments,
            },
        ) => http_route_patterns_overlap(left_segments, right_segments),
    }
}

pub fn http_route_patterns_overlap(
    left: &[HttpRoutePatternSegment],
    right: &[HttpRoutePatternSegment],
) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            !matches!(
                (left, right),
                (
                    HttpRoutePatternSegment::Literal(left),
                    HttpRoutePatternSegment::Literal(right)
                ) if left != right
            )
        })
}

pub fn http_route_same_pattern_language(
    left: &[HttpRoutePatternSegment],
    right: &[HttpRoutePatternSegment],
) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| match (left, right) {
                (
                    HttpRoutePatternSegment::Literal(left),
                    HttpRoutePatternSegment::Literal(right),
                ) => left == right,
                (HttpRoutePatternSegment::Capture(_), HttpRoutePatternSegment::Capture(_)) => true,
                _ => false,
            })
}

pub fn http_route_pattern_strictly_more_specific(
    left: &[HttpRoutePatternSegment],
    right: &[HttpRoutePatternSegment],
) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut strict = false;
    for (left, right) in left.iter().zip(right) {
        match (left, right) {
            (HttpRoutePatternSegment::Literal(left), HttpRoutePatternSegment::Literal(right))
                if left == right => {}
            (HttpRoutePatternSegment::Literal(_), HttpRoutePatternSegment::Capture(_)) => {
                strict = true;
            }
            (HttpRoutePatternSegment::Capture(_), HttpRoutePatternSegment::Capture(_)) => {}
            _ => return false,
        }
    }
    strict
}

fn pattern_matches_path(segments: &[HttpRoutePatternSegment], path: &str) -> bool {
    let Some(path) = path.strip_prefix('/') else {
        return false;
    };
    let mut values = path.split('/');
    for segment in segments {
        let Some(value) = values.next() else {
            return false;
        };
        if value.is_empty()
            || matches!(segment, HttpRoutePatternSegment::Literal(literal) if literal != value)
        {
            return false;
        }
    }
    values.next().is_none()
}

pub fn http_route_selector_cmp(
    left: &HttpRouteSelector,
    right: &HttpRouteSelector,
) -> std::cmp::Ordering {
    match (left, right) {
        (HttpRouteSelector::Exact { path: left }, HttpRouteSelector::Exact { path: right }) => {
            left.as_bytes().cmp(right.as_bytes())
        }
        (HttpRouteSelector::Exact { .. }, HttpRouteSelector::Pattern { .. }) => {
            std::cmp::Ordering::Less
        }
        (HttpRouteSelector::Pattern { .. }, HttpRouteSelector::Exact { .. }) => {
            std::cmp::Ordering::Greater
        }
        (
            HttpRouteSelector::Pattern { segments: left },
            HttpRouteSelector::Pattern { segments: right },
        ) => {
            for (left, right) in left.iter().zip(right) {
                let ordering = match (left, right) {
                    (
                        HttpRoutePatternSegment::Literal(left),
                        HttpRoutePatternSegment::Literal(right),
                    ) => left.as_bytes().cmp(right.as_bytes()),
                    (HttpRoutePatternSegment::Literal(_), HttpRoutePatternSegment::Capture(_)) => {
                        std::cmp::Ordering::Less
                    }
                    (HttpRoutePatternSegment::Capture(_), HttpRoutePatternSegment::Literal(_)) => {
                        std::cmp::Ordering::Greater
                    }
                    (
                        HttpRoutePatternSegment::Capture(left),
                        HttpRoutePatternSegment::Capture(right),
                    ) => left.as_str().as_bytes().cmp(right.as_str().as_bytes()),
                };
                if ordering != std::cmp::Ordering::Equal {
                    return ordering;
                }
            }
            left.len().cmp(&right.len())
        }
    }
}

fn hash_http_route_selector(hasher: &mut blake3::Hasher, selector: &HttpRouteSelector) {
    match selector {
        HttpRouteSelector::Exact { path } => {
            hasher.update(&[0]);
            hasher.update(&(path.len() as u64).to_be_bytes());
            hasher.update(path.as_bytes());
        }
        HttpRouteSelector::Pattern { segments } => {
            hasher.update(&[1]);
            hasher.update(&(segments.len() as u64).to_be_bytes());
            for segment in segments {
                match segment {
                    HttpRoutePatternSegment::Literal(value) => {
                        hasher.update(&[0]);
                        hasher.update(&(value.len() as u64).to_be_bytes());
                        hasher.update(value.as_bytes());
                    }
                    HttpRoutePatternSegment::Capture(name) => {
                        hasher.update(&[1]);
                        hasher.update(&(name.as_str().len() as u64).to_be_bytes());
                        hasher.update(name.as_str().as_bytes());
                    }
                }
            }
        }
    }
}

fn is_http_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
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
    if !super::contract::supported_graph_contract(header.contract_version) || header.kind != kind {
        return Err(owner_error(
            "kernel_owner_header",
            "owner header contract or kind does not match its record",
        ));
    }
    let valid_domain = matches!(
        (header.owner, kind),
        (OwnerKey::Module(_), OwnerKind::Module)
            | (OwnerKey::Declaration(_), OwnerKind::OwnedContract)
            | (OwnerKey::Declaration(_), OwnerKind::OwnedImplementation)
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
            | (OwnerKey::EffectParameter(_), OwnerKind::EffectParameter)
            | (
                OwnerKey::RequirementParameter(_),
                OwnerKind::RequirementParameter
            )
            | (OwnerKey::Field(_), OwnerKind::Field)
            | (OwnerKey::Case(_), OwnerKind::Case)
            | (OwnerKey::Operation(_), OwnerKind::Operation)
            | (OwnerKey::Parameter(_), OwnerKind::Parameter)
            | (OwnerKey::Binding(_), OwnerKind::Binding)
            | (OwnerKey::Expression(_), OwnerKind::Expression)
            | (OwnerKey::Requirement(_), OwnerKind::Requirement)
            | (OwnerKey::Port(_), OwnerKind::Port)
            | (OwnerKey::Target(_), OwnerKind::Target)
            | (OwnerKey::HttpRoute(_), OwnerKind::HttpRoute)
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
            format!("{label} count is outside the Graph 10 bound"),
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
