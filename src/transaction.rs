use crate::diff;
use crate::error::{ErrorCode, LkError, MAX_DRAFT_PATH_BYTES, Result};
use crate::graph::{Snapshot, Workspace, require_kind};
use crate::ids::{
    ChangeDigest, DraftSymbol, IdempotencyKey, NodeId, Revision, SnapshotHash, WorkspaceId,
};
use crate::query;
use crate::schema::{
    ByteString, MAXIMUM_BYTE_LITERAL_BYTES, MAXIMUM_TEXT_LITERAL_BYTES,
    MAXIMUM_TRANSACTION_BYTE_LITERAL_BYTES, MAXIMUM_TRANSACTION_TEXT_LITERAL_BYTES, MatchArm,
    MatchArmOperationDraft, Node, NodeKind, OperationCode, OperationDraft, OperationKind,
    ProductFieldValue, ProductFieldValueDraft, RegionArity, SemanticType, TextString, TypeDraft,
    ValueDraft, ValueRef,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum NodeTarget {
    Existing(NodeId),
    Draft(DraftSymbol),
}

pub const MAX_RETURNED_BINDINGS: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionMode {
    Commit,
    ValidateOnly,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Transaction {
    pub workspace: WorkspaceId,
    pub base_revision: Revision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    pub mode: TransactionMode,
    pub operations: Vec<TransactionOp>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionResponseSpec {
    pub return_symbols: Vec<DraftSymbol>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyTransactionRequest {
    pub transaction: Transaction,
    pub response: TransactionResponseSpec,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionOpCode {
    CreatePackage,
    CreateModule,
    CreateFunction,
    DefineFunctionBody,
    ReplaceFunctionBody,
    InsertExpression,
    SetEntryFunction,
    RenameNode,
    ReplaceOperation,
    ReplaceOperand,
    DeleteOwnedSubtree,
    RefineHole,
    CreateProductType,
    AddProductField,
    CreateSumType,
    CreateSequenceType,
    CreateBuildTarget,
    ReplaceBuildTarget,
    AddReleaseTargetExport,
    SetReleaseTargetExport,
    SetApplicationQueryBoundary,
    AddApplicationTargetTest,
}
impl TransactionOpCode {
    pub const ALL: [Self; 22] = [
        Self::CreatePackage,
        Self::CreateModule,
        Self::CreateFunction,
        Self::DefineFunctionBody,
        Self::ReplaceFunctionBody,
        Self::InsertExpression,
        Self::SetEntryFunction,
        Self::RenameNode,
        Self::ReplaceOperation,
        Self::ReplaceOperand,
        Self::DeleteOwnedSubtree,
        Self::RefineHole,
        Self::CreateProductType,
        Self::AddProductField,
        Self::CreateSumType,
        Self::CreateSequenceType,
        Self::CreateBuildTarget,
        Self::ReplaceBuildTarget,
        Self::AddReleaseTargetExport,
        Self::SetReleaseTargetExport,
        Self::SetApplicationQueryBoundary,
        Self::AddApplicationTargetTest,
    ];
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::CreatePackage => "create_package",
            Self::CreateModule => "create_module",
            Self::CreateFunction => "create_function",
            Self::DefineFunctionBody => "define_function_body",
            Self::ReplaceFunctionBody => "replace_function_body",
            Self::InsertExpression => "insert_expression",
            Self::SetEntryFunction => "set_entry_function",
            Self::RenameNode => "rename_node",
            Self::ReplaceOperation => "replace_operation",
            Self::ReplaceOperand => "replace_operand",
            Self::RefineHole => "refine_hole",
            Self::DeleteOwnedSubtree => "delete_owned_subtree",
            Self::CreateProductType => "create_product_type",
            Self::AddProductField => "add_product_field",
            Self::CreateSumType => "create_sum_type",
            Self::CreateSequenceType => "create_sequence_type",
            Self::CreateBuildTarget => "create_build_target",
            Self::ReplaceBuildTarget => "replace_build_target",
            Self::AddReleaseTargetExport => "add_release_target_export",
            Self::SetReleaseTargetExport => "set_release_target_export",
            Self::SetApplicationQueryBoundary => "set_application_query_boundary",
            Self::AddApplicationTargetTest => "add_application_target_test",
        }
    }
}

pub const MAX_STRUCTURED_DRAFT_DEPTH: usize = 16;
pub const MAX_STRUCTURED_DRAFT_ITEMS: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionDraftCode {
    ConstUnit,
    ConstBool,
    ConstI64,
    AddI64,
    LtI64,
    Call,
    Hole,
    If,
    ForI64,
    ConstructProduct,
    ProjectField,
    ConstructVariant,
    MatchSum,
    ConstBytes,
    BytesLen,
    BytesAt,
    BytesSlice,
    BytesEqual,
    BytesConcat,
    ConstText,
    EqualI64,
    NotBool,
    AndBool,
    OrBool,
    TextLen,
    TextEqual,
    TextConcat,
    SequenceEmpty,
    SequenceLen,
    SequenceGet,
    SequenceAppend,
    SequenceReplace,
    SequenceSlice,
    SequenceConcat,
}
impl ExpressionDraftCode {
    pub const ALL: [Self; 34] = [
        Self::ConstUnit,
        Self::ConstBool,
        Self::ConstI64,
        Self::AddI64,
        Self::LtI64,
        Self::Call,
        Self::Hole,
        Self::If,
        Self::ForI64,
        Self::ConstructProduct,
        Self::ProjectField,
        Self::ConstructVariant,
        Self::MatchSum,
        Self::ConstBytes,
        Self::BytesLen,
        Self::BytesAt,
        Self::BytesSlice,
        Self::BytesEqual,
        Self::BytesConcat,
        Self::ConstText,
        Self::EqualI64,
        Self::NotBool,
        Self::AndBool,
        Self::OrBool,
        Self::TextLen,
        Self::TextEqual,
        Self::TextConcat,
        Self::SequenceEmpty,
        Self::SequenceLen,
        Self::SequenceGet,
        Self::SequenceAppend,
        Self::SequenceReplace,
        Self::SequenceSlice,
        Self::SequenceConcat,
    ];
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::ConstUnit => "const_unit",
            Self::ConstBool => "const_bool",
            Self::ConstI64 => "const_i64",
            Self::AddI64 => "add_i64",
            Self::LtI64 => "lt_i64",
            Self::Call => "call",
            Self::Hole => "hole",
            Self::If => "if",
            Self::ForI64 => "for_i64",
            Self::ConstructProduct => "construct_product",
            Self::ProjectField => "project_field",
            Self::ConstructVariant => "construct_variant",
            Self::MatchSum => "match_sum",
            Self::ConstBytes => "const_bytes",
            Self::BytesLen => "bytes_len",
            Self::BytesAt => "bytes_at",
            Self::BytesSlice => "bytes_slice",
            Self::BytesEqual => "bytes_equal",
            Self::BytesConcat => "bytes_concat",
            Self::ConstText => "const_text",
            Self::EqualI64 => "equal_i64",
            Self::NotBool => "not_bool",
            Self::AndBool => "and_bool",
            Self::OrBool => "or_bool",
            Self::TextLen => "text_len",
            Self::TextEqual => "text_equal",
            Self::TextConcat => "text_concat",
            Self::SequenceEmpty => "sequence_empty",
            Self::SequenceLen => "sequence_len",
            Self::SequenceGet => "sequence_get",
            Self::SequenceAppend => "sequence_append",
            Self::SequenceReplace => "sequence_replace",
            Self::SequenceSlice => "sequence_slice",
            Self::SequenceConcat => "sequence_concat",
        }
    }

    pub const fn operation_code(self) -> OperationCode {
        match self {
            Self::ConstUnit => OperationCode::ConstUnit,
            Self::ConstBool => OperationCode::ConstBool,
            Self::ConstI64 => OperationCode::ConstI64,
            Self::AddI64 => OperationCode::AddI64,
            Self::LtI64 => OperationCode::LtI64,
            Self::Call => OperationCode::Call,
            Self::Hole => OperationCode::Hole,
            Self::If => OperationCode::If,
            Self::ForI64 => OperationCode::ForI64,
            Self::ConstructProduct => OperationCode::ConstructProduct,
            Self::ProjectField => OperationCode::ProjectField,
            Self::ConstructVariant => OperationCode::ConstructVariant,
            Self::MatchSum => OperationCode::MatchSum,
            Self::ConstBytes => OperationCode::ConstBytes,
            Self::BytesLen => OperationCode::BytesLen,
            Self::BytesAt => OperationCode::BytesAt,
            Self::BytesSlice => OperationCode::BytesSlice,
            Self::BytesEqual => OperationCode::BytesEqual,
            Self::BytesConcat => OperationCode::BytesConcat,
            Self::ConstText => OperationCode::ConstText,
            Self::EqualI64 => OperationCode::EqualI64,
            Self::NotBool => OperationCode::NotBool,
            Self::AndBool => OperationCode::AndBool,
            Self::OrBool => OperationCode::OrBool,
            Self::TextLen => OperationCode::TextLen,
            Self::TextEqual => OperationCode::TextEqual,
            Self::TextConcat => OperationCode::TextConcat,
            Self::SequenceEmpty => OperationCode::SequenceEmpty,
            Self::SequenceLen => OperationCode::SequenceLen,
            Self::SequenceGet => OperationCode::SequenceGet,
            Self::SequenceAppend => OperationCode::SequenceAppend,
            Self::SequenceReplace => OperationCode::SequenceReplace,
            Self::SequenceSlice => OperationCode::SequenceSlice,
            Self::SequenceConcat => OperationCode::SequenceConcat,
        }
    }

    pub fn is_inline_eligible(self) -> bool {
        let descriptor = self.operation_code().descriptor();
        descriptor.complete
            && !descriptor.terminator
            && descriptor.results.len() == 1
            && matches!(descriptor.region_arity, RegionArity::Fixed(0))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueDraftCode {
    FunctionParameter,
    OperationResult,
    BlockArgument,
    InlineExpression,
}
impl ValueDraftCode {
    pub const ALL: [Self; 4] = [
        Self::FunctionParameter,
        Self::OperationResult,
        Self::BlockArgument,
        Self::InlineExpression,
    ];
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::FunctionParameter => "function_parameter",
            Self::OperationResult => "operation_result",
            Self::BlockArgument => "block_argument",
            Self::InlineExpression => "inline_expression",
        }
    }
}

impl ValueDraft {
    pub const fn code(&self) -> ValueDraftCode {
        match self {
            Self::FunctionParameter(_) => ValueDraftCode::FunctionParameter,
            Self::OperationResult { .. } => ValueDraftCode::OperationResult,
            Self::BlockArgument(_) => ValueDraftCode::BlockArgument,
            Self::InlineExpression(_) => ValueDraftCode::InlineExpression,
        }
    }
}

impl ExpressionKindDraft {
    pub const fn code(&self) -> ExpressionDraftCode {
        match self {
            Self::ConstUnit => ExpressionDraftCode::ConstUnit,
            Self::ConstBool(_) => ExpressionDraftCode::ConstBool,
            Self::ConstI64(_) => ExpressionDraftCode::ConstI64,
            Self::AddI64 { .. } => ExpressionDraftCode::AddI64,
            Self::LtI64 { .. } => ExpressionDraftCode::LtI64,
            Self::Call { .. } => ExpressionDraftCode::Call,
            Self::Hole { .. } => ExpressionDraftCode::Hole,
            Self::If { .. } => ExpressionDraftCode::If,
            Self::ForI64 { .. } => ExpressionDraftCode::ForI64,
            Self::ConstructProduct { .. } => ExpressionDraftCode::ConstructProduct,
            Self::ProjectField { .. } => ExpressionDraftCode::ProjectField,
            Self::ConstructVariant { .. } => ExpressionDraftCode::ConstructVariant,
            Self::MatchSum { .. } => ExpressionDraftCode::MatchSum,
            Self::ConstBytes(_) => ExpressionDraftCode::ConstBytes,
            Self::ConstText(_) => ExpressionDraftCode::ConstText,
            Self::EqualI64 { .. } => ExpressionDraftCode::EqualI64,
            Self::NotBool { .. } => ExpressionDraftCode::NotBool,
            Self::AndBool { .. } => ExpressionDraftCode::AndBool,
            Self::OrBool { .. } => ExpressionDraftCode::OrBool,
            Self::TextLen { .. } => ExpressionDraftCode::TextLen,
            Self::TextEqual { .. } => ExpressionDraftCode::TextEqual,
            Self::TextConcat { .. } => ExpressionDraftCode::TextConcat,
            Self::SequenceEmpty { .. } => ExpressionDraftCode::SequenceEmpty,
            Self::SequenceLen { .. } => ExpressionDraftCode::SequenceLen,
            Self::SequenceGet { .. } => ExpressionDraftCode::SequenceGet,
            Self::SequenceAppend { .. } => ExpressionDraftCode::SequenceAppend,
            Self::SequenceReplace { .. } => ExpressionDraftCode::SequenceReplace,
            Self::SequenceSlice { .. } => ExpressionDraftCode::SequenceSlice,
            Self::SequenceConcat { .. } => ExpressionDraftCode::SequenceConcat,
            Self::BytesLen { .. } => ExpressionDraftCode::BytesLen,
            Self::BytesAt { .. } => ExpressionDraftCode::BytesAt,
            Self::BytesSlice { .. } => ExpressionDraftCode::BytesSlice,
            Self::BytesEqual { .. } => ExpressionDraftCode::BytesEqual,
            Self::BytesConcat { .. } => ExpressionDraftCode::BytesConcat,
        }
    }

    pub const fn operation_code(&self) -> OperationCode {
        self.code().operation_code()
    }

    pub fn is_inline_eligible(&self) -> bool {
        self.code().is_inline_eligible()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionParameterDraft {
    pub symbol: DraftSymbol,
    pub name: String,
    pub ty: TypeDraft,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProductFieldDraft {
    pub symbol: DraftSymbol,
    pub name: String,
    pub ty: TypeDraft,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SumVariantDraft {
    pub symbol: DraftSymbol,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<TypeDraft>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionBodyDraft {
    pub operations: Vec<ExpressionDraft>,
    pub return_value: ValueDraft,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct YieldingBodyDraft {
    pub operations: Vec<ExpressionDraft>,
    pub yield_value: ValueDraft,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MatchArmDraft {
    pub variant: NodeTarget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_symbol: Option<DraftSymbol>,
    pub body: YieldingBodyDraft,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpressionDraft {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<DraftSymbol>,
    pub operation: ExpressionKindDraft,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ExpressionKindDraft {
    ConstUnit,
    ConstBool(bool),
    ConstI64(i64),
    ConstBytes(ByteString),
    ConstText(TextString),
    AddI64 {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    LtI64 {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    EqualI64 {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    NotBool {
        value: ValueDraft,
    },
    AndBool {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    OrBool {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    BytesLen {
        value: ValueDraft,
    },
    BytesAt {
        value: ValueDraft,
        index: ValueDraft,
    },
    BytesSlice {
        value: ValueDraft,
        start: ValueDraft,
        length: ValueDraft,
    },
    BytesEqual {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    BytesConcat {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    TextLen {
        value: ValueDraft,
    },
    TextEqual {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    TextConcat {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    SequenceEmpty {
        sequence: NodeTarget,
    },
    SequenceLen {
        sequence: NodeTarget,
        value: ValueDraft,
    },
    SequenceGet {
        sequence: NodeTarget,
        value: ValueDraft,
        index: ValueDraft,
    },
    SequenceAppend {
        sequence: NodeTarget,
        value: ValueDraft,
        element: ValueDraft,
    },
    SequenceReplace {
        sequence: NodeTarget,
        value: ValueDraft,
        index: ValueDraft,
        element: ValueDraft,
    },
    SequenceSlice {
        sequence: NodeTarget,
        value: ValueDraft,
        start: ValueDraft,
        end_exclusive: ValueDraft,
    },
    SequenceConcat {
        sequence: NodeTarget,
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    Call {
        function: NodeTarget,
        arguments: Vec<ValueDraft>,
    },
    Hole {
        expected: TypeDraft,
    },
    If {
        condition: ValueDraft,
        result: TypeDraft,
        then_body: YieldingBodyDraft,
        else_body: YieldingBodyDraft,
    },
    ForI64 {
        start: ValueDraft,
        end_exclusive: ValueDraft,
        step: i64,
        initial: ValueDraft,
        carried: TypeDraft,
        index_symbol: DraftSymbol,
        carried_symbol: DraftSymbol,
        body: YieldingBodyDraft,
    },
    ConstructProduct {
        product: NodeTarget,
        fields: Vec<ProductFieldValueDraft>,
    },
    ProjectField {
        value: ValueDraft,
        field: NodeTarget,
    },
    ConstructVariant {
        variant: NodeTarget,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload: Option<ValueDraft>,
    },
    MatchSum {
        scrutinee: ValueDraft,
        result: TypeDraft,
        arms: Vec<MatchArmDraft>,
    },
}

// This is the one bounded public DTO authority; boxing selected variants would complicate direct
// typed construction without changing wire bytes.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TransactionOp {
    CreatePackage {
        symbol: DraftSymbol,
        name: String,
    },
    CreateModule {
        symbol: DraftSymbol,
        package: NodeTarget,
        name: String,
    },
    CreateProductType {
        symbol: DraftSymbol,
        module: NodeTarget,
        name: String,
        fields: Vec<ProductFieldDraft>,
    },
    AddProductField {
        symbol: DraftSymbol,
        product: NodeTarget,
        name: String,
        ty: TypeDraft,
    },
    CreateSumType {
        symbol: DraftSymbol,
        module: NodeTarget,
        name: String,
        variants: Vec<SumVariantDraft>,
    },
    CreateSequenceType {
        symbol: DraftSymbol,
        module: NodeTarget,
        name: String,
        element: TypeDraft,
    },
    CreateFunction {
        symbol: DraftSymbol,
        module: NodeTarget,
        name: String,
        parameters: Vec<FunctionParameterDraft>,
        result: TypeDraft,
        body: Option<FunctionBodyDraft>,
    },
    CreateBuildTarget {
        symbol: DraftSymbol,
        name: String,
        definition: crate::target::BuildTargetDefinition,
    },
    ReplaceBuildTarget {
        target: NodeId,
        definition: crate::target::BuildTargetDefinition,
    },
    AddReleaseTargetExport {
        target: NodeId,
        name: String,
        item: NodeId,
    },
    SetReleaseTargetExport {
        target: NodeId,
        name: String,
        item: NodeId,
    },
    SetApplicationQueryBoundary {
        target: NodeId,
        query_entry: crate::target::TargetItem,
        query: crate::target::TargetItem,
    },
    AddApplicationTargetTest {
        target: NodeId,
        case: crate::target::TargetApplicationTestCase,
    },
    DefineFunctionBody {
        function: NodeId,
        body: FunctionBodyDraft,
    },
    ReplaceFunctionBody {
        function: NodeId,
        body: FunctionBodyDraft,
    },
    InsertExpression {
        block: NodeId,
        before: Option<NodeId>,
        expression: ExpressionDraft,
    },
    SetEntryFunction {
        package: NodeTarget,
        function: NodeTarget,
    },
    RenameNode {
        node: NodeTarget,
        name: String,
    },
    ReplaceOperation {
        operation: NodeTarget,
        replacement: OperationDraft,
    },
    ReplaceOperand {
        operation: NodeTarget,
        index: u64,
        value: ValueDraft,
    },
    RefineHole {
        hole: NodeTarget,
        replacement: OperationDraft,
    },
    DeleteOwnedSubtree {
        root: NodeTarget,
    },
}

impl TransactionOp {
    pub const fn code(&self) -> TransactionOpCode {
        match self {
            Self::CreatePackage { .. } => TransactionOpCode::CreatePackage,
            Self::CreateModule { .. } => TransactionOpCode::CreateModule,
            Self::CreateProductType { .. } => TransactionOpCode::CreateProductType,
            Self::AddProductField { .. } => TransactionOpCode::AddProductField,
            Self::CreateSumType { .. } => TransactionOpCode::CreateSumType,
            Self::CreateSequenceType { .. } => TransactionOpCode::CreateSequenceType,
            Self::CreateFunction { .. } => TransactionOpCode::CreateFunction,
            Self::CreateBuildTarget { .. } => TransactionOpCode::CreateBuildTarget,
            Self::ReplaceBuildTarget { .. } => TransactionOpCode::ReplaceBuildTarget,
            Self::AddReleaseTargetExport { .. } => TransactionOpCode::AddReleaseTargetExport,
            Self::SetReleaseTargetExport { .. } => TransactionOpCode::SetReleaseTargetExport,
            Self::SetApplicationQueryBoundary { .. } => {
                TransactionOpCode::SetApplicationQueryBoundary
            }
            Self::AddApplicationTargetTest { .. } => TransactionOpCode::AddApplicationTargetTest,
            Self::DefineFunctionBody { .. } => TransactionOpCode::DefineFunctionBody,
            Self::ReplaceFunctionBody { .. } => TransactionOpCode::ReplaceFunctionBody,
            Self::InsertExpression { .. } => TransactionOpCode::InsertExpression,
            Self::SetEntryFunction { .. } => TransactionOpCode::SetEntryFunction,
            Self::RenameNode { .. } => TransactionOpCode::RenameNode,
            Self::ReplaceOperation { .. } => TransactionOpCode::ReplaceOperation,
            Self::ReplaceOperand { .. } => TransactionOpCode::ReplaceOperand,
            Self::RefineHole { .. } => TransactionOpCode::RefineHole,
            Self::DeleteOwnedSubtree { .. } => TransactionOpCode::DeleteOwnedSubtree,
        }
    }
    pub const fn created_symbol(&self) -> Option<DraftSymbol> {
        match self {
            Self::CreatePackage { symbol, .. }
            | Self::CreateModule { symbol, .. }
            | Self::CreateProductType { symbol, .. }
            | Self::AddProductField { symbol, .. }
            | Self::CreateSumType { symbol, .. }
            | Self::CreateSequenceType { symbol, .. }
            | Self::CreateFunction { symbol, .. }
            | Self::CreateBuildTarget { symbol, .. } => Some(*symbol),
            Self::InsertExpression { expression, .. } => expression.symbol,
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
enum CanonicalEdit {
    CreatePackage {
        symbol: DraftSymbol,
        name: String,
    },
    CreateBuildTarget {
        symbol: DraftSymbol,
        name: String,
        definition: crate::target::BuildTargetDefinition,
    },
    ReplaceBuildTarget {
        target: NodeId,
        definition: crate::target::BuildTargetDefinition,
    },
    AddReleaseTargetExport {
        target: NodeId,
        name: String,
        item: NodeId,
    },
    SetReleaseTargetExport {
        target: NodeId,
        name: String,
        item: NodeId,
    },
    SetApplicationQueryBoundary {
        target: NodeId,
        query_entry: crate::target::TargetItem,
        query: crate::target::TargetItem,
    },
    AddApplicationTargetTest {
        target: NodeId,
        case: crate::target::TargetApplicationTestCase,
    },
    CreateModule {
        symbol: DraftSymbol,
        package: NodeTarget,
        name: String,
    },
    CreateProductType {
        symbol: DraftSymbol,
        module: NodeTarget,
        name: String,
    },
    CreateProductField {
        symbol: DraftSymbol,
        product: NodeTarget,
        name: String,
        ty: TypeDraft,
    },
    CreateSumType {
        symbol: DraftSymbol,
        module: NodeTarget,
        name: String,
    },
    CreateSequenceType {
        symbol: DraftSymbol,
        module: NodeTarget,
        name: String,
        element: TypeDraft,
    },
    CreateSumVariant {
        symbol: DraftSymbol,
        sum: NodeTarget,
        name: String,
        payload: Option<TypeDraft>,
    },
    CreateFunction {
        symbol: DraftSymbol,
        module: NodeTarget,
        name: String,
        result: TypeDraft,
    },
    CreateParameter {
        symbol: DraftSymbol,
        function: NodeTarget,
        name: String,
        ty: TypeDraft,
    },
    CreateRegion {
        symbol: DraftSymbol,
        owner: NodeTarget,
    },
    CreateBlock {
        symbol: DraftSymbol,
        region: NodeTarget,
    },
    CreateBlockArgument {
        symbol: DraftSymbol,
        block: NodeTarget,
        ty: TypeDraft,
    },
    CreateMatchPayloadArgument {
        symbol: DraftSymbol,
        block: NodeTarget,
        variant: NodeTarget,
    },
    CreateOperation {
        symbol: DraftSymbol,
        block: NodeTarget,
        before: Option<NodeTarget>,
        operation: OperationDraft,
    },
    SetFunctionBody {
        function: NodeTarget,
        region: NodeTarget,
    },
    ClearFunctionBody {
        function: NodeTarget,
    },
    SetEntryFunction {
        package: NodeTarget,
        function: NodeTarget,
    },
    RenameNode {
        node: NodeTarget,
        name: String,
    },
    ReplaceOperation {
        operation: NodeTarget,
        replacement: OperationDraft,
    },
    ReplaceOperand {
        operation: NodeTarget,
        index: u64,
        value: ValueDraft,
    },
    RefineHole {
        hole: NodeTarget,
        replacement: OperationDraft,
    },
    DeleteOwnedSubtree {
        root: NodeTarget,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionReceipt {
    pub workspace: WorkspaceId,
    pub base_revision: Revision,
    pub revision: Revision,
    pub hash: SnapshotHash,
    pub published: bool,
    pub created_count: u64,
    pub returned_bindings: Vec<(DraftSymbol, NodeId)>,
    pub change_count: u64,
    pub change_digest: ChangeDigest,
    pub complete_before: bool,
    pub complete_after: bool,
    pub blocker_count_before: u64,
    pub blocker_count_after: u64,
}

#[derive(Debug)]
pub(crate) struct PreparedTransaction {
    pub snapshot: Arc<Snapshot>,
    pub receipt: TransactionReceipt,
}

struct ExpandedTransaction {
    edits: Vec<CanonicalEdit>,
    edit_sources: Vec<usize>,
    explicit_symbols: BTreeSet<DraftSymbol>,
    anonymous_paths: BTreeMap<DraftSymbol, String>,
    nominal_catalogue: StagedNominalCatalogue,
}

#[derive(Clone, Default)]
struct StagedNominalCatalogue {
    products: BTreeMap<NodeTarget, Vec<NodeTarget>>,
    sums: BTreeMap<NodeTarget, Vec<NodeTarget>>,
    field_owners: BTreeMap<NodeTarget, NodeTarget>,
    variants: BTreeMap<NodeTarget, (NodeTarget, Option<TypeDraft>)>,
}

impl StagedNominalCatalogue {
    fn build(base: &Snapshot, operations: &[TransactionOp]) -> Self {
        let mut catalogue = Self::default();
        for (id, node) in base.nodes() {
            match node {
                Node::ProductType { fields, .. } => {
                    let declaration = NodeTarget::Existing(id);
                    let members = fields
                        .iter()
                        .copied()
                        .map(NodeTarget::Existing)
                        .collect::<Vec<_>>();
                    for member in &members {
                        catalogue.field_owners.insert(*member, declaration);
                    }
                    catalogue.products.insert(declaration, members);
                }
                Node::SumType { variants, .. } => {
                    let declaration = NodeTarget::Existing(id);
                    let members = variants
                        .iter()
                        .copied()
                        .map(NodeTarget::Existing)
                        .collect::<Vec<_>>();
                    for member in &members {
                        let payload = match member {
                            NodeTarget::Existing(member) => match base.node(*member) {
                                Ok(Node::SumVariant { payload, .. }) => {
                                    payload.map(TypeDraft::from)
                                }
                                _ => None,
                            },
                            NodeTarget::Draft(_) => None,
                        };
                        catalogue.variants.insert(*member, (declaration, payload));
                    }
                    catalogue.sums.insert(declaration, members);
                }
                _ => {}
            }
        }
        for operation in operations {
            match operation {
                TransactionOp::CreateProductType { symbol, fields, .. } => {
                    let declaration = NodeTarget::Draft(*symbol);
                    let members = fields
                        .iter()
                        .map(|field| NodeTarget::Draft(field.symbol))
                        .collect::<Vec<_>>();
                    for member in &members {
                        catalogue.field_owners.insert(*member, declaration);
                    }
                    catalogue.products.insert(declaration, members);
                }
                TransactionOp::AddProductField {
                    symbol,
                    product,
                    ty: _,
                    ..
                } => {
                    let member = NodeTarget::Draft(*symbol);
                    catalogue.field_owners.insert(member, *product);
                    catalogue.products.entry(*product).or_default().push(member);
                }
                TransactionOp::CreateSumType {
                    symbol, variants, ..
                } => {
                    let declaration = NodeTarget::Draft(*symbol);
                    let members = variants
                        .iter()
                        .map(|variant| NodeTarget::Draft(variant.symbol))
                        .collect::<Vec<_>>();
                    for (member, variant) in members.iter().zip(variants) {
                        catalogue
                            .variants
                            .insert(*member, (declaration, variant.payload));
                    }
                    catalogue.sums.insert(declaration, members);
                }
                _ => {}
            }
        }
        catalogue
    }

    fn normalize_match_arms(
        &self,
        arms: Vec<MatchArmDraft>,
        source: usize,
    ) -> Result<Vec<MatchArmDraft>> {
        let first = arms.first().ok_or_else(|| {
            LkError::new(
                ErrorCode::InvalidOperand,
                "match_sum requires exhaustive arms",
            )
            .at_operation(source)
        })?;
        let (sum, _) = self.variants.get(&first.variant).ok_or_else(|| {
            LkError::new(
                ErrorCode::WrongKind,
                "match arm does not name a known sum variant",
            )
            .at_operation(source)
        })?;
        let declared = self.sums.get(sum).ok_or_else(|| {
            LkError::new(
                ErrorCode::WrongKind,
                "match variant owner is not a known sum declaration",
            )
            .at_operation(source)
        })?;
        let mut by_variant = BTreeMap::new();
        for arm in arms {
            let Some((owner, payload)) = self.variants.get(&arm.variant) else {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "match arm does not name a known sum variant",
                )
                .at_operation(source));
            };
            if owner != sum {
                return Err(LkError::new(
                    ErrorCode::OwnerMismatch,
                    "match arm variant belongs to another sum declaration",
                )
                .at_operation(source));
            }
            if payload.is_some() != arm.payload_symbol.is_some() {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "match payload symbol presence does not match the variant payload",
                )
                .at_operation(source));
            }
            let variant = arm.variant;
            if by_variant.insert(variant, arm).is_some() {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "match arm variant is duplicated",
                )
                .at_operation(source));
            }
        }
        let normalized = declared
            .iter()
            .map(|variant| {
                by_variant.remove(variant).ok_or_else(|| {
                    LkError::new(
                        ErrorCode::InvalidOperand,
                        "match is missing a declared variant arm",
                    )
                    .at_operation(source)
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if !by_variant.is_empty() {
            return Err(LkError::new(
                ErrorCode::OwnerMismatch,
                "match contains a foreign variant arm",
            )
            .at_operation(source));
        }
        Ok(normalized)
    }

    fn normalize_product_fields(
        &self,
        product: NodeTarget,
        fields: Vec<ProductFieldValueDraft>,
        source: usize,
    ) -> Result<Vec<ProductFieldValueDraft>> {
        let declared = self.products.get(&product).ok_or_else(|| {
            LkError::new(
                ErrorCode::WrongKind,
                "product construction must name a known product declaration",
            )
            .at_operation(source)
        })?;
        let mut by_field = BTreeMap::new();
        for field in fields {
            if self.field_owners.get(&field.field) != Some(&product) {
                return Err(LkError::new(
                    ErrorCode::OwnerMismatch,
                    "product field binding belongs to another declaration",
                )
                .at_operation(source));
            }
            let target = field.field;
            if by_field.insert(target, field).is_some() {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "product field binding is duplicated",
                )
                .at_operation(source));
            }
        }
        let normalized = declared
            .iter()
            .map(|field| {
                by_field.remove(field).ok_or_else(|| {
                    LkError::new(
                        ErrorCode::InvalidOperand,
                        "product construction is missing a declared field",
                    )
                    .at_operation(source)
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if !by_field.is_empty() {
            return Err(LkError::new(
                ErrorCode::OwnerMismatch,
                "product construction contains a foreign field",
            )
            .at_operation(source));
        }
        Ok(normalized)
    }
}

#[derive(Clone)]
enum ExpandEvent {
    Source(usize),
    Edit(CanonicalEdit),
    FunctionBody {
        function: NodeTarget,
        body: FunctionBodyDraft,
        path: String,
    },
    YieldingBody {
        owner: NodeTarget,
        region: DraftSymbol,
        arguments: Vec<(DraftSymbol, TypeDraft)>,
        body: YieldingBodyDraft,
        path: String,
    },
    MatchArmBody {
        owner: NodeTarget,
        region: DraftSymbol,
        variant: NodeTarget,
        payload_symbol: Option<DraftSymbol>,
        body: YieldingBodyDraft,
        path: String,
    },
    Expression {
        block: NodeTarget,
        before: Option<NodeTarget>,
        expression: ExpressionDraft,
        path: String,
    },
    CreateExpression {
        block: NodeTarget,
        before: Option<NodeTarget>,
        symbol: DraftSymbol,
        operation: ExpressionKindDraft,
        path: String,
    },
    Terminal {
        block: NodeTarget,
        symbol: DraftSymbol,
        value: ValueDraft,
        code: OperationCode,
        path: String,
    },
}

fn expand_transaction(
    base: &Snapshot,
    operations: &[TransactionOp],
) -> Result<ExpandedTransaction> {
    let explicit_symbols = scan_explicit_symbols(operations)?;
    let nominal_catalogue = StagedNominalCatalogue::build(base, operations);
    let mut synthetic = SyntheticSymbols::new(&explicit_symbols);
    let mut events = Vec::new();
    for (source, operation) in operations.iter().enumerate().rev() {
        match operation {
            TransactionOp::CreatePackage { symbol, name } => {
                events.push(ExpandEvent::Edit(CanonicalEdit::CreatePackage {
                    symbol: *symbol,
                    name: name.clone(),
                }))
            }
            TransactionOp::CreateBuildTarget {
                symbol,
                name,
                definition,
            } => events.push(ExpandEvent::Edit(CanonicalEdit::CreateBuildTarget {
                symbol: *symbol,
                name: name.clone(),
                definition: definition.clone(),
            })),
            TransactionOp::ReplaceBuildTarget { target, definition } => {
                events.push(ExpandEvent::Edit(CanonicalEdit::ReplaceBuildTarget {
                    target: *target,
                    definition: definition.clone(),
                }))
            }
            TransactionOp::AddReleaseTargetExport { target, name, item } => {
                events.push(ExpandEvent::Edit(CanonicalEdit::AddReleaseTargetExport {
                    target: *target,
                    name: name.clone(),
                    item: *item,
                }))
            }
            TransactionOp::SetReleaseTargetExport { target, name, item } => {
                events.push(ExpandEvent::Edit(CanonicalEdit::SetReleaseTargetExport {
                    target: *target,
                    name: name.clone(),
                    item: *item,
                }))
            }
            TransactionOp::SetApplicationQueryBoundary {
                target,
                query_entry,
                query,
            } => events.push(ExpandEvent::Edit(
                CanonicalEdit::SetApplicationQueryBoundary {
                    target: *target,
                    query_entry: *query_entry,
                    query: *query,
                },
            )),
            TransactionOp::AddApplicationTargetTest { target, case } => {
                events.push(ExpandEvent::Edit(CanonicalEdit::AddApplicationTargetTest {
                    target: *target,
                    case: case.clone(),
                }))
            }
            TransactionOp::CreateModule {
                symbol,
                package,
                name,
            } => events.push(ExpandEvent::Edit(CanonicalEdit::CreateModule {
                symbol: *symbol,
                package: *package,
                name: name.clone(),
            })),
            TransactionOp::CreateProductType {
                symbol,
                module,
                name,
                fields,
            } => {
                for field in fields.iter().rev() {
                    events.push(ExpandEvent::Edit(CanonicalEdit::CreateProductField {
                        symbol: field.symbol,
                        product: NodeTarget::Draft(*symbol),
                        name: field.name.clone(),
                        ty: field.ty,
                    }));
                }
                events.push(ExpandEvent::Edit(CanonicalEdit::CreateProductType {
                    symbol: *symbol,
                    module: *module,
                    name: name.clone(),
                }));
            }
            TransactionOp::AddProductField {
                symbol,
                product,
                name,
                ty,
            } => events.push(ExpandEvent::Edit(CanonicalEdit::CreateProductField {
                symbol: *symbol,
                product: *product,
                name: name.clone(),
                ty: *ty,
            })),
            TransactionOp::CreateSumType {
                symbol,
                module,
                name,
                variants,
            } => {
                for variant in variants.iter().rev() {
                    events.push(ExpandEvent::Edit(CanonicalEdit::CreateSumVariant {
                        symbol: variant.symbol,
                        sum: NodeTarget::Draft(*symbol),
                        name: variant.name.clone(),
                        payload: variant.payload,
                    }));
                }
                events.push(ExpandEvent::Edit(CanonicalEdit::CreateSumType {
                    symbol: *symbol,
                    module: *module,
                    name: name.clone(),
                }));
            }
            TransactionOp::CreateSequenceType {
                symbol,
                module,
                name,
                element,
            } => events.push(ExpandEvent::Edit(CanonicalEdit::CreateSequenceType {
                symbol: *symbol,
                module: *module,
                name: name.clone(),
                element: *element,
            })),
            TransactionOp::CreateFunction {
                symbol,
                module,
                name,
                parameters,
                result,
                body,
            } => {
                if let Some(body) = body {
                    events.push(ExpandEvent::FunctionBody {
                        function: NodeTarget::Draft(*symbol),
                        body: body.clone(),
                        path: format!("op[{source}].body"),
                    });
                }
                for parameter in parameters.iter().rev() {
                    events.push(ExpandEvent::Edit(CanonicalEdit::CreateParameter {
                        symbol: parameter.symbol,
                        function: NodeTarget::Draft(*symbol),
                        name: parameter.name.clone(),
                        ty: parameter.ty,
                    }));
                }
                events.push(ExpandEvent::Edit(CanonicalEdit::CreateFunction {
                    symbol: *symbol,
                    module: *module,
                    name: name.clone(),
                    result: *result,
                }));
            }
            TransactionOp::DefineFunctionBody { function, body } => {
                events.push(ExpandEvent::FunctionBody {
                    function: NodeTarget::Existing(*function),
                    body: body.clone(),
                    path: format!("op[{source}].body"),
                })
            }
            TransactionOp::ReplaceFunctionBody { function, body } => {
                events.push(ExpandEvent::FunctionBody {
                    function: NodeTarget::Existing(*function),
                    body: body.clone(),
                    path: format!("op[{source}].body"),
                });
                events.push(ExpandEvent::Edit(CanonicalEdit::ClearFunctionBody {
                    function: NodeTarget::Existing(*function),
                }));
            }
            TransactionOp::InsertExpression {
                block,
                before,
                expression,
            } => {
                let block_node = base
                    .node(*block)
                    .map_err(|error| error.at_operation(source))?;
                if block_node.kind() != NodeKind::Block {
                    return Err(LkError::new(
                        ErrorCode::WrongKind,
                        "insert target must be a block",
                    )
                    .for_node(*block)
                    .with_kinds(NodeKind::Block, block_node.kind())
                    .at_operation(source));
                }
                if let Some(before) = before {
                    let Node::Block { operations, .. } = block_node else {
                        unreachable!()
                    };
                    if !operations.contains(before) {
                        return Err(LkError::new(
                            ErrorCode::InvalidContainment,
                            "insert anchor must be a regular operation in the base block",
                        )
                        .for_node(*before)
                        .with_related([*block])
                        .at_operation(source));
                    }
                }
                events.push(ExpandEvent::Expression {
                    block: NodeTarget::Existing(*block),
                    before: before.map(NodeTarget::Existing),
                    expression: expression.clone(),
                    path: format!("op[{source}].expression"),
                });
            }
            TransactionOp::SetEntryFunction { package, function } => {
                events.push(ExpandEvent::Edit(CanonicalEdit::SetEntryFunction {
                    package: *package,
                    function: *function,
                }))
            }
            TransactionOp::RenameNode { node, name } => {
                events.push(ExpandEvent::Edit(CanonicalEdit::RenameNode {
                    node: *node,
                    name: name.clone(),
                }))
            }
            TransactionOp::ReplaceOperation {
                operation,
                replacement,
            } => events.push(ExpandEvent::Edit(CanonicalEdit::ReplaceOperation {
                operation: *operation,
                replacement: replacement.clone(),
            })),
            TransactionOp::ReplaceOperand {
                operation,
                index,
                value,
            } => events.push(ExpandEvent::Edit(CanonicalEdit::ReplaceOperand {
                operation: *operation,
                index: *index,
                value: value.clone(),
            })),
            TransactionOp::RefineHole { hole, replacement } => {
                events.push(ExpandEvent::Edit(CanonicalEdit::RefineHole {
                    hole: *hole,
                    replacement: replacement.clone(),
                }))
            }
            TransactionOp::DeleteOwnedSubtree { root } => {
                events.push(ExpandEvent::Edit(CanonicalEdit::DeleteOwnedSubtree {
                    root: *root,
                }))
            }
        }
        events.push(ExpandEvent::Source(source));
    }
    let mut edits = Vec::new();
    let mut edit_sources = Vec::new();
    let mut anonymous_paths = BTreeMap::new();
    let mut current_source = 0;
    while let Some(event) = events.pop() {
        match event {
            ExpandEvent::Source(source) => current_source = source,
            ExpandEvent::Edit(edit) => edits.push(edit),
            ExpandEvent::FunctionBody {
                function,
                body,
                path,
            } => {
                let region = synthetic.next(current_source)?;
                let block = synthetic.next(current_source)?;
                let terminator = synthetic.next(current_source)?;
                edits.push(CanonicalEdit::CreateRegion {
                    symbol: region,
                    owner: function,
                });
                edits.push(CanonicalEdit::CreateBlock {
                    symbol: block,
                    region: NodeTarget::Draft(region),
                });
                events.push(ExpandEvent::Edit(CanonicalEdit::SetFunctionBody {
                    function,
                    region: NodeTarget::Draft(region),
                }));
                events.push(ExpandEvent::Terminal {
                    block: NodeTarget::Draft(block),
                    symbol: terminator,
                    value: body.return_value,
                    code: OperationCode::Return,
                    path: format!("{path}.return"),
                });
                for (index, expression) in body.operations.into_iter().enumerate().rev() {
                    events.push(ExpandEvent::Expression {
                        block: NodeTarget::Draft(block),
                        before: None,
                        expression,
                        path: format!("{path}.e[{index}]"),
                    });
                }
            }
            ExpandEvent::YieldingBody {
                owner,
                region,
                arguments,
                body,
                path,
            } => {
                let block = synthetic.next(current_source)?;
                edits.push(CanonicalEdit::CreateRegion {
                    symbol: region,
                    owner,
                });
                edits.push(CanonicalEdit::CreateBlock {
                    symbol: block,
                    region: NodeTarget::Draft(region),
                });
                for (symbol, ty) in arguments {
                    edits.push(CanonicalEdit::CreateBlockArgument {
                        symbol,
                        block: NodeTarget::Draft(block),
                        ty,
                    });
                }
                let terminator = synthetic.next(current_source)?;
                events.push(ExpandEvent::Terminal {
                    block: NodeTarget::Draft(block),
                    symbol: terminator,
                    value: body.yield_value,
                    code: OperationCode::Yield,
                    path: format!("{path}.yield"),
                });
                for (index, expression) in body.operations.into_iter().enumerate().rev() {
                    events.push(ExpandEvent::Expression {
                        block: NodeTarget::Draft(block),
                        before: None,
                        expression,
                        path: format!("{path}.e[{index}]"),
                    });
                }
            }
            ExpandEvent::MatchArmBody {
                owner,
                region,
                variant,
                payload_symbol,
                body,
                path,
            } => {
                let block = synthetic.next(current_source)?;
                edits.push(CanonicalEdit::CreateRegion {
                    symbol: region,
                    owner,
                });
                edits.push(CanonicalEdit::CreateBlock {
                    symbol: block,
                    region: NodeTarget::Draft(region),
                });
                if let Some(symbol) = payload_symbol {
                    edits.push(CanonicalEdit::CreateMatchPayloadArgument {
                        symbol,
                        block: NodeTarget::Draft(block),
                        variant,
                    });
                }
                let terminator = synthetic.next(current_source)?;
                events.push(ExpandEvent::Terminal {
                    block: NodeTarget::Draft(block),
                    symbol: terminator,
                    value: body.yield_value,
                    code: OperationCode::Yield,
                    path: format!("{path}.yield"),
                });
                for (index, expression) in body.operations.into_iter().enumerate().rev() {
                    events.push(ExpandEvent::Expression {
                        block: NodeTarget::Draft(block),
                        before: None,
                        expression,
                        path: format!("{path}.e[{index}]"),
                    });
                }
            }
            ExpandEvent::Expression {
                block,
                before,
                expression,
                path,
            } => {
                let explicit = expression.symbol.is_some();
                let expression_symbol =
                    expression.symbol.unwrap_or(synthetic.next(current_source)?);
                if !explicit {
                    anonymous_paths.insert(expression_symbol, path.clone());
                }
                let mut operation = expression.operation;
                if let ExpressionKindDraft::ConstructProduct { product, fields } = operation {
                    operation = ExpressionKindDraft::ConstructProduct {
                        product,
                        fields: nominal_catalogue.normalize_product_fields(
                            product,
                            fields,
                            current_source,
                        )?,
                    };
                }
                let children = extract_inline_children(
                    &mut operation,
                    &path,
                    current_source,
                    &mut synthetic,
                    &mut anonymous_paths,
                )?;
                events.push(ExpandEvent::CreateExpression {
                    block,
                    before,
                    symbol: expression_symbol,
                    operation,
                    path,
                });
                for (expression, path) in children.into_iter().rev() {
                    events.push(ExpandEvent::Expression {
                        block,
                        before,
                        expression,
                        path,
                    });
                }
            }
            ExpandEvent::Terminal {
                block,
                symbol,
                mut value,
                code,
                path,
            } => {
                let child = extract_inline_value(
                    &mut value,
                    &path,
                    current_source,
                    &mut synthetic,
                    &mut anonymous_paths,
                )?;
                let operation = match code {
                    OperationCode::Return => OperationDraft::Return { value },
                    OperationCode::Yield => OperationDraft::Yield { value },
                    _ => {
                        return Err(LkError::new(
                            ErrorCode::InvalidOperand,
                            "structured terminal work item has an invalid operation code",
                        )
                        .at_operation(current_source));
                    }
                };
                events.push(ExpandEvent::Edit(CanonicalEdit::CreateOperation {
                    symbol,
                    block,
                    before: None,
                    operation,
                }));
                if let Some((expression, path)) = child {
                    events.push(ExpandEvent::Expression {
                        block,
                        before: None,
                        expression,
                        path,
                    });
                }
            }
            ExpandEvent::CreateExpression {
                block,
                before,
                symbol: expression_symbol,
                operation,
                path,
            } => match operation {
                ExpressionKindDraft::ConstUnit => edits.push(CanonicalEdit::CreateOperation {
                    symbol: expression_symbol,
                    block,
                    before,
                    operation: OperationDraft::ConstUnit,
                }),
                ExpressionKindDraft::ConstBool(value) => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::ConstBool(value),
                    })
                }
                ExpressionKindDraft::ConstI64(value) => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::ConstI64(value),
                    })
                }
                ExpressionKindDraft::ConstBytes(value) => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::ConstBytes(value),
                    })
                }
                ExpressionKindDraft::ConstText(value) => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::ConstText(value),
                    })
                }
                ExpressionKindDraft::AddI64 { lhs, rhs } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::AddI64 { lhs, rhs },
                    })
                }
                ExpressionKindDraft::LtI64 { lhs, rhs } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::LtI64 { lhs, rhs },
                    })
                }
                ExpressionKindDraft::EqualI64 { lhs, rhs } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::EqualI64 { lhs, rhs },
                    })
                }
                ExpressionKindDraft::NotBool { value } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::NotBool { value },
                    })
                }
                ExpressionKindDraft::AndBool { lhs, rhs } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::AndBool { lhs, rhs },
                    })
                }
                ExpressionKindDraft::OrBool { lhs, rhs } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::OrBool { lhs, rhs },
                    })
                }
                ExpressionKindDraft::BytesLen { value } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::BytesLen { value },
                    })
                }
                ExpressionKindDraft::BytesAt { value, index } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::BytesAt { value, index },
                    })
                }
                ExpressionKindDraft::BytesSlice {
                    value,
                    start,
                    length,
                } => edits.push(CanonicalEdit::CreateOperation {
                    symbol: expression_symbol,
                    block,
                    before,
                    operation: OperationDraft::BytesSlice {
                        value,
                        start,
                        length,
                    },
                }),
                ExpressionKindDraft::BytesEqual { lhs, rhs } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::BytesEqual { lhs, rhs },
                    })
                }
                ExpressionKindDraft::BytesConcat { lhs, rhs } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::BytesConcat { lhs, rhs },
                    })
                }
                ExpressionKindDraft::TextLen { value } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::TextLen { value },
                    })
                }
                ExpressionKindDraft::TextEqual { lhs, rhs } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::TextEqual { lhs, rhs },
                    })
                }
                ExpressionKindDraft::TextConcat { lhs, rhs } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::TextConcat { lhs, rhs },
                    })
                }
                ExpressionKindDraft::SequenceEmpty { sequence } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::SequenceEmpty { sequence },
                    })
                }
                ExpressionKindDraft::SequenceLen { sequence, value } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::SequenceLen { sequence, value },
                    })
                }
                ExpressionKindDraft::SequenceGet {
                    sequence,
                    value,
                    index,
                } => edits.push(CanonicalEdit::CreateOperation {
                    symbol: expression_symbol,
                    block,
                    before,
                    operation: OperationDraft::SequenceGet {
                        sequence,
                        value,
                        index,
                    },
                }),
                ExpressionKindDraft::SequenceAppend {
                    sequence,
                    value,
                    element,
                } => edits.push(CanonicalEdit::CreateOperation {
                    symbol: expression_symbol,
                    block,
                    before,
                    operation: OperationDraft::SequenceAppend {
                        sequence,
                        value,
                        element,
                    },
                }),
                ExpressionKindDraft::SequenceReplace {
                    sequence,
                    value,
                    index,
                    element,
                } => edits.push(CanonicalEdit::CreateOperation {
                    symbol: expression_symbol,
                    block,
                    before,
                    operation: OperationDraft::SequenceReplace {
                        sequence,
                        value,
                        index,
                        element,
                    },
                }),
                ExpressionKindDraft::SequenceSlice {
                    sequence,
                    value,
                    start,
                    end_exclusive,
                } => edits.push(CanonicalEdit::CreateOperation {
                    symbol: expression_symbol,
                    block,
                    before,
                    operation: OperationDraft::SequenceSlice {
                        sequence,
                        value,
                        start,
                        end_exclusive,
                    },
                }),
                ExpressionKindDraft::SequenceConcat { sequence, lhs, rhs } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::SequenceConcat { sequence, lhs, rhs },
                    })
                }
                ExpressionKindDraft::Call {
                    function,
                    arguments,
                } => edits.push(CanonicalEdit::CreateOperation {
                    symbol: expression_symbol,
                    block,
                    before,
                    operation: OperationDraft::Call {
                        function,
                        arguments,
                    },
                }),
                ExpressionKindDraft::Hole { expected } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::Hole { expected },
                    })
                }
                ExpressionKindDraft::If {
                    condition,
                    result,
                    then_body,
                    else_body,
                } => {
                    let then_region = synthetic.next(current_source)?;
                    let else_region = synthetic.next(current_source)?;
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::If {
                            condition,
                            result,
                            then_region: NodeTarget::Draft(then_region),
                            else_region: NodeTarget::Draft(else_region),
                        },
                    });
                    events.push(ExpandEvent::YieldingBody {
                        owner: NodeTarget::Draft(expression_symbol),
                        region: else_region,
                        arguments: Vec::new(),
                        body: else_body,
                        path: format!("{path}.else"),
                    });
                    events.push(ExpandEvent::YieldingBody {
                        owner: NodeTarget::Draft(expression_symbol),
                        region: then_region,
                        arguments: Vec::new(),
                        body: then_body,
                        path: format!("{path}.then"),
                    });
                }
                ExpressionKindDraft::ForI64 {
                    start,
                    end_exclusive,
                    step,
                    initial,
                    carried,
                    index_symbol,
                    carried_symbol,
                    body,
                } => {
                    let body_region = synthetic.next(current_source)?;
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::ForI64 {
                            start,
                            end_exclusive,
                            step,
                            initial,
                            carried,
                            body_region: NodeTarget::Draft(body_region),
                        },
                    });
                    events.push(ExpandEvent::YieldingBody {
                        owner: NodeTarget::Draft(expression_symbol),
                        region: body_region,
                        arguments: vec![(index_symbol, TypeDraft::I64), (carried_symbol, carried)],
                        body,
                        path: format!("{path}.loop"),
                    });
                }
                ExpressionKindDraft::ConstructProduct { product, fields } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::ConstructProduct { product, fields },
                    })
                }
                ExpressionKindDraft::ProjectField { value, field } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::ProjectField { value, field },
                    })
                }
                ExpressionKindDraft::ConstructVariant { variant, payload } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::ConstructVariant { variant, payload },
                    })
                }
                ExpressionKindDraft::MatchSum {
                    scrutinee,
                    result,
                    arms,
                } => {
                    let arms = nominal_catalogue.normalize_match_arms(arms, current_source)?;
                    let mut canonical_arms = Vec::with_capacity(arms.len());
                    let mut arm_events = Vec::with_capacity(arms.len());
                    for (index, arm) in arms.into_iter().enumerate() {
                        let region = synthetic.next(current_source)?;
                        canonical_arms.push(MatchArmOperationDraft {
                            variant: arm.variant,
                            region: NodeTarget::Draft(region),
                        });
                        arm_events.push(ExpandEvent::MatchArmBody {
                            owner: NodeTarget::Draft(expression_symbol),
                            region,
                            variant: arm.variant,
                            payload_symbol: arm.payload_symbol,
                            body: arm.body,
                            path: format!("{path}.arm[{index}]"),
                        });
                    }
                    edits.push(CanonicalEdit::CreateOperation {
                        symbol: expression_symbol,
                        block,
                        before,
                        operation: OperationDraft::MatchSum {
                            scrutinee,
                            result,
                            arms: canonical_arms,
                        },
                    });
                    for event in arm_events.into_iter().rev() {
                        events.push(event);
                    }
                }
            },
        }
        edit_sources.resize(edits.len(), current_source);
    }
    debug_assert_eq!(edits.len(), edit_sources.len());
    Ok(ExpandedTransaction {
        edits,
        edit_sources,
        explicit_symbols,
        anonymous_paths,
        nominal_catalogue,
    })
}

