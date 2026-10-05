//! Frozen package interface generation 11. Variant tags and field order are immutable.
use super::*;
use crate::platform::semantic_id::*;
use bincode::{Decode, Encode};

#[derive(Clone, Debug, Decode, Encode)]
pub enum PackageInterfaceRecord11 {
    Declaration(PackageInterfaceDeclaration11),
    TypeParameter(TypeParameterRecord),
    EffectParameter(EffectParameterRecord),
    Field(FieldRecord),
    Case(CaseRecord),
    Operation(OperationRecord),
    Parameter(ParameterRecord),
    Requirement(RequirementRecord),
    Port(PackageInterfacePort),
    RequirementParameter(super::RequirementParameterRecord),
}

#[derive(Clone, Debug, Decode, Encode)]
pub struct PackageInterfaceDeclaration11 {
    pub header: OwnerHeader,
    pub name: Name,
    pub payload: PackageInterfaceDeclarationPayload11,
}

#[derive(Clone, Debug, Decode, Encode)]
pub enum PackageInterfaceDeclarationPayload11 {
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
    Function(PackageFunctionSignature11),
    Constant {
        ty: TypeObjectDigest,
    },
    Component {
        requirements: Vec<RequirementId>,
        ports: Vec<PortId>,
    },
}

#[derive(Clone, Debug, Decode, Encode)]
pub struct PackageFunctionSignature11 {
    pub requirement_parameters: Vec<crate::platform::semantic_id::RequirementParameterId>,
    pub effect_parameters: Vec<EffectParameterId>,
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    pub effect: FunctionEffect,
}

impl From<PackageInterfaceRecord11> for PackageInterfaceRecord {
    fn from(value: PackageInterfaceRecord11) -> Self {
        match value {
            PackageInterfaceRecord11::Declaration(v) => Self::Declaration(v.into()),
            PackageInterfaceRecord11::TypeParameter(v) => Self::TypeParameter(v),
            PackageInterfaceRecord11::EffectParameter(v) => Self::EffectParameter(v),
            PackageInterfaceRecord11::Field(v) => Self::Field(v),
            PackageInterfaceRecord11::Case(v) => Self::Case(v),
            PackageInterfaceRecord11::Operation(v) => Self::Operation(v),
            PackageInterfaceRecord11::Parameter(v) => Self::Parameter(v),
            PackageInterfaceRecord11::Requirement(v) => Self::Requirement(v),
            PackageInterfaceRecord11::Port(v) => Self::Port(v),
            PackageInterfaceRecord11::RequirementParameter(v) => Self::RequirementParameter(v),
        }
    }
}
impl TryFrom<PackageInterfaceRecord> for PackageInterfaceRecord11 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(value: PackageInterfaceRecord) -> Result<Self, Self::Error> {
        Ok(match value {
            PackageInterfaceRecord::Declaration(v) => Self::Declaration(v.try_into()?),
            PackageInterfaceRecord::TypeParameter(v) => Self::TypeParameter(v),
            PackageInterfaceRecord::EffectParameter(v) => Self::EffectParameter(v),
            PackageInterfaceRecord::Field(v) => Self::Field(v),
            PackageInterfaceRecord::Case(v) => Self::Case(v),
            PackageInterfaceRecord::Operation(v) => Self::Operation(v),
            PackageInterfaceRecord::Parameter(v) => Self::Parameter(v),
            PackageInterfaceRecord::Requirement(v) => Self::Requirement(v),
            PackageInterfaceRecord::Port(v) => Self::Port(v),
            PackageInterfaceRecord::RequirementParameter(v) => Self::RequirementParameter(v),
        })
    }
}
impl From<PackageInterfaceDeclaration11> for PackageInterfaceDeclaration {
    fn from(v: PackageInterfaceDeclaration11) -> Self {
        Self {
            header: v.header,
            name: v.name,
            payload: v.payload.into(),
        }
    }
}
impl TryFrom<PackageInterfaceDeclaration> for PackageInterfaceDeclaration11 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: PackageInterfaceDeclaration) -> Result<Self, Self::Error> {
        Ok(Self {
            header: v.header,
            name: v.name,
            payload: v.payload.try_into()?,
        })
    }
}
impl From<PackageInterfaceDeclarationPayload11> for PackageInterfaceDeclarationPayload {
    fn from(v: PackageInterfaceDeclarationPayload11) -> Self {
        match v {
            PackageInterfaceDeclarationPayload11::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            PackageInterfaceDeclarationPayload11::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            PackageInterfaceDeclarationPayload11::Interface { operations } => {
                Self::Interface { operations }
            }
            PackageInterfaceDeclarationPayload11::Constant { ty } => Self::Constant { ty },
            PackageInterfaceDeclarationPayload11::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
            PackageInterfaceDeclarationPayload11::External(v) => Self::External(v),
            PackageInterfaceDeclarationPayload11::Function(v) => Self::Function(v.into()),
        }
    }
}
impl TryFrom<PackageInterfaceDeclarationPayload> for PackageInterfaceDeclarationPayload11 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: PackageInterfaceDeclarationPayload) -> Result<Self, Self::Error> {
        Ok(match v {
            PackageInterfaceDeclarationPayload::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            PackageInterfaceDeclarationPayload::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            PackageInterfaceDeclarationPayload::Interface { operations } => {
                Self::Interface { operations }
            }
            PackageInterfaceDeclarationPayload::Constant { ty } => Self::Constant { ty },
            PackageInterfaceDeclarationPayload::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
            PackageInterfaceDeclarationPayload::External(v) => Self::External(v),
            PackageInterfaceDeclarationPayload::Function(v) => Self::Function(v.try_into()?),
            PackageInterfaceDeclarationPayload::OwnedContract(_)
            | PackageInterfaceDeclarationPayload::OwnedImplementation(_) => return Err(extension()),
        })
    }
}
impl From<PackageFunctionSignature11> for PackageFunctionSignature {
    fn from(v: PackageFunctionSignature11) -> Self {
        Self {
            implementation_parameters: Vec::new(),
            requirement_parameters: v.requirement_parameters,
            effect_parameters: v.effect_parameters,
            type_parameters: v.type_parameters,
            parameters: v.parameters,
            result: v.result,
            result_borrow: None,
            effect: v.effect,
        }
    }
}
impl TryFrom<PackageFunctionSignature> for PackageFunctionSignature11 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: PackageFunctionSignature) -> Result<Self, Self::Error> {
        if v.result_borrow.is_some() {
            return Err(extension());
        }
        if !v.implementation_parameters.is_empty() {
            return Err(extension());
        }
        Ok(Self {
            requirement_parameters: v.requirement_parameters,
            effect_parameters: v.effect_parameters,
            type_parameters: v.type_parameters,
            parameters: v.parameters,
            result: v.result,
            effect: v.effect,
        })
    }
}
fn extension() -> crate::platform::diagnostic::Diagnostic {
    crate::platform::diagnostic::Diagnostic::new(
        crate::platform::diagnostic::DiagnosticClass::Semantic,
        "package_interface_generation",
        "owned contracts and witness signatures require interface generation 12",
    )
}
