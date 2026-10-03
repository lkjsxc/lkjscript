//! Frozen Graph 15–17 owner layout; only current admission grants permission.
use super::*;
use crate::platform::semantic_id::*;
use bincode::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub enum OwnerRecord17 {
    Module(ModuleRecord),
    Declaration(DeclarationRecord17),
    TypeParameter(TypeParameterRecord),
    EffectParameter(EffectParameterRecord),
    Field(FieldRecord),
    Case(CaseRecord),
    Operation(OperationRecord),
    Parameter(ParameterRecord),
    Binding(BindingRecord),
    Expression(super::wire22::ExpressionRecord22),
    Requirement(RequirementRecord),
    Port(PortRecord),
    Target(TargetRecord),
    Documentation(DocumentationRecord),
    Annotation(AnnotationRecord),
    HttpRoute(HttpRouteRecord),
    RequirementParameter(RequirementParameterRecord),
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct DeclarationRecord17 {
    pub header: OwnerHeader,
    pub module: ModuleId,
    pub name: Name,
    pub visibility: DeclarationVisibility,
    pub payload: DeclarationPayload17,
}

#[derive(Clone, Debug, Encode, Decode)]
pub enum DeclarationPayload17 {
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
    Function(FunctionDeclaration17),
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

#[derive(Clone, Debug, Encode, Decode)]
pub struct FunctionDeclaration17 {
    pub requirement_parameters: Vec<crate::platform::semantic_id::RequirementParameterId>,
    pub effect_parameters: Vec<EffectParameterId>,
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    pub effect: FunctionEffect,
    pub body: ExpressionId,
}

impl From<OwnerRecord17> for OwnerRecord {
    fn from(value: OwnerRecord17) -> Self {
        match value {
            OwnerRecord17::Module(v) => Self::Module(v),
            OwnerRecord17::Declaration(v) => Self::Declaration(v.into()),
            OwnerRecord17::TypeParameter(v) => Self::TypeParameter(v),
            OwnerRecord17::EffectParameter(v) => Self::EffectParameter(v),
            OwnerRecord17::Field(v) => Self::Field(v),
            OwnerRecord17::Case(v) => Self::Case(v),
            OwnerRecord17::Operation(v) => Self::Operation(v),
            OwnerRecord17::Parameter(v) => Self::Parameter(v),
            OwnerRecord17::Binding(v) => Self::Binding(v),
            OwnerRecord17::Expression(v) => Self::Expression(v.into()),
            OwnerRecord17::Requirement(v) => Self::Requirement(v),
            OwnerRecord17::Port(v) => Self::Port(v),
            OwnerRecord17::Target(v) => Self::Target(v),
            OwnerRecord17::Documentation(v) => Self::Documentation(v),
            OwnerRecord17::Annotation(v) => Self::Annotation(v),
            OwnerRecord17::HttpRoute(v) => Self::HttpRoute(v),
            OwnerRecord17::RequirementParameter(v) => Self::RequirementParameter(v),
        }
    }
}
impl TryFrom<OwnerRecord> for OwnerRecord17 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(value: OwnerRecord) -> Result<Self, Self::Error> {
        Ok(match value {
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
impl From<DeclarationRecord17> for DeclarationRecord {
    fn from(v: DeclarationRecord17) -> Self {
        Self {
            header: v.header,
            module: v.module,
            name: v.name,
            visibility: v.visibility,
            payload: v.payload.into(),
        }
    }
}
impl TryFrom<DeclarationRecord> for DeclarationRecord17 {
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
impl From<DeclarationPayload17> for DeclarationPayload {
    fn from(v: DeclarationPayload17) -> Self {
        match v {
            DeclarationPayload17::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            DeclarationPayload17::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            DeclarationPayload17::Interface { operations } => Self::Interface { operations },
            DeclarationPayload17::External(v) => Self::External(v),
            DeclarationPayload17::Function(v) => Self::Function(v.into()),
            DeclarationPayload17::Constant { ty, value } => Self::Constant { ty, value },
            DeclarationPayload17::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
            DeclarationPayload17::Test {
                actual,
                expected,
                comparison,
            } => Self::Test {
                actual,
                expected,
                comparison,
            },
        }
    }
}
impl TryFrom<DeclarationPayload> for DeclarationPayload17 {
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
            DeclarationPayload::OwnedContract(_) | DeclarationPayload::OwnedImplementation(_) => {
                return Err(unsupported());
            }
        })
    }
}
impl From<FunctionDeclaration17> for FunctionDeclaration {
    fn from(v: FunctionDeclaration17) -> Self {
        Self {
            implementation_parameters: Vec::new(),
            requirement_parameters: v.requirement_parameters,
            effect_parameters: v.effect_parameters,
            type_parameters: v.type_parameters,
            parameters: v.parameters,
            result: v.result,
            effect: v.effect,
            body: v.body,
        }
    }
}
impl TryFrom<FunctionDeclaration> for FunctionDeclaration17 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: FunctionDeclaration) -> Result<Self, Self::Error> {
        if !v.implementation_parameters.is_empty() {
            return Err(unsupported());
        }
        Ok(Self {
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
fn unsupported() -> crate::platform::diagnostic::Diagnostic {
    crate::platform::diagnostic::Diagnostic::new(
        crate::platform::diagnostic::DiagnosticClass::Semantic,
        "kernel_owned_generation",
        "owned contracts and witnesses require Graph 18",
    )
}