fn child_draft_path(base: &str, segment: &str, source: usize) -> Result<String> {
    let path = format!("{base}.{segment}");
    if path.len() > MAX_DRAFT_PATH_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "structured draft path exceeds diagnostic policy",
        )
        .at_operation(source));
    }
    Ok(path)
}

fn extract_inline_value(
    value: &mut ValueDraft,
    path: &str,
    source: usize,
    synthetic: &mut SyntheticSymbols,
    anonymous_paths: &mut BTreeMap<DraftSymbol, String>,
) -> Result<Option<(ExpressionDraft, String)>> {
    let ValueDraft::InlineExpression(operation) = value else {
        return Ok(None);
    };
    if !operation.is_inline_eligible() {
        return Err(LkError::new(
            ErrorCode::InvalidOperand,
            "inline expressions must be complete non-terminators with one result and no owned region",
        )
        .at_operation(source)
        .at_draft_path(path));
    }
    let symbol = synthetic.next(source)?;
    let replacement = ValueDraft::OperationResult {
        operation: NodeTarget::Draft(symbol),
        output: 0,
    };
    let ValueDraft::InlineExpression(operation) = std::mem::replace(value, replacement) else {
        unreachable!()
    };
    anonymous_paths.insert(symbol, path.to_owned());
    Ok(Some((
        ExpressionDraft {
            symbol: Some(symbol),
            operation: *operation,
        },
        path.to_owned(),
    )))
}

