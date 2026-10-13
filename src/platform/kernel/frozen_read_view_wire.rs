//! Exact pre-read-view Graph 33 and Type 10 layouts.
//! These DTOs preserve predecessor bytes without depending on live callable,
//! binding, associated-inventory, expression, or type-form layouts.

use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::*;
use bincode::{Decode, Encode};

pub(crate) fn read_view_extension() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Semantic,
        "kernel_read_view_encoding_generation",
        "read-view meaning cannot be represented by a predecessor codec",
    )
}

pub(crate) fn require_empty_read_scope(is_empty: bool) -> Result<(), Diagnostic> {
    if is_empty {
        Ok(())
    } else {
        Err(read_view_extension())
    }
}

pub(crate) fn legacy_result<S>(ty: TypeObjectDigest, source: Option<S>) -> CallableResult<S> {
    CallableResult {
        ty,
        relation: match source {
            Some(source) => ResultRelation::ReadFrom { source },
            None => ResultRelation::Value,
        },
    }
}

pub(crate) fn split_result<S>(
    result: CallableResult<S>,
) -> Result<(TypeObjectDigest, Option<S>), Diagnostic> {
    Ok((
        result.ty,
        match result.relation {
            ResultRelation::Value => None,
            ResultRelation::ReadFrom { source } => Some(source),
            ResultRelation::ViewFrom { .. } => return Err(read_view_extension()),
        },
    ))
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum FrozenOwnerRecord33 {
    Module(ModuleRecord),
    Declaration(FrozenDeclarationRecord33),
    TypeParameter(TypeParameterRecord),
    EffectParameter(EffectParameterRecord),
    Field(FieldRecord),
    Case(CaseRecord),
    Operation(OperationRecord),
    Parameter(ParameterRecord),
    Binding(FrozenBindingRecord33),
    Expression(FrozenExpressionRecord33),
    Requirement(RequirementRecord),
    Port(PortRecord),
    Target(TargetRecord),
    Documentation(DocumentationRecord),
    Annotation(AnnotationRecord),
    HttpRoute(HttpRouteRecord),
    RequirementParameter(RequirementParameterRecord),
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct FrozenDeclarationRecord33 {
    pub header: OwnerHeader,
    pub module: ModuleId,
    pub name: Name,
    pub visibility: DeclarationVisibility,
    pub payload: FrozenDeclarationPayload33,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum FrozenDeclarationPayload33 {
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
    Function(FrozenFunctionDeclaration33),
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
    OwnedContract(OwnedContract33),
    OwnedImplementation(OwnedImplementation33),
    OwnedRecord {
        type_parameters: Vec<TypeParameterId>,
        fields: Vec<FieldId>,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct FrozenFunctionDeclaration33 {
    pub implementation_parameters: Vec<super::ImplementationParameter>,
    pub requirement_parameters: Vec<crate::platform::semantic_id::RequirementParameterId>,
    pub effect_parameters: Vec<EffectParameterId>,
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<ParameterId>,
    pub result: TypeObjectDigest,
    /// Exact borrowed input whose custody protects the read-only result.
    pub result_borrow: Option<ParameterId>,
    pub effect: FunctionEffect,
    pub body: ExpressionId,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct FrozenBindingRecord33 {
    pub header: OwnerHeader,
    pub name: Name,
    pub kind: FrozenBindingKind33,
    pub value: Option<ExpressionId>,
    pub declared_type: Option<TypeObjectDigest>,
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub enum FrozenBindingKind33 {
    Let,
    MatchPayload,
    Transaction,
    OwnedUnpack,
    OwnedChoicePayload,
    OwnedBorrow,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct FrozenExpressionRecord33 {
    pub contract_version: u16,
    pub id: ExpressionId,
    pub operation: FrozenExpressionOperation33,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum FrozenExpressionOperation33 {
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
    /// Evaluate the index before taking a short source read; returns ordinary data.
    SequenceGet {
        sequence_type: TypeObjectDigest,
        source: ExpressionId,
        index: ExpressionId,
    },
    /// Evaluate index, replacement value and source, returning rest and displaced value.
    SequenceReplace {
        sequence_type: TypeObjectDigest,
        result_type: TypeObjectDigest,
        index: ExpressionId,
        value: ExpressionId,
        source: ExpressionId,
    },
    PackOwnedRecord {
        record_type: TypeObjectDigest,
        fields: Vec<OwnedRecordExpressionField>,
    },
    UnpackOwnedRecord {
        record_type: TypeObjectDigest,
        source: ExpressionId,
        fields: Vec<OwnedRecordBinding>,
        body: ExpressionId,
    },
    BorrowOwnedRecordField {
        record_type: TypeObjectDigest,
        source: ExpressionId,
        field: FieldReference,
        binding: BindingId,
        body: ExpressionId,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct FrozenTypeObject10 {
    pub contract_version: u16,
    pub form: FrozenTypeForm10,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum FrozenTypeForm10 {
    Unit,
    Bool,
    I64,
    Bytes,
    Text,
    StaticText,
    Secret,
    TypeParameter {
        parameter: TypeParameterId,
    },
    Named {
        declaration: DeclarationReference,
    },
    CapabilityResource {
        interface: DeclarationReference,
    },
    StructuralRecord {
        fields: Vec<StructuralTypeField>,
    },
    List {
        item: TypeObjectDigest,
    },
    Map {
        key: TypeObjectDigest,
        value: TypeObjectDigest,
    },
    Option {
        item: TypeObjectDigest,
    },
    Result {
        ok: TypeObjectDigest,
        error: TypeObjectDigest,
    },
    Stream {
        item: TypeObjectDigest,
    },
    Function {
        parameters: Vec<TypeObjectDigest>,
        result: TypeObjectDigest,
    },
    /// Positive-arity nominal identity. Its wire form has a separate explicit envelope/tag;
    /// it is never encoded as a new ordinal in the retained base-type envelope.
    Applied {
        declaration: DeclarationReference,
        arguments: Vec<TypeObjectDigest>,
    },
    /// Disjoint encoding preserves the bytes of all existing pure and nominal forms.
    TaskFunction {
        parameters: Vec<TypeObjectDigest>,
        result: TypeObjectDigest,
        effect: super::EffectRow,
    },
    /// A disjoint envelope preserves every predecessor type object's bytes.
    F64,
    ByteBuffer,
    OwnedI64Cell,
    /// Explicit affine structure; never encoded in the ordinary base type envelope.
    OwnedProduct {
        fields: Vec<StructuralTypeField>,
    },
    /// An explicit affine sum. Exactly one named case carries one typed payload.
    /// Its ownership is invariant under case selection and generic substitution.
    OwnedChoice {
        cases: Vec<StructuralTypeField>,
    },
    /// One affine owner of a runtime-sized sequence of admitted owned or ordinary data.
    OwnedSequence {
        item: TypeObjectDigest,
    },
    /// Contract-local abstract member, legal only in its exact contract's method signatures.
    AssociatedContractMember {
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
    },
    /// Exact lexical witness identity. Equal Self types confer no selection authority.
    AssociatedWitnessProjection {
        scope: DeclarationReference,
        parameter: ImplementationParameterId,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
    },
}

impl From<FrozenOwnerRecord33> for OwnerRecord {
    fn from(value: FrozenOwnerRecord33) -> Self {
        match value {
            FrozenOwnerRecord33::Module(value) => Self::Module(value),
            FrozenOwnerRecord33::Declaration(value) => Self::Declaration(value.into()),
            FrozenOwnerRecord33::TypeParameter(value) => Self::TypeParameter(value),
            FrozenOwnerRecord33::EffectParameter(value) => Self::EffectParameter(value),
            FrozenOwnerRecord33::Field(value) => Self::Field(value),
            FrozenOwnerRecord33::Case(value) => Self::Case(value),
            FrozenOwnerRecord33::Operation(value) => Self::Operation(value),
            FrozenOwnerRecord33::Parameter(value) => Self::Parameter(value),
            FrozenOwnerRecord33::Binding(value) => Self::Binding(value.into()),
            FrozenOwnerRecord33::Expression(value) => Self::Expression(value.into()),
            FrozenOwnerRecord33::Requirement(value) => Self::Requirement(value),
            FrozenOwnerRecord33::Port(value) => Self::Port(value),
            FrozenOwnerRecord33::Target(value) => Self::Target(value),
            FrozenOwnerRecord33::Documentation(value) => Self::Documentation(value),
            FrozenOwnerRecord33::Annotation(value) => Self::Annotation(value),
            FrozenOwnerRecord33::HttpRoute(value) => Self::HttpRoute(value),
            FrozenOwnerRecord33::RequirementParameter(value) => Self::RequirementParameter(value),
        }
    }
}

impl TryFrom<OwnerRecord> for FrozenOwnerRecord33 {
    type Error = Diagnostic;
    fn try_from(value: OwnerRecord) -> Result<Self, Self::Error> {
        Ok(match value {
            OwnerRecord::Module(value) => Self::Module(value),
            OwnerRecord::Declaration(value) => Self::Declaration(value.try_into()?),
            OwnerRecord::TypeParameter(value) => Self::TypeParameter(value),
            OwnerRecord::EffectParameter(value) => Self::EffectParameter(value),
            OwnerRecord::Field(value) => Self::Field(value),
            OwnerRecord::Case(value) => Self::Case(value),
            OwnerRecord::Operation(value) => Self::Operation(value),
            OwnerRecord::Parameter(value) => Self::Parameter(value),
            OwnerRecord::Binding(value) => Self::Binding(value.try_into()?),
            OwnerRecord::Expression(value) => Self::Expression(value.try_into()?),
            OwnerRecord::Requirement(value) => Self::Requirement(value),
            OwnerRecord::Port(value) => Self::Port(value),
            OwnerRecord::Target(value) => Self::Target(value),
            OwnerRecord::Documentation(value) => Self::Documentation(value),
            OwnerRecord::Annotation(value) => Self::Annotation(value),
            OwnerRecord::HttpRoute(value) => Self::HttpRoute(value),
            OwnerRecord::RequirementParameter(value) => Self::RequirementParameter(value),
        })
    }
}

impl From<FrozenDeclarationPayload33> for DeclarationPayload {
    fn from(value: FrozenDeclarationPayload33) -> Self {
        match value {
            FrozenDeclarationPayload33::Record {
                type_parameters,
                fields,
            } => Self::Record {
                type_parameters,
                fields,
            },
            FrozenDeclarationPayload33::Variant {
                type_parameters,
                cases,
            } => Self::Variant {
                type_parameters,
                cases,
            },
            FrozenDeclarationPayload33::Interface { operations } => Self::Interface { operations },
            FrozenDeclarationPayload33::External(value) => Self::External(value),
            FrozenDeclarationPayload33::Function(value) => Self::Function(value.into()),
            FrozenDeclarationPayload33::Constant { ty, value } => Self::Constant { ty, value },
            FrozenDeclarationPayload33::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports,
            },
            FrozenDeclarationPayload33::Test {
                actual,
                expected,
                comparison,
            } => Self::Test {
                actual,
                expected,
                comparison,
            },
            FrozenDeclarationPayload33::OwnedContract(value) => Self::OwnedContract(value.into()),
            FrozenDeclarationPayload33::OwnedImplementation(value) => {
                Self::OwnedImplementation(value.into())
            }
            FrozenDeclarationPayload33::OwnedRecord {
                type_parameters,
                fields,
            } => Self::OwnedRecord {
                type_parameters,
                fields,
            },
        }
    }
}

impl TryFrom<DeclarationPayload> for FrozenDeclarationPayload33 {
    type Error = Diagnostic;
    fn try_from(value: DeclarationPayload) -> Result<Self, Self::Error> {
        Ok(match value {
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
            DeclarationPayload::External(value) => Self::External(value),
            DeclarationPayload::Function(value) => Self::Function(value.try_into()?),
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
            DeclarationPayload::OwnedContract(value) => Self::OwnedContract(value.try_into()?),
            DeclarationPayload::OwnedImplementation(value) => {
                Self::OwnedImplementation(value.try_into()?)
            }
            DeclarationPayload::OwnedRecord {
                type_parameters,
                fields,
            } => Self::OwnedRecord {
                type_parameters,
                fields,
            },
            _ => return Err(read_view_extension()),
        })
    }
}

impl From<FrozenBindingKind33> for BindingKind {
    fn from(value: FrozenBindingKind33) -> Self {
        match value {
            FrozenBindingKind33::Let => Self::Let,
            FrozenBindingKind33::MatchPayload => Self::MatchPayload,
            FrozenBindingKind33::Transaction => Self::Transaction,
            FrozenBindingKind33::OwnedUnpack => Self::OwnedUnpack,
            FrozenBindingKind33::OwnedChoicePayload => Self::OwnedChoicePayload,
            FrozenBindingKind33::OwnedBorrow => Self::OwnedBorrow,
        }
    }
}

impl TryFrom<BindingKind> for FrozenBindingKind33 {
    type Error = Diagnostic;
    fn try_from(value: BindingKind) -> Result<Self, Self::Error> {
        Ok(match value {
            BindingKind::Let => Self::Let,
            BindingKind::MatchPayload => Self::MatchPayload,
            BindingKind::Transaction => Self::Transaction,
            BindingKind::OwnedUnpack => Self::OwnedUnpack,
            BindingKind::OwnedChoicePayload => Self::OwnedChoicePayload,
            BindingKind::OwnedBorrow => Self::OwnedBorrow,
            _ => return Err(read_view_extension()),
        })
    }
}

impl From<FrozenExpressionOperation33> for ExpressionOperation {
    fn from(value: FrozenExpressionOperation33) -> Self {
        match value {
            FrozenExpressionOperation33::Unit {} => Self::Unit {},
            FrozenExpressionOperation33::Bool { value } => Self::Bool { value },
            FrozenExpressionOperation33::I64 { value } => Self::I64 { value },
            FrozenExpressionOperation33::Text { value } => Self::Text { value },
            FrozenExpressionOperation33::StaticText { value } => Self::StaticText { value },
            FrozenExpressionOperation33::Local { value } => Self::Local { value },
            FrozenExpressionOperation33::Constant { declaration } => Self::Constant { declaration },
            FrozenExpressionOperation33::If {
                condition,
                when_true,
                when_false,
            } => Self::If {
                condition,
                when_true,
                when_false,
            },
            FrozenExpressionOperation33::Let { bindings, body } => Self::Let { bindings, body },
            FrozenExpressionOperation33::Sequence { items } => Self::Sequence { items },
            FrozenExpressionOperation33::Call {
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
                read_scope_arguments: Vec::new(),
            },
            FrozenExpressionOperation33::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
            } => Self::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                read_scope_arguments: Vec::new(),
            },
            FrozenExpressionOperation33::Invoke { callee, arguments } => {
                Self::Invoke { callee, arguments }
            }
            FrozenExpressionOperation33::Record {
                nominal_type,
                type_arguments,
                fields,
            } => Self::Record {
                nominal_type,
                type_arguments,
                fields,
            },
            FrozenExpressionOperation33::Variant {
                case,
                type_arguments,
                payload,
            } => Self::Variant {
                case,
                type_arguments,
                payload,
            },
            FrozenExpressionOperation33::Field { value, selector } => {
                Self::Field { value, selector }
            }
            FrozenExpressionOperation33::List { item_type, items } => {
                Self::List { item_type, items }
            }
            FrozenExpressionOperation33::Map {
                key_type,
                value_type,
                entries,
            } => Self::Map {
                key_type,
                value_type,
                entries,
            },
            FrozenExpressionOperation33::Match { value, arms } => Self::Match { value, arms },
            FrozenExpressionOperation33::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => Self::CapabilityCall {
                requirement,
                operation,
                arguments,
            },
            FrozenExpressionOperation33::Transaction {
                requirement,
                binding,
                body,
            } => Self::Transaction {
                requirement,
                binding,
                body,
            },
            FrozenExpressionOperation33::Bind { callee, arguments } => {
                Self::Bind { callee, arguments }
            }
            FrozenExpressionOperation33::TransactionOutcome {
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
            FrozenExpressionOperation33::F64 { value } => Self::F64 { value },
            FrozenExpressionOperation33::ImplementationCall {
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
                implementations,
                arguments,
                read_scope_arguments: Vec::new(),
            },
            FrozenExpressionOperation33::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => Self::MethodCall {
                witness,
                contract,
                method,
                arguments,
                read_scope_arguments: Vec::new(),
            },
            FrozenExpressionOperation33::PackOwned {
                product_type,
                fields,
            } => Self::PackOwned {
                product_type,
                fields,
            },
            FrozenExpressionOperation33::UnpackOwned {
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
            FrozenExpressionOperation33::ChooseOwned {
                choice_type,
                case,
                value,
            } => Self::ChooseOwned {
                choice_type,
                case,
                value,
            },
            FrozenExpressionOperation33::MatchOwned {
                choice_type,
                source,
                arms,
            } => Self::MatchOwned {
                choice_type,
                source,
                arms,
            },
            FrozenExpressionOperation33::Parallel { left, right } => Self::Parallel { left, right },
            FrozenExpressionOperation33::BorrowOwnedField {
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
            FrozenExpressionOperation33::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            } => Self::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            },
            FrozenExpressionOperation33::SequenceEmpty { sequence_type } => {
                Self::SequenceEmpty { sequence_type }
            }
            FrozenExpressionOperation33::SequenceLength {
                sequence_type,
                source,
            } => Self::SequenceLength {
                sequence_type,
                source,
            },
            FrozenExpressionOperation33::SequencePush {
                sequence_type,
                value,
                source,
            } => Self::SequencePush {
                sequence_type,
                value,
                source,
            },
            FrozenExpressionOperation33::SequencePop {
                sequence_type,
                result_type,
                source,
            } => Self::SequencePop {
                sequence_type,
                result_type,
                source,
            },
            FrozenExpressionOperation33::BorrowOwnedItem {
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
            FrozenExpressionOperation33::BorrowCall {
                call,
                binding,
                body,
            } => Self::BorrowCall {
                call,
                binding,
                body,
            },
            FrozenExpressionOperation33::SequenceGet {
                sequence_type,
                source,
                index,
            } => Self::SequenceGet {
                sequence_type,
                source,
                index,
            },
            FrozenExpressionOperation33::SequenceReplace {
                sequence_type,
                result_type,
                index,
                value,
                source,
            } => Self::SequenceReplace {
                sequence_type,
                result_type,
                index,
                value,
                source,
            },
            FrozenExpressionOperation33::PackOwnedRecord {
                record_type,
                fields,
            } => Self::PackOwnedRecord {
                record_type,
                fields,
            },
            FrozenExpressionOperation33::UnpackOwnedRecord {
                record_type,
                source,
                fields,
                body,
            } => Self::UnpackOwnedRecord {
                record_type,
                source,
                fields,
                body,
            },
            FrozenExpressionOperation33::BorrowOwnedRecordField {
                record_type,
                source,
                field,
                binding,
                body,
            } => Self::BorrowOwnedRecordField {
                record_type,
                source,
                field,
                binding,
                body,
            },
        }
    }
}

impl TryFrom<ExpressionOperation> for FrozenExpressionOperation33 {
    type Error = Diagnostic;
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
                read_scope_arguments,
            } => {
                require_empty_read_scope(read_scope_arguments.is_empty())?;
                Self::Call {
                    requirement_arguments,
                    effect_arguments,
                    function,
                    type_arguments,
                    arguments,
                }
            }
            ExpressionOperation::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                read_scope_arguments,
            } => {
                require_empty_read_scope(read_scope_arguments.is_empty())?;
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
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                implementations,
                arguments,
                read_scope_arguments,
            } => {
                require_empty_read_scope(read_scope_arguments.is_empty())?;
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
                witness,
                contract,
                method,
                arguments,
                read_scope_arguments,
            } => {
                require_empty_read_scope(read_scope_arguments.is_empty())?;
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
            ExpressionOperation::SequenceGet {
                sequence_type,
                source,
                index,
            } => Self::SequenceGet {
                sequence_type,
                source,
                index,
            },
            ExpressionOperation::SequenceReplace {
                sequence_type,
                result_type,
                index,
                value,
                source,
            } => Self::SequenceReplace {
                sequence_type,
                result_type,
                index,
                value,
                source,
            },
            ExpressionOperation::PackOwnedRecord {
                record_type,
                fields,
            } => Self::PackOwnedRecord {
                record_type,
                fields,
            },
            ExpressionOperation::UnpackOwnedRecord {
                record_type,
                source,
                fields,
                body,
            } => Self::UnpackOwnedRecord {
                record_type,
                source,
                fields,
                body,
            },
            ExpressionOperation::BorrowOwnedRecordField {
                record_type,
                source,
                field,
                binding,
                body,
            } => Self::BorrowOwnedRecordField {
                record_type,
                source,
                field,
                binding,
                body,
            },
            _ => return Err(read_view_extension()),
        })
    }
}

impl From<FrozenTypeForm10> for TypeForm {
    fn from(value: FrozenTypeForm10) -> Self {
        match value {
            FrozenTypeForm10::Unit => Self::Unit,
            FrozenTypeForm10::Bool => Self::Bool,
            FrozenTypeForm10::I64 => Self::I64,
            FrozenTypeForm10::Bytes => Self::Bytes,
            FrozenTypeForm10::Text => Self::Text,
            FrozenTypeForm10::StaticText => Self::StaticText,
            FrozenTypeForm10::Secret => Self::Secret,
            FrozenTypeForm10::TypeParameter { parameter } => Self::TypeParameter { parameter },
            FrozenTypeForm10::Named { declaration } => Self::Named { declaration },
            FrozenTypeForm10::CapabilityResource { interface } => {
                Self::CapabilityResource { interface }
            }
            FrozenTypeForm10::StructuralRecord { fields } => Self::StructuralRecord { fields },
            FrozenTypeForm10::List { item } => Self::List { item },
            FrozenTypeForm10::Map { key, value } => Self::Map { key, value },
            FrozenTypeForm10::Option { item } => Self::Option { item },
            FrozenTypeForm10::Result { ok, error } => Self::Result { ok, error },
            FrozenTypeForm10::Stream { item } => Self::Stream { item },
            FrozenTypeForm10::Function { parameters, result } => {
                Self::Function { parameters, result }
            }
            FrozenTypeForm10::Applied {
                declaration,
                arguments,
            } => Self::Applied {
                declaration,
                arguments,
            },
            FrozenTypeForm10::TaskFunction {
                parameters,
                result,
                effect,
            } => Self::TaskFunction {
                parameters,
                result,
                effect,
            },
            FrozenTypeForm10::F64 => Self::F64,
            FrozenTypeForm10::ByteBuffer => Self::ByteBuffer,
            FrozenTypeForm10::OwnedI64Cell => Self::OwnedI64Cell,
            FrozenTypeForm10::OwnedProduct { fields } => Self::OwnedProduct { fields },
            FrozenTypeForm10::OwnedChoice { cases } => Self::OwnedChoice { cases },
            FrozenTypeForm10::OwnedSequence { item } => Self::OwnedSequence { item },
            FrozenTypeForm10::AssociatedContractMember { contract, member } => {
                Self::AssociatedContractMember { contract, member }
            }
            FrozenTypeForm10::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            } => Self::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            },
        }
    }
}

impl TryFrom<TypeForm> for FrozenTypeForm10 {
    type Error = Diagnostic;
    fn try_from(value: TypeForm) -> Result<Self, Self::Error> {
        Ok(match value {
            TypeForm::Unit => Self::Unit,
            TypeForm::Bool => Self::Bool,
            TypeForm::I64 => Self::I64,
            TypeForm::Bytes => Self::Bytes,
            TypeForm::Text => Self::Text,
            TypeForm::StaticText => Self::StaticText,
            TypeForm::Secret => Self::Secret,
            TypeForm::TypeParameter { parameter } => Self::TypeParameter { parameter },
            TypeForm::Named { declaration } => Self::Named { declaration },
            TypeForm::CapabilityResource { interface } => Self::CapabilityResource { interface },
            TypeForm::StructuralRecord { fields } => Self::StructuralRecord { fields },
            TypeForm::List { item } => Self::List { item },
            TypeForm::Map { key, value } => Self::Map { key, value },
            TypeForm::Option { item } => Self::Option { item },
            TypeForm::Result { ok, error } => Self::Result { ok, error },
            TypeForm::Stream { item } => Self::Stream { item },
            TypeForm::Function { parameters, result } => Self::Function { parameters, result },
            TypeForm::Applied {
                declaration,
                arguments,
            } => Self::Applied {
                declaration,
                arguments,
            },
            TypeForm::TaskFunction {
                parameters,
                result,
                effect,
            } => Self::TaskFunction {
                parameters,
                result,
                effect,
            },
            TypeForm::F64 => Self::F64,
            TypeForm::ByteBuffer => Self::ByteBuffer,
            TypeForm::OwnedI64Cell => Self::OwnedI64Cell,
            TypeForm::OwnedProduct { fields } => Self::OwnedProduct { fields },
            TypeForm::OwnedChoice { cases } => Self::OwnedChoice { cases },
            TypeForm::OwnedSequence { item } => Self::OwnedSequence { item },
            TypeForm::AssociatedContractMember { contract, member } => {
                Self::AssociatedContractMember { contract, member }
            }
            TypeForm::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            } => Self::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            },
            _ => return Err(read_view_extension()),
        })
    }
}

impl From<FrozenDeclarationRecord33> for DeclarationRecord {
    fn from(value: FrozenDeclarationRecord33) -> Self {
        Self {
            header: value.header,
            module: value.module,
            name: value.name,
            visibility: value.visibility,
            payload: value.payload.into(),
        }
    }
}

impl TryFrom<DeclarationRecord> for FrozenDeclarationRecord33 {
    type Error = Diagnostic;
    fn try_from(value: DeclarationRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            header: value.header,
            module: value.module,
            name: value.name,
            visibility: value.visibility,
            payload: value.payload.try_into()?,
        })
    }
}

impl From<FrozenBindingRecord33> for BindingRecord {
    fn from(value: FrozenBindingRecord33) -> Self {
        Self {
            header: value.header,
            name: value.name,
            kind: value.kind.into(),
            value: value.value,
            declared_type: value.declared_type,
        }
    }
}

impl TryFrom<BindingRecord> for FrozenBindingRecord33 {
    type Error = Diagnostic;
    fn try_from(value: BindingRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            header: value.header,
            name: value.name,
            kind: value.kind.try_into()?,
            value: value.value,
            declared_type: value.declared_type,
        })
    }
}

