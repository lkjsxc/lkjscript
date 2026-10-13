//! Frozen Graph 29–30 owner expression layouts.
//! Generalized sequence operations must never reinterpret predecessor bytes.

use super::frozen_read_view_wire as frozen;
use super::*;
use crate::platform::semantic_id::*;
use bincode::{Decode, Encode};

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum OwnerRecord30 {
    Module(ModuleRecord),
    Declaration(frozen::FrozenDeclarationRecord33),
    TypeParameter(TypeParameterRecord),
    EffectParameter(EffectParameterRecord),
    Field(FieldRecord),
    Case(CaseRecord),
    Operation(OperationRecord),
    Parameter(ParameterRecord),
    Binding(frozen::FrozenBindingRecord33),
    Expression(ExpressionRecord30),
    Requirement(RequirementRecord),
    Port(PortRecord),
    Target(TargetRecord),
    Documentation(DocumentationRecord),
    Annotation(AnnotationRecord),
    HttpRoute(HttpRouteRecord),
    RequirementParameter(RequirementParameterRecord),
}

impl From<OwnerRecord30> for OwnerRecord {
    fn from(v: OwnerRecord30) -> Self {
        match v {
            OwnerRecord30::Module(v) => Self::Module(v),
            OwnerRecord30::Declaration(v) => Self::Declaration(v.into()),
            OwnerRecord30::TypeParameter(v) => Self::TypeParameter(v),
            OwnerRecord30::EffectParameter(v) => Self::EffectParameter(v),
            OwnerRecord30::Field(v) => Self::Field(v),
            OwnerRecord30::Case(v) => Self::Case(v),
            OwnerRecord30::Operation(v) => Self::Operation(v),
            OwnerRecord30::Parameter(v) => Self::Parameter(v),
            OwnerRecord30::Binding(v) => Self::Binding(v.into()),
            OwnerRecord30::Expression(v) => Self::Expression(v.into()),
            OwnerRecord30::Requirement(v) => Self::Requirement(v),
            OwnerRecord30::Port(v) => Self::Port(v),
            OwnerRecord30::Target(v) => Self::Target(v),
            OwnerRecord30::Documentation(v) => Self::Documentation(v),
            OwnerRecord30::Annotation(v) => Self::Annotation(v),
            OwnerRecord30::HttpRoute(v) => Self::HttpRoute(v),
            OwnerRecord30::RequirementParameter(v) => Self::RequirementParameter(v),
        }
    }
}

impl TryFrom<OwnerRecord> for OwnerRecord30 {
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
            OwnerRecord::Binding(v) => Self::Binding(v.try_into()?),
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

pub(crate) fn generalized_sequence_extension() -> crate::platform::diagnostic::Diagnostic {
    crate::platform::diagnostic::Diagnostic::new(
        crate::platform::diagnostic::DiagnosticClass::Source,
        "kernel_sequence_generation",
        "ordinary sequence elements and indexed sequence operations require Graph 31",
    )
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct ExpressionRecord30 {
    pub contract_version: u16,
    pub id: ExpressionId,
    pub operation: ExpressionOperation30,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum ExpressionOperation30 {
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

impl From<ExpressionRecord30> for ExpressionRecord {
    fn from(v: ExpressionRecord30) -> Self {
        Self {
            contract_version: v.contract_version,
            id: v.id,
            operation: v.operation.into(),
        }
    }
}
impl TryFrom<ExpressionRecord> for ExpressionRecord30 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: ExpressionRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            contract_version: v.contract_version,
            id: v.id,
            operation: v.operation.try_into()?,
        })
    }
}