fn extract_inline_children(
    operation: &mut ExpressionKindDraft,
    path: &str,
    source: usize,
    synthetic: &mut SyntheticSymbols,
    anonymous_paths: &mut BTreeMap<DraftSymbol, String>,
) -> Result<Vec<(ExpressionDraft, String)>> {
    let mut children = Vec::new();
    let mut extract = |value: &mut ValueDraft, segment: String| -> Result<()> {
        let child_path = child_draft_path(path, &segment, source)?;
        if let Some(child) =
            extract_inline_value(value, &child_path, source, synthetic, anonymous_paths)?
        {
            children.push(child);
        }
        Ok(())
    };
    match operation {
        ExpressionKindDraft::ConstUnit
        | ExpressionKindDraft::ConstBool(_)
        | ExpressionKindDraft::ConstI64(_)
        | ExpressionKindDraft::ConstBytes(_)
        | ExpressionKindDraft::ConstText(_)
        | ExpressionKindDraft::SequenceEmpty { .. }
        | ExpressionKindDraft::Hole { .. } => {}
        ExpressionKindDraft::AddI64 { lhs, rhs }
        | ExpressionKindDraft::LtI64 { lhs, rhs }
        | ExpressionKindDraft::EqualI64 { lhs, rhs }
        | ExpressionKindDraft::AndBool { lhs, rhs }
        | ExpressionKindDraft::OrBool { lhs, rhs }
        | ExpressionKindDraft::BytesEqual { lhs, rhs }
        | ExpressionKindDraft::BytesConcat { lhs, rhs }
        | ExpressionKindDraft::TextEqual { lhs, rhs }
        | ExpressionKindDraft::TextConcat { lhs, rhs } => {
            extract(lhs, "lhs".to_owned())?;
            extract(rhs, "rhs".to_owned())?;
        }
        ExpressionKindDraft::NotBool { value }
        | ExpressionKindDraft::BytesLen { value }
        | ExpressionKindDraft::TextLen { value }
        | ExpressionKindDraft::SequenceLen { value, .. } => extract(value, "value".to_owned())?,
        ExpressionKindDraft::BytesAt { value, index } => {
            extract(value, "value".to_owned())?;
            extract(index, "index".to_owned())?;
        }
        ExpressionKindDraft::BytesSlice {
            value,
            start,
            length,
        } => {
            extract(value, "value".to_owned())?;
            extract(start, "start".to_owned())?;
            extract(length, "length".to_owned())?;
        }
        ExpressionKindDraft::SequenceGet { value, index, .. } => {
            extract(value, "value".to_owned())?;
            extract(index, "index".to_owned())?;
        }
        ExpressionKindDraft::SequenceAppend { value, element, .. } => {
            extract(value, "value".to_owned())?;
            extract(element, "element".to_owned())?;
        }
        ExpressionKindDraft::SequenceReplace {
            value,
            index,
            element,
            ..
        } => {
            extract(value, "value".to_owned())?;
            extract(index, "index".to_owned())?;
            extract(element, "element".to_owned())?;
        }
        ExpressionKindDraft::SequenceSlice {
            value,
            start,
            end_exclusive,
            ..
        } => {
            extract(value, "value".to_owned())?;
            extract(start, "start".to_owned())?;
            extract(end_exclusive, "end_exclusive".to_owned())?;
        }
        ExpressionKindDraft::SequenceConcat { lhs, rhs, .. } => {
            extract(lhs, "lhs".to_owned())?;
            extract(rhs, "rhs".to_owned())?;
        }
        ExpressionKindDraft::Call { arguments, .. } => {
            for (index, value) in arguments.iter_mut().enumerate() {
                extract(value, format!("arg[{index}]"))?;
            }
        }
        ExpressionKindDraft::If { condition, .. } => extract(condition, "condition".to_owned())?,
        ExpressionKindDraft::ForI64 {
            start,
            end_exclusive,
            initial,
            ..
        } => {
            extract(start, "start".to_owned())?;
            extract(end_exclusive, "end".to_owned())?;
            extract(initial, "initial".to_owned())?;
        }
        ExpressionKindDraft::ConstructProduct { fields, .. } => {
            for (index, field) in fields.iter_mut().enumerate() {
                extract(&mut field.value, format!("field[{index}]"))?;
            }
        }
        ExpressionKindDraft::ProjectField { value, .. } => extract(value, "value".to_owned())?,
        ExpressionKindDraft::ConstructVariant { payload, .. } => {
            if let Some(payload) = payload {
                extract(payload, "payload".to_owned())?;
            }
        }
        ExpressionKindDraft::MatchSum { scrutinee, .. } => {
            extract(scrutinee, "scrutinee".to_owned())?
        }
    }
    Ok(children)
}

