//! Frozen Graph 18–25 declaration layouts and Graph 23–25 complete owner layout.
//! Added contract parameters and application arguments must never reinterpret old bytes.

use super::*;
use crate::platform::semantic_id::*;
use bincode::{Decode, Encode};

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum OwnerRecord25 {
    Module(ModuleRecord),
    Declaration(DeclarationRecord25),
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
pub struct DeclarationRecord25 {
    pub header: OwnerHeader,
    pub module: ModuleId,
    pub name: Name,
    pub visibility: DeclarationVisibility,
    pub payload: DeclarationPayload25,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum DeclarationPayload25 {
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
    Function(FunctionDeclaration25),
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
    OwnedContract(OwnedContract25),
    OwnedImplementation(OwnedImplementation25),
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct FunctionDeclaration25 {
    pub implementation_parameters: Vec<ImplementationParameter25>,
    pub requirement_parameters: Vec<crate::platform::semantic_id::RequirementParameterId>,
    pub effect_parameters: Vec<EffectParameterId>,
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    pub effect: FunctionEffect,
    pub body: ExpressionId,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum PackageInterfaceRecord25 {
    Declaration(PackageInterfaceDeclaration25),
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
pub struct PackageInterfaceDeclaration25 {
    pub header: OwnerHeader,
    pub name: Name,
    pub payload: PackageInterfaceDeclarationPayload25,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum PackageInterfaceDeclarationPayload25 {
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
    Function(PackageFunctionSignature25),
    OwnedContract(OwnedContract25),
    OwnedImplementation(OwnedImplementation25),
    Constant {
        ty: TypeObjectDigest,
    },
    Component {
        requirements: Vec<RequirementId>,
        ports: Vec<PortId>,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageFunctionSignature25 {
    pub implementation_parameters: Vec<ImplementationParameter25>,
    pub requirement_parameters: Vec<crate::platform::semantic_id::RequirementParameterId>,
    pub effect_parameters: Vec<EffectParameterId>,
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    pub effect: FunctionEffect,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct OwnedContract25 {
    pub self_parameter: TypeParameterId,
    pub methods: Vec<super::wire26::OwnedMethod26>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct OwnedImplementation25 {
    pub contract: DeclarationReference,
    pub self_type: TypeObjectDigest,
    pub methods: Vec<super::wire27::OwnedMethodImplementation27>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct ImplementationParameter25 {
    pub id: ImplementationParameterId,
    pub name: Name,
    pub contract: DeclarationReference,
    pub self_type: TypeObjectDigest,
}

impl From<DeclarationRecord25> for DeclarationRecord {
    fn from(v: DeclarationRecord25) -> Self {
        Self {
            header: v.header,
            module: v.module,
            name: v.name,
            visibility: v.visibility,
            payload: v.payload.into(),
        }
    }
}

impl TryFrom<DeclarationRecord> for DeclarationRecord25 {
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

impl From<FunctionDeclaration25> for FunctionDeclaration {
    fn from(v: FunctionDeclaration25) -> Self {
        Self {
            implementation_parameters: v
                .implementation_parameters
                .into_iter()
                .map(Into::into)
                .collect(),
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

impl TryFrom<FunctionDeclaration> for FunctionDeclaration25 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: FunctionDeclaration) -> Result<Self, Self::Error> {
        if v.result_borrow.is_some() {
            return Err(super::wire26::borrow_result_extension());
        }
        Ok(Self {
            implementation_parameters: v
                .implementation_parameters
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
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

impl From<PackageInterfaceDeclaration25> for PackageInterfaceDeclaration {
    fn from(v: PackageInterfaceDeclaration25) -> Self {
        Self {
            header: v.header,
            name: v.name,
            payload: v.payload.into(),
        }
    }
}

impl TryFrom<PackageInterfaceDeclaration> for PackageInterfaceDeclaration25 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: PackageInterfaceDeclaration) -> Result<Self, Self::Error> {
        Ok(Self {
            header: v.header,
            name: v.name,
            payload: v.payload.try_into()?,
        })
    }
}

impl From<PackageFunctionSignature25> for PackageFunctionSignature {
    fn from(v: PackageFunctionSignature25) -> Self {
        Self {
            implementation_parameters: v
                .implementation_parameters
                .into_iter()
                .map(Into::into)
                .collect(),
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

impl TryFrom<PackageFunctionSignature> for PackageFunctionSignature25 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: PackageFunctionSignature) -> Result<Self, Self::Error> {
        if v.result_borrow.is_some() {
            return Err(super::wire26::borrow_result_extension());
        }
        Ok(Self {
            implementation_parameters: v
                .implementation_parameters
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            requirement_parameters: v.requirement_parameters,
            effect_parameters: v.effect_parameters,
            type_parameters: v.type_parameters,
            parameters: v.parameters,
            result: v.result,
            effect: v.effect,
        })
    }
}

impl From<OwnedContract25> for OwnedContract {
    fn from(v: OwnedContract25) -> Self {
        Self {
            self_parameter: v.self_parameter,
            methods: v.methods.into_iter().map(Into::into).collect(),
            type_parameters: Vec::new(),
        }
    }
}

impl TryFrom<OwnedContract> for OwnedContract25 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: OwnedContract) -> Result<Self, Self::Error> {
        if !v.type_parameters.is_empty() {
            return Err(extension());
        }
        Ok(Self {
            self_parameter: v.self_parameter,
            methods: v
                .methods
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<OwnedImplementation25> for OwnedImplementation {
    fn from(v: OwnedImplementation25) -> Self {
        Self {
            contract: v.contract,
            self_type: v.self_type,
            methods: v.methods.into_iter().map(Into::into).collect(),
            type_parameters: Vec::new(),
            implementation_parameters: Vec::new(),
            type_arguments: Vec::new(),
        }
    }
}

impl TryFrom<OwnedImplementation> for OwnedImplementation25 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: OwnedImplementation) -> Result<Self, Self::Error> {
        if !v.implementation_parameters.is_empty()
            || v.methods.iter().any(|m| !m.implementations.is_empty())
        {
            return Err(super::wire28::implementation_scheme_extension());
        }
        if !v.type_parameters.is_empty() || v.methods.iter().any(|m| !m.type_arguments.is_empty()) {
            return Err(super::wire27::implementation_scheme_extension());
        }
        if !v.type_arguments.is_empty() {
            return Err(extension());
        }
        Ok(Self {
            contract: v.contract,
            self_type: v.self_type,
            methods: v
                .methods
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<ImplementationParameter25> for ImplementationParameter {
    fn from(v: ImplementationParameter25) -> Self {
        Self {
            id: v.id,
            name: v.name,
            contract: v.contract,
            self_type: v.self_type,
            type_arguments: Vec::new(),
        }
    }
}

impl TryFrom<ImplementationParameter> for ImplementationParameter25 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: ImplementationParameter) -> Result<Self, Self::Error> {
        if !v.type_arguments.is_empty() {
            return Err(extension());
        }
        Ok(Self {
            id: v.id,
            name: v.name,
            contract: v.contract,
            self_type: v.self_type,
        })
    }
}

impl From<OwnerRecord25> for OwnerRecord {
    fn from(v: OwnerRecord25) -> Self {
        match v {
            OwnerRecord25::Module(v) => Self::Module(v),
            OwnerRecord25::Declaration(v) => Self::Declaration(v.into()),
            OwnerRecord25::TypeParameter(v) => Self::TypeParameter(v),
            OwnerRecord25::EffectParameter(v) => Self::EffectParameter(v),
            OwnerRecord25::Field(v) => Self::Field(v),
            OwnerRecord25::Case(v) => Self::Case(v),
            OwnerRecord25::Operation(v) => Self::Operation(v),
            OwnerRecord25::Parameter(v) => Self::Parameter(v),
            OwnerRecord25::Binding(v) => Self::Binding(v),
            OwnerRecord25::Expression(v) => Self::Expression(v.into()),
            OwnerRecord25::Requirement(v) => Self::Requirement(v),
            OwnerRecord25::Port(v) => Self::Port(v),
            OwnerRecord25::Target(v) => Self::Target(v),
            OwnerRecord25::Documentation(v) => Self::Documentation(v),
            OwnerRecord25::Annotation(v) => Self::Annotation(v),
            OwnerRecord25::HttpRoute(v) => Self::HttpRoute(v),
            OwnerRecord25::RequirementParameter(v) => Self::RequirementParameter(v),
        }
    }
}

impl TryFrom<OwnerRecord> for OwnerRecord25 {
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

impl From<PackageInterfaceRecord25> for PackageInterfaceRecord {
    fn from(v: PackageInterfaceRecord25) -> Self {
        match v {
            PackageInterfaceRecord25::Declaration(v) => Self::Declaration(v.into()),
            PackageInterfaceRecord25::TypeParameter(v) => Self::TypeParameter(v),
            PackageInterfaceRecord25::EffectParameter(v) => Self::EffectParameter(v),
            PackageInterfaceRecord25::Field(v) => Self::Field(v),
            PackageInterfaceRecord25::Case(v) => Self::Case(v),
            PackageInterfaceRecord25::Operation(v) => Self::Operation(v),
            PackageInterfaceRecord25::Parameter(v) => Self::Parameter(v),
            PackageInterfaceRecord25::Requirement(v) => Self::Requirement(v),
            PackageInterfaceRecord25::Port(v) => Self::Port(v),
            PackageInterfaceRecord25::RequirementParameter(v) => Self::RequirementParameter(v),
        }
    }
}

impl TryFrom<PackageInterfaceRecord> for PackageInterfaceRecord25 {
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

impl From<DeclarationPayload25> for DeclarationPayload {
    fn from(v: DeclarationPayload25) -> Self {
        match v {
            DeclarationPayload25::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            DeclarationPayload25::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            DeclarationPayload25::Interface { operations } => Self::Interface { operations },
            DeclarationPayload25::External(v) => Self::External(v),
            DeclarationPayload25::Function(v) => Self::Function(v.into()),
            DeclarationPayload25::Constant { ty, value } => Self::Constant { ty, value },
            DeclarationPayload25::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
            DeclarationPayload25::Test {
                actual,
                expected,
                comparison,
            } => Self::Test {
                actual,
                expected,
                comparison,
            },
            DeclarationPayload25::OwnedContract(v) => Self::OwnedContract(v.into()),
            DeclarationPayload25::OwnedImplementation(v) => Self::OwnedImplementation(v.into()),
        }
    }
}

impl TryFrom<DeclarationPayload> for DeclarationPayload25 {
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

impl From<PackageInterfaceDeclarationPayload25> for PackageInterfaceDeclarationPayload {
    fn from(v: PackageInterfaceDeclarationPayload25) -> Self {
        match v {
            PackageInterfaceDeclarationPayload25::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            PackageInterfaceDeclarationPayload25::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            PackageInterfaceDeclarationPayload25::Interface { operations } => {
                Self::Interface { operations }
            }
            PackageInterfaceDeclarationPayload25::External(v) => Self::External(v),
            PackageInterfaceDeclarationPayload25::Function(v) => Self::Function(v.into()),
            PackageInterfaceDeclarationPayload25::OwnedContract(v) => Self::OwnedContract(v.into()),
            PackageInterfaceDeclarationPayload25::OwnedImplementation(v) => {
                Self::OwnedImplementation(v.into())
            }
            PackageInterfaceDeclarationPayload25::Constant { ty } => Self::Constant { ty },
            PackageInterfaceDeclarationPayload25::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
        }
    }
}

impl TryFrom<PackageInterfaceDeclarationPayload> for PackageInterfaceDeclarationPayload25 {
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

fn extension() -> crate::platform::diagnostic::Diagnostic {
    crate::platform::diagnostic::Diagnostic::new(
        crate::platform::diagnostic::DiagnosticClass::Semantic,
        "kernel_parameterized_contract_generation",
        "owned contract parameters and arguments require Graph 26",
    )
}
