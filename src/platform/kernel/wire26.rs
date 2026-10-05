//! Frozen Graph 26 declarations and interface signatures.
//! Borrowed-result relationships cannot reinterpret original Graph 26 bytes.

use super::*;
use crate::platform::semantic_id::*;
use bincode::{Decode, Encode};

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum OwnerRecord26 {
    Module(ModuleRecord),
    Declaration(DeclarationRecord26),
    TypeParameter(TypeParameterRecord),
    EffectParameter(EffectParameterRecord),
    Field(FieldRecord),
    Case(CaseRecord),
    Operation(OperationRecord),
    Parameter(ParameterRecord),
    Binding(BindingRecord),
    Expression(super::wire27::ExpressionRecord27),
    Requirement(RequirementRecord),
    Port(PortRecord),
    Target(TargetRecord),
    Documentation(DocumentationRecord),
    Annotation(AnnotationRecord),
    HttpRoute(HttpRouteRecord),
    RequirementParameter(RequirementParameterRecord),
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct DeclarationRecord26 {
    pub header: OwnerHeader,
    pub module: ModuleId,
    pub name: Name,
    pub visibility: DeclarationVisibility,
    pub payload: DeclarationPayload26,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum DeclarationPayload26 {
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
    Function(FunctionDeclaration26),
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
    OwnedContract(OwnedContract26),
    OwnedImplementation(super::wire27::OwnedImplementation27),
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct FunctionDeclaration26 {
    pub implementation_parameters: Vec<ImplementationParameter>,
    pub requirement_parameters: Vec<crate::platform::semantic_id::RequirementParameterId>,
    pub effect_parameters: Vec<EffectParameterId>,
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    pub effect: FunctionEffect,
    pub body: ExpressionId,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum PackageInterfaceRecord26 {
    Declaration(PackageInterfaceDeclaration26),
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

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageInterfaceDeclaration26 {
    pub header: OwnerHeader,
    pub name: Name,
    pub payload: PackageInterfaceDeclarationPayload26,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum PackageInterfaceDeclarationPayload26 {
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
    Function(PackageFunctionSignature26),
    OwnedContract(OwnedContract26),
    OwnedImplementation(super::wire27::OwnedImplementation27),
    Constant {
        ty: TypeObjectDigest,
    },
    Component {
        requirements: Vec<RequirementId>,
        ports: Vec<PortId>,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageFunctionSignature26 {
    pub implementation_parameters: Vec<ImplementationParameter>,
    pub requirement_parameters: Vec<crate::platform::semantic_id::RequirementParameterId>,
    pub effect_parameters: Vec<EffectParameterId>,
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    pub effect: FunctionEffect,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct OwnedContract26 {
    pub self_parameter: TypeParameterId,
    pub type_parameters: Vec<TypeParameterId>,
    pub methods: Vec<OwnedMethod26>,
}

impl From<DeclarationRecord26> for DeclarationRecord {
    fn from(v: DeclarationRecord26) -> Self {
        Self {
            header: v.header,
            module: v.module,
            name: v.name,
            visibility: v.visibility,
            payload: v.payload.into(),
        }
    }
}

impl TryFrom<DeclarationRecord> for DeclarationRecord26 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: DeclarationRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            header: v.header,
            module: v.module,
            name: v.name,
            visibility: v.visibility,
            payload: v.payload.try_into()?,
        })
    }
}

impl From<FunctionDeclaration26> for FunctionDeclaration {
    fn from(v: FunctionDeclaration26) -> Self {
        Self {
            implementation_parameters: v.implementation_parameters,
            requirement_parameters: v.requirement_parameters,
            effect_parameters: v.effect_parameters,
            type_parameters: v.type_parameters,
            parameters: v.parameters,
            result: v.result,
            result_borrow: None,
            effect: v.effect,
            body: v.body,
        }
    }
}

impl TryFrom<FunctionDeclaration> for FunctionDeclaration26 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: FunctionDeclaration) -> Result<Self, Self::Error> {
        if v.result_borrow.is_some() {
            return Err(extension());
        }
        Ok(Self {
            implementation_parameters: v.implementation_parameters,
            requirement_parameters: v.requirement_parameters,
            effect_parameters: v.effect_parameters,
            type_parameters: v.type_parameters,
            parameters: v.parameters,
            result: v.result,
            effect: v.effect,
            body: v.body,
        })
    }
}

impl From<PackageInterfaceDeclaration26> for PackageInterfaceDeclaration {
    fn from(v: PackageInterfaceDeclaration26) -> Self {
        Self {
            header: v.header,
            name: v.name,
            payload: v.payload.into(),
        }
    }
}

impl TryFrom<PackageInterfaceDeclaration> for PackageInterfaceDeclaration26 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: PackageInterfaceDeclaration) -> Result<Self, Self::Error> {
        Ok(Self {
            header: v.header,
            name: v.name,
            payload: v.payload.try_into()?,
        })
    }
}

impl From<PackageFunctionSignature26> for PackageFunctionSignature {
    fn from(v: PackageFunctionSignature26) -> Self {
        Self {
            implementation_parameters: v.implementation_parameters,
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

impl TryFrom<PackageFunctionSignature> for PackageFunctionSignature26 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: PackageFunctionSignature) -> Result<Self, Self::Error> {
        if v.result_borrow.is_some() {
            return Err(extension());
        }
        Ok(Self {
            implementation_parameters: v.implementation_parameters,
            requirement_parameters: v.requirement_parameters,
            effect_parameters: v.effect_parameters,
            type_parameters: v.type_parameters,
            parameters: v.parameters,
            result: v.result,
            effect: v.effect,
        })
    }
}

impl From<OwnedContract26> for OwnedContract {
    fn from(v: OwnedContract26) -> Self {
        Self {
            self_parameter: v.self_parameter,
            methods: v.methods.into_iter().map(Into::into).collect(),
            type_parameters: v.type_parameters,
        }
    }
}

impl TryFrom<OwnedContract> for OwnedContract26 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: OwnedContract) -> Result<Self, Self::Error> {
        Ok(Self {
            self_parameter: v.self_parameter,
            type_parameters: v.type_parameters,
            methods: v
                .methods
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<OwnerRecord26> for OwnerRecord {
    fn from(v: OwnerRecord26) -> Self {
        match v {
            OwnerRecord26::Module(v) => Self::Module(v),
            OwnerRecord26::Declaration(v) => Self::Declaration(v.into()),
            OwnerRecord26::TypeParameter(v) => Self::TypeParameter(v),
            OwnerRecord26::EffectParameter(v) => Self::EffectParameter(v),
            OwnerRecord26::Field(v) => Self::Field(v),
            OwnerRecord26::Case(v) => Self::Case(v),
            OwnerRecord26::Operation(v) => Self::Operation(v),
            OwnerRecord26::Parameter(v) => Self::Parameter(v),
            OwnerRecord26::Binding(v) => Self::Binding(v),
            OwnerRecord26::Expression(v) => Self::Expression(v.into()),
            OwnerRecord26::Requirement(v) => Self::Requirement(v),
            OwnerRecord26::Port(v) => Self::Port(v),
            OwnerRecord26::Target(v) => Self::Target(v),
            OwnerRecord26::Documentation(v) => Self::Documentation(v),
            OwnerRecord26::Annotation(v) => Self::Annotation(v),
            OwnerRecord26::HttpRoute(v) => Self::HttpRoute(v),
            OwnerRecord26::RequirementParameter(v) => Self::RequirementParameter(v),
        }
    }
}

impl TryFrom<OwnerRecord> for OwnerRecord26 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: OwnerRecord) -> Result<Self, Self::Error> {
        Ok(match v {
            OwnerRecord::Module(v) => Self::Module(v),
            OwnerRecord::Declaration(v) => Self::Declaration(v.try_into()?),
            OwnerRecord::TypeParameter(v) => Self::TypeParameter(v),
            OwnerRecord::EffectParameter(v) => Self::EffectParameter(v),
            OwnerRecord::Field(v) => Self::Field(v),
            OwnerRecord::Case(v) => Self::Case(v),
            OwnerRecord::Operation(v) => Self::Operation(v),
            OwnerRecord::Parameter(v) => Self::Parameter(v),
            OwnerRecord::Binding(v) => Self::Binding(v),
            OwnerRecord::Expression(v) => Self::Expression(v.try_into()?),
            OwnerRecord::Requirement(v) => Self::Requirement(v),
            OwnerRecord::Port(v) => Self::Port(v),
            OwnerRecord::Target(v) => Self::Target(v),
            OwnerRecord::Documentation(v) => Self::Documentation(v),
            OwnerRecord::Annotation(v) => Self::Annotation(v),
            OwnerRecord::HttpRoute(v) => Self::HttpRoute(v),
            OwnerRecord::RequirementParameter(v) => Self::RequirementParameter(v),
        })
    }
}

impl From<PackageInterfaceRecord26> for PackageInterfaceRecord {
    fn from(v: PackageInterfaceRecord26) -> Self {
        match v {
            PackageInterfaceRecord26::Declaration(v) => Self::Declaration(v.into()),
            PackageInterfaceRecord26::TypeParameter(v) => Self::TypeParameter(v),
            PackageInterfaceRecord26::EffectParameter(v) => Self::EffectParameter(v),
            PackageInterfaceRecord26::Field(v) => Self::Field(v),
            PackageInterfaceRecord26::Case(v) => Self::Case(v),
            PackageInterfaceRecord26::Operation(v) => Self::Operation(v),
            PackageInterfaceRecord26::Parameter(v) => Self::Parameter(v),
            PackageInterfaceRecord26::Requirement(v) => Self::Requirement(v),
            PackageInterfaceRecord26::Port(v) => Self::Port(v),
            PackageInterfaceRecord26::RequirementParameter(v) => Self::RequirementParameter(v),
        }
    }
}

impl TryFrom<PackageInterfaceRecord> for PackageInterfaceRecord26 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: PackageInterfaceRecord) -> Result<Self, Self::Error> {
        Ok(match v {
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

impl From<DeclarationPayload26> for DeclarationPayload {
    fn from(v: DeclarationPayload26) -> Self {
        match v {
            DeclarationPayload26::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            DeclarationPayload26::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            DeclarationPayload26::Interface { operations } => Self::Interface { operations },
            DeclarationPayload26::External(v) => Self::External(v),
            DeclarationPayload26::Function(v) => Self::Function(v.into()),
            DeclarationPayload26::Constant { ty, value } => Self::Constant { ty, value },
            DeclarationPayload26::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
            DeclarationPayload26::Test {
                actual,
                expected,
                comparison,
            } => Self::Test {
                actual,
                expected,
                comparison,
            },
            DeclarationPayload26::OwnedContract(v) => Self::OwnedContract(v.into()),
            DeclarationPayload26::OwnedImplementation(v) => Self::OwnedImplementation(v.into()),
        }
    }
}

impl TryFrom<DeclarationPayload> for DeclarationPayload26 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: DeclarationPayload) -> Result<Self, Self::Error> {
        Ok(match v {
            DeclarationPayload::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            DeclarationPayload::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            DeclarationPayload::Interface { operations } => Self::Interface { operations },
            DeclarationPayload::External(v) => Self::External(v),
            DeclarationPayload::Function(v) => Self::Function(v.try_into()?),
            DeclarationPayload::Constant { ty, value } => Self::Constant { ty, value },
            DeclarationPayload::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
            DeclarationPayload::Test {
                actual,
                expected,
                comparison,
            } => Self::Test {
                actual,
                expected,
                comparison,
            },
            DeclarationPayload::OwnedContract(v) => Self::OwnedContract(v.try_into()?),
            DeclarationPayload::OwnedImplementation(v) => Self::OwnedImplementation(v.try_into()?),
        })
    }
}

impl From<PackageInterfaceDeclarationPayload26> for PackageInterfaceDeclarationPayload {
    fn from(v: PackageInterfaceDeclarationPayload26) -> Self {
        match v {
            PackageInterfaceDeclarationPayload26::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            PackageInterfaceDeclarationPayload26::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            PackageInterfaceDeclarationPayload26::Interface { operations } => {
                Self::Interface { operations }
            }
            PackageInterfaceDeclarationPayload26::External(v) => Self::External(v),
            PackageInterfaceDeclarationPayload26::Function(v) => Self::Function(v.into()),
            PackageInterfaceDeclarationPayload26::OwnedContract(v) => Self::OwnedContract(v.into()),
            PackageInterfaceDeclarationPayload26::OwnedImplementation(v) => {
                Self::OwnedImplementation(v.into())
            }
            PackageInterfaceDeclarationPayload26::Constant { ty } => Self::Constant { ty },
            PackageInterfaceDeclarationPayload26::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
        }
    }
}

impl TryFrom<PackageInterfaceDeclarationPayload> for PackageInterfaceDeclarationPayload26 {
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
            PackageInterfaceDeclarationPayload::External(v) => Self::External(v),
            PackageInterfaceDeclarationPayload::Function(v) => Self::Function(v.try_into()?),
            PackageInterfaceDeclarationPayload::OwnedContract(v) => {
                Self::OwnedContract(v.try_into()?)
            }
            PackageInterfaceDeclarationPayload::OwnedImplementation(v) => {
                Self::OwnedImplementation(v.try_into()?)
            }
            PackageInterfaceDeclarationPayload::Constant { ty } => Self::Constant { ty },
            PackageInterfaceDeclarationPayload::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
        })
    }
}