struct SyntheticSymbols {
    used: BTreeSet<DraftSymbol>,
    next: u32,
}
impl SyntheticSymbols {
    fn new(explicit: &BTreeSet<DraftSymbol>) -> Self {
        Self {
            used: explicit.clone(),
            next: u32::MAX,
        }
    }
    fn next(&mut self, source: usize) -> Result<DraftSymbol> {
        if self.next == 0 {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "private structured symbol space exhausted",
            )
            .at_operation(source));
        }
        let symbol = DraftSymbol::synthetic(self.next);
        self.used.insert(symbol);
        self.next -= 1;
        Ok(symbol)
    }
}

#[cfg(test)]
pub(crate) fn validate_structured_request(operations: &[TransactionOp]) -> Result<()> {
    scan_explicit_symbols(operations).map(|_| ())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DraftSymbolKind {
    Package,
    BuildTarget,
    Module,
    ProductType,
    ProductField,
    SumType,
    SumVariant,
    SequenceType,
    Function,
    Parameter,
    Region,
    BlockArgument,
    Operation,
}

#[derive(Clone, Copy)]
enum DraftReferenceKind {
    Any,
    NominalType,
    Package,
    Module,
    ProductType,
    ProductField,
    SumVariant,
    SequenceType,
    Function,
    Parameter,
    Region,
    BlockArgument,
    Operation,
}

impl DraftReferenceKind {
    fn accepts(self, actual: DraftSymbolKind) -> bool {
        match self {
            Self::Any => true,
            Self::NominalType => matches!(
                actual,
                DraftSymbolKind::ProductType
                    | DraftSymbolKind::SumType
                    | DraftSymbolKind::SequenceType
            ),
            Self::Package => actual == DraftSymbolKind::Package,
            Self::Module => actual == DraftSymbolKind::Module,
            Self::ProductType => actual == DraftSymbolKind::ProductType,
            Self::ProductField => actual == DraftSymbolKind::ProductField,
            Self::SumVariant => actual == DraftSymbolKind::SumVariant,
            Self::SequenceType => actual == DraftSymbolKind::SequenceType,
            Self::Function => actual == DraftSymbolKind::Function,
            Self::Parameter => actual == DraftSymbolKind::Parameter,
            Self::Region => actual == DraftSymbolKind::Region,
            Self::BlockArgument => actual == DraftSymbolKind::BlockArgument,
            Self::Operation => actual == DraftSymbolKind::Operation,
        }
    }
}

fn scan_explicit_symbols(operations: &[TransactionOp]) -> Result<BTreeSet<DraftSymbol>> {
    enum Scan<'a> {
        Expression(&'a ExpressionDraft, usize, usize, String),
        Inline(&'a ExpressionKindDraft, usize, usize, String),
        Body(&'a [ExpressionDraft], &'a ValueDraft, usize, usize, String),
    }
    #[derive(Default)]
    struct DraftBudget {
        items: usize,
        byte_literals: usize,
        text_literals: usize,
    }
    impl DraftBudget {
        fn add(&mut self, count: usize, source: usize) -> Result<()> {
            self.items = self.items.checked_add(count).ok_or_else(|| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "structured draft item count overflow",
                )
                .at_operation(source)
            })?;
            if self.items > MAX_STRUCTURED_DRAFT_ITEMS {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "structured draft exceeds request item policy",
                )
                .at_operation(source));
            }
            Ok(())
        }

        fn add_byte_literal(&mut self, value: &ByteString, source: usize) -> Result<()> {
            if value.len() > MAXIMUM_BYTE_LITERAL_BYTES {
                return Err(LkError::new(
                    ErrorCode::ByteLiteralTooLarge,
                    "byte literal exceeds the per-literal policy",
                )
                .at_operation(source));
            }
            self.byte_literals = self.byte_literals.checked_add(value.len()).ok_or_else(|| {
                LkError::new(
                    ErrorCode::ByteLiteralTooLarge,
                    "aggregate byte literal size overflows",
                )
                .at_operation(source)
            })?;
            if self.byte_literals > MAXIMUM_TRANSACTION_BYTE_LITERAL_BYTES {
                return Err(LkError::new(
                    ErrorCode::ByteLiteralTooLarge,
                    "transaction exceeds the aggregate byte literal policy",
                )
                .at_operation(source));
            }
            Ok(())
        }

        fn add_text_literal(&mut self, value: &TextString, source: usize) -> Result<()> {
            if value.len_bytes() > MAXIMUM_TEXT_LITERAL_BYTES {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "text literal exceeds the per-literal policy",
                )
                .at_operation(source));
            }
            self.text_literals = self
                .text_literals
                .checked_add(value.len_bytes())
                .ok_or_else(|| {
                    LkError::new(
                        ErrorCode::PolicyExceeded,
                        "aggregate text literal size overflows",
                    )
                    .at_operation(source)
                })?;
            if self.text_literals > MAXIMUM_TRANSACTION_TEXT_LITERAL_BYTES {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "transaction exceeds the aggregate text literal policy",
                )
                .at_operation(source));
            }
            Ok(())
        }
    }
    fn declare(
        symbols: &mut BTreeSet<DraftSymbol>,
        kinds: &mut BTreeMap<DraftSymbol, DraftSymbolKind>,
        symbol: DraftSymbol,
        kind: DraftSymbolKind,
        source: usize,
    ) -> Result<()> {
        if let Err(message) = symbol.validate() {
            return Err(LkError::new(ErrorCode::InvalidDraftSymbol, message)
                .at_operation(source)
                .for_symbol(symbol));
        }
        if !symbols.insert(symbol) {
            return Err(LkError::new(
                ErrorCode::DuplicateDraftSymbol,
                "transaction-local symbol is declared more than once",
            )
            .at_operation(source)
            .for_symbol(symbol));
        }
        kinds.insert(symbol, kind);
        Ok(())
    }
    fn reference(
        target: NodeTarget,
        expected: DraftReferenceKind,
        source: usize,
        references: &mut Vec<(DraftSymbol, DraftReferenceKind, usize)>,
    ) {
        if let NodeTarget::Draft(symbol) = target {
            references.push((symbol, expected, source));
        }
    }
    fn type_reference(
        ty: TypeDraft,
        source: usize,
        references: &mut Vec<(DraftSymbol, DraftReferenceKind, usize)>,
    ) {
        if let TypeDraft::Nominal(target) = ty {
            reference(target, DraftReferenceKind::NominalType, source, references);
        }
    }
    fn value_reference(
        value: &ValueDraft,
        source: usize,
        references: &mut Vec<(DraftSymbol, DraftReferenceKind, usize)>,
    ) -> Result<()> {
        validate_draft_value(value, source)?;
        match value {
            ValueDraft::FunctionParameter(target) => {
                reference(*target, DraftReferenceKind::Parameter, source, references)
            }
            ValueDraft::BlockArgument(target) => reference(
                *target,
                DraftReferenceKind::BlockArgument,
                source,
                references,
            ),
            ValueDraft::OperationResult { operation, .. } => reference(
                *operation,
                DraftReferenceKind::Operation,
                source,
                references,
            ),
            ValueDraft::InlineExpression(_) => {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "inline expressions are accepted only in structured value positions",
                )
                .at_operation(source));
            }
        }
        Ok(())
    }
    fn structured_value<'a>(
        value: &'a ValueDraft,
        depth: usize,
        source: usize,
        path: String,
        stack: &mut Vec<Scan<'a>>,
        references: &mut Vec<(DraftSymbol, DraftReferenceKind, usize)>,
    ) -> Result<()> {
        match value {
            ValueDraft::InlineExpression(operation) => {
                let depth = depth.checked_add(1).ok_or_else(|| {
                    LkError::new(
                        ErrorCode::PolicyExceeded,
                        "structured draft nesting depth overflows",
                    )
                    .at_operation(source)
                    .at_draft_path(path.clone())
                })?;
                if depth > MAX_STRUCTURED_DRAFT_DEPTH {
                    return Err(LkError::new(
                        ErrorCode::PolicyExceeded,
                        "structured draft nesting exceeds request depth policy",
                    )
                    .at_operation(source)
                    .at_draft_path(path));
                }
                if !operation.is_inline_eligible() {
                    return Err(LkError::new(
                        ErrorCode::InvalidOperand,
                        "inline expressions must be complete non-terminators with one result and no owned region",
                    )
                    .at_operation(source)
                    .at_draft_path(path));
                }
                stack.push(Scan::Inline(operation, depth, source, path));
                Ok(())
            }
            _ => value_reference(value, source, references),
        }
    }
    fn operation_references(
        operation: &OperationDraft,
        source: usize,
        budget: &mut DraftBudget,
        references: &mut Vec<(DraftSymbol, DraftReferenceKind, usize)>,
    ) -> Result<()> {
        match operation {
            OperationDraft::ConstUnit
            | OperationDraft::ConstI64(_)
            | OperationDraft::ConstBool(_) => {}
            OperationDraft::ConstBytes(value) => budget.add_byte_literal(value, source)?,
            OperationDraft::ConstText(value) => budget.add_text_literal(value, source)?,
            OperationDraft::AddI64 { lhs, rhs }
            | OperationDraft::LtI64 { lhs, rhs }
            | OperationDraft::EqualI64 { lhs, rhs }
            | OperationDraft::AndBool { lhs, rhs }
            | OperationDraft::OrBool { lhs, rhs }
            | OperationDraft::BytesEqual { lhs, rhs }
            | OperationDraft::BytesConcat { lhs, rhs }
            | OperationDraft::TextEqual { lhs, rhs }
            | OperationDraft::TextConcat { lhs, rhs } => {
                value_reference(lhs, source, references)?;
                value_reference(rhs, source, references)?;
            }
            OperationDraft::NotBool { value }
            | OperationDraft::BytesLen { value }
            | OperationDraft::TextLen { value } => {
                value_reference(value, source, references)?;
            }
            OperationDraft::BytesAt { value, index } => {
                value_reference(value, source, references)?;
                value_reference(index, source, references)?;
            }
            OperationDraft::BytesSlice {
                value,
                start,
                length,
            } => {
                value_reference(value, source, references)?;
                value_reference(start, source, references)?;
                value_reference(length, source, references)?;
            }
            OperationDraft::SequenceEmpty { sequence } => {
                reference(
                    *sequence,
                    DraftReferenceKind::SequenceType,
                    source,
                    references,
                );
            }
            OperationDraft::SequenceLen { sequence, value } => {
                reference(
                    *sequence,
                    DraftReferenceKind::SequenceType,
                    source,
                    references,
                );
                value_reference(value, source, references)?;
            }
            OperationDraft::SequenceGet {
                sequence,
                value,
                index,
            } => {
                reference(
                    *sequence,
                    DraftReferenceKind::SequenceType,
                    source,
                    references,
                );
                value_reference(value, source, references)?;
                value_reference(index, source, references)?;
            }
            OperationDraft::SequenceAppend {
                sequence,
                value,
                element,
            } => {
                reference(
                    *sequence,
                    DraftReferenceKind::SequenceType,
                    source,
                    references,
                );
                value_reference(value, source, references)?;
                value_reference(element, source, references)?;
            }
            OperationDraft::SequenceReplace {
                sequence,
                value,
                index,
                element,
            } => {
                reference(
                    *sequence,
                    DraftReferenceKind::SequenceType,
                    source,
                    references,
                );
                value_reference(value, source, references)?;
                value_reference(index, source, references)?;
                value_reference(element, source, references)?;
            }
            OperationDraft::SequenceSlice {
                sequence,
                value,
                start,
                end_exclusive,
            } => {
                reference(
                    *sequence,
                    DraftReferenceKind::SequenceType,
                    source,
                    references,
                );
                value_reference(value, source, references)?;
                value_reference(start, source, references)?;
                value_reference(end_exclusive, source, references)?;
            }
            OperationDraft::SequenceConcat { sequence, lhs, rhs } => {
                reference(
                    *sequence,
                    DraftReferenceKind::SequenceType,
                    source,
                    references,
                );
                value_reference(lhs, source, references)?;
                value_reference(rhs, source, references)?;
            }
            OperationDraft::Call {
                function,
                arguments,
            } => {
                reference(*function, DraftReferenceKind::Function, source, references);
                budget.add(arguments.len(), source)?;
                for value in arguments {
                    value_reference(value, source, references)?;
                }
            }
            OperationDraft::Hole { expected } => type_reference(*expected, source, references),
            OperationDraft::If {
                condition,
                result,
                then_region,
                else_region,
            } => {
                value_reference(condition, source, references)?;
                type_reference(*result, source, references);
                reference(*then_region, DraftReferenceKind::Region, source, references);
                reference(*else_region, DraftReferenceKind::Region, source, references);
            }
            OperationDraft::ForI64 {
                start,
                end_exclusive,
                initial,
                carried,
                body_region,
                ..
            } => {
                value_reference(start, source, references)?;
                value_reference(end_exclusive, source, references)?;
                value_reference(initial, source, references)?;
                type_reference(*carried, source, references);
                reference(*body_region, DraftReferenceKind::Region, source, references);
            }
            OperationDraft::Return { value } | OperationDraft::Yield { value } => {
                value_reference(value, source, references)?;
            }
            OperationDraft::ConstructProduct { product, fields } => {
                reference(
                    *product,
                    DraftReferenceKind::ProductType,
                    source,
                    references,
                );
                budget.add(fields.len(), source)?;
                for field in fields {
                    reference(
                        field.field,
                        DraftReferenceKind::ProductField,
                        source,
                        references,
                    );
                    value_reference(&field.value, source, references)?;
                }
            }
            OperationDraft::ProjectField { value, field } => {
                value_reference(value, source, references)?;
                reference(*field, DraftReferenceKind::ProductField, source, references);
            }
            OperationDraft::ConstructVariant { variant, payload } => {
                reference(*variant, DraftReferenceKind::SumVariant, source, references);
                if let Some(value) = payload {
                    value_reference(value, source, references)?;
                }
            }
            OperationDraft::MatchSum {
                scrutinee,
                result,
                arms,
            } => {
                value_reference(scrutinee, source, references)?;
                type_reference(*result, source, references);
                budget.add(arms.len(), source)?;
                for arm in arms {
                    reference(
                        arm.variant,
                        DraftReferenceKind::SumVariant,
                        source,
                        references,
                    );
                    reference(arm.region, DraftReferenceKind::Any, source, references);
                }
            }
        }
        Ok(())
    }

    let mut symbols = BTreeSet::new();
    let mut kinds = BTreeMap::new();
    let mut references = Vec::<(DraftSymbol, DraftReferenceKind, usize)>::new();
    let mut stack = Vec::new();
    let mut budget = DraftBudget::default();
    for (source, operation) in operations.iter().enumerate() {
        budget.add(1, source)?;
        match operation {
            TransactionOp::CreatePackage { symbol, .. } => declare(
                &mut symbols,
                &mut kinds,
                *symbol,
                DraftSymbolKind::Package,
                source,
            )?,
            TransactionOp::CreateBuildTarget { symbol, .. } => declare(
                &mut symbols,
                &mut kinds,
                *symbol,
                DraftSymbolKind::BuildTarget,
                source,
            )?,
            TransactionOp::ReplaceBuildTarget { .. }
            | TransactionOp::AddReleaseTargetExport { .. }
            | TransactionOp::SetReleaseTargetExport { .. }
            | TransactionOp::SetApplicationQueryBoundary { .. }
            | TransactionOp::AddApplicationTargetTest { .. } => {}
            TransactionOp::CreateModule {
                symbol, package, ..
            } => {
                declare(
                    &mut symbols,
                    &mut kinds,
                    *symbol,
                    DraftSymbolKind::Module,
                    source,
                )?;
                reference(
                    *package,
                    DraftReferenceKind::Package,
                    source,
                    &mut references,
                );
            }
            TransactionOp::CreateProductType {
                symbol,
                module,
                fields,
                ..
            } => {
                declare(
                    &mut symbols,
                    &mut kinds,
                    *symbol,
                    DraftSymbolKind::ProductType,
                    source,
                )?;
                reference(*module, DraftReferenceKind::Module, source, &mut references);
                budget.add(fields.len(), source)?;
                for field in fields {
                    declare(
                        &mut symbols,
                        &mut kinds,
                        field.symbol,
                        DraftSymbolKind::ProductField,
                        source,
                    )?;
                    type_reference(field.ty, source, &mut references);
                }
            }
            TransactionOp::AddProductField {
                symbol,
                product,
                ty,
                ..
            } => {
                declare(
                    &mut symbols,
                    &mut kinds,
                    *symbol,
                    DraftSymbolKind::ProductField,
                    source,
                )?;
                reference(
                    *product,
                    DraftReferenceKind::ProductType,
                    source,
                    &mut references,
                );
                type_reference(*ty, source, &mut references);
            }
            TransactionOp::CreateSumType {
                symbol,
                module,
                variants,
                ..
            } => {
                declare(
                    &mut symbols,
                    &mut kinds,
                    *symbol,
                    DraftSymbolKind::SumType,
                    source,
                )?;
                reference(*module, DraftReferenceKind::Module, source, &mut references);
                if variants.is_empty() {
                    return Err(LkError::new(
                        ErrorCode::InvalidOperand,
                        "sum declarations require at least one variant",
                    )
                    .at_operation(source)
                    .for_symbol(*symbol));
                }
                budget.add(variants.len(), source)?;
                for variant in variants {
                    declare(
                        &mut symbols,
                        &mut kinds,
                        variant.symbol,
                        DraftSymbolKind::SumVariant,
                        source,
                    )?;
                    if let Some(payload) = variant.payload {
                        type_reference(payload, source, &mut references);
                    }
                }
            }
            TransactionOp::CreateSequenceType {
                symbol,
                module,
                element,
                ..
            } => {
                declare(
                    &mut symbols,
                    &mut kinds,
                    *symbol,
                    DraftSymbolKind::SequenceType,
                    source,
                )?;
                reference(*module, DraftReferenceKind::Module, source, &mut references);
                type_reference(*element, source, &mut references);
            }
            TransactionOp::CreateFunction {
                symbol,
                module,
                parameters,
                result,
                body,
                ..
            } => {
                declare(
                    &mut symbols,
                    &mut kinds,
                    *symbol,
                    DraftSymbolKind::Function,
                    source,
                )?;
                reference(*module, DraftReferenceKind::Module, source, &mut references);
                type_reference(*result, source, &mut references);
                budget.add(parameters.len(), source)?;
                for parameter in parameters {
                    declare(
                        &mut symbols,
                        &mut kinds,
                        parameter.symbol,
                        DraftSymbolKind::Parameter,
                        source,
                    )?;
                    type_reference(parameter.ty, source, &mut references);
                }
                if let Some(body) = body {
                    budget.add(1, source)?;
                    stack.push(Scan::Body(
                        &body.operations,
                        &body.return_value,
                        0,
                        source,
                        format!("op[{source}].body"),
                    ));
                }
            }
            TransactionOp::DefineFunctionBody { body, .. }
            | TransactionOp::ReplaceFunctionBody { body, .. } => {
                budget.add(1, source)?;
                stack.push(Scan::Body(
                    &body.operations,
                    &body.return_value,
                    0,
                    source,
                    format!("op[{source}].body"),
                ));
            }
            TransactionOp::InsertExpression { expression, .. } => stack.push(Scan::Expression(
                expression,
                0,
                source,
                format!("op[{source}].expression"),
            )),
            TransactionOp::SetEntryFunction { package, function } => {
                reference(
                    *package,
                    DraftReferenceKind::Package,
                    source,
                    &mut references,
                );
                reference(
                    *function,
                    DraftReferenceKind::Function,
                    source,
                    &mut references,
                );
            }
            TransactionOp::RenameNode { node, .. } => {
                reference(*node, DraftReferenceKind::Any, source, &mut references)
            }
            TransactionOp::ReplaceOperation {
                operation,
                replacement,
            } => {
                reference(
                    *operation,
                    DraftReferenceKind::Operation,
                    source,
                    &mut references,
                );
                if matches!(replacement, OperationDraft::MatchSum { .. }) {
                    return Err(LkError::new(ErrorCode::InvalidOperand, "match_sum cannot be authored through a region-scaffolding maintenance operation").at_operation(source));
                }
                operation_references(replacement, source, &mut budget, &mut references)?;
            }
            TransactionOp::ReplaceOperand {
                operation, value, ..
            } => {
                reference(
                    *operation,
                    DraftReferenceKind::Operation,
                    source,
                    &mut references,
                );
                value_reference(value, source, &mut references)?;
            }
            TransactionOp::RefineHole { hole, replacement } => {
                reference(
                    *hole,
                    DraftReferenceKind::Operation,
                    source,
                    &mut references,
                );
                if matches!(replacement, OperationDraft::MatchSum { .. }) {
                    return Err(LkError::new(ErrorCode::InvalidOperand, "match_sum cannot be authored through a region-scaffolding maintenance operation").at_operation(source));
                }
                operation_references(replacement, source, &mut budget, &mut references)?;
            }
            TransactionOp::DeleteOwnedSubtree { root } => {
                reference(*root, DraftReferenceKind::Any, source, &mut references)
            }
        }
    }
    while let Some(event) = stack.pop() {
        let (operation, depth, source, path) = match event {
            Scan::Body(expressions, terminal, depth, source, path) => {
                if depth > MAX_STRUCTURED_DRAFT_DEPTH {
                    return Err(LkError::new(
                        ErrorCode::PolicyExceeded,
                        "structured draft nesting exceeds request depth policy",
                    )
                    .at_operation(source)
                    .at_draft_path(path));
                }
                structured_value(
                    terminal,
                    depth,
                    source,
                    child_draft_path(&path, "term", source)?,
                    &mut stack,
                    &mut references,
                )?;
                for (index, expression) in expressions.iter().enumerate().rev() {
                    stack.push(Scan::Expression(
                        expression,
                        depth,
                        source,
                        child_draft_path(&path, &format!("e[{index}]"), source)?,
                    ));
                }
                continue;
            }
            Scan::Expression(expression, depth, source, path) => {
                budget.add(1, source)?;
                if let Some(symbol) = expression.symbol {
                    declare(
                        &mut symbols,
                        &mut kinds,
                        symbol,
                        DraftSymbolKind::Operation,
                        source,
                    )?;
                }
                (&expression.operation, depth, source, path)
            }
            Scan::Inline(operation, depth, source, path) => {
                budget.add(1, source)?;
                (operation, depth, source, path)
            }
        };

        match operation {
            ExpressionKindDraft::ConstUnit
            | ExpressionKindDraft::ConstBool(_)
            | ExpressionKindDraft::ConstI64(_) => {}
            ExpressionKindDraft::ConstBytes(value) => {
                budget.add_byte_literal(value, source)?;
            }
            ExpressionKindDraft::ConstText(value) => {
                budget.add_text_literal(value, source)?;
            }
            ExpressionKindDraft::AddI64 { lhs, rhs }
            | ExpressionKindDraft::LtI64 { lhs, rhs }
            | ExpressionKindDraft::EqualI64 { lhs, rhs }
            | ExpressionKindDraft::AndBool { lhs, rhs }
            | ExpressionKindDraft::OrBool { lhs, rhs }
            | ExpressionKindDraft::BytesEqual { lhs, rhs }
            | ExpressionKindDraft::BytesConcat { lhs, rhs }
            | ExpressionKindDraft::TextEqual { lhs, rhs }
            | ExpressionKindDraft::TextConcat { lhs, rhs } => {
                structured_value(
                    rhs,
                    depth,
                    source,
                    child_draft_path(&path, "rhs", source)?,
                    &mut stack,
                    &mut references,
                )?;
                structured_value(
                    lhs,
                    depth,
                    source,
                    child_draft_path(&path, "lhs", source)?,
                    &mut stack,
                    &mut references,
                )?;
            }
            ExpressionKindDraft::NotBool { value }
            | ExpressionKindDraft::BytesLen { value }
            | ExpressionKindDraft::TextLen { value }
            | ExpressionKindDraft::SequenceLen { value, .. } => {
                structured_value(
                    value,
                    depth,
                    source,
                    child_draft_path(&path, "value", source)?,
                    &mut stack,
                    &mut references,
                )?;
            }
            ExpressionKindDraft::BytesAt { value, index } => {
                for (value, segment) in [(index, "index"), (value, "value")] {
                    structured_value(
                        value,
                        depth,
                        source,
                        child_draft_path(&path, segment, source)?,
                        &mut stack,
                        &mut references,
                    )?;
                }
            }
            ExpressionKindDraft::BytesSlice {
                value,
                start,
                length,
            } => {
                for (value, segment) in [(length, "length"), (start, "start"), (value, "value")] {
                    structured_value(
                        value,
                        depth,
                        source,
                        child_draft_path(&path, segment, source)?,
                        &mut stack,
                        &mut references,
                    )?;
                }
            }
            ExpressionKindDraft::SequenceEmpty { sequence } => reference(
                *sequence,
                DraftReferenceKind::SequenceType,
                source,
                &mut references,
            ),
            ExpressionKindDraft::SequenceGet {
                sequence,
                value,
                index,
            } => {
                reference(
                    *sequence,
                    DraftReferenceKind::SequenceType,
                    source,
                    &mut references,
                );
                for (value, segment) in [(index, "index"), (value, "value")] {
                    structured_value(
                        value,
                        depth,
                        source,
                        child_draft_path(&path, segment, source)?,
                        &mut stack,
                        &mut references,
                    )?;
                }
            }
            ExpressionKindDraft::SequenceAppend {
                sequence,
                value,
                element,
            } => {
                reference(
                    *sequence,
                    DraftReferenceKind::SequenceType,
                    source,
                    &mut references,
                );
                for (value, segment) in [(element, "element"), (value, "value")] {
                    structured_value(
                        value,
                        depth,
                        source,
                        child_draft_path(&path, segment, source)?,
                        &mut stack,
                        &mut references,
                    )?;
                }
            }
            ExpressionKindDraft::SequenceReplace {
                sequence,
                value,
                index,
                element,
            } => {
                reference(
                    *sequence,
                    DraftReferenceKind::SequenceType,
                    source,
                    &mut references,
                );
                for (value, segment) in [(element, "element"), (index, "index"), (value, "value")] {
                    structured_value(
                        value,
                        depth,
                        source,
                        child_draft_path(&path, segment, source)?,
                        &mut stack,
                        &mut references,
                    )?;
                }
            }
            ExpressionKindDraft::SequenceSlice {
                sequence,
                value,
                start,
                end_exclusive,
            } => {
                reference(
                    *sequence,
                    DraftReferenceKind::SequenceType,
                    source,
                    &mut references,
                );
                for (value, segment) in [
                    (end_exclusive, "end_exclusive"),
                    (start, "start"),
                    (value, "value"),
                ] {
                    structured_value(
                        value,
                        depth,
                        source,
                        child_draft_path(&path, segment, source)?,
                        &mut stack,
                        &mut references,
                    )?;
                }
            }
            ExpressionKindDraft::SequenceConcat { sequence, lhs, rhs } => {
                reference(
                    *sequence,
                    DraftReferenceKind::SequenceType,
                    source,
                    &mut references,
                );
                for (value, segment) in [(rhs, "rhs"), (lhs, "lhs")] {
                    structured_value(
                        value,
                        depth,
                        source,
                        child_draft_path(&path, segment, source)?,
                        &mut stack,
                        &mut references,
                    )?;
                }
            }
            ExpressionKindDraft::Call {
                function,
                arguments,
            } => {
                reference(
                    *function,
                    DraftReferenceKind::Function,
                    source,
                    &mut references,
                );
                budget.add(arguments.len(), source)?;
                for (index, value) in arguments.iter().enumerate().rev() {
                    structured_value(
                        value,
                        depth,
                        source,
                        child_draft_path(&path, &format!("arg[{index}]"), source)?,
                        &mut stack,
                        &mut references,
                    )?;
                }
            }
            ExpressionKindDraft::Hole { expected } => {
                type_reference(*expected, source, &mut references)
            }
            ExpressionKindDraft::If {
                condition,
                result,
                then_body,
                else_body,
            } => {
                structured_value(
                    condition,
                    depth,
                    source,
                    child_draft_path(&path, "condition", source)?,
                    &mut stack,
                    &mut references,
                )?;
                type_reference(*result, source, &mut references);
                budget.add(2, source)?;
                stack.push(Scan::Body(
                    &else_body.operations,
                    &else_body.yield_value,
                    depth + 1,
                    source,
                    child_draft_path(&path, "else", source)?,
                ));
                stack.push(Scan::Body(
                    &then_body.operations,
                    &then_body.yield_value,
                    depth + 1,
                    source,
                    child_draft_path(&path, "then", source)?,
                ));
            }
            ExpressionKindDraft::ForI64 {
                start,
                end_exclusive,
                initial,
                carried,
                index_symbol,
                carried_symbol,
                body,
                ..
            } => {
                structured_value(
                    initial,
                    depth,
                    source,
                    child_draft_path(&path, "initial", source)?,
                    &mut stack,
                    &mut references,
                )?;
                structured_value(
                    end_exclusive,
                    depth,
                    source,
                    child_draft_path(&path, "end", source)?,
                    &mut stack,
                    &mut references,
                )?;
                structured_value(
                    start,
                    depth,
                    source,
                    child_draft_path(&path, "start", source)?,
                    &mut stack,
                    &mut references,
                )?;
                type_reference(*carried, source, &mut references);
                declare(
                    &mut symbols,
                    &mut kinds,
                    *index_symbol,
                    DraftSymbolKind::BlockArgument,
                    source,
                )?;
                declare(
                    &mut symbols,
                    &mut kinds,
                    *carried_symbol,
                    DraftSymbolKind::BlockArgument,
                    source,
                )?;
                budget.add(1, source)?;
                stack.push(Scan::Body(
                    &body.operations,
                    &body.yield_value,
                    depth + 1,
                    source,
                    child_draft_path(&path, "body", source)?,
                ));
            }
            ExpressionKindDraft::ConstructProduct { product, fields } => {
                reference(
                    *product,
                    DraftReferenceKind::ProductType,
                    source,
                    &mut references,
                );
                budget.add(fields.len(), source)?;
                for (index, field) in fields.iter().enumerate().rev() {
                    reference(
                        field.field,
                        DraftReferenceKind::ProductField,
                        source,
                        &mut references,
                    );
                    structured_value(
                        &field.value,
                        depth,
                        source,
                        child_draft_path(&path, &format!("field[{index}]"), source)?,
                        &mut stack,
                        &mut references,
                    )?;
                }
            }
            ExpressionKindDraft::ProjectField { value, field } => {
                structured_value(
                    value,
                    depth,
                    source,
                    child_draft_path(&path, "value", source)?,
                    &mut stack,
                    &mut references,
                )?;
                reference(
                    *field,
                    DraftReferenceKind::ProductField,
                    source,
                    &mut references,
                );
            }
            ExpressionKindDraft::ConstructVariant { variant, payload } => {
                reference(
                    *variant,
                    DraftReferenceKind::SumVariant,
                    source,
                    &mut references,
                );
                if let Some(value) = payload {
                    structured_value(
                        value,
                        depth,
                        source,
                        child_draft_path(&path, "payload", source)?,
                        &mut stack,
                        &mut references,
                    )?;
                }
            }
            ExpressionKindDraft::MatchSum {
                scrutinee,
                result,
                arms,
            } => {
                structured_value(
                    scrutinee,
                    depth,
                    source,
                    child_draft_path(&path, "scrutinee", source)?,
                    &mut stack,
                    &mut references,
                )?;
                type_reference(*result, source, &mut references);
                budget.add(arms.len(), source)?;
                let mut body_events = Vec::with_capacity(arms.len());
                for (index, arm) in arms.iter().enumerate() {
                    reference(
                        arm.variant,
                        DraftReferenceKind::SumVariant,
                        source,
                        &mut references,
                    );
                    if let Some(symbol) = arm.payload_symbol {
                        declare(
                            &mut symbols,
                            &mut kinds,
                            symbol,
                            DraftSymbolKind::BlockArgument,
                            source,
                        )?;
                    }
                    budget.add(1, source)?;
                    body_events.push(Scan::Body(
                        &arm.body.operations,
                        &arm.body.yield_value,
                        depth + 1,
                        source,
                        child_draft_path(&path, &format!("arm[{index}]"), source)?,
                    ));
                }
                stack.extend(body_events.into_iter().rev());
            }
        }
    }
    for (symbol, expected, source) in references {
        let Some(actual) = kinds.get(&symbol).copied() else {
            return Err(LkError::new(
                ErrorCode::InvalidDraftSymbol,
                "structured draft references an undeclared local symbol",
            )
            .at_operation(source)
            .for_symbol(symbol));
        };
        if !expected.accepts(actual) {
            return Err(LkError::new(
                ErrorCode::WrongKind,
                "transaction-local reference has the wrong declared category",
            )
            .at_operation(source)
            .for_symbol(symbol));
        }
    }
    Ok(symbols)
}

