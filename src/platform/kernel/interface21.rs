//! Frozen Interface 17..21 physical records used only by the ordinary bootstrap reader.
//! They preserve predecessor bytes and never provide read-view or scope authority.

use super::frozen_read_view_wire::{OwnedContract33, OwnedImplementation33};
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{
    CaseId, EffectParameterId, FieldId, OperationId, ParameterId, PortId, RequirementId,
    TypeParameterId,
};
use bincode::{Decode, Encode};

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub(crate) enum PackageInterfaceRecord21 {
    Declaration(PackageInterfaceDeclaration21),
    TypeParameter(TypeParameterRecord),
    EffectParameter(EffectParameterRecord),
    Field(FieldRecord),
    Case(CaseRecord),
    Operation(OperationRecord),
    Parameter(ParameterRecord),
    Requirement(RequirementRecord),
    Port(PackageInterfacePort),
    RequirementParameter(RequirementParameterRecord),
    RepresentationDeclaration(PackageInterfaceDeclaration21),
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub(crate) struct PackageInterfaceDeclaration21 {
    pub header: OwnerHeader,
    pub name: Name,
    pub payload: PackageInterfaceDeclarationPayload21,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub(crate) enum PackageInterfaceDeclarationPayload21 {
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
    Function(PackageFunctionSignature21),
    OwnedContract(OwnedContract33),
    OwnedImplementation(OwnedImplementation33),
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
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub(crate) struct PackageFunctionSignature21 {
    pub implementation_parameters: Vec<ImplementationParameter>,
    pub requirement_parameters: Vec<crate::platform::semantic_id::RequirementParameterId>,
    pub effect_parameters: Vec<EffectParameterId>,
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    pub result_borrow: Option<ParameterId>,
    pub effect: FunctionEffect,
}

impl From<PackageInterfaceDeclaration21> for PackageInterfaceDeclaration {
    fn from(value: PackageInterfaceDeclaration21) -> Self {
        Self {
            header: value.header,
            name: value.name,
            payload: value.payload.into(),
        }
    }
}
impl TryFrom<PackageInterfaceDeclaration> for PackageInterfaceDeclaration21 {
    type Error = Diagnostic;
    fn try_from(value: PackageInterfaceDeclaration) -> Result<Self, Diagnostic> {
        Ok(Self {
            header: value.header,
            name: value.name,
            payload: value.payload.try_into()?,
        })
    }
}
impl From<PackageInterfaceDeclarationPayload21> for PackageInterfaceDeclarationPayload {
    fn from(value: PackageInterfaceDeclarationPayload21) -> Self {
        use PackageInterfaceDeclarationPayload21 as W;
        match value {
            W::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            W::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            W::Interface { operations } => Self::Interface { operations },
            W::External(value) => Self::External(value),
            W::Function(value) => Self::Function(value.into()),
            W::OwnedContract(value) => Self::OwnedContract(value.into()),
            W::OwnedImplementation(value) => Self::OwnedImplementation(value.into()),
            W::Constant { ty } => Self::Constant { ty },
            W::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
            W::OwnedRecord {
                type_parameters,
                fields,
            } => Self::OwnedRecord {
                type_parameters,
                fields,
            },
        }
    }
}
impl TryFrom<PackageInterfaceDeclarationPayload> for PackageInterfaceDeclarationPayload21 {
    type Error = Diagnostic;
    fn try_from(value: PackageInterfaceDeclarationPayload) -> Result<Self, Diagnostic> {
        use PackageInterfaceDeclarationPayload as C;
        Ok(match value {
            C::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            C::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            C::Interface { operations } => Self::Interface { operations },
            C::External(value) => Self::External(value),
            C::Function(value) => Self::Function(value.try_into()?),
            C::OwnedContract(value) => Self::OwnedContract(value.try_into()?),
            C::OwnedImplementation(value) => Self::OwnedImplementation(value.try_into()?),
            C::Constant { ty } => Self::Constant { ty },
            C::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
            C::OwnedRecord {
                type_parameters,
                fields,
            } => Self::OwnedRecord {
                type_parameters,
                fields,
            },
            C::ReadView(_) => return Err(scope_cut()),
        })
    }
}
impl From<PackageFunctionSignature21> for PackageFunctionSignature {
    fn from(value: PackageFunctionSignature21) -> Self {
        Self {
            implementation_parameters: value.implementation_parameters,
            requirement_parameters: value.requirement_parameters,
            effect_parameters: value.effect_parameters,
            type_parameters: value.type_parameters,
            parameters: value.parameters,
            result: CallableResult {
                ty: value.result,
                relation: match value.result_borrow {
                    Some(source) => ResultRelation::ReadFrom { source },
                    None => ResultRelation::Value,
                },
            },
            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
            effect: value.effect,
        }
    }
}
impl TryFrom<PackageFunctionSignature> for PackageFunctionSignature21 {
    type Error = Diagnostic;
    fn try_from(value: PackageFunctionSignature) -> Result<Self, Diagnostic> {
        if !value.read_scope_parameters.is_empty() || !value.read_scope_sources.is_empty() {
            return Err(scope_cut());
        }
        let result_borrow = match value.result.relation {
            ResultRelation::Value => None,
            ResultRelation::ReadFrom { source } => Some(source),
            ResultRelation::ViewFrom { .. } => return Err(scope_cut()),
        };
        Ok(Self {
            implementation_parameters: value.implementation_parameters,
            requirement_parameters: value.requirement_parameters,
            effect_parameters: value.effect_parameters,
            type_parameters: value.type_parameters,
            parameters: value.parameters,
            result: value.result.ty,
            result_borrow,
            effect: value.effect,
        })
    }
}
impl From<PackageInterfaceRecord21> for PackageInterfaceRecord {
    fn from(value: PackageInterfaceRecord21) -> Self {
        use PackageInterfaceRecord21 as W;
        match value {
            W::Declaration(v) => Self::Declaration(v.into()),
            W::RepresentationDeclaration(v) => Self::RepresentationDeclaration(v.into()),
            W::TypeParameter(v) => Self::TypeParameter(v),
            W::EffectParameter(v) => Self::EffectParameter(v),
            W::Field(v) => Self::Field(v),
            W::Case(v) => Self::Case(v),
            W::Operation(v) => Self::Operation(v),
            W::Parameter(v) => Self::Parameter(v),
            W::Requirement(v) => Self::Requirement(v),
            W::Port(v) => Self::Port(v),
            W::RequirementParameter(v) => Self::RequirementParameter(v),
        }
    }
}
impl TryFrom<PackageInterfaceRecord> for PackageInterfaceRecord21 {
    type Error = Diagnostic;
    fn try_from(value: PackageInterfaceRecord) -> Result<Self, Diagnostic> {
        use PackageInterfaceRecord as C;
        Ok(match value {
            C::Declaration(v) => Self::Declaration(v.try_into()?),
            C::RepresentationDeclaration(v) => Self::RepresentationDeclaration(v.try_into()?),
            C::TypeParameter(v) => Self::TypeParameter(v),
            C::EffectParameter(v) => Self::EffectParameter(v),
            C::Field(v) => Self::Field(v),
            C::Case(v) => Self::Case(v),
            C::Operation(v) => Self::Operation(v),
            C::Parameter(v) => Self::Parameter(v),
            C::Requirement(v) => Self::Requirement(v),
            C::Port(v) => Self::Port(v),
            C::RequirementParameter(v) => Self::RequirementParameter(v),
        })
    }
}
fn scope_cut() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Semantic,
        "kernel_read_view_generation",
        "read-view and read-scope meaning cannot be represented by a frozen Interface 21 codec",
    )
}