impl From<FrozenExpressionRecord33> for ExpressionRecord {
    fn from(value: FrozenExpressionRecord33) -> Self {
        Self {
            contract_version: value.contract_version,
            id: value.id,
            operation: value.operation.into(),
        }
    }
}

impl TryFrom<ExpressionRecord> for FrozenExpressionRecord33 {
    type Error = Diagnostic;
    fn try_from(value: ExpressionRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            contract_version: value.contract_version,
            id: value.id,
            operation: value.operation.try_into()?,
        })
    }
}

impl From<FrozenTypeObject10> for TypeObject {
    fn from(value: FrozenTypeObject10) -> Self {
        Self {
            contract_version: value.contract_version,
            form: value.form.into(),
        }
    }
}

impl TryFrom<TypeObject> for FrozenTypeObject10 {
    type Error = Diagnostic;
    fn try_from(value: TypeObject) -> Result<Self, Self::Error> {
        if value.contract_version != 10 {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "kernel_type_generation",
                "base type encoding requires the exact TypeObject 10 profile",
            ));
        }
        value.validate_local()?;
        Ok(Self {
            contract_version: value.contract_version,
            form: value.form.try_into()?,
        })
    }
}

impl From<FrozenFunctionDeclaration33> for FunctionDeclaration {
    fn from(value: FrozenFunctionDeclaration33) -> Self {
        Self {
            implementation_parameters: value.implementation_parameters,
            requirement_parameters: value.requirement_parameters,
            effect_parameters: value.effect_parameters,
            type_parameters: value.type_parameters,
            parameters: value.parameters,
            result: legacy_result(value.result, value.result_borrow),
            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
            effect: value.effect,
            body: value.body,
        }
    }
}