fn validate_draft_value(value: &ValueDraft, source: usize) -> Result<()> {
    if let ValueDraft::OperationResult { operation, output } = value
        && *output != 0
    {
        let mut error = LkError::new(
            ErrorCode::InvalidOperand,
            "structured operation result output must be zero for the closed single-result schema",
        )
        .at_operation(source);
        if let NodeTarget::Draft(symbol) = operation {
            error = error.for_symbol(*symbol);
        }
        return Err(error);
    }
    Ok(())
}

impl Workspace {
    pub(crate) fn prepare_transaction(
        &self,
        request: &ApplyTransactionRequest,
    ) -> Result<PreparedTransaction> {
        let transaction = &request.transaction;
        if transaction.workspace != self.id() {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "transaction names a different workspace",
            )
            .for_workspace(self.id()));
        }
        if transaction.mode == TransactionMode::ValidateOnly
            && transaction.idempotency_key.is_some()
        {
            return Err(LkError::new(
                ErrorCode::InvalidOperand,
                "validate-only transactions cannot carry an idempotency key",
            )
            .for_workspace(self.id())
            .at_revision(transaction.base_revision));
        }
        if transaction.base_revision != self.head_revision() {
            return Err(LkError::new(
                ErrorCode::RevisionConflict,
                "transaction base revision is not the current head",
            )
            .for_workspace(self.id())
            .at_revision(transaction.base_revision));
        }
        if transaction.operations.is_empty() {
            return Err(LkError::new(
                ErrorCode::NoChange,
                "empty transactions do not publish revisions",
            )
            .for_workspace(self.id())
            .at_revision(transaction.base_revision));
        }
        let base = self.snapshot(transaction.base_revision)?;
        let expanded = expand_transaction(base, &transaction.operations)?;
        let (allocations, next_serial) = allocate_symbols(
            base,
            &expanded.edits,
            &expanded.edit_sources,
            &expanded.explicit_symbols,
            &expanded.anonymous_paths,
        )?;
        validate_response_spec(&request.response, &allocations, &expanded.explicit_symbols)?;
        let mut nodes = base.nodes.clone();
        let mut tombstones = base.tombstones.clone();
        let mut provenance = BTreeMap::new();

        for (operation, source) in expanded.edits.iter().zip(&expanded.edit_sources) {
            if let Err(mut error) = apply_operation(
                base,
                &mut nodes,
                &mut tombstones,
                &allocations,
                &expanded.nominal_catalogue,
                operation,
            ) {
                if error.operation_index.is_none() {
                    error = error.at_operation(*source);
                }
                if error.draft_symbol.is_none() && error.draft_path.is_none() {
                    error = decorate_created_edit_error(
                        error,
                        operation,
                        &expanded.explicit_symbols,
                        &expanded.anonymous_paths,
                    );
                }
                return Err(error);
            }
            record_edit_provenance(
                operation,
                *source,
                &allocations,
                &expanded.anonymous_paths,
                &mut provenance,
            )?;
        }

        if nodes == base.nodes && tombstones == base.tombstones && next_serial == base.next_serial {
            return Err(LkError::new(
                ErrorCode::NoChange,
                "transaction produced no canonical state change",
            )
            .for_workspace(self.id())
            .at_revision(transaction.base_revision));
        }
        let revision = transaction.base_revision.next().ok_or_else(|| {
            LkError::new(
                ErrorCode::RevisionConflict,
                "workspace revision is exhausted",
            )
            .for_workspace(self.id())
            .at_revision(transaction.base_revision)
        })?;
        let candidate = match Snapshot::from_parts(
            self.id(),
            revision,
            base.root,
            next_serial,
            tombstones,
            nodes,
        ) {
            Ok(candidate) => Arc::new(candidate),
            Err(mut error) => {
                if error.operation_index.is_none() {
                    let source = error_source_from_provenance(&error, &provenance)
                        .unwrap_or_else(|| transaction.operations.len().saturating_sub(1));
                    error = error.at_operation(source);
                    if error.draft_symbol.is_none() {
                        error.draft_symbol = error_symbol_from_provenance(
                            &error,
                            &provenance,
                            &allocations,
                            &expanded.explicit_symbols,
                        );
                        if error.draft_symbol.is_none()
                            && let Some(path) = error_path_from_provenance(&error, &provenance)
                        {
                            error = error.at_draft_path(path);
                        }
                        if error.draft_symbol.is_none() && error.draft_path.is_none() {
                            error = error.at_draft_path(format!("operations[{source}]"));
                        }
                    }
                }
                return Err(error);
            }
        };
        if let Err(mut error) = crate::graph::validate_history_transition(base, &candidate) {
            error.code = ErrorCode::InvalidOperand;
            if error.operation_index.is_none() {
                let source = error_source_from_provenance(&error, &provenance)
                    .unwrap_or_else(|| transaction.operations.len().saturating_sub(1));
                error = error.at_operation(source);
                if error.draft_symbol.is_none() {
                    error.draft_symbol = error_symbol_from_provenance(
                        &error,
                        &provenance,
                        &allocations,
                        &expanded.explicit_symbols,
                    );
                    if error.draft_symbol.is_none()
                        && let Some(path) = error_path_from_provenance(&error, &provenance)
                    {
                        error = error.at_draft_path(path);
                    }
                    if error.draft_symbol.is_none() && error.draft_path.is_none() {
                        error = error.at_draft_path(format!("operations[{source}]"));
                    }
                }
            }
            return Err(error);
        }
        let semantic_diff = diff::between(base, &candidate);
        let blockers_before = query::workspace_blockers(base);
        let blockers_after = query::workspace_blockers(&candidate);
        let returned_bindings = request
            .response
            .return_symbols
            .iter()
            .map(|symbol| allocated(&allocations, *symbol).map(|node| (*symbol, node)))
            .collect::<Result<Vec<_>>>()?;
        let receipt = TransactionReceipt {
            workspace: self.id(),
            base_revision: transaction.base_revision,
            revision,
            hash: candidate.hash(),
            published: transaction.mode == TransactionMode::Commit,
            created_count: u64::try_from(allocations.values().filter(|id| id.is_durable()).count())
                .map_err(|_| {
                    LkError::new(
                        ErrorCode::PolicyExceeded,
                        "created durable identity count does not fit receipt representation",
                    )
                })?,
            returned_bindings,
            change_count: semantic_diff.change_count(),
            change_digest: semantic_diff.digest,
            complete_before: blockers_before.is_empty(),
            complete_after: blockers_after.is_empty(),
            blocker_count_before: u64::try_from(blockers_before.len()).map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "prior blocker count does not fit receipt representation",
                )
            })?,
            blocker_count_after: u64::try_from(blockers_after.len()).map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "result blocker count does not fit receipt representation",
                )
            })?,
        };
        Ok(PreparedTransaction {
            snapshot: candidate,
            receipt,
        })
    }
}