pub(crate) fn borrow_result_extension() -> crate::platform::diagnostic::Diagnostic {
    crate::platform::diagnostic::Diagnostic::new(
        crate::platform::diagnostic::DiagnosticClass::Semantic,
        "kernel_borrow_result_generation",
        "source-tied borrowed results require Graph 27",
    )
}

fn extension() -> crate::platform::diagnostic::Diagnostic {
    borrow_result_extension()
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct OwnedMethod26 {
    pub id: MethodId,
    pub name: Name,
    pub parameters: Vec<OwnedMethodParameter>,
    pub result: TypeObjectDigest,
    pub effect: FunctionEffect,
}

impl From<OwnedMethod26> for OwnedMethod {
    fn from(v: OwnedMethod26) -> Self {
        Self {
            id: v.id,
            name: v.name,
            parameters: v.parameters,
            result: v.result,
            result_borrow: None,
            effect: v.effect,
        }
    }
}
impl TryFrom<OwnedMethod> for OwnedMethod26 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: OwnedMethod) -> Result<Self, Self::Error> {
        if v.result_borrow.is_some() {
            return Err(extension());
        }
        Ok(Self {
            id: v.id,
            name: v.name,
            parameters: v.parameters,
            result: v.result,
            effect: v.effect,
        })
    }
}