impl From<ExpressionOperation30> for ExpressionOperation {
    fn from(v: ExpressionOperation30) -> Self {
        match v {
            ExpressionOperation30::Unit {} => Self::Unit {},
            ExpressionOperation30::Bool { value } => Self::Bool { value },
            ExpressionOperation30::I64 { value } => Self::I64 { value },
            ExpressionOperation30::Text { value } => Self::Text { value },
            ExpressionOperation30::StaticText { value } => Self::StaticText { value },
            ExpressionOperation30::Local { value } => Self::Local { value },
            ExpressionOperation30::Constant { declaration } => Self::Constant { declaration },
            ExpressionOperation30::If {
                condition,
                when_true,
                when_false,
            } => Self::If {
                condition,
                when_true,
                when_false,
            },
            ExpressionOperation30::Let { bindings, body } => Self::Let { bindings, body },
            ExpressionOperation30::Sequence { items } => Self::Sequence { items },
            ExpressionOperation30::Call {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
            } => Self::Call {
                read_scope_arguments: Vec::new(),
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
            },
            ExpressionOperation30::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
            } => Self::FunctionValue {
                read_scope_arguments: Vec::new(),
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
            },
            ExpressionOperation30::Invoke { callee, arguments } => {
                Self::Invoke { callee, arguments }
            }
            ExpressionOperation30::Record {
                nominal_type,
                type_arguments,
                fields,
            } => Self::Record {
                nominal_type,
                type_arguments,
                fields,
            },
            ExpressionOperation30::Variant {
                case,
                type_arguments,
                payload,
            } => Self::Variant {
                case,
                type_arguments,
                payload,
            },
            ExpressionOperation30::Field { value, selector } => Self::Field { value, selector },
            ExpressionOperation30::List { item_type, items } => Self::List { item_type, items },
            ExpressionOperation30::Map {
                key_type,
                value_type,
                entries,
            } => Self::Map {
                key_type,
                value_type,
                entries,
            },
            ExpressionOperation30::Match { value, arms } => Self::Match { value, arms },
            ExpressionOperation30::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => Self::CapabilityCall {
                requirement,
                operation,
                arguments,
            },
            ExpressionOperation30::Transaction {
                requirement,
                binding,
                body,
            } => Self::Transaction {
                requirement,
                binding,
                body,
            },
            ExpressionOperation30::Bind { callee, arguments } => Self::Bind { callee, arguments },
            ExpressionOperation30::TransactionOutcome {
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
            ExpressionOperation30::F64 { value } => Self::F64 { value },
            ExpressionOperation30::ImplementationCall {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                implementations,
                arguments,
            } => Self::ImplementationCall {
                read_scope_arguments: Vec::new(),
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                implementations,
                arguments,
            },
            ExpressionOperation30::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => Self::MethodCall {
                read_scope_arguments: Vec::new(),
                witness,
                contract,
                method,
                arguments,
            },
            ExpressionOperation30::PackOwned {
                product_type,
                fields,
            } => Self::PackOwned {
                product_type,
                fields,
            },
            ExpressionOperation30::UnpackOwned {
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
            ExpressionOperation30::ChooseOwned {
                choice_type,
                case,
                value,
            } => Self::ChooseOwned {
                choice_type,
                case,
                value,
            },
            ExpressionOperation30::MatchOwned {
                choice_type,
                source,
                arms,
            } => Self::MatchOwned {
                choice_type,
                source,
                arms,
            },
            ExpressionOperation30::Parallel { left, right } => Self::Parallel { left, right },
            ExpressionOperation30::BorrowOwnedField {
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
            ExpressionOperation30::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            } => Self::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            },
            ExpressionOperation30::SequenceEmpty { sequence_type } => {
                Self::SequenceEmpty { sequence_type }
            }
            ExpressionOperation30::SequenceLength {
                sequence_type,
                source,
            } => Self::SequenceLength {
                sequence_type,
                source,
            },
            ExpressionOperation30::SequencePush {
                sequence_type,
                value,
                source,
            } => Self::SequencePush {
                sequence_type,
                value,
                source,
            },
            ExpressionOperation30::SequencePop {
                sequence_type,
                result_type,
                source,
            } => Self::SequencePop {
                sequence_type,
                result_type,
                source,
            },
            ExpressionOperation30::BorrowOwnedItem {
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
            ExpressionOperation30::BorrowCall {
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

impl TryFrom<ExpressionOperation> for ExpressionOperation30 {
    type Error = crate::platform::diagnostic::Diagnostic;
    fn try_from(v: ExpressionOperation) -> Result<Self, Self::Error> {
        Ok(match v {
            super::ExpressionOperation::CreateReadView { .. }
            | super::ExpressionOperation::BorrowReadView { .. }
            | super::ExpressionOperation::ReborrowReadView { .. }
            | super::ExpressionOperation::MoveReadView { .. }
            | super::ExpressionOperation::DropReadView { .. }
            | super::ExpressionOperation::ReadViewDataField { .. }
            | super::ExpressionOperation::BorrowReadViewField { .. } => {
                return Err(frozen::read_view_extension());
            }
            super::ExpressionOperation::PackOwnedRecord { .. }
            | super::ExpressionOperation::UnpackOwnedRecord { .. }
            | super::ExpressionOperation::BorrowOwnedRecordField { .. } => {
                return Err(super::owned_record::reject(
                    "nominal owned operations require Graph 32",
                ));
            }
            ExpressionOperation::SequenceGet { .. }
            | ExpressionOperation::SequenceReplace { .. } => {
                return Err(generalized_sequence_extension());
            }
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
                read_scope_arguments,
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
            } => {
                frozen::require_empty_read_scope(read_scope_arguments.is_empty())?;
                Self::Call {
                    requirement_arguments,
                    effect_arguments,
                    function,
                    type_arguments,
                    arguments,
                }
            }
            ExpressionOperation::FunctionValue {
                read_scope_arguments,
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
            } => {
                frozen::require_empty_read_scope(read_scope_arguments.is_empty())?;
                Self::FunctionValue {
                    requirement_arguments,
                    effect_arguments,
                    function,
                    type_arguments,
                }
            }
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
                read_scope_arguments,
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                implementations,
                arguments,
            } => {
                frozen::require_empty_read_scope(read_scope_arguments.is_empty())?;
                Self::ImplementationCall {
                    requirement_arguments,
                    effect_arguments,
                    function,
                    type_arguments,
                    implementations,
                    arguments,
                }
            }
            ExpressionOperation::MethodCall {
                read_scope_arguments,
                witness,
                contract,
                method,
                arguments,
            } => {
                frozen::require_empty_read_scope(read_scope_arguments.is_empty())?;
                Self::MethodCall {
                    witness,
                    contract,
                    method,
                    arguments,
                }
            }
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