#[derive(Clone)]
struct NodeProvenance {
    source: usize,
    offending_use: bool,
    draft_path: Option<String>,
}

fn record_edit_provenance(
    edit: &CanonicalEdit,
    source: usize,
    allocations: &BTreeMap<DraftSymbol, NodeId>,
    anonymous_paths: &BTreeMap<DraftSymbol, String>,
    provenance: &mut BTreeMap<NodeId, NodeProvenance>,
) -> Result<()> {
    let (target, offending_use, draft_path) = match edit {
        CanonicalEdit::CreateOperation { symbol, .. } => (
            Some(allocated(allocations, *symbol)?),
            true,
            anonymous_paths.get(symbol).cloned(),
        ),
        CanonicalEdit::CreatePackage { symbol, .. }
        | CanonicalEdit::CreateBuildTarget { symbol, .. }
        | CanonicalEdit::CreateModule { symbol, .. }
        | CanonicalEdit::CreateProductType { symbol, .. }
        | CanonicalEdit::CreateProductField { symbol, .. }
        | CanonicalEdit::CreateSumType { symbol, .. }
        | CanonicalEdit::CreateSequenceType { symbol, .. }
        | CanonicalEdit::CreateSumVariant { symbol, .. }
        | CanonicalEdit::CreateFunction { symbol, .. }
        | CanonicalEdit::CreateParameter { symbol, .. }
        | CanonicalEdit::CreateRegion { symbol, .. }
        | CanonicalEdit::CreateBlock { symbol, .. }
        | CanonicalEdit::CreateBlockArgument { symbol, .. }
        | CanonicalEdit::CreateMatchPayloadArgument { symbol, .. } => {
            (Some(allocated(allocations, *symbol)?), false, None)
        }
        CanonicalEdit::ReplaceOperation { operation, .. }
        | CanonicalEdit::ReplaceOperand { operation, .. } => (
            Some(resolve_for_provenance(*operation, allocations)?),
            true,
            None,
        ),
        CanonicalEdit::RefineHole { hole, .. } => (
            Some(resolve_for_provenance(*hole, allocations)?),
            true,
            None,
        ),
        CanonicalEdit::SetFunctionBody { function, .. } => (
            Some(resolve_for_provenance(*function, allocations)?),
            false,
            None,
        ),
        CanonicalEdit::ClearFunctionBody { function } => (
            Some(resolve_for_provenance(*function, allocations)?),
            false,
            None,
        ),
        CanonicalEdit::SetEntryFunction { package, .. } => (
            Some(resolve_for_provenance(*package, allocations)?),
            false,
            None,
        ),
        CanonicalEdit::RenameNode { node, .. } => (
            Some(resolve_for_provenance(*node, allocations)?),
            false,
            None,
        ),
        CanonicalEdit::ReplaceBuildTarget { target, .. }
        | CanonicalEdit::AddReleaseTargetExport { target, .. }
        | CanonicalEdit::SetReleaseTargetExport { target, .. }
        | CanonicalEdit::SetApplicationQueryBoundary { target, .. }
        | CanonicalEdit::AddApplicationTargetTest { target, .. } => (Some(*target), false, None),
        CanonicalEdit::DeleteOwnedSubtree { root } => (
            Some(resolve_for_provenance(*root, allocations)?),
            false,
            None,
        ),
    };
    if let Some(target) = target {
        provenance.insert(
            target,
            NodeProvenance {
                source,
                offending_use,
                draft_path,
            },
        );
    }
    Ok(())
}

fn resolve_for_provenance(
    target: NodeTarget,
    allocations: &BTreeMap<DraftSymbol, NodeId>,
) -> Result<NodeId> {
    match target {
        NodeTarget::Existing(node) => Ok(node),
        NodeTarget::Draft(symbol) => allocated(allocations, symbol),
    }
}

fn preferred_error_provenance<'a>(
    error: &LkError,
    provenance: &'a BTreeMap<NodeId, NodeProvenance>,
) -> Option<(NodeId, &'a NodeProvenance)> {
    error
        .related
        .iter()
        .find_map(|node| {
            provenance
                .get(node)
                .filter(|fact| fact.offending_use)
                .map(|fact| (*node, fact))
        })
        .or_else(|| {
            error.target.and_then(|node| {
                provenance
                    .get(&node)
                    .filter(|fact| fact.offending_use)
                    .map(|fact| (node, fact))
            })
        })
        .or_else(|| {
            error
                .target
                .and_then(|node| provenance.get(&node).map(|fact| (node, fact)))
        })
        .or_else(|| {
            error
                .related
                .iter()
                .find_map(|node| provenance.get(node).map(|fact| (*node, fact)))
        })
}

fn error_source_from_provenance(
    error: &LkError,
    provenance: &BTreeMap<NodeId, NodeProvenance>,
) -> Option<usize> {
    preferred_error_provenance(error, provenance).map(|(_, fact)| fact.source)
}

fn error_symbol_from_provenance(
    error: &LkError,
    provenance: &BTreeMap<NodeId, NodeProvenance>,
    allocations: &BTreeMap<DraftSymbol, NodeId>,
    explicit_symbols: &BTreeSet<DraftSymbol>,
) -> Option<DraftSymbol> {
    let (node, _) = preferred_error_provenance(error, provenance)?;
    allocations.iter().find_map(|(symbol, allocated)| {
        (*allocated == node && explicit_symbols.contains(symbol)).then_some(*symbol)
    })
}

fn error_path_from_provenance(
    error: &LkError,
    provenance: &BTreeMap<NodeId, NodeProvenance>,
) -> Option<String> {
    preferred_error_provenance(error, provenance)?
        .1
        .draft_path
        .clone()
}

fn decorate_created_edit_error(
    error: LkError,
    edit: &CanonicalEdit,
    explicit_symbols: &BTreeSet<DraftSymbol>,
    anonymous_paths: &BTreeMap<DraftSymbol, String>,
) -> LkError {
    let Some(symbol) = canonical_created_symbol(edit) else {
        return error;
    };
    if explicit_symbols.contains(&symbol) {
        error.for_symbol(symbol)
    } else if let Some(path) = anonymous_paths.get(&symbol) {
        error.at_draft_path(path.clone())
    } else {
        error
    }
}

fn validate_response_spec(
    response: &TransactionResponseSpec,
    allocations: &BTreeMap<DraftSymbol, NodeId>,
    explicit_symbols: &BTreeSet<DraftSymbol>,
) -> Result<()> {
    if response.return_symbols.len() > MAX_RETURNED_BINDINGS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "selected return symbols exceed transaction response policy",
        ));
    }
    let mut selected = BTreeSet::new();
    for symbol in &response.return_symbols {
        if !selected.insert(*symbol) {
            return Err(LkError::new(
                ErrorCode::InvalidDraftSymbol,
                "selected return symbols must be unique",
            )
            .for_symbol(*symbol));
        }
        if !explicit_symbols.contains(symbol) || !allocations.contains_key(symbol) {
            return Err(LkError::new(
                ErrorCode::InvalidDraftSymbol,
                "selected return symbol is not declared by this transaction",
            )
            .for_symbol(*symbol));
        }
    }
    Ok(())
}

fn allocation_error(
    code: ErrorCode,
    message: impl Into<String>,
    source: usize,
    symbol: DraftSymbol,
    explicit_symbols: &BTreeSet<DraftSymbol>,
    anonymous_paths: &BTreeMap<DraftSymbol, String>,
) -> LkError {
    let error = LkError::new(code, message).at_operation(source);
    if explicit_symbols.contains(&symbol) {
        error.for_symbol(symbol)
    } else if let Some(path) = anonymous_paths.get(&symbol) {
        error.at_draft_path(path.clone())
    } else {
        error.at_draft_path(format!("operations[{source}]"))
    }
}

fn allocate_symbols(
    base: &Snapshot,
    operations: &[CanonicalEdit],
    edit_sources: &[usize],
    explicit_symbols: &BTreeSet<DraftSymbol>,
    anonymous_paths: &BTreeMap<DraftSymbol, String>,
) -> Result<(BTreeMap<DraftSymbol, NodeId>, u64)> {
    let mut allocations = BTreeMap::new();
    let mut owning_functions = BTreeMap::new();
    let mut next_local = BTreeMap::<NodeId, u32>::new();
    for (id, _) in base.nodes() {
        let (Some(function_serial), Some(ordinal)) =
            (id.local_function_serial(), id.local_ordinal())
        else {
            continue;
        };
        let function = NodeId::new(base.workspace(), function_serial).map_err(|error| {
            LkError::new(
                ErrorCode::InvalidContainment,
                format!("stored local function domain is invalid: {error}"),
            )
            .for_node(id)
        })?;
        let following = ordinal.checked_add(1).ok_or_else(|| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                "function-local identity ordinal is exhausted",
            )
            .for_node(id)
        })?;
        next_local
            .entry(function)
            .and_modify(|next| *next = (*next).max(following))
            .or_insert(following);
    }
    let mut next = base.next_serial;
    for (operation, source) in operations.iter().zip(edit_sources) {
        if let CanonicalEdit::ClearFunctionBody { function } = operation {
            let function = match function {
                NodeTarget::Existing(function) => *function,
                NodeTarget::Draft(symbol) => {
                    owning_functions.get(symbol).copied().ok_or_else(|| {
                        LkError::new(
                            ErrorCode::InvalidContainment,
                            "replacement function has no resolved durable identity",
                        )
                        .at_operation(*source)
                        .for_symbol(*symbol)
                    })?
                }
            };
            next_local.insert(function, 1);
            continue;
        }
        let Some(symbol) = canonical_created_symbol(operation) else {
            continue;
        };
        if allocations.contains_key(&symbol) {
            return Err(allocation_error(
                ErrorCode::DuplicateDraftSymbol,
                "transaction-local symbol is declared more than once",
                *source,
                symbol,
                explicit_symbols,
                anonymous_paths,
            ));
        }
        let local_owner = local_owner_target(operation);
        let id = if let Some(owner) = local_owner {
            let function =
                resolve_owning_function(base, owner, &allocations, &owning_functions, *source)?;
            let ordinal = *next_local.entry(function).or_insert(1);
            let id = NodeId::new_function_local(base.workspace(), function, ordinal).map_err(
                |error| {
                    allocation_error(
                        ErrorCode::PolicyExceeded,
                        format!("function-local identity allocation failed: {error}"),
                        *source,
                        symbol,
                        explicit_symbols,
                        anonymous_paths,
                    )
                },
            )?;
            let following = ordinal.checked_add(1).ok_or_else(|| {
                allocation_error(
                    ErrorCode::PolicyExceeded,
                    "function-local identity ordinal is exhausted",
                    *source,
                    symbol,
                    explicit_symbols,
                    anonymous_paths,
                )
            })?;
            next_local.insert(function, following);
            owning_functions.insert(symbol, function);
            id
        } else {
            let id = NodeId::new(base.workspace(), next).map_err(|error| {
                allocation_error(
                    ErrorCode::PolicyExceeded,
                    format!("durable identity allocation failed: {error}"),
                    *source,
                    symbol,
                    explicit_symbols,
                    anonymous_paths,
                )
            })?;
            next = next.checked_add(1).ok_or_else(|| {
                allocation_error(
                    ErrorCode::PolicyExceeded,
                    "durable identity serial is exhausted",
                    *source,
                    symbol,
                    explicit_symbols,
                    anonymous_paths,
                )
            })?;
            if matches!(operation, CanonicalEdit::CreateFunction { .. }) {
                owning_functions.insert(symbol, id);
            } else if let CanonicalEdit::CreateOperation { block, .. } = operation {
                let function = resolve_owning_function(
                    base,
                    *block,
                    &allocations,
                    &owning_functions,
                    *source,
                )?;
                owning_functions.insert(symbol, function);
            }
            id
        };
        allocations.insert(symbol, id);
    }
    Ok((allocations, next))
}

fn local_owner_target(operation: &CanonicalEdit) -> Option<NodeTarget> {
    match operation {
        CanonicalEdit::CreateRegion { owner, .. } => Some(*owner),
        CanonicalEdit::CreateBlock { region, .. } => Some(*region),
        CanonicalEdit::CreateBlockArgument { block, .. }
        | CanonicalEdit::CreateMatchPayloadArgument { block, .. } => Some(*block),
        CanonicalEdit::CreateOperation {
            block: _,
            operation: OperationDraft::Hole { .. },
            ..
        } => None,
        CanonicalEdit::CreateOperation { block, .. } => Some(*block),
        _ => None,
    }
}

fn resolve_owning_function(
    base: &Snapshot,
    target: NodeTarget,
    allocations: &BTreeMap<DraftSymbol, NodeId>,
    owning_functions: &BTreeMap<DraftSymbol, NodeId>,
    source: usize,
) -> Result<NodeId> {
    match target {
        NodeTarget::Draft(symbol) => owning_functions.get(&symbol).copied().ok_or_else(|| {
            LkError::new(
                ErrorCode::InvalidContainment,
                "function-local owner has no resolved owning function",
            )
            .at_operation(source)
            .for_symbol(symbol)
        }),
        NodeTarget::Existing(id) => owning_function_in_snapshot(base, id).map_err(|error| {
            if error.operation_index.is_none() {
                error.at_operation(source)
            } else {
                error
            }
        }),
    }
    .and_then(|function| {
        if function.is_durable() {
            Ok(function)
        } else {
            Err(LkError::new(
                ErrorCode::InvalidContainment,
                "function-local domain owner must be a durable function",
            )
            .at_operation(source)
            .for_node(function))
        }
    })
    .map_err(|error| {
        if let NodeTarget::Draft(symbol) = target
            && let Some(id) = allocations.get(&symbol)
        {
            error.with_related([*id])
        } else {
            error
        }
    })
}

fn owning_function_in_snapshot(snapshot: &Snapshot, start: NodeId) -> Result<NodeId> {
    let mut current = start;
    let mut remaining = snapshot.node_count().saturating_add(1);
    loop {
        if remaining == 0 {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "owner chain does not reach a function",
            )
            .for_node(start));
        }
        remaining -= 1;
        let node = snapshot.node(current)?;
        if matches!(node, Node::Function { .. }) {
            return Ok(current);
        }
        current = node.owner().ok_or_else(|| {
            LkError::new(
                ErrorCode::InvalidContainment,
                "function-local owner chain reached a non-function root",
            )
            .for_node(start)
        })?;
    }
}

fn canonical_created_symbol(operation: &CanonicalEdit) -> Option<DraftSymbol> {
    match operation {
        CanonicalEdit::CreatePackage { symbol, .. }
        | CanonicalEdit::CreateBuildTarget { symbol, .. }
        | CanonicalEdit::CreateModule { symbol, .. }
        | CanonicalEdit::CreateProductType { symbol, .. }
        | CanonicalEdit::CreateProductField { symbol, .. }
        | CanonicalEdit::CreateSumType { symbol, .. }
        | CanonicalEdit::CreateSequenceType { symbol, .. }
        | CanonicalEdit::CreateSumVariant { symbol, .. }
        | CanonicalEdit::CreateFunction { symbol, .. }
        | CanonicalEdit::CreateParameter { symbol, .. }
        | CanonicalEdit::CreateRegion { symbol, .. }
        | CanonicalEdit::CreateBlock { symbol, .. }
        | CanonicalEdit::CreateBlockArgument { symbol, .. }
        | CanonicalEdit::CreateMatchPayloadArgument { symbol, .. }
        | CanonicalEdit::CreateOperation { symbol, .. } => Some(*symbol),
        _ => None,
    }
}