impl TryFrom<FunctionDeclaration> for FrozenFunctionDeclaration33 {
    type Error = Diagnostic;
    fn try_from(value: FunctionDeclaration) -> Result<Self, Self::Error> {
        require_empty_read_scope(
            value.read_scope_parameters.is_empty() && value.read_scope_sources.is_empty(),
        )?;
        let (result, result_borrow) = split_result(value.result)?;
        Ok(Self {
            implementation_parameters: value.implementation_parameters,
            requirement_parameters: value.requirement_parameters,
            effect_parameters: value.effect_parameters,
            type_parameters: value.type_parameters,
            parameters: value.parameters,
            result,
            result_borrow,
            effect: value.effect,
            body: value.body,
        })
    }
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct OwnedContract33 {
    pub self_parameter: TypeParameterId,
    pub type_parameters: Vec<TypeParameterId>,
    pub methods: Vec<OwnedMethod33>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct OwnedMethod33 {
    pub id: MethodId,
    pub name: Name,
    pub parameters: Vec<OwnedMethodParameter>,
    pub result: TypeObjectDigest,
    pub result_borrow: Option<u32>,
    pub effect: FunctionEffect,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct OwnedImplementation33 {
    pub type_parameters: Vec<TypeParameterId>,
    pub implementation_parameters: Vec<ImplementationParameter>,
    pub contract: DeclarationReference,
    pub self_type: TypeObjectDigest,
    pub type_arguments: Vec<TypeObjectDigest>,
    pub methods: Vec<OwnedMethodImplementation33>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct OwnedMethodImplementation33 {
    pub method: MethodId,
    pub function: DeclarationReference,
    pub type_arguments: Vec<TypeObjectDigest>,
    pub implementations: Vec<ImplementationOperand>,
}

impl OwnedContract33 {
    pub(crate) fn fields(value: &OwnedContract) -> Result<Self, Diagnostic> {
        Ok(Self {
            self_parameter: value.self_parameter,
            type_parameters: value.type_parameters.clone(),
            methods: value
                .methods
                .iter()
                .cloned()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
    pub(crate) fn into_logical(self, associated_types: Vec<AssociatedTypeMember>) -> OwnedContract {
        OwnedContract {
            self_parameter: self.self_parameter,
            type_parameters: self.type_parameters,
            methods: self.methods.into_iter().map(Into::into).collect(),
            associated_types,
        }
    }
}

impl From<OwnedContract33> for OwnedContract {
    fn from(value: OwnedContract33) -> Self {
        value.into_logical(Vec::new())
    }
}

impl TryFrom<OwnedContract> for OwnedContract33 {
    type Error = Diagnostic;
    fn try_from(value: OwnedContract) -> Result<Self, Self::Error> {
        super::owned_associated_wire::require_base_inventory(value.associated_types.is_empty())?;
        Self::fields(&value)
    }
}

impl From<OwnedMethod33> for OwnedMethod {
    fn from(value: OwnedMethod33) -> Self {
        Self {
            id: value.id,
            name: value.name,
            parameters: value.parameters,
            result: legacy_result(value.result, value.result_borrow),
            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
            effect: value.effect,
        }
    }
}

impl TryFrom<OwnedMethod> for OwnedMethod33 {
    type Error = Diagnostic;
    fn try_from(value: OwnedMethod) -> Result<Self, Self::Error> {
        require_empty_read_scope(
            value.read_scope_parameters.is_empty() && value.read_scope_sources.is_empty(),
        )?;
        let (result, result_borrow) = split_result(value.result)?;
        Ok(Self {
            id: value.id,
            name: value.name,
            parameters: value.parameters,
            result,
            result_borrow,
            effect: value.effect,
        })
    }
}

impl OwnedImplementation33 {
    pub(crate) fn fields(value: &OwnedImplementation) -> Result<Self, Diagnostic> {
        Ok(Self {
            type_parameters: value.type_parameters.clone(),
            implementation_parameters: value.implementation_parameters.clone(),
            contract: value.contract,
            self_type: value.self_type,
            type_arguments: value.type_arguments.clone(),
            methods: value
                .methods
                .iter()
                .cloned()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
    pub(crate) fn into_logical(
        self,
        associated_types: Vec<AssociatedTypeBinding>,
    ) -> OwnedImplementation {
        OwnedImplementation {
            type_parameters: self.type_parameters,
            implementation_parameters: self.implementation_parameters,
            contract: self.contract,
            self_type: self.self_type,
            type_arguments: self.type_arguments,
            methods: self.methods.into_iter().map(Into::into).collect(),
            associated_types,
        }
    }
}

impl From<OwnedImplementation33> for OwnedImplementation {
    fn from(value: OwnedImplementation33) -> Self {
        value.into_logical(Vec::new())
    }
}

impl TryFrom<OwnedImplementation> for OwnedImplementation33 {
    type Error = Diagnostic;
    fn try_from(value: OwnedImplementation) -> Result<Self, Self::Error> {
        super::owned_associated_wire::require_base_inventory(value.associated_types.is_empty())?;
        Self::fields(&value)
    }
}

impl From<OwnedMethodImplementation33> for OwnedMethodImplementation {
    fn from(value: OwnedMethodImplementation33) -> Self {
        Self {
            method: value.method,
            function: value.function,
            type_arguments: value.type_arguments,
            implementations: value.implementations,
            read_scope_arguments: Vec::new(),
        }
    }
}

impl TryFrom<OwnedMethodImplementation> for OwnedMethodImplementation33 {
    type Error = Diagnostic;
    fn try_from(value: OwnedMethodImplementation) -> Result<Self, Self::Error> {
        require_empty_read_scope(value.read_scope_arguments.is_empty())?;
        Ok(Self {
            method: value.method,
            function: value.function,
            type_arguments: value.type_arguments,
            implementations: value.implementations,
        })
    }
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct AssociatedTypeMember33 {
    pub id: AssociatedTypeMemberId,
    pub name: Name,
    pub bound: AssociatedTypeBound33,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssociatedTypeBound33 {
    Owned,
}

impl Encode for AssociatedTypeBound33 {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        1_u8.encode(encoder)
    }
}

impl<C> Decode<C> for AssociatedTypeBound33 {
    fn decode<D: bincode::de::Decoder<Context = C>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        match u8::decode(decoder)? {
            1 => Ok(Self::Owned),
            _ => Err(bincode::error::DecodeError::Other(
                "unknown associated owned type bound",
            )),
        }
    }
}
bincode::impl_borrow_decode!(AssociatedTypeBound33);

impl From<AssociatedTypeMember33> for AssociatedTypeMember {
    fn from(value: AssociatedTypeMember33) -> Self {
        Self {
            id: value.id,
            name: value.name,
            bound: AssociatedTypeBound::Owned,
        }
    }
}

impl TryFrom<AssociatedTypeMember> for AssociatedTypeMember33 {
    type Error = Diagnostic;
    fn try_from(value: AssociatedTypeMember) -> Result<Self, Self::Error> {
        match value.bound {
            AssociatedTypeBound::Owned => Ok(Self {
                id: value.id,
                name: value.name,
                bound: AssociatedTypeBound33::Owned,
            }),
            AssociatedTypeBound::ReadViewFamily { .. } => Err(read_view_extension()),
        }
    }
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct AssociatedTypeBinding33 {
    pub member: AssociatedTypeMemberId,
    pub ty: TypeObjectDigest,
}

impl From<AssociatedTypeBinding33> for AssociatedTypeBinding {
    fn from(value: AssociatedTypeBinding33) -> Self {
        Self {
            member: value.member,
            value: AssociatedTypeBindingValue::Owned { ty: value.ty },
        }
    }
}

impl TryFrom<AssociatedTypeBinding> for AssociatedTypeBinding33 {
    type Error = Diagnostic;
    fn try_from(value: AssociatedTypeBinding) -> Result<Self, Self::Error> {
        match value.value {
            AssociatedTypeBindingValue::Owned { ty } => Ok(Self {
                member: value.member,
                ty,
            }),
            AssociatedTypeBindingValue::ReadViewFamily { .. } => Err(read_view_extension()),
        }
    }
}
