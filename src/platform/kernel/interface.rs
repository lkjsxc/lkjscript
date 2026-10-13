//! Implementation-free semantic owner records consumed across exact package boundaries.
//!
//! This is a logical kernel view, not a stored authority or transport. Package export decides
//! which public owners are reachable, binds these records to accepted summary dimensions, and
//! stores them in a separate derived interface map.

use super::contract::{GRAPH_CONTRACT_VERSION, MAXIMUM_CHILDREN};
use super::{
    CallableResult, CaseRecord, DeclarationPayload, DeclarationVisibility, EffectParameterRecord,
    FieldRecord, FunctionEffect, Name, OperationRecord, OwnerHeader, OwnerKey, OwnerKind,
    OwnerRecord, ParameterRecord, ReadScopeParameter, ReadScopeSource, RequirementRecord,
    ResultRelation, TypeObjectDigest, TypeParameterRecord,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{
    CaseId, DeclarationId, EffectParameterId, FieldId, OperationId, ParameterId, PortId,
    RequirementId, TypeParameterId,
};
use bincode::{Decode, Encode};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum PackageInterfaceRecord {
    Declaration(PackageInterfaceDeclaration),
    TypeParameter(TypeParameterRecord),
    EffectParameter(EffectParameterRecord),
    Field(FieldRecord),
    Case(CaseRecord),
    Operation(OperationRecord),
    Parameter(ParameterRecord),
    Requirement(RequirementRecord),
    Port(PackageInterfacePort),
    RequirementParameter(super::RequirementParameterRecord),
    /// Exact private layout evidence. Its presence grants no authored reference or lookup access.
    RepresentationDeclaration(PackageInterfaceDeclaration),
}

impl PackageInterfaceRecord {
    pub(crate) fn project_public(canonical: &OwnerRecord) -> Result<Option<Self>, Diagnostic> {
        Ok(Some(match canonical {
            OwnerRecord::Declaration(record) => {
                let projected = PackageInterfaceDeclaration::project(record)?;
                if record.visibility == DeclarationVisibility::Public {
                    Self::Declaration(projected)
                } else {
                    Self::RepresentationDeclaration(projected)
                }
            }
            OwnerRecord::TypeParameter(record) => Self::TypeParameter(record.clone()),
            OwnerRecord::EffectParameter(record) => Self::EffectParameter(record.clone()),
            OwnerRecord::RequirementParameter(record) => Self::RequirementParameter(record.clone()),
            OwnerRecord::Field(record) => Self::Field(record.clone()),
            OwnerRecord::Case(record) => Self::Case(record.clone()),
            OwnerRecord::Operation(record) => Self::Operation(record.clone()),
            OwnerRecord::Parameter(record) => Self::Parameter(record.clone()),
            OwnerRecord::Requirement(record) => Self::Requirement(record.clone()),
            OwnerRecord::Port(record) => Self::Port(PackageInterfacePort {
                header: record.header,
                declaration: record.declaration,
                name: record.name.clone(),
                function_type: record.function_type,
            }),
            OwnerRecord::Module(_)
            | OwnerRecord::Binding(_)
            | OwnerRecord::Expression(_)
            | OwnerRecord::Target(_)
            | OwnerRecord::HttpRoute(_)
            | OwnerRecord::Documentation(_)
            | OwnerRecord::Annotation(_) => return Ok(None),
        }))
    }

    pub fn header(&self) -> OwnerHeader {
        match self {
            Self::Declaration(record) | Self::RepresentationDeclaration(record) => record.header,
            Self::TypeParameter(record) => record.header,
            Self::EffectParameter(record) => record.header,
            Self::RequirementParameter(record) => record.header,
            Self::Field(record) => record.header,
            Self::Case(record) => record.header,
            Self::Operation(record) => record.header,
            Self::Parameter(record) => record.header,
            Self::Requirement(record) => record.header,
            Self::Port(record) => record.header,
        }
    }

    pub fn type_roots(&self) -> Vec<TypeObjectDigest> {
        match self {
            Self::Declaration(record) | Self::RepresentationDeclaration(record) => {
                record.type_roots()
            }
            Self::Field(record) => vec![record.ty],
            Self::Case(record) => record.payload.into_iter().collect(),
            Self::Operation(record) => vec![record.result],
            Self::Parameter(record) => vec![record.ty],
            Self::Port(record) => vec![record.function_type],
            Self::TypeParameter(_)
            | Self::EffectParameter(_)
            | Self::RequirementParameter(_)
            | Self::Requirement(_) => Vec::new(),
        }
    }

    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
        match self {
            Self::Declaration(record) => record.validate_local(),
            Self::RepresentationDeclaration(record) => {
                if !matches!(
                    record.payload,
                    PackageInterfaceDeclarationPayload::Record { .. }
                        | PackageInterfaceDeclarationPayload::Variant { .. }
                        | PackageInterfaceDeclarationPayload::OwnedRecord { .. }
                        | PackageInterfaceDeclarationPayload::ReadView(_)
                ) {
                    return Err(interface_error(
                        DiagnosticClass::Semantic,
                        "kernel_package_interface_representation",
                        "private representation evidence requires a nominal layout",
                    ));
                }
                record.validate_local()
            }
            Self::RequirementParameter(record) => {
                OwnerRecord::RequirementParameter(record.clone()).validate_local()
            }
            Self::EffectParameter(record) => {
                OwnerRecord::EffectParameter(record.clone()).validate_local()
            }
            Self::TypeParameter(record) => {
                OwnerRecord::TypeParameter(record.clone()).validate_local()
            }
            Self::Field(record) => OwnerRecord::Field(record.clone()).validate_local(),
            Self::Case(record) => OwnerRecord::Case(record.clone()).validate_local(),
            Self::Operation(record) => OwnerRecord::Operation(record.clone()).validate_local(),
            Self::Parameter(record) => OwnerRecord::Parameter(record.clone()).validate_local(),
            Self::Requirement(record) => OwnerRecord::Requirement(record.clone()).validate_local(),
            Self::Port(record) => record.validate_local(),
        }
    }
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageInterfaceDeclaration {
    pub header: OwnerHeader,
    pub name: Name,
    pub payload: PackageInterfaceDeclarationPayload,
}

impl PackageInterfaceDeclaration {
    fn project(record: &super::DeclarationRecord) -> Result<Self, Diagnostic> {
        if record.visibility != DeclarationVisibility::Public
            && !matches!(
                record.payload,
                DeclarationPayload::Record { .. }
                    | DeclarationPayload::Variant { .. }
                    | DeclarationPayload::OwnedRecord { .. }
                    | DeclarationPayload::ReadView(_)
            )
        {
            return Err(interface_error(
                DiagnosticClass::Corrupt,
                "kernel_package_interface_visibility",
                "package-interface selection admitted a non-public declaration",
            ));
        }
        let payload = match &record.payload {
            DeclarationPayload::Record {
                fields,
                type_parameters,
            } => PackageInterfaceDeclarationPayload::Record {
                type_parameters: type_parameters.clone(),
                fields: fields.clone(),
            },
            DeclarationPayload::OwnedRecord {
                fields,
                type_parameters,
            } => PackageInterfaceDeclarationPayload::OwnedRecord {
                type_parameters: type_parameters.clone(),
                fields: fields.clone(),
            },
            DeclarationPayload::Variant {
                cases,
                type_parameters,
            } => PackageInterfaceDeclarationPayload::Variant {
                type_parameters: type_parameters.clone(),
                cases: cases.clone(),
            },
            DeclarationPayload::Interface { operations } => {
                PackageInterfaceDeclarationPayload::Interface {
                    operations: operations.clone(),
                }
            }
            DeclarationPayload::External(function) => {
                PackageInterfaceDeclarationPayload::External(PackageExternalSignature {
                    type_parameters: function.type_parameters.clone(),
                    parameters: function.parameters.clone(),
                    result: function.result,
                })
            }
            DeclarationPayload::Function(function) => {
                PackageInterfaceDeclarationPayload::Function(PackageFunctionSignature {
                    implementation_parameters: function.implementation_parameters.clone(),
                    requirement_parameters: function.requirement_parameters.clone(),
                    effect_parameters: function.effect_parameters.clone(),
                    type_parameters: function.type_parameters.clone(),
                    parameters: function.parameters.clone(),
                    result: function.result,
                    read_scope_parameters: function.read_scope_parameters.clone(),
                    read_scope_sources: function.read_scope_sources.clone(),
                    effect: function.effect.clone(),
                })
            }
            DeclarationPayload::Constant { ty, .. } => {
                PackageInterfaceDeclarationPayload::Constant { ty: *ty }
            }
            DeclarationPayload::Component {
                requirements,
                ports,
            } => PackageInterfaceDeclarationPayload::Component {
                requirements: requirements.clone(),
                ports: ports.clone(),
            },
            DeclarationPayload::OwnedContract(c) => {
                PackageInterfaceDeclarationPayload::OwnedContract(c.clone())
            }
            DeclarationPayload::OwnedImplementation(i) => {
                PackageInterfaceDeclarationPayload::OwnedImplementation(i.clone())
            }
            DeclarationPayload::ReadView(view) => {
                PackageInterfaceDeclarationPayload::ReadView(view.clone())
            }
            DeclarationPayload::Test { .. } => {
                return Err(interface_error(
                    DiagnosticClass::Semantic,
                    "kernel_package_interface_public_test",
                    "tests are executable package-local evidence and cannot be exported as public declarations",
                ));
            }
        };
        Ok(Self {
            header: record.header,
            name: record.name.clone(),
            payload,
        })
    }

    pub fn type_roots(&self) -> Vec<TypeObjectDigest> {
        match &self.payload {
            PackageInterfaceDeclarationPayload::External(signature) => vec![signature.result],
            PackageInterfaceDeclarationPayload::Function(signature) => signature
                .implementation_parameters
                .iter()
                .flat_map(|p| std::iter::once(p.self_type).chain(p.type_arguments.iter().copied()))
                .chain([signature.result.ty])
                .collect(),
            PackageInterfaceDeclarationPayload::OwnedContract(c) => c.type_roots(),
            PackageInterfaceDeclarationPayload::OwnedImplementation(i) => i.type_roots(),
            PackageInterfaceDeclarationPayload::ReadView(view) => {
                view.fields.iter().map(|field| field.ty).collect()
            }
            PackageInterfaceDeclarationPayload::Constant { ty } => vec![*ty],
            PackageInterfaceDeclarationPayload::Record { .. }
            | PackageInterfaceDeclarationPayload::OwnedRecord { .. }
            | PackageInterfaceDeclarationPayload::Variant { .. }
            | PackageInterfaceDeclarationPayload::Interface { .. }
            | PackageInterfaceDeclarationPayload::Component { .. } => Vec::new(),
        }
    }

    fn validate_local(&self) -> Result<(), Diagnostic> {
        validate_header(self.header)?;
        if self.header.contract_version < 32
            && matches!(
                self.payload,
                PackageInterfaceDeclarationPayload::OwnedRecord { .. }
            )
        {
            return Err(interface_error(
                DiagnosticClass::Semantic,
                "kernel_owned_record_generation",
                "nominal owned record interfaces require Graph 32",
            ));
        }
        if self.header.contract_version < 29
            && matches!(&self.payload, PackageInterfaceDeclarationPayload::OwnedImplementation(i)
                if !i.implementation_parameters.is_empty() || i.methods.iter().any(|m| !m.implementations.is_empty()))
        {
            return Err(super::wire28::implementation_scheme_extension());
        }
        if self.header.contract_version < 28
            && matches!(&self.payload, PackageInterfaceDeclarationPayload::OwnedImplementation(i)
                if !i.type_parameters.is_empty()
                    || i.methods.iter().any(|m| !m.type_arguments.is_empty()))
        {
            return Err(super::wire27::implementation_scheme_extension());
        }
        if self.header.contract_version < 27
            && match &self.payload {
                PackageInterfaceDeclarationPayload::Function(f) => {
                    !matches!(f.result.relation, ResultRelation::Value)
                }
                PackageInterfaceDeclarationPayload::OwnedContract(c) => c
                    .methods
                    .iter()
                    .any(|m| !matches!(m.result.relation, ResultRelation::Value)),
                _ => false,
            }
        {
            return Err(interface_error(
                DiagnosticClass::Semantic,
                "kernel_borrow_result_generation",
                "source-tied borrowed results require Graph 27",
            ));
        }
        if self.header.contract_version < 26
            && match &self.payload {
                PackageInterfaceDeclarationPayload::OwnedContract(c) => {
                    !c.type_parameters.is_empty()
                }
                PackageInterfaceDeclarationPayload::OwnedImplementation(i) => {
                    !i.type_arguments.is_empty()
                }
                PackageInterfaceDeclarationPayload::Function(f) => f
                    .implementation_parameters
                    .iter()
                    .any(|p| !p.type_arguments.is_empty()),
                _ => false,
            }
        {
            return Err(interface_error(
                DiagnosticClass::Semantic,
                "kernel_parameterized_contract_generation",
                "owned contract parameters and arguments require Graph 26",
            ));
        }
        let expected = match &self.payload {
            PackageInterfaceDeclarationPayload::ReadView(view) => {
                if self.header.contract_version < 34 {
                    return Err(interface_error(
                        DiagnosticClass::Semantic,
                        "kernel_read_view_generation",
                        "read-view interfaces require Graph 34",
                    ));
                }
                validate_ordered("read-view type parameters", &view.type_parameters)?;
                view.validate_local()?;
                OwnerKind::ReadView
            }
            PackageInterfaceDeclarationPayload::Record {
                fields,
                type_parameters,
            } => {
                validate_ordered("record type parameters", type_parameters)?;
                validate_sorted("record fields", fields, false)?;
                OwnerKind::Record
            }
            PackageInterfaceDeclarationPayload::Variant {
                cases,
                type_parameters,
            } => {
                validate_ordered("variant type parameters", type_parameters)?;
                validate_sorted("variant cases", cases, false)?;
                OwnerKind::Variant
            }
            PackageInterfaceDeclarationPayload::OwnedRecord {
                fields,
                type_parameters,
            } => {
                validate_ordered("owned record type parameters", type_parameters)?;
                validate_sorted("owned record fields", fields, true)?;
                OwnerKind::OwnedRecord
            }
            PackageInterfaceDeclarationPayload::Interface { operations } => {
                validate_sorted("interface operations", operations, false)?;
                OwnerKind::Interface
            }
            PackageInterfaceDeclarationPayload::External(signature) => {
                validate_ordered("external type parameters", &signature.type_parameters)?;
                validate_ordered("external parameters", &signature.parameters)?;
                OwnerKind::External
            }
            PackageInterfaceDeclarationPayload::Function(signature) => {
                validate_ordered(
                    "function requirement parameters",
                    &signature.requirement_parameters,
                )?;
                validate_ordered("function effect parameters", &signature.effect_parameters)?;
                signature.effect.row().validate()?;
                validate_ordered("function type parameters", &signature.type_parameters)?;
                validate_ordered("function parameters", &signature.parameters)?;
                if let Some(source) = match &signature.result.relation {
                    ResultRelation::Value => None,
                    ResultRelation::ReadFrom { source }
                    | ResultRelation::ViewFrom { source, .. } => Some(*source),
                } && (!signature.parameters.contains(&source)
                    || signature.effect != FunctionEffect::Pure)
                {
                    return Err(interface_error(
                        DiagnosticClass::Semantic,
                        "kernel_borrow_result",
                        "a borrowed result requires an exact parameter of a pure function",
                    ));
                }
                validate_callable_scopes(self.header, signature)?;
                if let FunctionEffect::Task {
                    effect_parameters: _,
                    requirements,
                } = &signature.effect
                {
                    validate_sorted("task requirements", requirements, true)?;
                    OwnerKind::TaskFunction
                } else {
                    OwnerKind::PureFunction
                }
            }
            PackageInterfaceDeclarationPayload::OwnedContract(c) => {
                c.validate_local()?;
                OwnerKind::OwnedContract
            }
            PackageInterfaceDeclarationPayload::OwnedImplementation(i) => {
                i.validate_local()?;
                OwnerKind::OwnedImplementation
            }
            PackageInterfaceDeclarationPayload::Constant { .. } => OwnerKind::Constant,
            PackageInterfaceDeclarationPayload::Component {
                requirements,
                ports,
            } => {
                validate_sorted("component requirements", requirements, true)?;
                validate_sorted("component ports", ports, false)?;
                OwnerKind::Component
            }
        };
        if self.header.kind != expected || !matches!(self.header.owner, OwnerKey::Declaration(_)) {
            return Err(interface_error(
                DiagnosticClass::Corrupt,
                "kernel_package_interface_declaration_kind",
                "package-interface declaration header disagrees with its closed payload",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum PackageInterfaceDeclarationPayload {
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
    External(PackageExternalSignature),
    Function(PackageFunctionSignature),
    OwnedContract(super::OwnedContract),
    OwnedImplementation(super::OwnedImplementation),
    Constant {
        ty: TypeObjectDigest,
    },
    Component {
        requirements: Vec<RequirementId>,
        ports: Vec<PortId>,
    },
    OwnedRecord {
        type_parameters: Vec<TypeParameterId>,
        fields: Vec<FieldId>,
    },
    ReadView(super::ReadViewDeclaration),
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageExternalSignature {
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageFunctionSignature {
    pub implementation_parameters: Vec<super::ImplementationParameter>,
    pub requirement_parameters: Vec<crate::platform::semantic_id::RequirementParameterId>,
    pub effect_parameters: Vec<EffectParameterId>,
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: CallableResult<ParameterId>,
    pub read_scope_parameters: Vec<ReadScopeParameter>,
    pub read_scope_sources: Vec<ReadScopeSource<ParameterId>>,
    pub effect: FunctionEffect,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageInterfacePort {
    pub header: OwnerHeader,
    pub declaration: DeclarationId,
    pub name: Name,
    pub function_type: TypeObjectDigest,
}

impl PackageInterfacePort {
    fn validate_local(&self) -> Result<(), Diagnostic> {
        validate_header(self.header)?;
        if self.header.kind != OwnerKind::Port || !matches!(self.header.owner, OwnerKey::Port(_)) {
            return Err(interface_error(
                DiagnosticClass::Corrupt,
                "kernel_package_interface_port_kind",
                "package-interface port header has a foreign identity or owner kind",
            ));
        }
        Ok(())
    }
}

fn validate_callable_scopes(
    header: OwnerHeader,
    signature: &PackageFunctionSignature,
) -> Result<(), Diagnostic> {
    let ids = signature
        .read_scope_parameters
        .iter()
        .map(|parameter| parameter.id)
        .collect::<Vec<_>>();
    validate_ordered("function read-scope parameters", &ids)?;
    super::validate_read_scope_inventory(
        &signature.read_scope_parameters,
        &signature.read_scope_sources,
        |source| signature.parameters.contains(&source),
    )?;
    if header.contract_version < 34
        && (!ids.is_empty()
            || !signature.read_scope_sources.is_empty()
            || matches!(signature.result.relation, ResultRelation::ViewFrom { .. }))
    {
        return Err(interface_error(
            DiagnosticClass::Semantic,
            "kernel_read_view_generation",
            "read scopes and view results require Graph 34",
        ));
    }
    for scope in signature
        .read_scope_sources
        .iter()
        .map(|entry| &entry.scope)
        .chain(match &signature.result.relation {
            ResultRelation::ViewFrom { scope, .. } => Some(scope),
            _ => None,
        })
    {
        match scope {
            super::ReadScopeArgument::Parameter(reference)
                if ids.contains(&reference.parameter) => {}
            _ => {
                return Err(interface_error(
                    DiagnosticClass::Semantic,
                    "kernel_read_scope_signature",
                    "signature scopes must name an explicit read-scope formal",
                ));
            }
        }
    }
    Ok(())
}

fn validate_header(header: OwnerHeader) -> Result<(), Diagnostic> {
    if !super::contract::supported_graph_contract(header.contract_version)
        || !header.kind.accepts_owner(header.owner)
    {
        return Err(interface_error(
            DiagnosticClass::Corrupt,
            "kernel_package_interface_header",
            "package-interface owner header has a foreign contract, identity, or kind",
        ));
    }
    Ok(())
}

fn validate_sorted<T: Ord>(label: &str, values: &[T], allow_empty: bool) -> Result<(), Diagnostic> {
    if (!allow_empty && values.is_empty())
        || values.len() > MAXIMUM_CHILDREN
        || values.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(interface_error(
            DiagnosticClass::Semantic,
            "kernel_package_interface_child_set",
            format!("{label} must be within bounds and strictly sorted"),
        ));
    }
    Ok(())
}

fn validate_ordered<T: Ord + Copy>(label: &str, values: &[T]) -> Result<(), Diagnostic> {
    if values.len() > MAXIMUM_CHILDREN
        || values.iter().copied().collect::<BTreeSet<_>>().len() != values.len()
    {
        return Err(interface_error(
            DiagnosticClass::Semantic,
            "kernel_package_interface_child_order",
            format!("{label} must be within bounds and contain unique identities"),
        ));
    }
    Ok(())
}

fn interface_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