fn apply_operation(
    base: &Snapshot,
    nodes: &mut BTreeMap<NodeId, Node>,
    tombstones: &mut BTreeSet<u64>,
    allocations: &BTreeMap<DraftSymbol, NodeId>,
    nominal_catalogue: &StagedNominalCatalogue,
    operation: &CanonicalEdit,
) -> Result<()> {
    match operation {
        CanonicalEdit::CreatePackage { symbol, name } => {
            let id = allocated(allocations, *symbol)?;
            insert_new(
                nodes,
                id,
                Node::Package {
                    owner: base.root,
                    name: name.clone(),
                    modules: Vec::new(),
                    entry: None,
                },
            )?;
            let root = require_kind_mut(nodes, base.root, NodeKind::WorkspaceRoot)?;
            let Node::WorkspaceRoot { packages, .. } = root else {
                return Err(invariant("workspace root kind changed during staging"));
            };
            packages.push(id);
        }
        CanonicalEdit::CreateBuildTarget {
            symbol,
            name,
            definition,
        } => {
            let id = allocated(allocations, *symbol)?;
            insert_new(
                nodes,
                id,
                Node::BuildTarget {
                    owner: base.root,
                    name: name.clone(),
                    definition: definition.clone(),
                },
            )?;
            let root = require_kind_mut(nodes, base.root, NodeKind::WorkspaceRoot)?;
            let Node::WorkspaceRoot { targets, .. } = root else {
                return Err(invariant("workspace root kind changed during staging"));
            };
            targets.push(id);
        }
        CanonicalEdit::ReplaceBuildTarget { target, definition } => {
            let Node::BuildTarget {
                definition: current,
                ..
            } = require_kind_mut(nodes, *target, NodeKind::BuildTarget)?
            else {
                return Err(invariant("build target kind changed during staging"));
            };
            if current.kind() != definition.kind() {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "build-target replacement cannot change target kind",
                )
                .for_node(*target));
            }
            *current = definition.clone();
        }
        CanonicalEdit::AddReleaseTargetExport { target, name, item } => {
            let Node::BuildTarget { definition, .. } =
                require_kind_mut(nodes, *target, NodeKind::BuildTarget)?
            else {
                return Err(invariant("build target kind changed during staging"));
            };
            let crate::target::BuildTargetDefinition::Release(release) = definition else {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "release export edit requires a release target",
                )
                .for_node(*target));
            };
            if release.exports.iter().any(|export| export.name == *name) {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "release target already contains the named export",
                )
                .for_node(*target));
            }
            release.exports.push(crate::release::ReleaseExportRequest {
                name: name.clone(),
                target: *item,
            });
        }
        CanonicalEdit::SetReleaseTargetExport { target, name, item } => {
            let Node::BuildTarget { definition, .. } =
                require_kind_mut(nodes, *target, NodeKind::BuildTarget)?
            else {
                return Err(invariant("build target kind changed during staging"));
            };
            let crate::target::BuildTargetDefinition::Release(release) = definition else {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "release export edit requires a release target",
                )
                .for_node(*target));
            };
            let mut matches = release
                .exports
                .iter_mut()
                .filter(|export| export.name == *name);
            let export = matches.next().ok_or_else(|| {
                LkError::new(
                    ErrorCode::NodeNotFound,
                    "release target does not contain the named export",
                )
                .for_node(*target)
            })?;
            if matches.next().is_some() {
                return Err(invariant("release target contains duplicate export names"));
            }
            export.target = *item;
        }
        CanonicalEdit::SetApplicationQueryBoundary {
            target,
            query_entry,
            query,
        } => {
            let Node::BuildTarget { definition, .. } =
                require_kind_mut(nodes, *target, NodeKind::BuildTarget)?
            else {
                return Err(invariant("build target kind changed during staging"));
            };
            let crate::target::BuildTargetDefinition::Application(application) = definition else {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "query-boundary edit requires an application target",
                )
                .for_node(*target));
            };
            let crate::target::TargetInvocationProfile::Stateful(profile) =
                &mut application.profile
            else {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "query-boundary edit requires a stateful application target",
                )
                .for_node(*target));
            };
            profile.query_entry = *query_entry;
            profile.query = *query;
        }
        CanonicalEdit::AddApplicationTargetTest { target, case } => {
            let Node::BuildTarget { definition, .. } =
                require_kind_mut(nodes, *target, NodeKind::BuildTarget)?
            else {
                return Err(invariant("build target kind changed during staging"));
            };
            let crate::target::BuildTargetDefinition::Application(application) = definition else {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "application-case edit requires an application target",
                )
                .for_node(*target));
            };
            if application.tests.iter().any(|test| test.name == case.name) {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "application target already contains the named case",
                )
                .for_node(*target));
            }
            application.tests.push(case.clone());
        }
        CanonicalEdit::CreateModule {
            symbol,
            package,
            name,
        } => {
            let id = allocated(allocations, *symbol)?;
            let package = resolve(*package, allocations, base.workspace())?;
            require_kind(nodes, package, NodeKind::Package)?;
            insert_new(
                nodes,
                id,
                Node::Module {
                    owner: package,
                    name: name.clone(),
                    types: Vec::new(),
                    functions: Vec::new(),
                },
            )?;
            let Node::Package { modules, .. } =
                require_kind_mut(nodes, package, NodeKind::Package)?
            else {
                return Err(invariant("package kind changed during staging"));
            };
            modules.push(id);
        }
        CanonicalEdit::CreateProductType {
            symbol,
            module,
            name,
        } => {
            let id = allocated(allocations, *symbol)?;
            let module = resolve(*module, allocations, base.workspace())?;
            require_kind(nodes, module, NodeKind::Module)?;
            insert_new(
                nodes,
                id,
                Node::ProductType {
                    owner: module,
                    name: name.clone(),
                    fields: Vec::new(),
                },
            )?;
            let Node::Module { types, .. } = require_kind_mut(nodes, module, NodeKind::Module)?
            else {
                return Err(invariant("module kind changed during staging"));
            };
            types.push(id);
        }
        CanonicalEdit::CreateProductField {
            symbol,
            product,
            name,
            ty,
        } => {
            let id = allocated(allocations, *symbol)?;
            let product = resolve(*product, allocations, base.workspace())?;
            let ordinal = match require_kind(nodes, product, NodeKind::ProductType)? {
                Node::ProductType { fields, .. } => u32::try_from(fields.len()).map_err(|_| {
                    LkError::new(
                        ErrorCode::PolicyExceeded,
                        "product field ordinal exceeds representation",
                    )
                    .for_node(product)
                })?,
                _ => unreachable!(),
            };
            let ty = resolve_type_draft(*ty, allocations, base.workspace())?;
            insert_new(
                nodes,
                id,
                Node::ProductField {
                    owner: product,
                    ordinal,
                    name: name.clone(),
                    ty,
                },
            )?;
            let Node::ProductType { fields, .. } =
                require_kind_mut(nodes, product, NodeKind::ProductType)?
            else {
                unreachable!()
            };
            fields.push(id);
        }
        CanonicalEdit::CreateSumType {
            symbol,
            module,
            name,
        } => {
            let id = allocated(allocations, *symbol)?;
            let module = resolve(*module, allocations, base.workspace())?;
            require_kind(nodes, module, NodeKind::Module)?;
            insert_new(
                nodes,
                id,
                Node::SumType {
                    owner: module,
                    name: name.clone(),
                    variants: Vec::new(),
                },
            )?;
            let Node::Module { types, .. } = require_kind_mut(nodes, module, NodeKind::Module)?
            else {
                return Err(invariant("module kind changed during staging"));
            };
            types.push(id);
        }
        CanonicalEdit::CreateSequenceType {
            symbol,
            module,
            name,
            element,
        } => {
            let id = allocated(allocations, *symbol)?;
            let module = resolve(*module, allocations, base.workspace())?;
            require_kind(nodes, module, NodeKind::Module)?;
            let element = resolve_type_draft(*element, allocations, base.workspace())?;
            insert_new(
                nodes,
                id,
                Node::SequenceType {
                    owner: module,
                    name: name.clone(),
                    element,
                },
            )?;
            let Node::Module { types, .. } = require_kind_mut(nodes, module, NodeKind::Module)?
            else {
                return Err(invariant("module kind changed during staging"));
            };
            types.push(id);
        }
        CanonicalEdit::CreateSumVariant {
            symbol,
            sum,
            name,
            payload,
        } => {
            let id = allocated(allocations, *symbol)?;
            let sum = resolve(*sum, allocations, base.workspace())?;
            let ordinal = match require_kind(nodes, sum, NodeKind::SumType)? {
                Node::SumType { variants, .. } => u32::try_from(variants.len()).map_err(|_| {
                    LkError::new(
                        ErrorCode::PolicyExceeded,
                        "sum variant ordinal exceeds representation",
                    )
                    .for_node(sum)
                })?,
                _ => unreachable!(),
            };
            let payload = payload
                .map(|ty| resolve_type_draft(ty, allocations, base.workspace()))
                .transpose()?;
            insert_new(
                nodes,
                id,
                Node::SumVariant {
                    owner: sum,
                    ordinal,
                    name: name.clone(),
                    payload,
                },
            )?;
            let Node::SumType { variants, .. } = require_kind_mut(nodes, sum, NodeKind::SumType)?
            else {
                unreachable!()
            };
            variants.push(id);
        }
        CanonicalEdit::CreateFunction {
            symbol,
            module,
            name,
            result,
        } => {
            let id = allocated(allocations, *symbol)?;
            let module = resolve(*module, allocations, base.workspace())?;
            require_kind(nodes, module, NodeKind::Module)?;
            insert_new(
                nodes,
                id,
                Node::Function {
                    owner: module,
                    name: name.clone(),
                    parameters: Vec::new(),
                    result: resolve_type_draft(*result, allocations, base.workspace())?,
                    body: None,
                },
            )?;
            let Node::Module { functions, .. } = require_kind_mut(nodes, module, NodeKind::Module)?
            else {
                return Err(invariant("module kind changed during staging"));
            };
            functions.push(id);
        }
        CanonicalEdit::CreateParameter {
            symbol,
            function,
            name,
            ty,
        } => {
            let id = allocated(allocations, *symbol)?;
            let function = resolve(*function, allocations, base.workspace())?;
            let ordinal = match require_kind(nodes, function, NodeKind::Function)? {
                Node::Function { parameters, .. } => {
                    u32::try_from(parameters.len()).map_err(|_| {
                        LkError::new(
                            ErrorCode::PolicyExceeded,
                            "parameter ordinal exceeds protocol representation",
                        )
                        .for_node(function)
                    })?
                }
                _ => return Err(invariant("function kind changed during staging")),
            };
            insert_new(
                nodes,
                id,
                Node::Parameter {
                    owner: function,
                    ordinal,
                    name: name.clone(),
                    ty: resolve_type_draft(*ty, allocations, base.workspace())?,
                },
            )?;
            let Node::Function { parameters, .. } =
                require_kind_mut(nodes, function, NodeKind::Function)?
            else {
                return Err(invariant("function kind changed during staging"));
            };
            parameters.push(id);
        }
        CanonicalEdit::CreateRegion { symbol, owner } => {
            let id = allocated(allocations, *symbol)?;
            let owner = resolve(*owner, allocations, base.workspace())?;
            match require_kind(
                nodes,
                owner,
                nodes
                    .get(&owner)
                    .map(Node::kind)
                    .unwrap_or(NodeKind::WorkspaceRoot),
            )? {
                Node::Function { .. } | Node::Operation { .. } => {}
                node => {
                    return Err(LkError::new(
                        ErrorCode::WrongKind,
                        "region owner must be a function or structured operation",
                    )
                    .for_node(owner)
                    .with_kinds(NodeKind::Operation, node.kind()));
                }
            }
            insert_new(
                nodes,
                id,
                Node::Region {
                    owner,
                    blocks: Vec::new(),
                },
            )?;
        }
        CanonicalEdit::CreateBlock { symbol, region } => {
            let id = allocated(allocations, *symbol)?;
            let region = resolve(*region, allocations, base.workspace())?;
            require_kind(nodes, region, NodeKind::Region)?;
            insert_new(
                nodes,
                id,
                Node::Block {
                    owner: region,
                    arguments: Vec::new(),
                    operations: Vec::new(),
                    terminator: None,
                },
            )?;
            let Node::Region { blocks, .. } = require_kind_mut(nodes, region, NodeKind::Region)?
            else {
                return Err(invariant("region kind changed during staging"));
            };
            blocks.push(id);
        }
        CanonicalEdit::CreateBlockArgument { symbol, block, ty } => {
            let id = allocated(allocations, *symbol)?;
            let block = resolve(*block, allocations, base.workspace())?;
            let ordinal = match require_kind(nodes, block, NodeKind::Block)? {
                Node::Block { arguments, .. } => u32::try_from(arguments.len()).map_err(|_| {
                    LkError::new(
                        ErrorCode::PolicyExceeded,
                        "block argument ordinal exceeds representation",
                    )
                    .for_node(block)
                })?,
                _ => unreachable!(),
            };
            insert_new(
                nodes,
                id,
                Node::BlockArgument {
                    owner: block,
                    ordinal,
                    ty: resolve_type_draft(*ty, allocations, base.workspace())?,
                },
            )?;
            let Node::Block { arguments, .. } = require_kind_mut(nodes, block, NodeKind::Block)?
            else {
                unreachable!()
            };
            arguments.push(id);
        }
        CanonicalEdit::CreateMatchPayloadArgument {
            symbol,
            block,
            variant,
        } => {
            let id = allocated(allocations, *symbol)?;
            let block = resolve(*block, allocations, base.workspace())?;
            let variant_target = *variant;
            let variant = resolve(variant_target, allocations, base.workspace())?;
            let payload = nominal_catalogue
                .variants
                .get(&variant_target)
                .and_then(|(_, payload)| *payload)
                .ok_or_else(|| {
                    LkError::new(
                        ErrorCode::InvalidOperand,
                        "nullary match arm cannot declare a payload symbol",
                    )
                    .for_node(variant)
                })?;
            let payload = resolve_type_draft(payload, allocations, base.workspace())?;
            let ordinal = match require_kind(nodes, block, NodeKind::Block)? {
                Node::Block { arguments, .. } => u32::try_from(arguments.len()).map_err(|_| {
                    LkError::new(
                        ErrorCode::PolicyExceeded,
                        "block argument ordinal exceeds representation",
                    )
                    .for_node(block)
                })?,
                _ => unreachable!(),
            };
            insert_new(
                nodes,
                id,
                Node::BlockArgument {
                    owner: block,
                    ordinal,
                    ty: payload,
                },
            )?;
            let Node::Block { arguments, .. } = require_kind_mut(nodes, block, NodeKind::Block)?
            else {
                unreachable!()
            };
            arguments.push(id);
        }
        CanonicalEdit::CreateOperation {
            symbol,
            block,
            before,
            operation,
        } => {
            let id = allocated(allocations, *symbol)?;
            let block = resolve(*block, allocations, base.workspace())?;
            require_kind(nodes, block, NodeKind::Block)?;
            let operation =
                resolve_operation(operation, allocations, base.workspace(), nominal_catalogue)?;
            let terminator = operation.is_terminator();
            insert_new(
                nodes,
                id,
                Node::Operation {
                    owner: block,
                    operation,
                },
            )?;
            let before = before
                .map(|target| resolve(target, allocations, base.workspace()))
                .transpose()?;
            let Node::Block {
                operations,
                terminator: block_terminator,
                ..
            } = require_kind_mut(nodes, block, NodeKind::Block)?
            else {
                return Err(invariant("block kind changed during staging"));
            };
            if terminator {
                if before.is_some() || block_terminator.is_some() {
                    return Err(LkError::new(
                        ErrorCode::InvalidContainment,
                        "block already has a terminator or terminator requested an order anchor",
                    )
                    .for_node(block));
                }
                *block_terminator = Some(id);
            } else if let Some(before) = before {
                let position = operations
                    .iter()
                    .position(|candidate| *candidate == before)
                    .ok_or_else(|| {
                        LkError::new(
                            ErrorCode::InvalidContainment,
                            "operation order anchor is not a regular operation in this block",
                        )
                        .for_node(before)
                        .with_related([block])
                    })?;
                operations.insert(position, id);
            } else {
                operations.push(id);
            }
        }
        CanonicalEdit::SetFunctionBody { function, region } => {
            let function = resolve(*function, allocations, base.workspace())?;
            let region = resolve(*region, allocations, base.workspace())?;
            require_kind(nodes, region, NodeKind::Region)?;
            let Node::Function { body, .. } =
                require_kind_mut(nodes, function, NodeKind::Function)?
            else {
                return Err(invariant("function kind changed during staging"));
            };
            if body.is_some() {
                return Err(LkError::new(
                    ErrorCode::InvalidContainment,
                    "function body is already defined",
                )
                .for_node(function));
            }
            *body = Some(region);
        }
        CanonicalEdit::ClearFunctionBody { function } => {
            let function = resolve(*function, allocations, base.workspace())?;
            let body = match require_kind(nodes, function, NodeKind::Function)? {
                Node::Function {
                    body: Some(body), ..
                } => *body,
                Node::Function { body: None, .. } => {
                    return Err(LkError::new(
                        ErrorCode::InvalidContainment,
                        "function body is not defined",
                    )
                    .for_node(function));
                }
                _ => return Err(invariant("function kind changed during staging")),
            };
            reject_durable_body_descendants(nodes, function, body)?;
            delete_subtree(base.root, nodes, tombstones, body)?;
        }
        CanonicalEdit::SetEntryFunction { package, function } => {
            let package = resolve(*package, allocations, base.workspace())?;
            let function = resolve(*function, allocations, base.workspace())?;
            require_kind(nodes, function, NodeKind::Function)?;
            let Node::Package { entry, .. } = require_kind_mut(nodes, package, NodeKind::Package)?
            else {
                return Err(invariant("package kind changed during staging"));
            };
            *entry = Some(function);
        }
        CanonicalEdit::RenameNode { node, name } => {
            let node = resolve(*node, allocations, base.workspace())?;
            let target = nodes.get_mut(&node).ok_or_else(|| missing(node))?;
            if !target.set_name(name.clone()) {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "this node kind has no display name",
                )
                .for_node(node));
            }
        }
        CanonicalEdit::ReplaceOperation {
            operation,
            replacement,
        } => {
            let operation = resolve(*operation, allocations, base.workspace())?;
            let replacement = resolve_operation(
                replacement,
                allocations,
                base.workspace(),
                nominal_catalogue,
            )?;
            let current = match require_kind(nodes, operation, NodeKind::Operation)? {
                Node::Operation { operation, .. } => operation.clone(),
                _ => return Err(invariant("operation kind changed during staging")),
            };
            let current_results = operation_result_types_in_nodes(nodes, &current);
            let replacement_results = operation_result_types_in_nodes(nodes, &replacement);
            if current.code() != replacement.code()
                || current_results.is_none()
                || current_results != replacement_results
                || current.is_terminator() != replacement.is_terminator()
                || current.owned_region_count() != replacement.owned_region_count()
                || !(0..current.owned_region_count())
                    .all(|index| current.owned_region(index) == replacement.owned_region(index))
            {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "identity-preserving operation replacement requires the same operation, result, and region contract",
                )
                .for_node(operation));
            }
            let Node::Operation {
                operation: target, ..
            } = require_kind_mut(nodes, operation, NodeKind::Operation)?
            else {
                return Err(invariant("operation kind changed during staging"));
            };
            *target = replacement;
        }
        CanonicalEdit::ReplaceOperand {
            operation,
            index,
            value,
        } => {
            let operation = resolve(*operation, allocations, base.workspace())?;
            let value = resolve_value(value, allocations, base.workspace())?;
            let Node::Operation {
                operation: current, ..
            } = require_kind_mut(nodes, operation, NodeKind::Operation)?
            else {
                return Err(invariant("operation kind changed during staging"));
            };
            if !current.replace_operand(*index, value) {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "operand index is outside the operation contract",
                )
                .for_node(operation));
            }
        }
        CanonicalEdit::RefineHole { hole, replacement } => {
            let hole = resolve(*hole, allocations, base.workspace())?;
            let replacement = resolve_operation(
                replacement,
                allocations,
                base.workspace(),
                nominal_catalogue,
            )?;
            let (owner, expected, current_result_count) =
                match require_kind(nodes, hole, NodeKind::Operation)? {
                    Node::Operation {
                        owner,
                        operation: current @ OperationKind::Hole { expected },
                    } => (*owner, *expected, current.result_count()),
                    Node::Operation { .. } => {
                        return Err(LkError::new(
                            ErrorCode::InvalidOperand,
                            "hole refinement target is already a complete operation",
                        )
                        .for_node(hole));
                    }
                    _ => return Err(invariant("operation kind changed during staging")),
                };
            let Node::Block {
                operations,
                terminator,
                ..
            } = require_kind(nodes, owner, NodeKind::Block)?
            else {
                return Err(invariant("operation owner kind changed during staging"));
            };
            if !operations.contains(&hole) || *terminator == Some(hole) {
                return Err(LkError::new(
                    ErrorCode::InvalidContainment,
                    "hole refinement target must occupy a regular operation slot",
                )
                .for_node(hole)
                .with_related([owner]));
            }
            if !replacement.is_complete()
                || replacement.is_terminator()
                || replacement.owned_region_count() != 0
                || matches!(replacement, OperationKind::Hole { .. })
                || (matches!(expected, SemanticType::Nominal(_))
                    && !matches!(
                        replacement,
                        OperationKind::ConstructProduct { .. }
                            | OperationKind::ConstructVariant { .. }
                            | OperationKind::ProjectField { .. }
                    ))
            {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "hole replacement must be a complete non-terminator operation",
                )
                .for_node(hole));
            }
            let actual = operation_result_types_in_nodes(nodes, &replacement)
                .and_then(|types| types.first().copied());
            if current_result_count != replacement.result_count() || actual != Some(expected) {
                let mut error = LkError::new(
                    ErrorCode::TypeMismatch,
                    "hole replacement result contract does not match the expected type",
                )
                .for_node(hole);
                if let Some(actual) = actual {
                    error = error.with_types(expected, actual);
                } else {
                    error.expected_type = Some(expected);
                }
                return Err(error);
            }
            let Node::Operation {
                operation: current, ..
            } = require_kind_mut(nodes, hole, NodeKind::Operation)?
            else {
                return Err(invariant("operation kind changed during staging"));
            };
            *current = replacement;
        }
        CanonicalEdit::DeleteOwnedSubtree { root } => {
            let root = resolve(*root, allocations, base.workspace())?;
            delete_subtree(base.root, nodes, tombstones, root)?;
        }
    }
    Ok(())
}

