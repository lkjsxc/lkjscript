//! Frozen Graph 15–22 expression layout and Graph 18–22 owner layout.
//! Only implementation applications gain operands in Graph 23; predecessor bytes remain exact.

use super::*;
use crate::platform::semantic_id::*;
use bincode::{Decode, Encode};

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum OwnerRecord22 {
    Module(ModuleRecord),
    Declaration(DeclarationRecord),
    TypeParameter(TypeParameterRecord),
    EffectParameter(EffectParameterRecord),
    Field(FieldRecord),
    Case(CaseRecord),
    Operation(OperationRecord),
    Parameter(ParameterRecord),
    Binding(BindingRecord),
    Expression(ExpressionRecord22),
    Requirement(RequirementRecord),
    Port(PortRecord),
    Target(TargetRecord),
    Documentation(DocumentationRecord),
    Annotation(AnnotationRecord),
    HttpRoute(HttpRouteRecord),
    RequirementParameter(RequirementParameterRecord),
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct ExpressionRecord22 {
    pub contract_version: u16,
    pub id: ExpressionId,
    pub operation: ExpressionOperation22,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum ExpressionOperation22 {
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
        function: DeclarationReference,
        type_arguments: Vec<TypeObjectDigest>,
        implementations: Vec<super::ImplementationOperand>,
        arguments: Vec<ExpressionId>,
    },
    MethodCall {
        witness: super::ImplementationOperand,
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
}

impl From<OwnerRecord22> for OwnerRecord {
    fn from(value: OwnerRecord22) -> Self {
        match value {
            OwnerRecord22::Module(record) => Self::Module(record),
            OwnerRecord22::Declaration(record) => Self::Declaration(record),
            OwnerRecord22::TypeParameter(record) => Self::TypeParameter(record),
            OwnerRecord22::EffectParameter(record) => Self::EffectParameter(record),
            OwnerRecord22::Field(record) => Self::Field(record),
            OwnerRecord22::Case(record) => Self::Case(record),
            OwnerRecord22::Operation(record) => Self::Operation(record),
            OwnerRecord22::Parameter(record) => Self::Parameter(record),
            OwnerRecord22::Binding(record) => Self::Binding(record),
            OwnerRecord22::Expression(record) => Self::Expression(record.into()),
            OwnerRecord22::Requirement(record) => Self::Requirement(record),
            OwnerRecord22::Port(record) => Self::Port(record),
            OwnerRecord22::Target(record) => Self::Target(record),
            OwnerRecord22::Documentation(record) => Self::Documentation(record),
            OwnerRecord22::Annotation(record) => Self::Annotation(record),
            OwnerRecord22::HttpRoute(record) => Self::HttpRoute(record),
            OwnerRecord22::RequirementParameter(record) => Self::RequirementParameter(record),
        }
    }
}

impl TryFrom<OwnerRecord> for OwnerRecord22 {
    type Error = crate::platform::diagnostic::Diagnostic;

    fn try_from(value: OwnerRecord) -> Result<Self, Self::Error> {
        Ok(match value {
            OwnerRecord::Module(record) => Self::Module(record),
            OwnerRecord::Declaration(record) => Self::Declaration(record),
            OwnerRecord::TypeParameter(record) => Self::TypeParameter(record),
            OwnerRecord::EffectParameter(record) => Self::EffectParameter(record),
            OwnerRecord::Field(record) => Self::Field(record),
            OwnerRecord::Case(record) => Self::Case(record),
            OwnerRecord::Operation(record) => Self::Operation(record),
            OwnerRecord::Parameter(record) => Self::Parameter(record),
            OwnerRecord::Binding(record) => Self::Binding(record),
            OwnerRecord::Expression(record) => Self::Expression(record.try_into()?),
            OwnerRecord::Requirement(record) => Self::Requirement(record),
            OwnerRecord::Port(record) => Self::Port(record),
            OwnerRecord::Target(record) => Self::Target(record),
            OwnerRecord::Documentation(record) => Self::Documentation(record),
            OwnerRecord::Annotation(record) => Self::Annotation(record),
            OwnerRecord::HttpRoute(record) => Self::HttpRoute(record),
            OwnerRecord::RequirementParameter(record) => Self::RequirementParameter(record),
        })
    }
}

impl From<ExpressionRecord22> for ExpressionRecord {
    fn from(value: ExpressionRecord22) -> Self {
        Self {
            contract_version: value.contract_version,
            id: value.id,
            operation: value.operation.into(),
        }
    }
}

impl TryFrom<ExpressionRecord> for ExpressionRecord22 {
    type Error = crate::platform::diagnostic::Diagnostic;

    fn try_from(value: ExpressionRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            contract_version: value.contract_version,
            id: value.id,
            operation: value.operation.try_into()?,
        })
    }
}

