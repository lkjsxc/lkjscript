//! Implementation-free semantic owner records consumed across exact package boundaries.
//!
//! This is a logical kernel view, not a stored authority or transport. Package export decides
//! which public owners are reachable, binds these records to accepted summary dimensions, and
//! stores them in a separate derived interface map.

use super::contract::{GRAPH_CONTRACT_VERSION, MAXIMUM_CHILDREN};
use super::{
    CaseRecord, DeclarationPayload, DeclarationVisibility, FieldRecord, FunctionEffect, Name,
    OperationRecord, OwnerHeader, OwnerKey, OwnerKind, OwnerRecord, ParameterRecord,
    RequirementRecord, TypeObjectDigest, TypeParameterRecord,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{
    CaseId, DeclarationId, FieldId, OperationId, ParameterId, PortId, RequirementId,
    TypeParameterId,
};
use bincode::{Decode, Encode};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum PackageInterfaceRecord {
    Declaration(PackageInterfaceDeclaration),
    TypeParameter(TypeParameterRecord),
    Field(FieldRecord),
    Case(CaseRecord),
    Operation(OperationRecord),
    Parameter(ParameterRecord),
    Requirement(RequirementRecord),
    Port(PackageInterfacePort),
}

impl PackageInterfaceRecord {
    pub(crate) fn project_public(canonical: &OwnerRecord) -> Result<Option<Self>, Diagnostic> {
        Ok(Some(match canonical {
            OwnerRecord::Declaration(record) => {
                Self::Declaration(PackageInterfaceDeclaration::project(record)?)
            }
            OwnerRecord::TypeParameter(record) => Self::TypeParameter(record.clone()),
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
            | OwnerRecord::Documentation(_)
            | OwnerRecord::Annotation(_) => return Ok(None),
        }))
    }

    pub fn header(&self) -> OwnerHeader {
        match self {
            Self::Declaration(record) => record.header,
            Self::TypeParameter(record) => record.header,
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
            Self::Declaration(record) => record.type_roots(),
            Self::Field(record) => vec![record.ty],
            Self::Case(record) => record.payload.into_iter().collect(),
            Self::Operation(record) => vec![record.result],
            Self::Parameter(record) => vec![record.ty],
            Self::Port(record) => vec![record.function_type],
            Self::TypeParameter(_) | Self::Requirement(_) => Vec::new(),
        }
    }

    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
        match self {
            Self::Declaration(record) => record.validate_local(),
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
        if record.visibility != DeclarationVisibility::Public {
            return Err(interface_error(
                DiagnosticClass::Corrupt,
                "kernel_package_interface_visibility",
                "package-interface selection admitted a non-public declaration",
            ));
        }
        let payload = match &record.payload {
            DeclarationPayload::Record { fields } => PackageInterfaceDeclarationPayload::Record {
                fields: fields.clone(),
            },
            DeclarationPayload::Variant { cases } => PackageInterfaceDeclarationPayload::Variant {
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
                    type_parameters: function.type_parameters.clone(),
                    parameters: function.parameters.clone(),
                    result: function.result,
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
            PackageInterfaceDeclarationPayload::Function(signature) => vec![signature.result],
            PackageInterfaceDeclarationPayload::Constant { ty } => vec![*ty],
            PackageInterfaceDeclarationPayload::Record { .. }
            | PackageInterfaceDeclarationPayload::Variant { .. }
            | PackageInterfaceDeclarationPayload::Interface { .. }
            | PackageInterfaceDeclarationPayload::Component { .. } => Vec::new(),
        }
    }

    fn validate_local(&self) -> Result<(), Diagnostic> {
        validate_header(self.header)?;
        let expected = match &self.payload {
            PackageInterfaceDeclarationPayload::Record { fields } => {
                validate_sorted("record fields", fields, false)?;
                OwnerKind::Record
            }
            PackageInterfaceDeclarationPayload::Variant { cases } => {
                validate_sorted("variant cases", cases, false)?;
                OwnerKind::Variant
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
                validate_ordered("function type parameters", &signature.type_parameters)?;
                validate_ordered("function parameters", &signature.parameters)?;
                if let FunctionEffect::Task { requirements } = &signature.effect {
                    validate_sorted("task requirements", requirements, true)?;
                    OwnerKind::TaskFunction
                } else {
                    OwnerKind::PureFunction
                }
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
        fields: Vec<FieldId>,
    },
    Variant {
        cases: Vec<CaseId>,
    },
    Interface {
        operations: Vec<OperationId>,
    },
    External(PackageExternalSignature),
    Function(PackageFunctionSignature),
    Constant {
        ty: TypeObjectDigest,
    },
    Component {
        requirements: Vec<RequirementId>,
        ports: Vec<PortId>,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageExternalSignature {
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageFunctionSignature {
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
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

fn validate_header(header: OwnerHeader) -> Result<(), Diagnostic> {
    if header.contract_version != GRAPH_CONTRACT_VERSION || !header.kind.accepts_owner(header.owner)
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