fn resolve_operation(
    operation: &OperationDraft,
    allocations: &BTreeMap<DraftSymbol, NodeId>,
    workspace: WorkspaceId,
    nominal_catalogue: &StagedNominalCatalogue,
) -> Result<OperationKind> {
    Ok(match operation {
        OperationDraft::ConstUnit => OperationKind::ConstUnit,
        OperationDraft::ConstI64(value) => OperationKind::ConstI64(*value),
        OperationDraft::ConstBool(value) => OperationKind::ConstBool(*value),
        OperationDraft::ConstBytes(value) => OperationKind::ConstBytes(value.clone()),
        OperationDraft::ConstText(value) => OperationKind::ConstText(value.clone()),
        OperationDraft::AddI64 { lhs, rhs } => OperationKind::AddI64 {
            lhs: resolve_value(lhs, allocations, workspace)?,
            rhs: resolve_value(rhs, allocations, workspace)?,
        },
        OperationDraft::LtI64 { lhs, rhs } => OperationKind::LtI64 {
            lhs: resolve_value(lhs, allocations, workspace)?,
            rhs: resolve_value(rhs, allocations, workspace)?,
        },
        OperationDraft::EqualI64 { lhs, rhs } => OperationKind::EqualI64 {
            lhs: resolve_value(lhs, allocations, workspace)?,
            rhs: resolve_value(rhs, allocations, workspace)?,
        },
        OperationDraft::NotBool { value } => OperationKind::NotBool {
            value: resolve_value(value, allocations, workspace)?,
        },
        OperationDraft::AndBool { lhs, rhs } => OperationKind::AndBool {
            lhs: resolve_value(lhs, allocations, workspace)?,
            rhs: resolve_value(rhs, allocations, workspace)?,
        },
        OperationDraft::OrBool { lhs, rhs } => OperationKind::OrBool {
            lhs: resolve_value(lhs, allocations, workspace)?,
            rhs: resolve_value(rhs, allocations, workspace)?,
        },
        OperationDraft::BytesLen { value } => OperationKind::BytesLen {
            value: resolve_value(value, allocations, workspace)?,
        },
        OperationDraft::BytesAt { value, index } => OperationKind::BytesAt {
            value: resolve_value(value, allocations, workspace)?,
            index: resolve_value(index, allocations, workspace)?,
        },
        OperationDraft::BytesSlice {
            value,
            start,
            length,
        } => OperationKind::BytesSlice {
            value: resolve_value(value, allocations, workspace)?,
            start: resolve_value(start, allocations, workspace)?,
            length: resolve_value(length, allocations, workspace)?,
        },
        OperationDraft::BytesEqual { lhs, rhs } => OperationKind::BytesEqual {
            lhs: resolve_value(lhs, allocations, workspace)?,
            rhs: resolve_value(rhs, allocations, workspace)?,
        },
        OperationDraft::BytesConcat { lhs, rhs } => OperationKind::BytesConcat {
            lhs: resolve_value(lhs, allocations, workspace)?,
            rhs: resolve_value(rhs, allocations, workspace)?,
        },
        OperationDraft::TextLen { value } => OperationKind::TextLen {
            value: resolve_value(value, allocations, workspace)?,
        },
        OperationDraft::TextEqual { lhs, rhs } => OperationKind::TextEqual {
            lhs: resolve_value(lhs, allocations, workspace)?,
            rhs: resolve_value(rhs, allocations, workspace)?,
        },
        OperationDraft::TextConcat { lhs, rhs } => OperationKind::TextConcat {
            lhs: resolve_value(lhs, allocations, workspace)?,
            rhs: resolve_value(rhs, allocations, workspace)?,
        },
        OperationDraft::SequenceEmpty { sequence } => OperationKind::SequenceEmpty {
            sequence: resolve(*sequence, allocations, workspace)?,
        },
        OperationDraft::SequenceLen { sequence, value } => OperationKind::SequenceLen {
            sequence: resolve(*sequence, allocations, workspace)?,
            value: resolve_value(value, allocations, workspace)?,
        },
        OperationDraft::SequenceGet {
            sequence,
            value,
            index,
        } => OperationKind::SequenceGet {
            sequence: resolve(*sequence, allocations, workspace)?,
            value: resolve_value(value, allocations, workspace)?,
            index: resolve_value(index, allocations, workspace)?,
        },
        OperationDraft::SequenceAppend {
            sequence,
            value,
            element,
        } => OperationKind::SequenceAppend {
            sequence: resolve(*sequence, allocations, workspace)?,
            value: resolve_value(value, allocations, workspace)?,
            element: resolve_value(element, allocations, workspace)?,
        },
        OperationDraft::SequenceReplace {
            sequence,
            value,
            index,
            element,
        } => OperationKind::SequenceReplace {
            sequence: resolve(*sequence, allocations, workspace)?,
            value: resolve_value(value, allocations, workspace)?,
            index: resolve_value(index, allocations, workspace)?,
            element: resolve_value(element, allocations, workspace)?,
        },
        OperationDraft::SequenceSlice {
            sequence,
            value,
            start,
            end_exclusive,
        } => OperationKind::SequenceSlice {
            sequence: resolve(*sequence, allocations, workspace)?,
            value: resolve_value(value, allocations, workspace)?,
            start: resolve_value(start, allocations, workspace)?,
            end_exclusive: resolve_value(end_exclusive, allocations, workspace)?,
        },
        OperationDraft::SequenceConcat { sequence, lhs, rhs } => OperationKind::SequenceConcat {
            sequence: resolve(*sequence, allocations, workspace)?,
            lhs: resolve_value(lhs, allocations, workspace)?,
            rhs: resolve_value(rhs, allocations, workspace)?,
        },
        OperationDraft::Call {
            function,
            arguments,
        } => OperationKind::Call {
            function: resolve(*function, allocations, workspace)?,
            arguments: arguments
                .iter()
                .map(|value| resolve_value(value, allocations, workspace))
                .collect::<Result<Vec<_>>>()?,
        },
        OperationDraft::Hole { expected } => OperationKind::Hole {
            expected: resolve_type_draft(*expected, allocations, workspace)?,
        },
        OperationDraft::If {
            condition,
            result,
            then_region,
            else_region,
        } => OperationKind::If {
            condition: resolve_value(condition, allocations, workspace)?,
            result: resolve_type_draft(*result, allocations, workspace)?,
            then_region: resolve(*then_region, allocations, workspace)?,
            else_region: resolve(*else_region, allocations, workspace)?,
        },
        OperationDraft::ForI64 {
            start,
            end_exclusive,
            step,
            initial,
            carried,
            body_region,
        } => OperationKind::ForI64 {
            start: resolve_value(start, allocations, workspace)?,
            end_exclusive: resolve_value(end_exclusive, allocations, workspace)?,
            step: *step,
            initial: resolve_value(initial, allocations, workspace)?,
            carried: resolve_type_draft(*carried, allocations, workspace)?,
            body_region: resolve(*body_region, allocations, workspace)?,
        },
        OperationDraft::Return { value } => OperationKind::Return {
            value: resolve_value(value, allocations, workspace)?,
        },
        OperationDraft::Yield { value } => OperationKind::Yield {
            value: resolve_value(value, allocations, workspace)?,
        },
        OperationDraft::ConstructProduct { product, fields } => {
            let product_target = *product;
            let product = resolve(product_target, allocations, workspace)?;
            let declared = nominal_catalogue
                .products
                .get(&product_target)
                .ok_or_else(|| {
                    LkError::new(
                        ErrorCode::WrongKind,
                        "product construction must name a staged product declaration",
                    )
                    .for_node(product)
                })?;
            let declared = declared
                .iter()
                .copied()
                .map(|field| resolve(field, allocations, workspace))
                .collect::<Result<Vec<_>>>()?;
            let mut resolved = BTreeMap::new();
            for field in fields {
                let field_id = resolve(field.field, allocations, workspace)?;
                if resolved
                    .insert(
                        field_id,
                        resolve_value(&field.value, allocations, workspace)?,
                    )
                    .is_some()
                {
                    return Err(LkError::new(
                        ErrorCode::InvalidOperand,
                        "product field binding is duplicated",
                    )
                    .for_node(field_id));
                }
            }
            let fields = declared
                .iter()
                .map(|field| {
                    resolved
                        .remove(field)
                        .map(|value| ProductFieldValue {
                            field: *field,
                            value,
                        })
                        .ok_or_else(|| {
                            LkError::new(
                                ErrorCode::InvalidOperand,
                                "product construction is missing a declared field",
                            )
                            .for_node(*field)
                            .with_related([product])
                        })
                })
                .collect::<Result<Vec<_>>>()?;
            if let Some(foreign) = resolved.keys().next().copied() {
                return Err(LkError::new(
                    ErrorCode::OwnerMismatch,
                    "product construction contains a foreign field",
                )
                .for_node(foreign)
                .with_related([product]));
            }
            OperationKind::ConstructProduct { product, fields }
        }
        OperationDraft::ProjectField { value, field } => OperationKind::ProjectField {
            value: resolve_value(value, allocations, workspace)?,
            field: resolve(*field, allocations, workspace)?,
        },
        OperationDraft::ConstructVariant { variant, payload } => OperationKind::ConstructVariant {
            variant: resolve(*variant, allocations, workspace)?,
            payload: payload
                .as_ref()
                .map(|value| resolve_value(value, allocations, workspace))
                .transpose()?,
        },
        OperationDraft::MatchSum {
            scrutinee,
            result,
            arms,
        } => {
            let resolved = arms
                .iter()
                .map(|arm| {
                    Ok(MatchArm {
                        variant: resolve(arm.variant, allocations, workspace)?,
                        region: resolve(arm.region, allocations, workspace)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            OperationKind::MatchSum {
                scrutinee: resolve_value(scrutinee, allocations, workspace)?,
                result: resolve_type_draft(*result, allocations, workspace)?,
                arms: resolved,
            }
        }
    })
}

fn resolve_type_draft(
    ty: TypeDraft,
    allocations: &BTreeMap<DraftSymbol, NodeId>,
    workspace: WorkspaceId,
) -> Result<SemanticType> {
    Ok(match ty {
        TypeDraft::Unit => SemanticType::Unit,
        TypeDraft::Bool => SemanticType::Bool,
        TypeDraft::I64 => SemanticType::I64,
        TypeDraft::Bytes => SemanticType::Bytes,
        TypeDraft::Text => SemanticType::Text,
        TypeDraft::Nominal(target) => {
            SemanticType::Nominal(resolve(target, allocations, workspace)?)
        }
    })
}

fn resolve_value(
    value: &ValueDraft,
    allocations: &BTreeMap<DraftSymbol, NodeId>,
    workspace: WorkspaceId,
) -> Result<ValueRef> {
    Ok(match value {
        ValueDraft::FunctionParameter(parameter) => {
            ValueRef::FunctionParameter(resolve(*parameter, allocations, workspace)?)
        }
        ValueDraft::BlockArgument(argument) => {
            ValueRef::BlockArgument(resolve(*argument, allocations, workspace)?)
        }
        ValueDraft::OperationResult { operation, output } => ValueRef::OperationResult {
            operation: resolve(*operation, allocations, workspace)?,
            output: *output,
        },
        ValueDraft::InlineExpression(_) => {
            return Err(invariant(
                "inline expression survived structured proposal normalization",
            ));
        }
    })
}

fn operation_result_types_in_nodes(
    nodes: &BTreeMap<NodeId, Node>,
    operation: &OperationKind,
) -> Option<Vec<SemanticType>> {
    (0..operation.result_count())
        .map(|index| match operation {
            OperationKind::Call { function, .. } => match nodes.get(function) {
                Some(Node::Function { result, .. }) if index == 0 => Some(*result),
                _ => None,
            },
            OperationKind::ConstructProduct { product, .. } if index == 0 => {
                matches!(nodes.get(product), Some(Node::ProductType { .. }))
                    .then_some(SemanticType::Nominal(*product))
            }
            OperationKind::ProjectField { field, .. } if index == 0 => match nodes.get(field) {
                Some(Node::ProductField { ty, .. }) => Some(*ty),
                _ => None,
            },
            OperationKind::ConstructVariant { variant, .. } if index == 0 => {
                match nodes.get(variant) {
                    Some(Node::SumVariant { owner, .. }) => Some(SemanticType::Nominal(*owner)),
                    _ => None,
                }
            }
            OperationKind::MatchSum { result, .. } if index == 0 => Some(*result),
            OperationKind::SequenceEmpty { sequence }
            | OperationKind::SequenceAppend { sequence, .. }
            | OperationKind::SequenceReplace { sequence, .. }
            | OperationKind::SequenceSlice { sequence, .. }
            | OperationKind::SequenceConcat { sequence, .. }
                if index == 0 =>
            {
                matches!(nodes.get(sequence), Some(Node::SequenceType { .. }))
                    .then_some(SemanticType::Nominal(*sequence))
            }
            OperationKind::SequenceGet { sequence, .. } if index == 0 => {
                match nodes.get(sequence) {
                    Some(Node::SequenceType { element, .. }) => Some(*element),
                    _ => None,
                }
            }
            _ => operation.result_type(index, None),
        })
        .collect()
}

fn resolve(
    target: NodeTarget,
    allocations: &BTreeMap<DraftSymbol, NodeId>,
    workspace: WorkspaceId,
) -> Result<NodeId> {
    match target {
        NodeTarget::Existing(id) => {
            if id.workspace() != workspace {
                return Err(LkError::new(
                    ErrorCode::WrongWorkspace,
                    "transaction target belongs to another workspace",
                )
                .for_workspace(workspace)
                .for_node(id));
            }
            Ok(id)
        }
        NodeTarget::Draft(symbol) => allocations.get(&symbol).copied().ok_or_else(|| {
            LkError::new(
                ErrorCode::InvalidDraftSymbol,
                "transaction references an undeclared local symbol",
            )
            .for_symbol(symbol)
        }),
    }
}

fn allocated(allocations: &BTreeMap<DraftSymbol, NodeId>, symbol: DraftSymbol) -> Result<NodeId> {
    allocations.get(&symbol).copied().ok_or_else(|| {
        LkError::new(
            ErrorCode::InvalidDraftSymbol,
            "create operation has no staged node allocation",
        )
        .for_symbol(symbol)
    })
}

fn insert_new(nodes: &mut BTreeMap<NodeId, Node>, id: NodeId, node: Node) -> Result<()> {
    if nodes.insert(id, node).is_some() {
        return Err(LkError::new(
            ErrorCode::InvalidDraftSymbol,
            "staged node identity already exists",
        )
        .for_node(id));
    }
    Ok(())
}

fn require_kind_mut(
    nodes: &mut BTreeMap<NodeId, Node>,
    id: NodeId,
    expected: NodeKind,
) -> Result<&mut Node> {
    let node = nodes.get_mut(&id).ok_or_else(|| missing(id))?;
    let actual = node.kind();
    if actual != expected {
        return Err(
            LkError::new(ErrorCode::WrongKind, "target has the wrong node kind")
                .for_node(id)
                .with_kinds(expected, actual),
        );
    }
    Ok(node)
}

fn reject_durable_body_descendants(
    nodes: &BTreeMap<NodeId, Node>,
    function: NodeId,
    body: NodeId,
) -> Result<()> {
    let mut stack = vec![body];
    while let Some(id) = stack.pop() {
        if id.is_durable() {
            return Err(LkError::new(
                ErrorCode::DeleteBlocked,
                "function body replacement cannot discard a durable body anchor",
            )
            .for_node(id)
            .with_related([function]));
        }
        let node = nodes.get(&id).ok_or_else(|| missing(id))?;
        for index in (0..node.owned_child_count()).rev() {
            if let Some(child) = node.owned_child(index) {
                stack.push(child);
            }
        }
    }
    Ok(())
}

fn delete_subtree(
    workspace_root: NodeId,
    nodes: &mut BTreeMap<NodeId, Node>,
    tombstones: &mut BTreeSet<u64>,
    root: NodeId,
) -> Result<()> {
    if root == workspace_root {
        return Err(
            LkError::new(ErrorCode::DeleteBlocked, "workspace root cannot be deleted")
                .for_node(root),
        );
    }
    let root_node = nodes.get(&root).ok_or_else(|| missing(root))?;
    let owner = root_node.owner().ok_or_else(|| {
        LkError::new(
            ErrorCode::OwnerMismatch,
            "deleted subtree root has no owner",
        )
        .for_node(root)
    })?;
    let mut deleted = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if !deleted.insert(id) {
            continue;
        }
        let node = nodes.get(&id).ok_or_else(|| missing(id))?;
        for index in (0..node.owned_child_count()).rev() {
            if let Some(child) = node.owned_child(index) {
                stack.push(child);
            }
        }
    }
    for (source, node) in nodes.iter() {
        if deleted.contains(source) {
            continue;
        }
        let mut blockers = (0..node.direct_reference_count())
            .filter_map(|index| {
                node.direct_reference(index)
                    .map(|reference| reference.target())
            })
            .filter(|target| deleted.contains(target));
        if let Some(first) = blockers.next() {
            return Err(LkError::new(
                ErrorCode::DeleteBlocked,
                "surviving node directly references the requested deletion subtree",
            )
            .for_node(root)
            .with_related(
                std::iter::once(*source)
                    .chain(std::iter::once(first))
                    .chain(blockers),
            ));
        }
    }
    detach_child(nodes, owner, root)?;
    for id in deleted {
        if nodes.remove(&id).is_none() {
            return Err(invariant("deletion traversal lost a staged node"));
        }
        if id.is_durable() {
            tombstones.insert(id.serial());
        }
    }
    Ok(())
}

fn detach_child(nodes: &mut BTreeMap<NodeId, Node>, owner: NodeId, child: NodeId) -> Result<()> {
    let owner_node = nodes.get_mut(&owner).ok_or_else(|| missing(owner))?;
    let removed = match owner_node {
        Node::WorkspaceRoot { packages, targets } => {
            remove_one(packages, child) || remove_one(targets, child)
        }
        Node::BuildTarget { .. } => false,
        Node::Package { modules, .. } => remove_one(modules, child),
        Node::Module {
            types, functions, ..
        } => remove_one(types, child) || remove_one(functions, child),
        Node::ProductType { fields, .. } => remove_one(fields, child),
        Node::SumType { variants, .. } => remove_one(variants, child),
        Node::Function {
            parameters, body, ..
        } => {
            if *body == Some(child) {
                *body = None;
                true
            } else {
                remove_one(parameters, child)
            }
        }
        Node::Region { blocks, .. } => remove_one(blocks, child),
        Node::Block {
            arguments,
            operations,
            terminator,
            ..
        } => {
            if *terminator == Some(child) {
                *terminator = None;
                true
            } else {
                remove_one(arguments, child) || remove_one(operations, child)
            }
        }
        Node::ProductField { .. }
        | Node::SumVariant { .. }
        | Node::SequenceType { .. }
        | Node::Parameter { .. }
        | Node::BlockArgument { .. }
        | Node::Operation { .. } => false,
    };
    if !removed {
        return Err(LkError::new(
            ErrorCode::OwnerMismatch,
            "owner does not contain requested deletion root",
        )
        .for_node(child)
        .with_related([owner]));
    }
    Ok(())
}

fn remove_one(values: &mut Vec<NodeId>, target: NodeId) -> bool {
    let Some(position) = values.iter().position(|value| *value == target) else {
        return false;
    };
    values.remove(position);
    true
}

fn missing(id: NodeId) -> LkError {
    LkError::new(ErrorCode::NodeNotFound, "transaction target does not exist").for_node(id)
}

fn invariant(message: &str) -> LkError {
    LkError::new(ErrorCode::InvalidContainment, message)
}

#[cfg(test)]
mod tests;
