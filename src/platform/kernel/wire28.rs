//! Frozen Graph 28 owner and interface layouts.
//! Prerequisite applications must never reinterpret original bytes.

use super::*;
use crate::platform::semantic_id::*;
use bincode::{Decode, Encode};

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum OwnerRecord28 {
    Module(ModuleRecord),
    Declaration(DeclarationRecord28),
    TypeParameter(TypeParameterRecord),
    EffectParameter(EffectParameterRecord),
    Field(FieldRecord),
    Case(CaseRecord),
    Operation(OperationRecord),
    Parameter(ParameterRecord),
    Binding(BindingRecord),
    Expression(ExpressionRecord28),
    Requirement(RequirementRecord),
    Port(PortRecord),
    Target(TargetRecord),
    Documentation(DocumentationRecord),
    Annotation(AnnotationRecord),
    HttpRoute(HttpRouteRecord),
    RequirementParameter(RequirementParameterRecord),
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct DeclarationRecord28 {
    pub header: OwnerHeader,
    pub module: ModuleId,
    pub name: Name,
    pub visibility: DeclarationVisibility,
    pub payload: DeclarationPayload28,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum DeclarationPayload28 {
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
    Function(FunctionDeclaration28),
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
    OwnedContract(OwnedContract28),
    OwnedImplementation(OwnedImplementation28),
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct FunctionDeclaration28 {
    pub implementation_parameters: Vec<ImplementationParameter>,
    pub requirement_parameters: Vec<crate::platform::semantic_id::RequirementParameterId>,
    pub effect_parameters: Vec<EffectParameterId>,
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    pub result_borrow: Option<ParameterId>,
    pub effect: FunctionEffect,
    pub body: ExpressionId,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum PackageInterfaceRecord28 {
    Declaration(PackageInterfaceDeclaration28),
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
pub struct PackageInterfaceDeclaration28 {
    pub header: OwnerHeader,
    pub name: Name,
    pub payload: PackageInterfaceDeclarationPayload28,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum PackageInterfaceDeclarationPayload28 {
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
    Function(PackageFunctionSignature28),
    OwnedContract(OwnedContract28),
    OwnedImplementation(OwnedImplementation28),
    Constant {
        ty: TypeObjectDigest,
    },
    Component {
        requirements: Vec<RequirementId>,
        ports: Vec<PortId>,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageFunctionSignature28 {
    pub implementation_parameters: Vec<ImplementationParameter>,
    pub requirement_parameters: Vec<crate::platform::semantic_id::RequirementParameterId>,
    pub effect_parameters: Vec<EffectParameterId>,
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    pub result_borrow: Option<ParameterId>,
    pub effect: FunctionEffect,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct OwnedContract28 {
    pub self_parameter: TypeParameterId,
    pub type_parameters: Vec<TypeParameterId>,
    pub methods: Vec<OwnedMethod28>,
}

impl From<DeclarationRecord28> for DeclarationRecord {
    fn from(v: DeclarationRecord28) -> Self {
        Self {
            header: v.header,
            module: v.module,
            name: v.name,
            visibility: v.visibility,
            payload: v.payload.into(),
        }
    }
}

impl TryFrom<DeclarationRecord> for DeclarationRecord28 {
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

impl From<FunctionDeclaration28> for FunctionDeclaration {
    fn from(v: FunctionDeclaration28) -> Self {
        Self {
            implementation_parameters: v.implementation_parameters,
            requirement_parameters: v.requirement_parameters,
            effect_parameters: v.effect_parameters,
            type_parameters: v.type_parameters,
            parameters: v.parameters,
            result: v.result,
            result_borrow: v.result_borrow,
            effect: v.effect,
            body: v.body,
        }
    }
}

impl TryFrom<FunctionDeclaration> for FunctionDeclaration28 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: FunctionDeclaration) -> Result<Self, Self::Error> {
        Ok(Self {
            implementation_parameters: v.implementation_parameters,
            requirement_parameters: v.requirement_parameters,
            effect_parameters: v.effect_parameters,
            type_parameters: v.type_parameters,
            parameters: v.parameters,
            result: v.result,
            result_borrow: v.result_borrow,
            effect: v.effect,
            body: v.body,
        })
    }
}

impl From<PackageInterfaceDeclaration28> for PackageInterfaceDeclaration {
    fn from(v: PackageInterfaceDeclaration28) -> Self {
        Self {
            header: v.header,
            name: v.name,
            payload: v.payload.into(),
        }
    }
}

impl TryFrom<PackageInterfaceDeclaration> for PackageInterfaceDeclaration28 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: PackageInterfaceDeclaration) -> Result<Self, Self::Error> {
        Ok(Self {
            header: v.header,
            name: v.name,
            payload: v.payload.try_into()?,
        })
    }
}

impl From<PackageFunctionSignature28> for PackageFunctionSignature {
    fn from(v: PackageFunctionSignature28) -> Self {
        Self {
            implementation_parameters: v.implementation_parameters,
            requirement_parameters: v.requirement_parameters,
            effect_parameters: v.effect_parameters,
            type_parameters: v.type_parameters,
            parameters: v.parameters,
            result: v.result,
            result_borrow: v.result_borrow,
            effect: v.effect,
        }
    }
}

impl TryFrom<PackageFunctionSignature> for PackageFunctionSignature28 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: PackageFunctionSignature) -> Result<Self, Self::Error> {
        Ok(Self {
            implementation_parameters: v.implementation_parameters,
            requirement_parameters: v.requirement_parameters,
            effect_parameters: v.effect_parameters,
            type_parameters: v.type_parameters,
            parameters: v.parameters,
            result: v.result,
            result_borrow: v.result_borrow,
            effect: v.effect,
        })
    }
}

impl From<OwnedContract28> for OwnedContract {
    fn from(v: OwnedContract28) -> Self {
        Self {
            self_parameter: v.self_parameter,
            methods: v.methods.into_iter().map(Into::into).collect(),
            type_parameters: v.type_parameters,
        }
    }
}

impl TryFrom<OwnedContract> for OwnedContract28 {
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

impl From<OwnerRecord28> for OwnerRecord {
    fn from(v: OwnerRecord28) -> Self {
        match v {
            OwnerRecord28::Module(v) => Self::Module(v),
            OwnerRecord28::Declaration(v) => Self::Declaration(v.into()),
            OwnerRecord28::TypeParameter(v) => Self::TypeParameter(v),
            OwnerRecord28::EffectParameter(v) => Self::EffectParameter(v),
            OwnerRecord28::Field(v) => Self::Field(v),
            OwnerRecord28::Case(v) => Self::Case(v),
            OwnerRecord28::Operation(v) => Self::Operation(v),
            OwnerRecord28::Parameter(v) => Self::Parameter(v),
            OwnerRecord28::Binding(v) => Self::Binding(v),
            OwnerRecord28::Expression(v) => Self::Expression(v.into()),
            OwnerRecord28::Requirement(v) => Self::Requirement(v),
            OwnerRecord28::Port(v) => Self::Port(v),
            OwnerRecord28::Target(v) => Self::Target(v),
            OwnerRecord28::Documentation(v) => Self::Documentation(v),
            OwnerRecord28::Annotation(v) => Self::Annotation(v),
            OwnerRecord28::HttpRoute(v) => Self::HttpRoute(v),
            OwnerRecord28::RequirementParameter(v) => Self::RequirementParameter(v),
        }
    }
}

impl TryFrom<OwnerRecord> for OwnerRecord28 {
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

impl From<PackageInterfaceRecord28> for PackageInterfaceRecord {
    fn from(v: PackageInterfaceRecord28) -> Self {
        match v {
            PackageInterfaceRecord28::Declaration(v) => Self::Declaration(v.into()),
            PackageInterfaceRecord28::TypeParameter(v) => Self::TypeParameter(v),
            PackageInterfaceRecord28::EffectParameter(v) => Self::EffectParameter(v),
            PackageInterfaceRecord28::Field(v) => Self::Field(v),
            PackageInterfaceRecord28::Case(v) => Self::Case(v),
            PackageInterfaceRecord28::Operation(v) => Self::Operation(v),
            PackageInterfaceRecord28::Parameter(v) => Self::Parameter(v),
            PackageInterfaceRecord28::Requirement(v) => Self::Requirement(v),
            PackageInterfaceRecord28::Port(v) => Self::Port(v),
            PackageInterfaceRecord28::RequirementParameter(v) => Self::RequirementParameter(v),
        }
    }
}

impl TryFrom<PackageInterfaceRecord> for PackageInterfaceRecord28 {
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

impl From<DeclarationPayload28> for DeclarationPayload {
    fn from(v: DeclarationPayload28) -> Self {
        match v {
            DeclarationPayload28::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            DeclarationPayload28::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            DeclarationPayload28::Interface { operations } => Self::Interface { operations },
            DeclarationPayload28::External(v) => Self::External(v),
            DeclarationPayload28::Function(v) => Self::Function(v.into()),
            DeclarationPayload28::Constant { ty, value } => Self::Constant { ty, value },
            DeclarationPayload28::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
            DeclarationPayload28::Test {
                actual,
                expected,
                comparison,
            } => Self::Test {
                actual,
                expected,
                comparison,
            },
            DeclarationPayload28::OwnedContract(v) => Self::OwnedContract(v.into()),
            DeclarationPayload28::OwnedImplementation(v) => Self::OwnedImplementation(v.into()),
        }
    }
}

impl TryFrom<DeclarationPayload> for DeclarationPayload28 {
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

impl From<PackageInterfaceDeclarationPayload28> for PackageInterfaceDeclarationPayload {
    fn from(v: PackageInterfaceDeclarationPayload28) -> Self {
        match v {
            PackageInterfaceDeclarationPayload28::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            PackageInterfaceDeclarationPayload28::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            PackageInterfaceDeclarationPayload28::Interface { operations } => {
                Self::Interface { operations }
            }
            PackageInterfaceDeclarationPayload28::External(v) => Self::External(v),
            PackageInterfaceDeclarationPayload28::Function(v) => Self::Function(v.into()),
            PackageInterfaceDeclarationPayload28::OwnedContract(v) => Self::OwnedContract(v.into()),
            PackageInterfaceDeclarationPayload28::OwnedImplementation(v) => {
                Self::OwnedImplementation(v.into())
            }
            PackageInterfaceDeclarationPayload28::Constant { ty } => Self::Constant { ty },
            PackageInterfaceDeclarationPayload28::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
        }
    }
}

impl TryFrom<PackageInterfaceDeclarationPayload> for PackageInterfaceDeclarationPayload28 {
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

pub(crate) fn implementation_scheme_extension() -> crate::platform::diagnostic::Diagnostic {
    crate::platform::diagnostic::Diagnostic::new(
        crate::platform::diagnostic::DiagnosticClass::Semantic,
        "kernel_implementation_prerequisite_generation",
        "implementation prerequisites require Graph 29",
    )
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct OwnedMethod28 {
    pub id: MethodId,
    pub name: Name,
    pub parameters: Vec<OwnedMethodParameter>,
    pub result: TypeObjectDigest,
    pub result_borrow: Option<u32>,
    pub effect: FunctionEffect,
}

impl From<OwnedMethod28> for OwnedMethod {
    fn from(v: OwnedMethod28) -> Self {
        Self {
            id: v.id,
            name: v.name,
            parameters: v.parameters,
            result: v.result,
            result_borrow: v.result_borrow,
            effect: v.effect,
        }
    }
}
impl TryFrom<OwnedMethod> for OwnedMethod28 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: OwnedMethod) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v.id,
            name: v.name,
            parameters: v.parameters,
            result: v.result,
            result_borrow: v.result_borrow,
            effect: v.effect,
        })
    }
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct OwnedImplementation28 {
    pub type_parameters: Vec<TypeParameterId>,
    pub contract: DeclarationReference,
    pub self_type: TypeObjectDigest,
    pub type_arguments: Vec<TypeObjectDigest>,
    pub methods: Vec<OwnedMethodImplementation28>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct OwnedMethodImplementation28 {
    pub method: MethodId,
    pub function: DeclarationReference,
    pub type_arguments: Vec<TypeObjectDigest>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum ImplementationOperand28 {
    Concrete {
        implementation: DeclarationReference,
        type_arguments: Vec<TypeObjectDigest>,
    },
    Parameter {
        function: DeclarationReference,
        parameter: ImplementationParameterId,
    },
}

impl From<OwnedImplementation28> for OwnedImplementation {
    fn from(v: OwnedImplementation28) -> Self {
        Self {
            type_parameters: v.type_parameters,
            implementation_parameters: Vec::new(),
            contract: v.contract,
            self_type: v.self_type,
            type_arguments: v.type_arguments,
            methods: v.methods.into_iter().map(Into::into).collect(),
        }
    }
}
impl TryFrom<OwnedImplementation> for OwnedImplementation28 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: OwnedImplementation) -> Result<Self, Self::Error> {
        if !v.implementation_parameters.is_empty() {
            return Err(implementation_scheme_extension());
        }
        Ok(Self {
            type_parameters: v.type_parameters,
            contract: v.contract,
            self_type: v.self_type,
            type_arguments: v.type_arguments,
            methods: v
                .methods
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}
impl From<OwnedMethodImplementation28> for OwnedMethodImplementation {
    fn from(v: OwnedMethodImplementation28) -> Self {
        Self {
            method: v.method,
            function: v.function,
            type_arguments: v.type_arguments,
            implementations: Vec::new(),
        }
    }
}
impl TryFrom<OwnedMethodImplementation> for OwnedMethodImplementation28 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: OwnedMethodImplementation) -> Result<Self, Self::Error> {
        if !v.implementations.is_empty() {
            return Err(implementation_scheme_extension());
        }
        Ok(Self {
            method: v.method,
            function: v.function,
            type_arguments: v.type_arguments,
        })
    }
}
impl From<ImplementationOperand28> for ImplementationOperand {
    fn from(v: ImplementationOperand28) -> Self {
        match v {
            ImplementationOperand28::Concrete {
                implementation,
                type_arguments,
            } => Self::Concrete {
                implementation,
                type_arguments,
                implementations: Vec::new(),
            },
            ImplementationOperand28::Parameter {
                function,
                parameter,
            } => Self::Parameter {
                scope: function,
                parameter,
            },
        }
    }
}
impl TryFrom<ImplementationOperand> for ImplementationOperand28 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: ImplementationOperand) -> Result<Self, Self::Error> {
        Ok(match v {
            ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                implementations,
            } => {
                if !implementations.is_empty() {
                    return Err(implementation_scheme_extension());
                }
                Self::Concrete {
                    implementation,
                    type_arguments,
                }
            }
            ImplementationOperand::Parameter {
                scope: function,
                parameter,
            } => Self::Parameter {
                function,
                parameter,
            },
        })
    }
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct ExpressionRecord28 {
    pub contract_version: u16,
    pub id: ExpressionId,
    pub operation: ExpressionOperation28,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum ExpressionOperation28 {
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
        requirement_arguments: Vec<super::RequirementOperand>,
        effect_arguments: Vec<super::EffectRow>,
        function: DeclarationReference,
        type_arguments: Vec<TypeObjectDigest>,
        arguments: Vec<ExpressionId>,
    },
    FunctionValue {
        requirement_arguments: Vec<super::RequirementOperand>,
        effect_arguments: Vec<super::EffectRow>,
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
        requirement: super::RequirementOperand,
        operation: OperationReference,
        arguments: Vec<ExpressionId>,
    },
    Transaction {
        requirement: super::RequirementOperand,
        binding: BindingId,
        body: ExpressionId,
    },
    Bind {
        callee: ExpressionId,
        arguments: Vec<ExpressionId>,
    },
    TransactionOutcome {
        requirement: super::RequirementOperand,
        binding: BindingId,
        body: ExpressionId,
        outcome: TransactionOutcomeContract,
        type_argument: TypeObjectDigest,
    },
    F64 {
        value: crate::platform::binary64::Binary64,
    },
    ImplementationCall {
        requirement_arguments: Vec<super::RequirementOperand>,
        effect_arguments: Vec<super::EffectRow>,
        function: DeclarationReference,
        type_arguments: Vec<TypeObjectDigest>,
        implementations: Vec<ImplementationOperand28>,
        arguments: Vec<ExpressionId>,
    },
    MethodCall {
        witness: ImplementationOperand28,
        contract: DeclarationReference,
        method: crate::platform::semantic_id::MethodId,
        arguments: Vec<ExpressionId>,
    },
    PackOwned {
        product_type: TypeObjectDigest,
        /// Authored evaluation order, independent of canonical type field order.
        fields: Vec<OwnedProductExpressionField>,
    },
    UnpackOwned {
        product_type: TypeObjectDigest,
        source: ExpressionId,
        fields: Vec<OwnedProductBinding>,
        body: ExpressionId,
    },
    ChooseOwned {
        choice_type: TypeObjectDigest,
        case: Name,
        value: ExpressionId,
    },
    MatchOwned {
        choice_type: TypeObjectDigest,
        source: ExpressionId,
        arms: Vec<OwnedChoiceArm>,
    },
    /// Both children are exact named task calls. Their arguments are prepared in
    /// authored left-to-right order before either child invocation begins.
    Parallel {
        left: ExpressionId,
        right: ExpressionId,
    },
    BorrowOwnedField {
        product_type: TypeObjectDigest,
        source: ExpressionId,
        field: Name,
        binding: BindingId,
        body: ExpressionId,
    },
    MatchBorrowedOwned {
        choice_type: TypeObjectDigest,
        source: ExpressionId,
        arms: Vec<OwnedChoiceArm>,
    },
    SequenceEmpty {
        sequence_type: TypeObjectDigest,
    },
    SequenceLength {
        sequence_type: TypeObjectDigest,
        source: ExpressionId,
    },
    /// The element precedes the sequence in authored evaluation order.
    SequencePush {
        sequence_type: TypeObjectDigest,
        value: ExpressionId,
        source: ExpressionId,
    },
    SequencePop {
        sequence_type: TypeObjectDigest,
        result_type: TypeObjectDigest,
        source: ExpressionId,
    },
    /// Evaluate the ordinary index before acquiring source/ancestor loans.
    BorrowOwnedItem {
        sequence_type: TypeObjectDigest,
        source: ExpressionId,
        index: ExpressionId,
        binding: BindingId,
        body: ExpressionId,
    },
    /// Adopt a source-tied result into a lexical read-only scope.
    BorrowCall {
        call: ExpressionId,
        binding: BindingId,
        body: ExpressionId,
    },
}

impl From<ExpressionRecord28> for ExpressionRecord {
    fn from(v: ExpressionRecord28) -> Self {
        Self {
            contract_version: v.contract_version,
            id: v.id,
            operation: v.operation.into(),
        }
    }
}
impl TryFrom<ExpressionRecord> for ExpressionRecord28 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: ExpressionRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            contract_version: v.contract_version,
            id: v.id,
            operation: v.operation.try_into()?,
        })
    }
}

impl From<ExpressionOperation28> for ExpressionOperation {
    fn from(v: ExpressionOperation28) -> Self {
        match v {
            ExpressionOperation28::Unit {} => Self::Unit {},
            ExpressionOperation28::Bool { value } => Self::Bool { value },
            ExpressionOperation28::I64 { value } => Self::I64 { value },
            ExpressionOperation28::Text { value } => Self::Text { value },
            ExpressionOperation28::StaticText { value } => Self::StaticText { value },
            ExpressionOperation28::Local { value } => Self::Local { value },
            ExpressionOperation28::Constant { declaration } => Self::Constant { declaration },
            ExpressionOperation28::If {
                condition,
                when_true,
                when_false,
            } => Self::If {
                condition,
                when_true,
                when_false,
            },
            ExpressionOperation28::Let { bindings, body } => Self::Let { bindings, body },
            ExpressionOperation28::Sequence { items } => Self::Sequence { items },
            ExpressionOperation28::Call {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
            } => Self::Call {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
            },
            ExpressionOperation28::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
            } => Self::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
            },
            ExpressionOperation28::Invoke { callee, arguments } => {
                Self::Invoke { callee, arguments }
            }
            ExpressionOperation28::Record {
                nominal_type,
                type_arguments,
                fields,
            } => Self::Record {
                nominal_type,
                type_arguments,
                fields,
            },
            ExpressionOperation28::Variant {
                case,
                type_arguments,
                payload,
            } => Self::Variant {
                case,
                type_arguments,
                payload,
            },
            ExpressionOperation28::Field { value, selector } => Self::Field { value, selector },
            ExpressionOperation28::List { item_type, items } => Self::List { item_type, items },
            ExpressionOperation28::Map {
                key_type,
                value_type,
                entries,
            } => Self::Map {
                key_type,
                value_type,
                entries,
            },
            ExpressionOperation28::Match { value, arms } => Self::Match { value, arms },
            ExpressionOperation28::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => Self::CapabilityCall {
                requirement,
                operation,
                arguments,
            },
            ExpressionOperation28::Transaction {
                requirement,
                binding,
                body,
            } => Self::Transaction {
                requirement,
                binding,
                body,
            },
            ExpressionOperation28::Bind { callee, arguments } => Self::Bind { callee, arguments },
            ExpressionOperation28::TransactionOutcome {
                requirement,
                binding,
                body,
                outcome,
                type_argument,
            } => Self::TransactionOutcome {
                requirement,
                binding,
                body,
                outcome,
                type_argument,
            },
            ExpressionOperation28::F64 { value } => Self::F64 { value },
            ExpressionOperation28::ImplementationCall {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                implementations,
                arguments,
            } => Self::ImplementationCall {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                implementations: implementations.into_iter().map(Into::into).collect(),
                arguments,
            },
            ExpressionOperation28::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => Self::MethodCall {
                witness: witness.into(),
                contract,
                method,
                arguments,
            },
            ExpressionOperation28::PackOwned {
                product_type,
                fields,
            } => Self::PackOwned {
                product_type,
                fields,
            },
            ExpressionOperation28::UnpackOwned {
                product_type,
                source,
                fields,
                body,
            } => Self::UnpackOwned {
                product_type,
                source,
                fields,
                body,
            },
            ExpressionOperation28::ChooseOwned {
                choice_type,
                case,
                value,
            } => Self::ChooseOwned {
                choice_type,
                case,
                value,
            },
            ExpressionOperation28::MatchOwned {
                choice_type,
                source,
                arms,
            } => Self::MatchOwned {
                choice_type,
                source,
                arms,
            },
            ExpressionOperation28::Parallel { left, right } => Self::Parallel { left, right },
            ExpressionOperation28::BorrowOwnedField {
                product_type,
                source,
                field,
                binding,
                body,
            } => Self::BorrowOwnedField {
                product_type,
                source,
                field,
                binding,
                body,
            },
            ExpressionOperation28::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            } => Self::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            },
            ExpressionOperation28::SequenceEmpty { sequence_type } => {
                Self::SequenceEmpty { sequence_type }
            }
            ExpressionOperation28::SequenceLength {
                sequence_type,
                source,
            } => Self::SequenceLength {
                sequence_type,
                source,
            },
            ExpressionOperation28::SequencePush {
                sequence_type,
                value,
                source,
            } => Self::SequencePush {
                sequence_type,
                value,
                source,
            },
            ExpressionOperation28::SequencePop {
                sequence_type,
                result_type,
                source,
            } => Self::SequencePop {
                sequence_type,
                result_type,
                source,
            },
            ExpressionOperation28::BorrowOwnedItem {
                sequence_type,
                source,
                index,
                binding,
                body,
            } => Self::BorrowOwnedItem {
                sequence_type,
                source,
                index,
                binding,
                body,
            },
            ExpressionOperation28::BorrowCall {
                call,
                binding,
                body,
            } => Self::BorrowCall {
                call,
                binding,
                body,
            },
        }
    }
}

