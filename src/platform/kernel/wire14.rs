//! Exact predecessor wire layouts, frozen from 4306ef64 (Graph 14 / interface 10).
//! Conversion does not validate meaning or change stored bytes; the current checker owns admission.

use super::*;
use crate::platform::semantic_id::{
    BindingId, CaseId, DeclarationId, EffectParameterId, ExpressionId, FieldId, ModuleId,
    OperationId, ParameterId, PortId, RequirementId, TypeParameterId,
};
use bincode::{Decode, Encode};

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum OwnerRecord14 {
    Module(ModuleRecord),
    Declaration(DeclarationRecord14),
    TypeParameter(TypeParameterRecord),
    EffectParameter(EffectParameterRecord),
    Field(FieldRecord),
    Case(CaseRecord),
    Operation(OperationRecord),
    Parameter(ParameterRecord),
    Binding(BindingRecord),
    Expression(ExpressionRecord14),
    Requirement(RequirementRecord),
    Port(PortRecord),
    Target(TargetRecord),
    Documentation(DocumentationRecord),
    Annotation(AnnotationRecord),
    HttpRoute(HttpRouteRecord),
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct DeclarationRecord14 {
    pub header: OwnerHeader,
    pub module: ModuleId,
    pub name: Name,
    pub visibility: DeclarationVisibility,
    pub payload: DeclarationPayload14,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum DeclarationPayload14 {
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
    Function(FunctionDeclaration14),
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

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct FunctionDeclaration14 {
    pub effect_parameters: Vec<EffectParameterId>,
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    pub effect: FunctionEffect14,
    pub body: ExpressionId,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum FunctionEffect14 {
    Pure,
    Task {
        requirements: Vec<RequirementReference>,
        effect_parameters: Vec<super::EffectParameterReference>,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct ExpressionRecord14 {
    pub contract_version: u16,
    pub id: ExpressionId,
    pub operation: ExpressionOperation14,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum ExpressionOperation14 {
    Unit {},
    Bool {
        value: bool,
    },
    I64 {
        value: i64,
    },
    Text {
        value: TextValue,
    },
    StaticText {
        value: TextValue,
    },
    Local {
        value: LocalValueReference,
    },
    Constant {
        declaration: DeclarationReference,
    },
    If {
        condition: ExpressionId,
        when_true: ExpressionId,
        when_false: ExpressionId,
    },
    Let {
        bindings: Vec<BindingId>,
        body: ExpressionId,
    },
    Sequence {
        items: Vec<ExpressionId>,
    },
    Call {
        effect_arguments: Vec<EffectRow14>,
        function: DeclarationReference,
        type_arguments: Vec<TypeObjectDigest>,
        arguments: Vec<ExpressionId>,
    },
    FunctionValue {
        effect_arguments: Vec<EffectRow14>,
        function: DeclarationReference,
        type_arguments: Vec<TypeObjectDigest>,
    },
    Invoke {
        callee: ExpressionId,
        arguments: Vec<ExpressionId>,
    },
    Record {
        nominal_type: Option<DeclarationReference>,
        type_arguments: Vec<TypeObjectDigest>,
        fields: Vec<RecordExpressionField>,
    },
    Variant {
        case: CaseReference,
        type_arguments: Vec<TypeObjectDigest>,
        payload: Option<ExpressionId>,
    },
    Field {
        value: ExpressionId,
        selector: FieldSelector,
    },
    List {
        item_type: TypeObjectDigest,
        items: Vec<ExpressionId>,
    },
    Map {
        key_type: TypeObjectDigest,
        value_type: TypeObjectDigest,
        entries: Vec<MapExpressionEntry>,
    },
    Match {
        value: ExpressionId,
        arms: Vec<MatchExpressionArm>,
    },
    CapabilityCall {
        requirement: RequirementReference,
        operation: OperationReference,
        arguments: Vec<ExpressionId>,
    },
    Transaction {
        requirement: RequirementReference,
        binding: BindingId,
        body: ExpressionId,
    },
    Bind {
        callee: ExpressionId,
        arguments: Vec<ExpressionId>,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct EffectRow14 {
    pub requirements: Vec<RequirementReference>,
    pub parameters: Vec<EffectParameterReference>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum PackageInterfaceRecord14 {
    Declaration(PackageInterfaceDeclaration14),
    TypeParameter(TypeParameterRecord),
    EffectParameter(EffectParameterRecord),
    Field(FieldRecord),
    Case(CaseRecord),
    Operation(OperationRecord),
    Parameter(ParameterRecord),
    Requirement(RequirementRecord),
    Port(PackageInterfacePort),
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageInterfaceDeclaration14 {
    pub header: OwnerHeader,
    pub name: Name,
    pub payload: PackageInterfaceDeclarationPayload14,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum PackageInterfaceDeclarationPayload14 {
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
    Function(PackageFunctionSignature14),
    Constant {
        ty: TypeObjectDigest,
    },
    Component {
        requirements: Vec<RequirementId>,
        ports: Vec<PortId>,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageFunctionSignature14 {
    pub effect_parameters: Vec<EffectParameterId>,
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    pub effect: FunctionEffect14,
}

impl From<OwnerRecord14> for super::OwnerRecord {
    fn from(value: OwnerRecord14) -> Self {
        match value {
            OwnerRecord14::Module(value) => Self::Module(value),
            OwnerRecord14::Declaration(value) => Self::Declaration(value.into()),
            OwnerRecord14::TypeParameter(value) => Self::TypeParameter(value),
            OwnerRecord14::EffectParameter(value) => Self::EffectParameter(value),
            OwnerRecord14::Field(value) => Self::Field(value),
            OwnerRecord14::Case(value) => Self::Case(value),
            OwnerRecord14::Operation(value) => Self::Operation(value),
            OwnerRecord14::Parameter(value) => Self::Parameter(value),
            OwnerRecord14::Binding(value) => Self::Binding(value),
            OwnerRecord14::Expression(value) => Self::Expression(value.into()),
            OwnerRecord14::Requirement(value) => Self::Requirement(value),
            OwnerRecord14::Port(value) => Self::Port(value),
            OwnerRecord14::Target(value) => Self::Target(value),
            OwnerRecord14::Documentation(value) => Self::Documentation(value),
            OwnerRecord14::Annotation(value) => Self::Annotation(value),
            OwnerRecord14::HttpRoute(value) => Self::HttpRoute(value),
        }
    }
}

impl TryFrom<super::OwnerRecord> for OwnerRecord14 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(value: super::OwnerRecord) -> Result<Self, Self::Error> {
        Ok(match value {
            super::OwnerRecord::Module(value) => Self::Module(value),
            super::OwnerRecord::Declaration(value) => Self::Declaration(value.try_into()?),
            super::OwnerRecord::TypeParameter(value) => Self::TypeParameter(value),
            super::OwnerRecord::EffectParameter(value) => Self::EffectParameter(value),
            super::OwnerRecord::Field(value) => Self::Field(value),
            super::OwnerRecord::Case(value) => Self::Case(value),
            super::OwnerRecord::Operation(value) => Self::Operation(value),
            super::OwnerRecord::Parameter(value) => Self::Parameter(value),
            super::OwnerRecord::Binding(value) => Self::Binding(value),
            super::OwnerRecord::Expression(value) => Self::Expression(value.try_into()?),
            super::OwnerRecord::Requirement(value) => Self::Requirement(value),
            super::OwnerRecord::Port(value) => Self::Port(value),
            super::OwnerRecord::Target(value) => Self::Target(value),
            super::OwnerRecord::Documentation(value) => Self::Documentation(value),
            super::OwnerRecord::Annotation(value) => Self::Annotation(value),
            super::OwnerRecord::HttpRoute(value) => Self::HttpRoute(value),
            super::OwnerRecord::RequirementParameter(_) => return Err(extension()),
        })
    }
}

impl From<DeclarationRecord14> for super::DeclarationRecord {
    fn from(value: DeclarationRecord14) -> Self {
        let DeclarationRecord14 {
            header,
            module,
            name,
            visibility,
            payload,
        } = value;
        Self {
            header,
            module,
            name,
            visibility,
            payload: payload.into(),
        }
    }
}

impl TryFrom<super::DeclarationRecord> for DeclarationRecord14 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(value: super::DeclarationRecord) -> Result<Self, Self::Error> {
        let super::DeclarationRecord {
            header,
            module,
            name,
            visibility,
            payload,
        } = value;
        Ok(Self {
            header,
            module,
            name,
            visibility,
            payload: payload.try_into()?,
        })
    }
}

impl From<DeclarationPayload14> for super::DeclarationPayload {
    fn from(value: DeclarationPayload14) -> Self {
        match value {
            DeclarationPayload14::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            DeclarationPayload14::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            DeclarationPayload14::Interface { operations } => Self::Interface { operations },
            DeclarationPayload14::External(value) => Self::External(value),
            DeclarationPayload14::Function(value) => Self::Function(value.into()),
            DeclarationPayload14::Constant { ty, value } => Self::Constant { ty, value },
            DeclarationPayload14::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
            DeclarationPayload14::Test {
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

impl TryFrom<super::DeclarationPayload> for DeclarationPayload14 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(value: super::DeclarationPayload) -> Result<Self, Self::Error> {
        Ok(match value {
            super::DeclarationPayload::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            super::DeclarationPayload::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            super::DeclarationPayload::Interface { operations } => Self::Interface { operations },
            super::DeclarationPayload::External(value) => Self::External(value),
            super::DeclarationPayload::Function(value) => Self::Function(value.try_into()?),
            super::DeclarationPayload::Constant { ty, value } => Self::Constant { ty, value },
            super::DeclarationPayload::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
            super::DeclarationPayload::Test {
                actual,
                expected,
                comparison,
            } => Self::Test {
                actual,
                expected,
                comparison,
            },
        })
    }
}

impl From<FunctionDeclaration14> for super::FunctionDeclaration {
    fn from(value: FunctionDeclaration14) -> Self {
        let FunctionDeclaration14 {
            effect_parameters,
            type_parameters,
            parameters,
            result,
            effect,
            body,
        } = value;
        Self {
            requirement_parameters: Vec::new(),
            effect_parameters,
            type_parameters,
            parameters,
            result,
            effect: effect.into(),
            body,
        }
    }
}

impl TryFrom<super::FunctionDeclaration> for FunctionDeclaration14 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(value: super::FunctionDeclaration) -> Result<Self, Self::Error> {
        let super::FunctionDeclaration {
            effect_parameters,
            type_parameters,
            parameters,
            result,
            effect,
            body,
            requirement_parameters,
        } = value;
        require_empty(&requirement_parameters)?;
        Ok(Self {
            effect_parameters,
            type_parameters,
            parameters,
            result,
            effect: effect.try_into()?,
            body,
        })
    }
}

impl From<FunctionEffect14> for super::FunctionEffect {
    fn from(value: FunctionEffect14) -> Self {
        match value {
            FunctionEffect14::Pure => Self::Pure,
            FunctionEffect14::Task {
                requirements,
                effect_parameters,
            } => Self::Task {
                requirements: requirements.into_iter().map(Into::into).collect(),
                effect_parameters,
            },
        }
    }
}

impl TryFrom<super::FunctionEffect> for FunctionEffect14 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(value: super::FunctionEffect) -> Result<Self, Self::Error> {
        Ok(match value {
            super::FunctionEffect::Pure => Self::Pure,
            super::FunctionEffect::Task {
                requirements,
                effect_parameters,
            } => Self::Task {
                requirements: requirements
                    .into_iter()
                    .map(concrete)
                    .collect::<Result<_, _>>()?,
                effect_parameters,
            },
        })
    }
}

impl From<ExpressionRecord14> for super::ExpressionRecord {
    fn from(value: ExpressionRecord14) -> Self {
        let ExpressionRecord14 {
            contract_version,
            id,
            operation,
        } = value;
        Self {
            contract_version,
            id,
            operation: operation.into(),
        }
    }
}

impl TryFrom<super::ExpressionRecord> for ExpressionRecord14 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(value: super::ExpressionRecord) -> Result<Self, Self::Error> {
        let super::ExpressionRecord {
            contract_version,
            id,
            operation,
        } = value;
        Ok(Self {
            contract_version,
            id,
            operation: operation.try_into()?,
        })
    }
}

impl From<ExpressionOperation14> for super::ExpressionOperation {
    fn from(value: ExpressionOperation14) -> Self {
        match value {
            ExpressionOperation14::Unit {} => Self::Unit {},
            ExpressionOperation14::Bool { value } => Self::Bool { value },
            ExpressionOperation14::I64 { value } => Self::I64 { value },
            ExpressionOperation14::Text { value } => Self::Text { value },
            ExpressionOperation14::StaticText { value } => Self::StaticText { value },
            ExpressionOperation14::Local { value } => Self::Local { value },
            ExpressionOperation14::Constant { declaration } => Self::Constant { declaration },
            ExpressionOperation14::If {
                condition,
                when_true,
                when_false,
            } => Self::If {
                condition,
                when_true,
                when_false,
            },
            ExpressionOperation14::Let { bindings, body } => Self::Let { bindings, body },
            ExpressionOperation14::Sequence { items } => Self::Sequence { items },
            ExpressionOperation14::Call {
                effect_arguments,
                function,
                type_arguments,
                arguments,
            } => Self::Call {
                requirement_arguments: Vec::new(),
                effect_arguments: effect_arguments.into_iter().map(Into::into).collect(),
                function,
                type_arguments,
                arguments,
            },
            ExpressionOperation14::FunctionValue {
                effect_arguments,
                function,
                type_arguments,
            } => Self::FunctionValue {
                requirement_arguments: Vec::new(),
                effect_arguments: effect_arguments.into_iter().map(Into::into).collect(),
                function,
                type_arguments,
            },
            ExpressionOperation14::Invoke { callee, arguments } => {
                Self::Invoke { callee, arguments }
            }
            ExpressionOperation14::Record {
                nominal_type,
                type_arguments,
                fields,
            } => Self::Record {
                nominal_type,
                type_arguments,
                fields,
            },
            ExpressionOperation14::Variant {
                case,
                type_arguments,
                payload,
            } => Self::Variant {
                case,
                type_arguments,
                payload,
            },
            ExpressionOperation14::Field { value, selector } => Self::Field { value, selector },
            ExpressionOperation14::List { item_type, items } => Self::List { item_type, items },
            ExpressionOperation14::Map {
                key_type,
                value_type,
                entries,
            } => Self::Map {
                key_type,
                value_type,
                entries,
            },
            ExpressionOperation14::Match { value, arms } => Self::Match { value, arms },
            ExpressionOperation14::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => Self::CapabilityCall {
                requirement: requirement.into(),
                operation,
                arguments,
            },
            ExpressionOperation14::Transaction {
                requirement,
                binding,
                body,
            } => Self::Transaction {
                requirement: requirement.into(),
                binding,
                body,
            },
            ExpressionOperation14::Bind { callee, arguments } => Self::Bind { callee, arguments },
        }
    }
}

impl TryFrom<super::ExpressionOperation> for ExpressionOperation14 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(value: super::ExpressionOperation) -> Result<Self, Self::Error> {
        Ok(match value {
            super::ExpressionOperation::Unit {} => Self::Unit {},
            super::ExpressionOperation::Bool { value } => Self::Bool { value },
            super::ExpressionOperation::I64 { value } => Self::I64 { value },
            super::ExpressionOperation::Text { value } => Self::Text { value },
            super::ExpressionOperation::StaticText { value } => Self::StaticText { value },
            super::ExpressionOperation::Local { value } => Self::Local { value },
            super::ExpressionOperation::Constant { declaration } => Self::Constant { declaration },
            super::ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => Self::If {
                condition,
                when_true,
                when_false,
            },
            super::ExpressionOperation::Let { bindings, body } => Self::Let { bindings, body },
            super::ExpressionOperation::Sequence { items } => Self::Sequence { items },
            super::ExpressionOperation::Call {
                effect_arguments,
                function,
                type_arguments,
                arguments,
                requirement_arguments,
            } => {
                require_empty(&requirement_arguments)?;
                Self::Call {
                    effect_arguments: effect_arguments
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()?,
                    function,
                    type_arguments,
                    arguments,
                }
            }
            super::ExpressionOperation::FunctionValue {
                effect_arguments,
                function,
                type_arguments,
                requirement_arguments,
            } => {
                require_empty(&requirement_arguments)?;
                Self::FunctionValue {
                    effect_arguments: effect_arguments
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()?,
                    function,
                    type_arguments,
                }
            }
            super::ExpressionOperation::Invoke { callee, arguments } => {
                Self::Invoke { callee, arguments }
            }
            super::ExpressionOperation::Record {
                nominal_type,
                type_arguments,
                fields,
            } => Self::Record {
                nominal_type,
                type_arguments,
                fields,
            },
            super::ExpressionOperation::Variant {
                case,
                type_arguments,
                payload,
            } => Self::Variant {
                case,
                type_arguments,
                payload,
            },
            super::ExpressionOperation::Field { value, selector } => {
                Self::Field { value, selector }
            }
            super::ExpressionOperation::List { item_type, items } => {
                Self::List { item_type, items }
            }
            super::ExpressionOperation::Map {
                key_type,
                value_type,
                entries,
            } => Self::Map {
                key_type,
                value_type,
                entries,
            },
            super::ExpressionOperation::Match { value, arms } => Self::Match { value, arms },
            super::ExpressionOperation::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => Self::CapabilityCall {
                requirement: concrete(requirement)?,
                operation,
                arguments,
            },
            super::ExpressionOperation::Transaction {
                requirement,
                binding,
                body,
            } => Self::Transaction {
                requirement: concrete(requirement)?,
                binding,
                body,
            },
            super::ExpressionOperation::Bind { callee, arguments } => {
                Self::Bind { callee, arguments }
            }
            super::ExpressionOperation::TransactionOutcome { .. } => return Err(extension()),
        })
    }
}

impl From<EffectRow14> for super::EffectRow {
    fn from(value: EffectRow14) -> Self {
        let EffectRow14 {
            requirements,
            parameters,
        } = value;
        Self {
            requirements: requirements.into_iter().map(Into::into).collect(),
            parameters,
        }
    }
}

impl TryFrom<super::EffectRow> for EffectRow14 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(value: super::EffectRow) -> Result<Self, Self::Error> {
        let super::EffectRow {
            requirements,
            parameters,
        } = value;
        Ok(Self {
            requirements: requirements
                .into_iter()
                .map(concrete)
                .collect::<Result<_, _>>()?,
            parameters,
        })
    }
}

impl From<PackageInterfaceRecord14> for super::PackageInterfaceRecord {
    fn from(value: PackageInterfaceRecord14) -> Self {
        match value {
            PackageInterfaceRecord14::Declaration(value) => Self::Declaration(value.into()),
            PackageInterfaceRecord14::TypeParameter(value) => Self::TypeParameter(value),
            PackageInterfaceRecord14::EffectParameter(value) => Self::EffectParameter(value),
            PackageInterfaceRecord14::Field(value) => Self::Field(value),
            PackageInterfaceRecord14::Case(value) => Self::Case(value),
            PackageInterfaceRecord14::Operation(value) => Self::Operation(value),
            PackageInterfaceRecord14::Parameter(value) => Self::Parameter(value),
            PackageInterfaceRecord14::Requirement(value) => Self::Requirement(value),
            PackageInterfaceRecord14::Port(value) => Self::Port(value),
        }
    }
}

impl TryFrom<super::PackageInterfaceRecord> for PackageInterfaceRecord14 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(value: super::PackageInterfaceRecord) -> Result<Self, Self::Error> {
        Ok(match value {
            super::PackageInterfaceRecord::Declaration(value) => {
                Self::Declaration(value.try_into()?)
            }
            super::PackageInterfaceRecord::TypeParameter(value) => Self::TypeParameter(value),
            super::PackageInterfaceRecord::EffectParameter(value) => Self::EffectParameter(value),
            super::PackageInterfaceRecord::Field(value) => Self::Field(value),
            super::PackageInterfaceRecord::Case(value) => Self::Case(value),
            super::PackageInterfaceRecord::Operation(value) => Self::Operation(value),
            super::PackageInterfaceRecord::Parameter(value) => Self::Parameter(value),
            super::PackageInterfaceRecord::Requirement(value) => Self::Requirement(value),
            super::PackageInterfaceRecord::Port(value) => Self::Port(value),
            super::PackageInterfaceRecord::RequirementParameter(_) => return Err(extension()),
        })
    }
}

impl From<PackageInterfaceDeclaration14> for super::PackageInterfaceDeclaration {
    fn from(value: PackageInterfaceDeclaration14) -> Self {
        let PackageInterfaceDeclaration14 {
            header,
            name,
            payload,
        } = value;
        Self {
            header,
            name,
            payload: payload.into(),
        }
    }
}

impl TryFrom<super::PackageInterfaceDeclaration> for PackageInterfaceDeclaration14 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(value: super::PackageInterfaceDeclaration) -> Result<Self, Self::Error> {
        let super::PackageInterfaceDeclaration {
            header,
            name,
            payload,
        } = value;
        Ok(Self {
            header,
            name,
            payload: payload.try_into()?,
        })
    }
}

impl From<PackageInterfaceDeclarationPayload14> for super::PackageInterfaceDeclarationPayload {
    fn from(value: PackageInterfaceDeclarationPayload14) -> Self {
        match value {
            PackageInterfaceDeclarationPayload14::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            PackageInterfaceDeclarationPayload14::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            PackageInterfaceDeclarationPayload14::Interface { operations } => {
                Self::Interface { operations }
            }
            PackageInterfaceDeclarationPayload14::External(value) => Self::External(value),
            PackageInterfaceDeclarationPayload14::Function(value) => Self::Function(value.into()),
            PackageInterfaceDeclarationPayload14::Constant { ty } => Self::Constant { ty },
            PackageInterfaceDeclarationPayload14::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
        }
    }
}

impl TryFrom<super::PackageInterfaceDeclarationPayload> for PackageInterfaceDeclarationPayload14 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(value: super::PackageInterfaceDeclarationPayload) -> Result<Self, Self::Error> {
        Ok(match value {
            super::PackageInterfaceDeclarationPayload::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            super::PackageInterfaceDeclarationPayload::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            super::PackageInterfaceDeclarationPayload::Interface { operations } => {
                Self::Interface { operations }
            }
            super::PackageInterfaceDeclarationPayload::External(value) => Self::External(value),
            super::PackageInterfaceDeclarationPayload::Function(value) => {
                Self::Function(value.try_into()?)
            }
            super::PackageInterfaceDeclarationPayload::Constant { ty } => Self::Constant { ty },
            super::PackageInterfaceDeclarationPayload::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
        })
    }
}

impl From<PackageFunctionSignature14> for super::PackageFunctionSignature {
    fn from(value: PackageFunctionSignature14) -> Self {
        let PackageFunctionSignature14 {
            effect_parameters,
            type_parameters,
            parameters,
            result,
            effect,
        } = value;
        Self {
            requirement_parameters: Vec::new(),
            effect_parameters,
            type_parameters,
            parameters,
            result,
            effect: effect.into(),
        }
    }
}

impl TryFrom<super::PackageFunctionSignature> for PackageFunctionSignature14 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(value: super::PackageFunctionSignature) -> Result<Self, Self::Error> {
        let super::PackageFunctionSignature {
            effect_parameters,
            type_parameters,
            parameters,
            result,
            effect,
            requirement_parameters,
        } = value;
        require_empty(&requirement_parameters)?;
        Ok(Self {
            effect_parameters,
            type_parameters,
            parameters,
            result,
            effect: effect.try_into()?,
        })
    }
}

fn extension() -> crate::platform::diagnostic::Diagnostic {
    crate::platform::diagnostic::Diagnostic::new(
        crate::platform::diagnostic::DiagnosticClass::Corrupt,
        "kernel_predecessor_extension",
        "requirement extension cannot be encoded as predecessor meaning",
    )
}

fn require_empty<T>(values: &[T]) -> Result<(), crate::platform::diagnostic::Diagnostic> {
    if values.is_empty() {
        Ok(())
    } else {
        Err(extension())
    }
}

fn concrete(
    value: RequirementOperand,
) -> Result<RequirementReference, crate::platform::diagnostic::Diagnostic> {
    value.concrete().ok_or_else(extension)
}