impl From<ExpressionOperation22> for ExpressionOperation {
    fn from(value: ExpressionOperation22) -> Self {
        match value {
            ExpressionOperation22::Unit {} => Self::Unit {},
            ExpressionOperation22::Bool { value } => Self::Bool { value },
            ExpressionOperation22::I64 { value } => Self::I64 { value },
            ExpressionOperation22::Text { value } => Self::Text { value },
            ExpressionOperation22::StaticText { value } => Self::StaticText { value },
            ExpressionOperation22::Local { value } => Self::Local { value },
            ExpressionOperation22::Constant { declaration } => Self::Constant { declaration },
            ExpressionOperation22::If {
                condition,
                when_true,
                when_false,
            } => Self::If {
                condition,
                when_true,
                when_false,
            },
            ExpressionOperation22::Let { bindings, body } => Self::Let { bindings, body },
            ExpressionOperation22::Sequence { items } => Self::Sequence { items },
            ExpressionOperation22::Call {
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
            ExpressionOperation22::FunctionValue {
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
            ExpressionOperation22::Invoke { callee, arguments } => {
                Self::Invoke { callee, arguments }
            }
            ExpressionOperation22::Record {
                nominal_type,
                type_arguments,
                fields,
            } => Self::Record {
                nominal_type,
                type_arguments,
                fields,
            },
            ExpressionOperation22::Variant {
                case,
                type_arguments,
                payload,
            } => Self::Variant {
                case,
                type_arguments,
                payload,
            },
            ExpressionOperation22::Field { value, selector } => Self::Field { value, selector },
            ExpressionOperation22::List { item_type, items } => Self::List { item_type, items },
            ExpressionOperation22::Map {
                key_type,
                value_type,
                entries,
            } => Self::Map {
                key_type,
                value_type,
                entries,
            },
            ExpressionOperation22::Match { value, arms } => Self::Match { value, arms },
            ExpressionOperation22::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => Self::CapabilityCall {
                requirement,
                operation,
                arguments,
            },
            ExpressionOperation22::Transaction {
                requirement,
                binding,
                body,
            } => Self::Transaction {
                requirement,
                binding,
                body,
            },
            ExpressionOperation22::Bind { callee, arguments } => Self::Bind { callee, arguments },
            ExpressionOperation22::TransactionOutcome {
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
            ExpressionOperation22::F64 { value } => Self::F64 { value },
            ExpressionOperation22::ImplementationCall {
                function,
                type_arguments,
                implementations,
                arguments,
            } => Self::ImplementationCall {
                requirement_arguments: Vec::new(),
                effect_arguments: Vec::new(),
                function,
                type_arguments,
                implementations,
                arguments,
            },
            ExpressionOperation22::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => Self::MethodCall {
                witness,
                contract,
                method,
                arguments,
            },
            ExpressionOperation22::PackOwned {
                product_type,
                fields,
            } => Self::PackOwned {
                product_type,
                fields,
            },
            ExpressionOperation22::UnpackOwned {
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
            ExpressionOperation22::ChooseOwned {
                choice_type,
                case,
                value,
            } => Self::ChooseOwned {
                choice_type,
                case,
                value,
            },
            ExpressionOperation22::MatchOwned {
                choice_type,
                source,
                arms,
            } => Self::MatchOwned {
                choice_type,
                source,
                arms,
            },
            ExpressionOperation22::Parallel { left, right } => Self::Parallel { left, right },
        }
    }
}

impl TryFrom<ExpressionOperation> for ExpressionOperation22 {
    type Error = crate::platform::diagnostic::Diagnostic;

    fn try_from(value: ExpressionOperation) -> Result<Self, Self::Error> {
        Ok(match value {
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
            } => {
                if !requirement_arguments.is_empty() || !effect_arguments.is_empty() {
                    return Err(crate::platform::diagnostic::Diagnostic::new(
                        crate::platform::diagnostic::DiagnosticClass::Source,
                        "kernel_owned_effect_generation",
                        "implementation effect and requirement arguments require Graph 23",
                    ));
                }
                Self::ImplementationCall {
                    function,
                    type_arguments,
                    implementations,
                    arguments,
                }
            }
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => Self::MethodCall {
                witness,
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
        })
    }
}