impl TryFrom<ExpressionOperation> for ExpressionOperation28 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: ExpressionOperation) -> Result<Self, Self::Error> {
        Ok(match v {
            ExpressionOperation::Unit {} => Self::Unit {},
            ExpressionOperation::Bool { value } => Self::Bool { value },
            ExpressionOperation::I64 { value } => Self::I64 { value },
            ExpressionOperation::Text { value } => Self::Text { value },
            ExpressionOperation::StaticText { value } => Self::StaticText { value },
            ExpressionOperation::Local { value } => Self::Local { value },
            ExpressionOperation::Constant { declaration } => Self::Constant { declaration },
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => Self::If {
                condition,
                when_true,
                when_false,
            },
            ExpressionOperation::Let { bindings, body } => Self::Let { bindings, body },
            ExpressionOperation::Sequence { items } => Self::Sequence { items },
            ExpressionOperation::Call {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
            } => Self::Call {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
            },
            ExpressionOperation::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
            } => Self::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
            },
            ExpressionOperation::Invoke { callee, arguments } => Self::Invoke { callee, arguments },
            ExpressionOperation::Record {
                nominal_type,
                type_arguments,
                fields,
            } => Self::Record {
                nominal_type,
                type_arguments,
                fields,
            },
            ExpressionOperation::Variant {
                case,
                type_arguments,
                payload,
            } => Self::Variant {
                case,
                type_arguments,
                payload,
            },
            ExpressionOperation::Field { value, selector } => Self::Field { value, selector },
            ExpressionOperation::List { item_type, items } => Self::List { item_type, items },
            ExpressionOperation::Map {
                key_type,
                value_type,
                entries,
            } => Self::Map {
                key_type,
                value_type,
                entries,
            },
            ExpressionOperation::Match { value, arms } => Self::Match { value, arms },
            ExpressionOperation::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => Self::CapabilityCall {
                requirement,
                operation,
                arguments,
            },
            ExpressionOperation::Transaction {
                requirement,
                binding,
                body,
            } => Self::Transaction {
                requirement,
                binding,
                body,
            },
            ExpressionOperation::Bind { callee, arguments } => Self::Bind { callee, arguments },
            ExpressionOperation::TransactionOutcome {
                requirement,
                binding,
                body,
                outcome,
                type_argument,
            } => Self::TransactionOutcome {
                requirement,
                binding,
                body,
                outcome,
                type_argument,
            },
            ExpressionOperation::F64 { value } => Self::F64 { value },
            ExpressionOperation::ImplementationCall {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                implementations,
                arguments,
            } => Self::ImplementationCall {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                implementations: implementations
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()?,
                arguments,
            },
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => Self::MethodCall {
                witness: witness.try_into()?,
                contract,
                method,
                arguments,
            },
            ExpressionOperation::PackOwned {
                product_type,
                fields,
            } => Self::PackOwned {
                product_type,
                fields,
            },
            ExpressionOperation::UnpackOwned {
                product_type,
                source,
                fields,
                body,
            } => Self::UnpackOwned {
                product_type,
                source,
                fields,
                body,
            },
            ExpressionOperation::ChooseOwned {
                choice_type,
                case,
                value,
            } => Self::ChooseOwned {
                choice_type,
                case,
                value,
            },
            ExpressionOperation::MatchOwned {
                choice_type,
                source,
                arms,
            } => Self::MatchOwned {
                choice_type,
                source,
                arms,
            },
            ExpressionOperation::Parallel { left, right } => Self::Parallel { left, right },
            ExpressionOperation::BorrowOwnedField {
                product_type,
                source,
                field,
                binding,
                body,
            } => Self::BorrowOwnedField {
                product_type,
                source,
                field,
                binding,
                body,
            },
            ExpressionOperation::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            } => Self::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            },
            ExpressionOperation::SequenceEmpty { sequence_type } => {
                Self::SequenceEmpty { sequence_type }
            }
            ExpressionOperation::SequenceLength {
                sequence_type,
                source,
            } => Self::SequenceLength {
                sequence_type,
                source,
            },
            ExpressionOperation::SequencePush {
                sequence_type,
                value,
                source,
            } => Self::SequencePush {
                sequence_type,
                value,
                source,
            },
            ExpressionOperation::SequencePop {
                sequence_type,
                result_type,
                source,
            } => Self::SequencePop {
                sequence_type,
                result_type,
                source,
            },
            ExpressionOperation::BorrowOwnedItem {
                sequence_type,
                source,
                index,
                binding,
                body,
            } => Self::BorrowOwnedItem {
                sequence_type,
                source,
                index,
                binding,
                body,
            },
            ExpressionOperation::BorrowCall {
                call,
                binding,
                body,
            } => Self::BorrowCall {
                call,
                binding,
                body,
            },
        })
    }
}
