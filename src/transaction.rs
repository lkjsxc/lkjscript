use crate::diff;
use crate::error::{ErrorCode, LkError, MAX_DRAFT_PATH_BYTES, Result};
use crate::graph::{Snapshot, Workspace, require_kind};
use crate::ids::{
    ChangeDigest, DraftSymbol, IdempotencyKey, NodeId, Revision, SnapshotHash, WorkspaceId,
};
use crate::query;
use crate::schema::{
    MatchArm, MatchArmOperationDraft, Node, NodeKind, OperationCode, OperationDraft, OperationKind,
    ProductFieldValue, ProductFieldValueDraft, RegionArity, SemanticType, TypeDraft, ValueDraft,
    ValueRef,
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
    InsertExpression,
    SetEntryFunction,
    RenameNode,
    ReplaceOperation,
    ReplaceOperand,
    DeleteOwnedSubtree,
    RefineHole,
    CreateProductType,
    CreateSumType,
}
impl TransactionOpCode {
    pub const ALL: [Self; 13] = [
        Self::CreatePackage,
        Self::CreateModule,
        Self::CreateFunction,
        Self::DefineFunctionBody,
        Self::InsertExpression,
        Self::SetEntryFunction,
        Self::RenameNode,
        Self::ReplaceOperation,
        Self::ReplaceOperand,
        Self::DeleteOwnedSubtree,
        Self::RefineHole,
        Self::CreateProductType,
        Self::CreateSumType,
    ];
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::CreatePackage => "create_package",
            Self::CreateModule => "create_module",
            Self::CreateFunction => "create_function",
            Self::DefineFunctionBody => "define_function_body",
            Self::InsertExpression => "insert_expression",
            Self::SetEntryFunction => "set_entry_function",
            Self::RenameNode => "rename_node",
            Self::ReplaceOperation => "replace_operation",
            Self::ReplaceOperand => "replace_operand",
            Self::RefineHole => "refine_hole",
            Self::DeleteOwnedSubtree => "delete_owned_subtree",
            Self::CreateProductType => "create_product_type",
            Self::CreateSumType => "create_sum_type",
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
}
impl ExpressionDraftCode {
    pub const ALL: [Self; 13] = [
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
    AddI64 {
        lhs: ValueDraft,
        rhs: ValueDraft,
    },
    LtI64 {
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
    CreateSumType {
        symbol: DraftSymbol,
        module: NodeTarget,
        name: String,
        variants: Vec<SumVariantDraft>,
    },
    CreateFunction {
        symbol: DraftSymbol,
        module: NodeTarget,
        name: String,
        parameters: Vec<FunctionParameterDraft>,
        result: TypeDraft,
        body: Option<FunctionBodyDraft>,
    },
    DefineFunctionBody {
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
            Self::CreateSumType { .. } => TransactionOpCode::CreateSumType,
            Self::CreateFunction { .. } => TransactionOpCode::CreateFunction,
            Self::DefineFunctionBody { .. } => TransactionOpCode::DefineFunctionBody,
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
            | Self::CreateSumType { symbol, .. }
            | Self::CreateFunction { symbol, .. } => Some(*symbol),
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
        | ExpressionKindDraft::Hole { .. } => {}
        ExpressionKindDraft::AddI64 { lhs, rhs } | ExpressionKindDraft::LtI64 { lhs, rhs } => {
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
    Module,
    ProductType,
    ProductField,
    SumType,
    SumVariant,
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
                DraftSymbolKind::ProductType | DraftSymbolKind::SumType
            ),
            Self::Package => actual == DraftSymbolKind::Package,
            Self::Module => actual == DraftSymbolKind::Module,
            Self::ProductType => actual == DraftSymbolKind::ProductType,
            Self::ProductField => actual == DraftSymbolKind::ProductField,
            Self::SumVariant => actual == DraftSymbolKind::SumVariant,
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
    struct DraftBudget(usize);
    impl DraftBudget {
        fn add(&mut self, count: usize, source: usize) -> Result<()> {
            self.0 = self.0.checked_add(count).ok_or_else(|| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "structured draft item count overflow",
                )
                .at_operation(source)
            })?;
            if self.0 > MAX_STRUCTURED_DRAFT_ITEMS {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "structured draft exceeds request item policy",
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
            OperationDraft::AddI64 { lhs, rhs } | OperationDraft::LtI64 { lhs, rhs } => {
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
    let mut budget = DraftBudget(0);
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
            TransactionOp::DefineFunctionBody { body, .. } => {
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
            ExpressionKindDraft::AddI64 { lhs, rhs } | ExpressionKindDraft::LtI64 { lhs, rhs } => {
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
            created_count: u64::try_from(allocations.len()).map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "created node count does not fit receipt representation",
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
        | CanonicalEdit::CreateModule { symbol, .. }
        | CanonicalEdit::CreateProductType { symbol, .. }
        | CanonicalEdit::CreateProductField { symbol, .. }
        | CanonicalEdit::CreateSumType { symbol, .. }
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
    let mut next = base.next_serial;
    for (operation, source) in operations.iter().zip(edit_sources) {
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
        let id = NodeId::new(base.workspace(), next).map_err(|error| {
            allocation_error(
                ErrorCode::PolicyExceeded,
                format!("node identity allocation failed: {error}"),
                *source,
                symbol,
                explicit_symbols,
                anonymous_paths,
            )
        })?;
        next = next.checked_add(1).ok_or_else(|| {
            allocation_error(
                ErrorCode::PolicyExceeded,
                "node identity serial is exhausted",
                *source,
                symbol,
                explicit_symbols,
                anonymous_paths,
            )
        })?;
        allocations.insert(symbol, id);
    }
    Ok((allocations, next))
}

fn canonical_created_symbol(operation: &CanonicalEdit) -> Option<DraftSymbol> {
    match operation {
        CanonicalEdit::CreatePackage { symbol, .. }
        | CanonicalEdit::CreateModule { symbol, .. }
        | CanonicalEdit::CreateProductType { symbol, .. }
        | CanonicalEdit::CreateProductField { symbol, .. }
        | CanonicalEdit::CreateSumType { symbol, .. }
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
            let Node::WorkspaceRoot { packages } = root else {
                return Err(invariant("workspace root kind changed during staging"));
            };
            packages.push(id);
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
        OperationDraft::AddI64 { lhs, rhs } => OperationKind::AddI64 {
            lhs: resolve_value(lhs, allocations, workspace)?,
            rhs: resolve_value(rhs, allocations, workspace)?,
        },
        OperationDraft::LtI64 { lhs, rhs } => OperationKind::LtI64 {
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
        tombstones.insert(id.serial());
    }
    Ok(())
}

fn detach_child(nodes: &mut BTreeMap<NodeId, Node>, owner: NodeId, child: NodeId) -> Result<()> {
    let owner_node = nodes.get_mut(&owner).ok_or_else(|| missing(owner))?;
    let removed = match owner_node {
        Node::WorkspaceRoot { packages } => remove_one(packages, child),
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
mod tests {
    use super::*;
    use crate::artifact;

    fn request(transaction: &Transaction) -> ApplyTransactionRequest {
        let mut return_symbols: Vec<DraftSymbol> = scan_explicit_symbols(&transaction.operations)
            .expect("valid test symbols")
            .into_iter()
            .collect();
        return_symbols.sort();
        return_symbols.truncate(MAX_RETURNED_BINDINGS);
        ApplyTransactionRequest {
            transaction: transaction.clone(),
            response: TransactionResponseSpec { return_symbols },
        }
    }

    fn commit(workspace: &mut Workspace, transaction: &Transaction) -> Result<TransactionReceipt> {
        let prepared = workspace.prepare_transaction(&request(transaction))?;
        let receipt = prepared.receipt.clone();
        if transaction.mode == TransactionMode::Commit {
            workspace.publish(prepared.snapshot)?;
        }
        Ok(receipt)
    }

    fn create_package_and_module(id: WorkspaceId) -> Transaction {
        Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "package".to_owned(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: NodeTarget::Draft(DraftSymbol::generated(1)),
                    name: "module".to_owned(),
                },
            ],
        }
    }

    fn draft_symbol(value: u32) -> NodeTarget {
        NodeTarget::Draft(DraftSymbol::generated(value))
    }
    fn draft_result(value: u32) -> ValueDraft {
        ValueDraft::OperationResult {
            operation: draft_symbol(value),
            output: 0,
        }
    }
    fn draft_expression(symbol: u32, operation: ExpressionKindDraft) -> ExpressionDraft {
        ExpressionDraft {
            symbol: Some(DraftSymbol::generated(symbol)),
            operation,
        }
    }
    fn inline(operation: ExpressionKindDraft) -> ValueDraft {
        ValueDraft::InlineExpression(Box::new(operation))
    }
    fn structured_semantic_request(
        id: WorkspaceId,
        mut operations: Vec<TransactionOp>,
    ) -> ApplyTransactionRequest {
        let mut all = vec![
            TransactionOp::CreatePackage {
                symbol: DraftSymbol::generated(1),
                name: "package".into(),
            },
            TransactionOp::CreateModule {
                symbol: DraftSymbol::generated(2),
                package: draft_symbol(1),
                name: "module".into(),
            },
        ];
        all.append(&mut operations);
        ApplyTransactionRequest {
            transaction: Transaction {
                workspace: id,
                base_revision: Revision::INITIAL,
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: all,
            },
            response: TransactionResponseSpec::default(),
        }
    }

    fn equal_arithmetic_request(id: WorkspaceId, inline_values: bool) -> ApplyTransactionRequest {
        let operations = if inline_values {
            vec![draft_expression(
                8,
                ExpressionKindDraft::AddI64 {
                    lhs: inline(ExpressionKindDraft::AddI64 {
                        lhs: inline(ExpressionKindDraft::ConstI64(1)),
                        rhs: inline(ExpressionKindDraft::ConstI64(2)),
                    }),
                    rhs: inline(ExpressionKindDraft::ConstI64(3)),
                },
            )]
        } else {
            vec![
                draft_expression(4, ExpressionKindDraft::ConstI64(1)),
                draft_expression(5, ExpressionKindDraft::ConstI64(2)),
                draft_expression(
                    6,
                    ExpressionKindDraft::AddI64 {
                        lhs: draft_result(4),
                        rhs: draft_result(5),
                    },
                ),
                draft_expression(7, ExpressionKindDraft::ConstI64(3)),
                draft_expression(
                    8,
                    ExpressionKindDraft::AddI64 {
                        lhs: draft_result(6),
                        rhs: draft_result(7),
                    },
                ),
            ]
        };
        let mut request = structured_semantic_request(
            id,
            vec![
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(3),
                    module: draft_symbol(2),
                    name: "main".into(),
                    parameters: Vec::new(),
                    result: TypeDraft::I64,
                    body: Some(FunctionBodyDraft {
                        operations,
                        return_value: draft_result(8),
                    }),
                },
                TransactionOp::SetEntryFunction {
                    package: draft_symbol(1),
                    function: draft_symbol(3),
                },
            ],
        );
        request.response.return_symbols = [1, 2, 3, 8]
            .into_iter()
            .map(DraftSymbol::generated)
            .collect();
        request
    }

    #[test]
    fn inline_and_explicit_postorder_proposals_produce_identical_authority() {
        let id = WorkspaceId::from_bytes([0x70; 16]);
        let explicit_workspace = Workspace::new(id).expect("explicit workspace");
        let inline_workspace = Workspace::new(id).expect("inline workspace");
        let explicit = explicit_workspace
            .prepare_transaction(&equal_arithmetic_request(id, false))
            .expect("explicit proposal");
        let inline = inline_workspace
            .prepare_transaction(&equal_arithmetic_request(id, true))
            .expect("inline proposal");

        assert_eq!(explicit.receipt, inline.receipt);
        assert_eq!(explicit.snapshot.hash(), inline.snapshot.hash());
        assert_eq!(
            explicit.snapshot.nodes().collect::<Vec<_>>(),
            inline.snapshot.nodes().collect::<Vec<_>>()
        );
        assert_eq!(
            artifact::encode(&explicit.snapshot).expect("explicit artifact"),
            artifact::encode(&inline.snapshot).expect("inline artifact")
        );
    }

    fn equal_named_call_request(id: WorkspaceId, inline_values: bool) -> ApplyTransactionRequest {
        let value_tree = || {
            inline(ExpressionKindDraft::ConstructProduct {
                product: draft_symbol(3),
                fields: vec![ProductFieldValueDraft {
                    field: draft_symbol(4),
                    value: inline(ExpressionKindDraft::ProjectField {
                        value: inline(ExpressionKindDraft::Call {
                            function: draft_symbol(7),
                            arguments: vec![inline(ExpressionKindDraft::ConstructProduct {
                                product: draft_symbol(3),
                                fields: vec![ProductFieldValueDraft {
                                    field: draft_symbol(4),
                                    value: inline(ExpressionKindDraft::ConstI64(9)),
                                }],
                            })],
                        }),
                        field: draft_symbol(4),
                    }),
                }],
            })
        };
        let operations = if inline_values {
            vec![draft_expression(
                15,
                ExpressionKindDraft::ConstructVariant {
                    variant: draft_symbol(6),
                    payload: Some(value_tree()),
                },
            )]
        } else {
            vec![
                draft_expression(10, ExpressionKindDraft::ConstI64(9)),
                draft_expression(
                    11,
                    ExpressionKindDraft::ConstructProduct {
                        product: draft_symbol(3),
                        fields: vec![ProductFieldValueDraft {
                            field: draft_symbol(4),
                            value: draft_result(10),
                        }],
                    },
                ),
                draft_expression(
                    12,
                    ExpressionKindDraft::Call {
                        function: draft_symbol(7),
                        arguments: vec![draft_result(11)],
                    },
                ),
                draft_expression(
                    13,
                    ExpressionKindDraft::ProjectField {
                        value: draft_result(12),
                        field: draft_symbol(4),
                    },
                ),
                draft_expression(
                    14,
                    ExpressionKindDraft::ConstructProduct {
                        product: draft_symbol(3),
                        fields: vec![ProductFieldValueDraft {
                            field: draft_symbol(4),
                            value: draft_result(13),
                        }],
                    },
                ),
                draft_expression(
                    15,
                    ExpressionKindDraft::ConstructVariant {
                        variant: draft_symbol(6),
                        payload: Some(draft_result(14)),
                    },
                ),
            ]
        };
        let mut request = structured_semantic_request(
            id,
            vec![
                TransactionOp::CreateProductType {
                    symbol: DraftSymbol::generated(3),
                    module: draft_symbol(2),
                    name: "BoxedI64".into(),
                    fields: vec![ProductFieldDraft {
                        symbol: DraftSymbol::generated(4),
                        name: "value".into(),
                        ty: TypeDraft::I64,
                    }],
                },
                TransactionOp::CreateSumType {
                    symbol: DraftSymbol::generated(5),
                    module: draft_symbol(2),
                    name: "MaybeBox".into(),
                    variants: vec![SumVariantDraft {
                        symbol: DraftSymbol::generated(6),
                        name: "some".into(),
                        payload: Some(TypeDraft::Nominal(draft_symbol(3))),
                    }],
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(9),
                    module: draft_symbol(2),
                    name: "main".into(),
                    parameters: Vec::new(),
                    result: TypeDraft::Nominal(draft_symbol(5)),
                    body: Some(FunctionBodyDraft {
                        operations,
                        return_value: draft_result(15),
                    }),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(7),
                    module: draft_symbol(2),
                    name: "identity".into(),
                    parameters: vec![FunctionParameterDraft {
                        symbol: DraftSymbol::generated(8),
                        name: "value".into(),
                        ty: TypeDraft::Nominal(draft_symbol(3)),
                    }],
                    result: TypeDraft::Nominal(draft_symbol(3)),
                    body: Some(FunctionBodyDraft {
                        operations: Vec::new(),
                        return_value: ValueDraft::FunctionParameter(draft_symbol(8)),
                    }),
                },
                TransactionOp::SetEntryFunction {
                    package: draft_symbol(1),
                    function: draft_symbol(9),
                },
            ],
        );
        request.response.return_symbols = [1, 2, 3, 4, 5, 6, 7, 8, 9, 15]
            .into_iter()
            .map(DraftSymbol::generated)
            .collect();
        request
    }

    #[test]
    fn inline_calls_products_projections_and_variants_are_byte_identical() {
        let id = WorkspaceId::from_bytes([0x74; 16]);
        let explicit_workspace = Workspace::new(id).expect("explicit workspace");
        let inline_workspace = Workspace::new(id).expect("inline workspace");
        let explicit = explicit_workspace
            .prepare_transaction(&equal_named_call_request(id, false))
            .expect("explicit proposal");
        let inline = inline_workspace
            .prepare_transaction(&equal_named_call_request(id, true))
            .expect("inline proposal");
        assert_eq!(explicit.receipt, inline.receipt);
        assert_eq!(
            artifact::encode(&explicit.snapshot).expect("explicit artifact"),
            artifact::encode(&inline.snapshot).expect("inline artifact")
        );
    }

    #[test]
    fn inline_validate_only_and_commit_predict_the_same_ids_without_allocation() {
        let id = WorkspaceId::from_bytes([0x71; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let mut validate = equal_arithmetic_request(id, true);
        validate.transaction.mode = TransactionMode::ValidateOnly;
        let predicted = workspace
            .prepare_transaction(&validate)
            .expect("validate-only proposal");
        assert_eq!(workspace.head_revision(), Revision::INITIAL);
        assert_eq!(workspace.head().expect("head").next_serial(), 2);

        let committed = workspace
            .prepare_transaction(&equal_arithmetic_request(id, true))
            .expect("commit proposal");
        let mut expected = predicted.receipt;
        expected.published = true;
        assert_eq!(committed.receipt, expected);
        assert_eq!(committed.snapshot.hash(), predicted.snapshot.hash());
    }

    #[test]
    fn inline_depth_and_eligibility_reject_before_allocation_with_exact_paths() {
        let id = WorkspaceId::from_bytes([0x72; 16]);
        let block = NodeId::new(id, 2).expect("block");
        let operation = |value| TransactionOp::InsertExpression {
            block,
            before: None,
            expression: ExpressionDraft {
                symbol: Some(DraftSymbol::generated(1)),
                operation: ExpressionKindDraft::AddI64 {
                    lhs: value,
                    rhs: inline(ExpressionKindDraft::ConstI64(0)),
                },
            },
        };
        let nested = |depth: usize| {
            let mut value = inline(ExpressionKindDraft::ConstI64(1));
            for _ in 1..depth {
                value = inline(ExpressionKindDraft::AddI64 {
                    lhs: value,
                    rhs: inline(ExpressionKindDraft::ConstI64(1)),
                });
            }
            value
        };

        validate_structured_request(&[operation(nested(MAX_STRUCTURED_DRAFT_DEPTH))])
            .expect("maximum accepted mixed inline depth");
        let excessive =
            validate_structured_request(&[operation(nested(MAX_STRUCTURED_DRAFT_DEPTH + 1))])
                .expect_err("first excessive mixed inline depth");
        assert_eq!(excessive.code, ErrorCode::PolicyExceeded);
        assert_eq!(excessive.operation_index, Some(0));
        assert!(excessive.draft_path.is_some());

        for forbidden in [
            ExpressionKindDraft::Hole {
                expected: TypeDraft::I64,
            },
            ExpressionKindDraft::If {
                condition: inline(ExpressionKindDraft::ConstBool(true)),
                result: TypeDraft::I64,
                then_body: YieldingBodyDraft {
                    operations: Vec::new(),
                    yield_value: inline(ExpressionKindDraft::ConstI64(1)),
                },
                else_body: YieldingBodyDraft {
                    operations: Vec::new(),
                    yield_value: inline(ExpressionKindDraft::ConstI64(2)),
                },
            },
        ] {
            let error = validate_structured_request(&[operation(inline(forbidden))])
                .expect_err("ineligible inline expression");
            assert_eq!(error.code, ErrorCode::InvalidOperand);
            assert_eq!(error.draft_path.as_deref(), Some("op[0].expression.lhs"));
        }

        let exact_inline_count = (MAX_STRUCTURED_DRAFT_ITEMS - 2) / 2;
        let call_with_inline_arguments = |count| TransactionOp::InsertExpression {
            block,
            before: None,
            expression: ExpressionDraft {
                symbol: Some(DraftSymbol::generated(2)),
                operation: ExpressionKindDraft::Call {
                    function: NodeTarget::Existing(block),
                    arguments: vec![inline(ExpressionKindDraft::ConstI64(1)); count],
                },
            },
        };
        validate_structured_request(&[call_with_inline_arguments(exact_inline_count)])
            .expect("exact inline item limit");
        assert_eq!(
            validate_structured_request(&[call_with_inline_arguments(exact_inline_count + 1)])
                .expect_err("first excessive inline item")
                .code,
            ErrorCode::PolicyExceeded
        );
    }

    #[test]
    fn invalid_inline_type_reports_anonymous_path_and_rolls_back() {
        let id = WorkspaceId::from_bytes([0x73; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let mut request = equal_arithmetic_request(id, true);
        let TransactionOp::CreateFunction {
            body: Some(body), ..
        } = &mut request.transaction.operations[2]
        else {
            panic!("function body");
        };
        let ExpressionKindDraft::AddI64 { lhs, .. } = &mut body.operations[0].operation else {
            panic!("outer add");
        };
        *lhs = inline(ExpressionKindDraft::ConstBool(true));

        let error = workspace
            .prepare_transaction(&request)
            .expect_err("inline type mismatch");
        assert_eq!(error.code, ErrorCode::TypeMismatch);
        assert_eq!(error.operation_index, Some(2));
        assert_eq!(error.draft_path.as_deref(), Some("op[2].body.e[0].lhs"));
        assert!(error.draft_symbol.is_none());
        assert_eq!(workspace.head_revision(), Revision::INITIAL);
        assert_eq!(workspace.head().expect("head").next_serial(), 2);
    }

    #[test]
    fn maintenance_operations_reject_inline_values_before_allocation() {
        let id = WorkspaceId::from_bytes([0x75; 16]);
        let node = NodeTarget::Existing(NodeId::new(id, 2).expect("node"));
        for operation in [
            TransactionOp::ReplaceOperand {
                operation: node,
                index: 0,
                value: inline(ExpressionKindDraft::ConstI64(1)),
            },
            TransactionOp::ReplaceOperation {
                operation: node,
                replacement: OperationDraft::AddI64 {
                    lhs: inline(ExpressionKindDraft::ConstI64(1)),
                    rhs: ValueDraft::OperationResult {
                        operation: node,
                        output: 0,
                    },
                },
            },
        ] {
            let error = validate_structured_request(&[operation])
                .expect_err("maintenance inline value must reject");
            assert_eq!(error.code, ErrorCode::InvalidOperand);
            assert_eq!(error.operation_index, Some(0));
        }
    }

    #[test]
    fn negative_for_step_rejects_atomically() {
        let id = WorkspaceId::from_bytes([0xa1; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let request = structured_semantic_request(
            id,
            vec![TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: draft_symbol(2),
                name: "negative".into(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: vec![
                        draft_expression(4, ExpressionKindDraft::ConstI64(0)),
                        draft_expression(5, ExpressionKindDraft::ConstI64(2)),
                        draft_expression(
                            6,
                            ExpressionKindDraft::ForI64 {
                                start: draft_result(4),
                                end_exclusive: draft_result(5),
                                step: -1,
                                initial: draft_result(4),
                                carried: SemanticType::I64.into(),
                                index_symbol: DraftSymbol::generated(7),
                                carried_symbol: DraftSymbol::generated(8),
                                body: YieldingBodyDraft {
                                    operations: Vec::new(),
                                    yield_value: ValueDraft::BlockArgument(draft_symbol(8)),
                                },
                            },
                        ),
                    ],
                    return_value: draft_result(6),
                }),
            }],
        );
        let error = workspace
            .prepare_transaction(&request)
            .expect_err("negative step");
        assert_eq!(error.code, ErrorCode::InvalidOperand);
        assert_eq!(workspace.head().expect("head").next_serial(), 2);
    }

    #[test]
    fn sibling_if_arm_local_capture_rejects_atomically() {
        let id = WorkspaceId::from_bytes([0xa2; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let request = structured_semantic_request(
            id,
            vec![TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: draft_symbol(2),
                name: "sibling".into(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: vec![
                        draft_expression(4, ExpressionKindDraft::ConstBool(true)),
                        draft_expression(
                            5,
                            ExpressionKindDraft::If {
                                condition: draft_result(4),
                                result: SemanticType::I64.into(),
                                then_body: YieldingBodyDraft {
                                    operations: vec![draft_expression(
                                        6,
                                        ExpressionKindDraft::ConstI64(1),
                                    )],
                                    yield_value: draft_result(6),
                                },
                                else_body: YieldingBodyDraft {
                                    operations: Vec::new(),
                                    yield_value: draft_result(6),
                                },
                            },
                        ),
                    ],
                    return_value: draft_result(5),
                }),
            }],
        );
        let error = workspace
            .prepare_transaction(&request)
            .expect_err("sibling capture");
        assert_eq!(error.code, ErrorCode::InvalidOperand);
        assert_eq!(workspace.head().expect("head").next_serial(), 2);
    }

    #[test]
    fn nested_local_escape_after_owning_operation_rejects_atomically() {
        let id = WorkspaceId::from_bytes([0xa3; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let request = structured_semantic_request(
            id,
            vec![TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: draft_symbol(2),
                name: "escape".into(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: vec![
                        draft_expression(4, ExpressionKindDraft::ConstBool(true)),
                        draft_expression(
                            5,
                            ExpressionKindDraft::If {
                                condition: draft_result(4),
                                result: SemanticType::I64.into(),
                                then_body: YieldingBodyDraft {
                                    operations: vec![draft_expression(
                                        6,
                                        ExpressionKindDraft::ConstI64(1),
                                    )],
                                    yield_value: draft_result(6),
                                },
                                else_body: YieldingBodyDraft {
                                    operations: vec![draft_expression(
                                        7,
                                        ExpressionKindDraft::ConstI64(2),
                                    )],
                                    yield_value: draft_result(7),
                                },
                            },
                        ),
                        draft_expression(
                            8,
                            ExpressionKindDraft::AddI64 {
                                lhs: draft_result(6),
                                rhs: draft_result(5),
                            },
                        ),
                    ],
                    return_value: draft_result(8),
                }),
            }],
        );
        let error = workspace
            .prepare_transaction(&request)
            .expect_err("nested escape");
        assert_eq!(error.code, ErrorCode::InvalidOperand);
        assert_eq!(workspace.head().expect("head").next_serial(), 2);
    }

    #[test]
    fn cross_function_direct_value_use_rejects_atomically() {
        let id = WorkspaceId::from_bytes([0xa4; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let request = structured_semantic_request(
            id,
            vec![
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(3),
                    module: draft_symbol(2),
                    name: "producer".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![draft_expression(4, ExpressionKindDraft::ConstI64(1))],
                        return_value: draft_result(4),
                    }),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(5),
                    module: draft_symbol(2),
                    name: "consumer".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: Vec::new(),
                        return_value: draft_result(4),
                    }),
                },
            ],
        );
        let error = workspace
            .prepare_transaction(&request)
            .expect_err("cross function value");
        assert_eq!(error.code, ErrorCode::InvalidOperand);
        assert_eq!(workspace.head().expect("head").next_serial(), 2);
    }

    #[test]
    fn same_workspace_cross_module_call_succeeds() {
        let id = WorkspaceId::from_bytes([0xa5; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let request = ApplyTransactionRequest {
            transaction: Transaction {
                workspace: id,
                base_revision: Revision::INITIAL,
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![
                    TransactionOp::CreatePackage {
                        symbol: DraftSymbol::generated(1),
                        name: "package".into(),
                    },
                    TransactionOp::CreateModule {
                        symbol: DraftSymbol::generated(2),
                        package: draft_symbol(1),
                        name: "left".into(),
                    },
                    TransactionOp::CreateModule {
                        symbol: DraftSymbol::generated(3),
                        package: draft_symbol(1),
                        name: "right".into(),
                    },
                    TransactionOp::CreateFunction {
                        symbol: DraftSymbol::generated(4),
                        module: draft_symbol(2),
                        name: "callee".into(),
                        parameters: Vec::new(),
                        result: SemanticType::I64.into(),
                        body: Some(FunctionBodyDraft {
                            operations: vec![draft_expression(5, ExpressionKindDraft::ConstI64(7))],
                            return_value: draft_result(5),
                        }),
                    },
                    TransactionOp::CreateFunction {
                        symbol: DraftSymbol::generated(6),
                        module: draft_symbol(3),
                        name: "caller".into(),
                        parameters: Vec::new(),
                        result: SemanticType::I64.into(),
                        body: Some(FunctionBodyDraft {
                            operations: vec![draft_expression(
                                7,
                                ExpressionKindDraft::Call {
                                    function: draft_symbol(4),
                                    arguments: Vec::new(),
                                },
                            )],
                            return_value: draft_result(7),
                        }),
                    },
                    TransactionOp::SetEntryFunction {
                        package: draft_symbol(1),
                        function: draft_symbol(6),
                    },
                ],
            },
            response: TransactionResponseSpec::default(),
        };
        let prepared = workspace
            .prepare_transaction(&request)
            .expect("cross-module call");
        assert!(query::workspace_blockers(&prepared.snapshot).is_empty());
    }

    #[test]
    fn failed_batches_and_validate_only_do_not_consume_node_ids() {
        let id = WorkspaceId::from_bytes([11; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let first = commit(&mut workspace, &create_package_and_module(id)).expect("first commit");
        let module = first.returned_bindings[1].1;
        assert_eq!(module.serial(), 3);

        let failed = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(3),
                    module: NodeTarget::Existing(module),
                    name: "duplicate".to_owned(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: None,
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(4),
                    module: NodeTarget::Existing(module),
                    name: "duplicate".to_owned(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: None,
                },
            ],
        };
        let error = workspace
            .prepare_transaction(&request(&failed))
            .expect_err("duplicate names must reject");
        assert_eq!(error.code, ErrorCode::DuplicateName);
        assert_eq!(workspace.head_revision(), Revision::new(1));
        assert_eq!(workspace.head().expect("head").next_serial(), 4);

        let validate_only = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::ValidateOnly,
            operations: vec![TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(5),
                module: NodeTarget::Existing(module),
                name: "function".to_owned(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: None,
            }],
        };
        let predicted = commit(&mut workspace, &validate_only).expect("validate only");
        assert_eq!(predicted.returned_bindings[0].1.serial(), 4);
        assert_eq!(workspace.head_revision(), Revision::new(1));

        let mut real = validate_only;
        real.mode = TransactionMode::Commit;
        let committed = commit(&mut workspace, &real).expect("real commit");
        assert_eq!(
            committed.returned_bindings[0].1,
            predicted.returned_bindings[0].1
        );
        assert_eq!(workspace.head_revision(), Revision::new(2));
    }

    #[test]
    fn deletion_tombstones_identity_and_old_snapshots_retain_nodes() {
        let id = WorkspaceId::from_bytes([12; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let first = commit(&mut workspace, &create_package_and_module(id)).expect("first commit");
        let module = first.returned_bindings[1].1;
        let create = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: NodeTarget::Existing(module),
                name: "function".to_owned(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: None,
            }],
        };
        let created = commit(&mut workspace, &create).expect("create function");
        let function = created.returned_bindings[0].1;
        assert_eq!(function.serial(), 4);

        let delete = Transaction {
            workspace: id,
            base_revision: Revision::new(2),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::DeleteOwnedSubtree {
                root: NodeTarget::Existing(function),
            }],
        };
        commit(&mut workspace, &delete).expect("delete function");
        assert!(
            workspace
                .snapshot(Revision::new(2))
                .expect("old snapshot")
                .node(function)
                .is_ok()
        );
        let current = workspace.head().expect("current snapshot");
        assert!(current.node(function).is_err());
        assert!(current.contains_tombstone(function.serial()));

        let replacement = Transaction {
            workspace: id,
            base_revision: Revision::new(3),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(4),
                module: NodeTarget::Existing(module),
                name: "replacement".to_owned(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: None,
            }],
        };
        let replacement = commit(&mut workspace, &replacement).expect("replacement function");
        assert_eq!(replacement.returned_bindings[0].1.serial(), 5);
    }

    #[test]
    fn large_user_controlled_subtree_deletion_uses_an_explicit_work_stack() {
        let id = WorkspaceId::from_bytes([15; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let package = DraftSymbol::generated(1);
        let module = DraftSymbol::generated(2);
        let function = DraftSymbol::generated(3);
        let first_value = DraftSymbol::generated(6);
        let body_operations = (0..10_000_u32)
            .map(|offset| ExpressionDraft {
                symbol: Some(DraftSymbol::generated(6 + offset)),
                operation: ExpressionKindDraft::ConstI64(i64::from(offset)),
            })
            .collect();
        let operations = vec![
            TransactionOp::CreatePackage {
                symbol: package,
                name: "package".to_owned(),
            },
            TransactionOp::CreateModule {
                symbol: module,
                package: NodeTarget::Draft(package),
                name: "module".to_owned(),
            },
            TransactionOp::CreateFunction {
                symbol: function,
                module: NodeTarget::Draft(module),
                name: "main".to_owned(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: body_operations,
                    return_value: ValueDraft::OperationResult {
                        operation: NodeTarget::Draft(first_value),
                        output: 0,
                    },
                }),
            },
            TransactionOp::SetEntryFunction {
                package: NodeTarget::Draft(package),
                function: NodeTarget::Draft(function),
            },
        ];
        let create = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations,
        };
        let created = commit(&mut workspace, &create).expect("large graph commit");
        let package_id = created.returned_bindings[0].1;
        assert_eq!(workspace.head().expect("head").node_count(), 10_007);

        let delete = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::DeleteOwnedSubtree {
                root: NodeTarget::Existing(package_id),
            }],
        };
        commit(&mut workspace, &delete).expect("iterative subtree deletion");
        assert_eq!(workspace.head().expect("head").node_count(), 1);
        assert!(
            workspace
                .head()
                .expect("head")
                .contains_tombstone(package_id.serial())
        );
    }

    fn incomplete_program(id: WorkspaceId) -> Transaction {
        let local = NodeTarget::Draft;
        let value = |symbol| ValueDraft::OperationResult {
            operation: local(symbol),
            output: 0,
        };
        Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "app".to_owned(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: local(DraftSymbol::generated(1)),
                    name: "root".to_owned(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(3),
                    module: local(DraftSymbol::generated(2)),
                    name: "main".to_owned(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(6)),
                                operation: ExpressionKindDraft::ConstI64(40),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(7)),
                                operation: ExpressionKindDraft::ConstI64(2),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(8)),
                                operation: ExpressionKindDraft::ConstBool(true),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(9)),
                                operation: ExpressionKindDraft::Hole {
                                    expected: SemanticType::I64.into(),
                                },
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(10)),
                                operation: ExpressionKindDraft::ConstI64(99),
                            },
                        ],
                        return_value: value(DraftSymbol::generated(9)),
                    }),
                },
                TransactionOp::SetEntryFunction {
                    package: local(DraftSymbol::generated(1)),
                    function: local(DraftSymbol::generated(3)),
                },
            ],
        }
    }

    fn prepared_operation_owner(snapshot: &Snapshot, operation: NodeId) -> NodeId {
        match snapshot.node(operation).expect("operation") {
            Node::Operation { owner, .. } => *owner,
            _ => panic!("operation kind"),
        }
    }

    fn binding(receipt: &TransactionReceipt, symbol: u32) -> NodeId {
        receipt
            .returned_bindings
            .iter()
            .find_map(|(candidate, node)| (candidate.generated_number() == symbol).then_some(*node))
            .expect("selected binding")
    }

    #[test]
    fn response_projection_is_selected_bounded_and_validate_only_is_predictive() {
        let id = WorkspaceId::from_bytes([0x71; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let transaction = create_package_and_module(id);
        let selected = ApplyTransactionRequest {
            transaction: transaction.clone(),
            response: TransactionResponseSpec {
                return_symbols: vec![DraftSymbol::generated(2)],
            },
        };
        let prepared = workspace
            .prepare_transaction(&selected)
            .expect("selected receipt");
        assert_eq!(prepared.receipt.created_count, 2);
        assert_eq!(prepared.receipt.returned_bindings.len(), 1);
        assert_eq!(
            prepared.receipt.returned_bindings[0].0,
            DraftSymbol::generated(2)
        );

        for return_symbols in [
            vec![DraftSymbol::generated(1), DraftSymbol::generated(1)],
            vec![DraftSymbol::generated(3)],
        ] {
            let invalid = ApplyTransactionRequest {
                transaction: transaction.clone(),
                response: TransactionResponseSpec { return_symbols },
            };
            assert_eq!(
                workspace
                    .prepare_transaction(&invalid)
                    .expect_err("invalid response projection")
                    .code,
                ErrorCode::InvalidDraftSymbol
            );
        }

        let mut too_many = Vec::new();
        for value in 0..=MAX_RETURNED_BINDINGS {
            too_many.push(DraftSymbol::generated(
                u32::try_from(value).expect("symbol"),
            ));
        }
        let invalid = ApplyTransactionRequest {
            transaction: transaction.clone(),
            response: TransactionResponseSpec {
                return_symbols: too_many,
            },
        };
        assert_eq!(
            workspace
                .prepare_transaction(&invalid)
                .expect_err("oversized response projection")
                .code,
            ErrorCode::PolicyExceeded
        );

        let mut validate = selected.clone();
        validate.transaction.mode = TransactionMode::ValidateOnly;
        let predicted = workspace
            .prepare_transaction(&validate)
            .expect("validate-only receipt")
            .receipt;
        assert!(!predicted.published);
        let mut commit_request = validate.clone();
        commit_request.transaction.mode = TransactionMode::Commit;
        let committed = workspace
            .prepare_transaction(&commit_request)
            .expect("commit receipt")
            .receipt;
        let mut expected = predicted;
        expected.published = true;
        assert_eq!(committed, expected);

        validate.transaction.idempotency_key = Some(IdempotencyKey::from_bytes([1; 16]));
        assert_eq!(
            workspace
                .prepare_transaction(&validate)
                .expect_err("validate-only idempotency")
                .code,
            ErrorCode::InvalidOperand
        );
    }

    #[test]
    fn change_digest_includes_exact_scalar_details() {
        let id = WorkspaceId::from_bytes([0x76; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
        let two = binding(&created, 7);
        let edit = |value| Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::ReplaceOperation {
                operation: NodeTarget::Existing(two),
                replacement: OperationDraft::ConstI64(value),
            }],
        };
        let three = workspace
            .prepare_transaction(&request(&edit(3)))
            .expect("replace with three")
            .receipt;
        let four = workspace
            .prepare_transaction(&request(&edit(4)))
            .expect("replace with four")
            .receipt;
        assert_eq!(three.change_count, four.change_count);
        assert_ne!(three.change_digest, four.change_digest);
        assert_ne!(three.hash, four.hash);
    }

    #[test]
    fn same_typed_nominal_definition_changes_are_classified_and_hashed() {
        let id = WorkspaceId::from_bytes([0x78; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let created = commit(
            &mut workspace,
            &Transaction {
                workspace: id,
                base_revision: Revision::INITIAL,
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![
                    TransactionOp::CreatePackage {
                        symbol: DraftSymbol::generated(1),
                        name: "p".into(),
                    },
                    TransactionOp::CreateModule {
                        symbol: DraftSymbol::generated(2),
                        package: draft_symbol(1),
                        name: "m".into(),
                    },
                    TransactionOp::CreateProductType {
                        symbol: DraftSymbol::generated(3),
                        module: draft_symbol(2),
                        name: "Pair".into(),
                        fields: vec![
                            ProductFieldDraft {
                                symbol: DraftSymbol::generated(4),
                                name: "left".into(),
                                ty: TypeDraft::I64,
                            },
                            ProductFieldDraft {
                                symbol: DraftSymbol::generated(5),
                                name: "right".into(),
                                ty: TypeDraft::I64,
                            },
                        ],
                    },
                    TransactionOp::CreateSumType {
                        symbol: DraftSymbol::generated(6),
                        module: draft_symbol(2),
                        name: "Choice".into(),
                        variants: vec![
                            SumVariantDraft {
                                symbol: DraftSymbol::generated(7),
                                name: "First".into(),
                                payload: None,
                            },
                            SumVariantDraft {
                                symbol: DraftSymbol::generated(8),
                                name: "Second".into(),
                                payload: None,
                            },
                        ],
                    },
                    TransactionOp::CreateFunction {
                        symbol: DraftSymbol::generated(9),
                        module: draft_symbol(2),
                        name: "main".into(),
                        parameters: Vec::new(),
                        result: TypeDraft::I64,
                        body: Some(FunctionBodyDraft {
                            operations: vec![
                                draft_expression(10, ExpressionKindDraft::ConstI64(1)),
                                draft_expression(
                                    11,
                                    ExpressionKindDraft::ConstructProduct {
                                        product: draft_symbol(3),
                                        fields: vec![
                                            ProductFieldValueDraft {
                                                field: draft_symbol(4),
                                                value: draft_result(10),
                                            },
                                            ProductFieldValueDraft {
                                                field: draft_symbol(5),
                                                value: draft_result(10),
                                            },
                                        ],
                                    },
                                ),
                                draft_expression(
                                    12,
                                    ExpressionKindDraft::ProjectField {
                                        value: draft_result(11),
                                        field: draft_symbol(4),
                                    },
                                ),
                                draft_expression(
                                    13,
                                    ExpressionKindDraft::ConstructVariant {
                                        variant: draft_symbol(7),
                                        payload: None,
                                    },
                                ),
                            ],
                            return_value: draft_result(12),
                        }),
                    },
                ],
            },
        )
        .expect("nominal definitions");
        let field_before = binding(&created, 4);
        let field_after = binding(&created, 5);
        let variant_before = binding(&created, 7);
        let variant_after = binding(&created, 8);
        let product_value = binding(&created, 11);
        let projection = binding(&created, 12);
        let construction = binding(&created, 13);
        let base = workspace.snapshot(Revision::new(1)).expect("base");

        let cases = [
            (
                projection,
                OperationDraft::ProjectField {
                    value: ValueDraft::OperationResult {
                        operation: NodeTarget::Existing(product_value),
                        output: 0,
                    },
                    field: NodeTarget::Existing(field_after),
                },
                field_before,
                field_after,
            ),
            (
                construction,
                OperationDraft::ConstructVariant {
                    variant: NodeTarget::Existing(variant_after),
                    payload: None,
                },
                variant_before,
                variant_after,
            ),
        ];
        for (operation, replacement, before, after) in cases {
            let prepared = workspace
                .prepare_transaction(&ApplyTransactionRequest {
                    transaction: Transaction {
                        workspace: id,
                        base_revision: Revision::new(1),
                        idempotency_key: None,
                        mode: TransactionMode::ValidateOnly,
                        operations: vec![TransactionOp::ReplaceOperation {
                            operation: NodeTarget::Existing(operation),
                            replacement,
                        }],
                    },
                    response: TransactionResponseSpec::default(),
                })
                .expect("same-typed definition replacement");
            let semantic_diff = diff::between(base, &prepared.snapshot);
            assert_eq!(semantic_diff, diff::between(base, &prepared.snapshot));
            assert_ne!(semantic_diff.digest.as_bytes(), [0; 32]);
            assert_eq!(prepared.receipt.change_digest, semantic_diff.digest);
            assert!(semantic_diff.changes.iter().any(|change| {
                change.node == operation
                    && matches!(
                        change.kind,
                        crate::diff::ChangeKind::DefinitionChanged {
                            before: actual_before,
                            after: actual_after,
                        } if actual_before == before && actual_after == after
                    )
            }));
        }
    }

    #[test]
    fn change_digest_distinguishes_refinement_payloads_and_same_typed_operands() {
        let id = WorkspaceId::from_bytes([0x77; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
        let forty = binding(&created, 6);
        let two = binding(&created, 7);
        let hole = binding(&created, 9);
        let refinement = |value| Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::ConstI64(value),
            }],
        };
        let two_refinement = workspace
            .prepare_transaction(&request(&refinement(2)))
            .expect("refine to two");
        let three_refinement = workspace
            .prepare_transaction(&request(&refinement(3)))
            .expect("refine to three");
        assert_ne!(two_refinement.receipt.hash, three_refinement.receipt.hash);
        assert_ne!(
            two_refinement.receipt.change_digest,
            three_refinement.receipt.change_digest
        );
        let two_change = diff::between(
            workspace.snapshot(Revision::new(1)).expect("base"),
            &two_refinement.snapshot,
        );
        assert!(two_change.changes.iter().any(|change| {
            matches!(
                &change.kind,
                crate::diff::ChangeKind::OperationRefined {
                    replacement: OperationKind::ConstI64(2),
                    ..
                }
            )
        }));

        let add_refinement = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::AddI64 {
                    lhs: ValueDraft::OperationResult {
                        operation: NodeTarget::Existing(forty),
                        output: 0,
                    },
                    rhs: ValueDraft::OperationResult {
                        operation: NodeTarget::Existing(two),
                        output: 0,
                    },
                },
            }],
        };
        commit(&mut workspace, &add_refinement).expect("publish add refinement");
        let replacement = |index, operation| Transaction {
            workspace: id,
            base_revision: Revision::new(2),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::ReplaceOperand {
                operation: NodeTarget::Existing(hole),
                index,
                value: ValueDraft::OperationResult {
                    operation: NodeTarget::Existing(operation),
                    output: 0,
                },
            }],
        };
        let replace_left = workspace
            .prepare_transaction(&request(&replacement(0, two)))
            .expect("replace left operand");
        let replace_right = workspace
            .prepare_transaction(&request(&replacement(1, forty)))
            .expect("replace right operand");
        assert_ne!(replace_left.receipt.hash, replace_right.receipt.hash);
        assert_ne!(
            replace_left.receipt.change_digest,
            replace_right.receipt.change_digest
        );
        let left_diff = diff::between(
            workspace.snapshot(Revision::new(2)).expect("refined base"),
            &replace_left.snapshot,
        );
        assert!(left_diff.changes.iter().any(|change| {
            matches!(
                change.kind,
                crate::diff::ChangeKind::OperandChanged {
                    index: 0,
                    before: Some(ValueRef::OperationResult { operation, .. }),
                    after: Some(ValueRef::OperationResult {
                        operation: replacement,
                        ..
                    }),
                } if operation == forty && replacement == two
            )
        }));
    }

    #[test]
    fn create_then_delete_returns_selected_tombstoned_identity_and_explicit_change() {
        let id = WorkspaceId::from_bytes([0x74; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "temporary".to_owned(),
                },
                TransactionOp::DeleteOwnedSubtree {
                    root: NodeTarget::Draft(DraftSymbol::generated(1)),
                },
            ],
        };
        let prepared = workspace
            .prepare_transaction(&request(&transaction))
            .expect("create then delete");
        let allocated = binding(&prepared.receipt, 1);
        assert!(prepared.snapshot.contains_tombstone(allocated.serial()));
        assert!(prepared.receipt.change_count > 0);
        let before = workspace.head().expect("before");
        let semantic_diff = diff::between(before, &prepared.snapshot);
        assert!(semantic_diff.changes.iter().any(|change| {
            change.node == allocated
                && matches!(change.kind, crate::diff::ChangeKind::AllocatedAndTombstoned)
        }));
    }

    #[test]
    fn hole_refinement_preserves_identity_position_use_history_and_diff() {
        let id = WorkspaceId::from_bytes([0x72; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
        let hole = binding(&created, 9);
        let forty = binding(&created, 6);
        let two = binding(&created, 7);
        let block = prepared_operation_owner(workspace.head().expect("head"), hole);
        let return_operation = match workspace.head().expect("head").node(block).expect("block") {
            Node::Block {
                terminator: Some(terminator),
                ..
            } => *terminator,
            _ => panic!("block terminator"),
        };
        let old = workspace
            .snapshot(Revision::new(1))
            .expect("old snapshot")
            .clone();
        let refine = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::AddI64 {
                    lhs: ValueDraft::OperationResult {
                        operation: NodeTarget::Existing(forty),
                        output: 0,
                    },
                    rhs: ValueDraft::OperationResult {
                        operation: NodeTarget::Existing(two),
                        output: 0,
                    },
                },
            }],
        };
        let refined = commit(&mut workspace, &refine).expect("refine hole");
        assert_eq!(refined.created_count, 0);
        assert!(!refined.complete_before);
        assert!(refined.complete_after);
        let current = workspace.head().expect("refined snapshot");
        assert!(matches!(
            old.node(hole).expect("old hole"),
            Node::Operation {
                operation: OperationKind::Hole { .. },
                ..
            }
        ));
        assert!(matches!(
            current.node(hole).expect("refined operation"),
            Node::Operation {
                operation: OperationKind::AddI64 { .. },
                ..
            }
        ));
        let Node::Block { operations, .. } = current.node(block).expect("block") else {
            panic!("block kind");
        };
        assert_eq!(operations.iter().position(|id| *id == hole), Some(3));
        let Node::Operation {
            operation: OperationKind::Return { value },
            ..
        } = current.node(return_operation).expect("return")
        else {
            panic!("return kind");
        };
        assert_eq!(
            *value,
            ValueRef::OperationResult {
                operation: hole,
                output: 0,
            }
        );
        let semantic_diff = diff::between(&old, current);
        assert_eq!(semantic_diff.change_count(), refined.change_count);
        assert_eq!(semantic_diff.digest, refined.change_digest);
        assert!(semantic_diff.changes.iter().any(|change| {
            change.node == hole
                && matches!(
                    change.kind,
                    crate::diff::ChangeKind::OperationRefined {
                        before: crate::schema::OperationCode::Hole,
                        after: crate::schema::OperationCode::AddI64,
                        result_type: SemanticType::I64,
                        ..
                    }
                )
        }));
    }

    #[test]
    fn hole_refinement_to_identity_targeted_call_uses_snapshot_signature() {
        let id = WorkspaceId::from_bytes([0x79; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
        let module = binding(&created, 2);
        let hole = binding(&created, 9);
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(12),
                    module: NodeTarget::Existing(module),
                    name: "callee".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: None,
                },
                TransactionOp::RefineHole {
                    hole: NodeTarget::Existing(hole),
                    replacement: OperationDraft::Call {
                        function: NodeTarget::Draft(DraftSymbol::generated(12)),
                        arguments: vec![],
                    },
                },
            ],
        };
        let prepared = workspace
            .prepare_transaction(&request(&transaction))
            .expect("call refinement");
        let Node::Operation {
            operation:
                OperationKind::Call {
                    function,
                    arguments,
                },
            ..
        } = prepared.snapshot.node(hole).expect("refined call")
        else {
            panic!("call refinement kind")
        };
        assert_eq!(*function, binding(&prepared.receipt, 12));
        assert!(arguments.is_empty());
        assert!(
            diff::between(workspace.head().expect("old head"), &prepared.snapshot)
                .changes
                .iter()
                .any(|change| matches!(
                    change.kind,
                    crate::diff::ChangeKind::OperationRefined {
                        after: crate::schema::OperationCode::Call,
                        ..
                    }
                ))
        );
    }

    #[test]
    fn structured_expansion_is_depth_first_predictive_and_supports_forward_calls() {
        let id = WorkspaceId::from_bytes([0x7a; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let local = |symbol| NodeTarget::Draft(DraftSymbol::generated(symbol));
        let result = |symbol| ValueDraft::OperationResult {
            operation: local(symbol),
            output: 0,
        };
        let block_argument = |symbol| ValueDraft::BlockArgument(local(symbol));
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::ValidateOnly,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: local(1),
                    name: "root".into(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(5),
                    module: local(2),
                    name: "main".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(6)),
                                operation: ExpressionKindDraft::ConstI64(0),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(7)),
                                operation: ExpressionKindDraft::ConstI64(10),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(8)),
                                operation: ExpressionKindDraft::LtI64 {
                                    lhs: result(6),
                                    rhs: result(7),
                                },
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(9)),
                                operation: ExpressionKindDraft::ForI64 {
                                    start: result(6),
                                    end_exclusive: result(7),
                                    step: 1,
                                    initial: result(6),
                                    carried: SemanticType::I64.into(),
                                    index_symbol: DraftSymbol::generated(10),
                                    carried_symbol: DraftSymbol::generated(11),
                                    body: YieldingBodyDraft {
                                        operations: vec![ExpressionDraft {
                                            symbol: Some(DraftSymbol::generated(12)),
                                            operation: ExpressionKindDraft::AddI64 {
                                                lhs: block_argument(11),
                                                rhs: block_argument(10),
                                            },
                                        }],
                                        yield_value: result(12),
                                    },
                                },
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(13)),
                                operation: ExpressionKindDraft::If {
                                    condition: result(8),
                                    result: SemanticType::I64.into(),
                                    then_body: YieldingBodyDraft {
                                        operations: vec![ExpressionDraft {
                                            symbol: Some(DraftSymbol::generated(14)),
                                            operation: ExpressionKindDraft::Call {
                                                function: local(20),
                                                arguments: vec![result(9)],
                                            },
                                        }],
                                        yield_value: result(14),
                                    },
                                    else_body: YieldingBodyDraft {
                                        operations: vec![ExpressionDraft {
                                            symbol: Some(DraftSymbol::generated(15)),
                                            operation: ExpressionKindDraft::ConstI64(0),
                                        }],
                                        yield_value: result(15),
                                    },
                                },
                            },
                        ],
                        return_value: result(13),
                    }),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(20),
                    module: local(2),
                    name: "later".into(),
                    parameters: vec![FunctionParameterDraft {
                        symbol: DraftSymbol::generated(21),
                        name: "value".into(),
                        ty: SemanticType::I64.into(),
                    }],
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: Vec::new(),
                        return_value: ValueDraft::FunctionParameter(local(21)),
                    }),
                },
                TransactionOp::SetEntryFunction {
                    package: local(1),
                    function: local(5),
                },
            ],
        };
        let response = TransactionResponseSpec {
            return_symbols: [1, 2, 5, 6, 9, 10, 11, 12, 13, 14, 15, 20, 21]
                .into_iter()
                .map(DraftSymbol::generated)
                .collect(),
        };
        let predicted = workspace
            .prepare_transaction(&ApplyTransactionRequest {
                transaction: transaction.clone(),
                response: response.clone(),
            })
            .expect("validate structured");
        assert_eq!(predicted.receipt.created_count, 30);
        assert_eq!(binding(&predicted.receipt, 5).serial(), 4);
        assert_eq!(binding(&predicted.receipt, 10).serial(), 13);
        assert_eq!(binding(&predicted.receipt, 11).serial(), 14);
        assert_eq!(binding(&predicted.receipt, 20).serial(), 27);
        let Node::Operation {
            operation: OperationKind::Call { function, .. },
            ..
        } = predicted
            .snapshot
            .node(binding(&predicted.receipt, 14))
            .expect("call")
        else {
            panic!("call kind")
        };
        assert_eq!(*function, binding(&predicted.receipt, 20));
        let mut committed_transaction = transaction;
        committed_transaction.mode = TransactionMode::Commit;
        let committed = workspace
            .prepare_transaction(&ApplyTransactionRequest {
                transaction: committed_transaction,
                response,
            })
            .expect("commit structured");
        let mut expected = predicted.receipt;
        expected.published = true;
        assert_eq!(committed.receipt, expected);
    }

    #[test]
    fn structured_symbols_reject_zero_duplicates_undeclared_and_private_selection_atomically() {
        let id = WorkspaceId::from_bytes([0x7b; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let base_transaction = |expression: ExpressionDraft| Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: NodeTarget::Draft(DraftSymbol::generated(1)),
                    name: "root".into(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(3),
                    module: NodeTarget::Draft(DraftSymbol::generated(2)),
                    name: "main".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![expression],
                        return_value: ValueDraft::OperationResult {
                            operation: NodeTarget::Draft(DraftSymbol::generated(4)),
                            output: 0,
                        },
                    }),
                },
            ],
        };
        let unchecked = |transaction| ApplyTransactionRequest {
            transaction,
            response: TransactionResponseSpec::default(),
        };
        let invalid = serde_json::from_str::<DraftSymbol>("\"\"").expect("raw invalid symbol");
        let zero = base_transaction(ExpressionDraft {
            symbol: Some(invalid),
            operation: ExpressionKindDraft::ConstI64(1),
        });
        let error = workspace
            .prepare_transaction(&unchecked(zero))
            .expect_err("zero");
        assert_eq!(error.code, ErrorCode::InvalidDraftSymbol);
        assert_eq!(error.operation_index, Some(2));
        assert_eq!(error.draft_symbol, Some(invalid));
        for raw in ["Bad", &"x".repeat(crate::ids::MAX_DRAFT_SYMBOL_BYTES + 1)] {
            let encoded = serde_json::to_string(raw).expect("invalid symbol JSON");
            let invalid = serde_json::from_str::<DraftSymbol>(&encoded)
                .expect("raw invalid symbol remains typed for semantic diagnostics");
            let error = workspace
                .prepare_transaction(&unchecked(base_transaction(ExpressionDraft {
                    symbol: Some(invalid),
                    operation: ExpressionKindDraft::ConstI64(1),
                })))
                .expect_err("invalid symbol");
            assert_eq!(error.code, ErrorCode::InvalidDraftSymbol);
            assert_eq!(error.operation_index, Some(2));
            assert_eq!(error.draft_symbol, Some(invalid));
        }
        let duplicate = base_transaction(ExpressionDraft {
            symbol: Some(DraftSymbol::generated(3)),
            operation: ExpressionKindDraft::ConstI64(1),
        });
        let error = workspace
            .prepare_transaction(&unchecked(duplicate))
            .expect_err("duplicate");
        assert_eq!(error.code, ErrorCode::DuplicateDraftSymbol);
        assert_eq!(error.operation_index, Some(2));
        assert_eq!(error.draft_symbol, Some(DraftSymbol::generated(3)));
        let undeclared = base_transaction(ExpressionDraft {
            symbol: Some(DraftSymbol::generated(4)),
            operation: ExpressionKindDraft::AddI64 {
                lhs: ValueDraft::OperationResult {
                    operation: NodeTarget::Draft(DraftSymbol::generated(99)),
                    output: 0,
                },
                rhs: ValueDraft::OperationResult {
                    operation: NodeTarget::Draft(DraftSymbol::generated(4)),
                    output: 0,
                },
            },
        });
        let error = workspace
            .prepare_transaction(&unchecked(undeclared))
            .expect_err("undeclared");
        assert_eq!(error.code, ErrorCode::InvalidDraftSymbol);
        assert_eq!(error.operation_index, Some(2));
        assert_eq!(error.draft_symbol, Some(DraftSymbol::generated(99)));
        let valid = base_transaction(ExpressionDraft {
            symbol: Some(DraftSymbol::generated(4)),
            operation: ExpressionKindDraft::ConstI64(1),
        });
        let private = ApplyTransactionRequest {
            transaction: valid,
            response: TransactionResponseSpec {
                return_symbols: vec![DraftSymbol::generated(u32::MAX)],
            },
        };
        assert_eq!(
            workspace
                .prepare_transaction(&private)
                .expect_err("private binding")
                .code,
            ErrorCode::InvalidDraftSymbol
        );
        assert_eq!(workspace.head().expect("head").next_serial(), 2);
    }

    #[test]
    fn canonical_allocation_errors_remap_to_public_source_and_explicit_symbol() {
        let id = WorkspaceId::from_bytes([0x7a; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let symbol = DraftSymbol::generated(77);
        let edits = vec![
            CanonicalEdit::CreatePackage {
                symbol,
                name: "first".into(),
            },
            CanonicalEdit::CreatePackage {
                symbol,
                name: "duplicate".into(),
            },
        ];
        let error = allocate_symbols(
            workspace.head().expect("head"),
            &edits,
            &[3, 8],
            &BTreeSet::from([symbol]),
            &BTreeMap::new(),
        )
        .expect_err("duplicate canonical allocation");
        assert_eq!(error.code, ErrorCode::DuplicateDraftSymbol);
        assert_eq!(error.operation_index, Some(8));
        assert_eq!(error.draft_symbol, Some(symbol));
    }

    #[test]
    fn insert_expression_rejects_staged_block_and_anchor_with_public_source_atomically() {
        let id = WorkspaceId::from_bytes([0x7e; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let local = |symbol| NodeTarget::Draft(DraftSymbol::generated(symbol));
        let staged_block = NodeId::new(id, 6).expect("predicted staged block");
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: local(1),
                    name: "root".into(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(3),
                    module: local(2),
                    name: "main".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![ExpressionDraft {
                            symbol: Some(DraftSymbol::generated(4)),
                            operation: ExpressionKindDraft::ConstI64(1),
                        }],
                        return_value: ValueDraft::OperationResult {
                            operation: local(4),
                            output: 0,
                        },
                    }),
                },
                TransactionOp::InsertExpression {
                    block: staged_block,
                    before: None,
                    expression: ExpressionDraft {
                        symbol: Some(DraftSymbol::generated(5)),
                        operation: ExpressionKindDraft::ConstI64(2),
                    },
                },
            ],
        };
        let error = workspace
            .prepare_transaction(&ApplyTransactionRequest {
                transaction,
                response: TransactionResponseSpec::default(),
            })
            .expect_err("staged block");
        assert_eq!(error.operation_index, Some(3));
        assert_eq!(error.target, Some(staged_block));
        assert_eq!(workspace.head().expect("head").next_serial(), 2);

        let committed_id = WorkspaceId::from_bytes([0x7f; 16]);
        let mut committed = Workspace::new(committed_id).expect("workspace");
        let created = commit(&mut committed, &incomplete_program(committed_id)).expect("fixture");
        let hole = binding(&created, 9);
        let block = prepared_operation_owner(committed.head().expect("head"), hole);
        let predicted_anchor = NodeId::new(
            committed.id(),
            committed.head().expect("head").next_serial(),
        )
        .expect("predicted anchor");
        let transaction = Transaction {
            workspace: committed.id(),
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::InsertExpression {
                    block,
                    before: Some(hole),
                    expression: ExpressionDraft {
                        symbol: Some(DraftSymbol::generated(100)),
                        operation: ExpressionKindDraft::ConstI64(1),
                    },
                },
                TransactionOp::InsertExpression {
                    block,
                    before: Some(predicted_anchor),
                    expression: ExpressionDraft {
                        symbol: Some(DraftSymbol::generated(101)),
                        operation: ExpressionKindDraft::ConstI64(2),
                    },
                },
            ],
        };
        let frontier = committed.head().expect("head").next_serial();
        let error = committed
            .prepare_transaction(&ApplyTransactionRequest {
                transaction,
                response: TransactionResponseSpec::default(),
            })
            .expect_err("staged anchor");
        assert_eq!(error.operation_index, Some(1));
        assert_eq!(error.target, Some(predicted_anchor));
        assert_eq!(committed.head().expect("head").next_serial(), frontier);
    }

    #[test]
    fn preallocation_scan_maps_wrong_local_call_target_to_public_operation() {
        let id = WorkspaceId::from_bytes([0x80; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let local = |symbol| NodeTarget::Draft(DraftSymbol::generated(symbol));
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: local(1),
                    name: "root".into(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(3),
                    module: local(2),
                    name: "bad".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![ExpressionDraft {
                            symbol: Some(DraftSymbol::generated(4)),
                            operation: ExpressionKindDraft::Call {
                                function: local(1),
                                arguments: Vec::new(),
                            },
                        }],
                        return_value: ValueDraft::OperationResult {
                            operation: local(4),
                            output: 0,
                        },
                    }),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(5),
                    module: local(2),
                    name: "later".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: None,
                },
            ],
        };
        let error = workspace
            .prepare_transaction(&ApplyTransactionRequest {
                transaction,
                response: TransactionResponseSpec::default(),
            })
            .expect_err("bad call target");
        assert_eq!(error.code, ErrorCode::WrongKind);
        assert_eq!(error.operation_index, Some(2));
        assert_eq!(error.draft_symbol, Some(DraftSymbol::generated(1)));
        assert_eq!(workspace.head().expect("head").next_serial(), 2);
    }

    #[test]
    fn structured_request_depth_item_and_output_policies_reject_before_allocation() {
        let id = WorkspaceId::from_bytes([0x7d; 16]);
        let block = NodeId::new(id, 2).expect("block ID");
        let existing = NodeTarget::Existing(block);

        let top_level = (0..=MAX_STRUCTURED_DRAFT_ITEMS)
            .map(|_| TransactionOp::RenameNode {
                node: existing,
                name: "x".into(),
            })
            .collect::<Vec<_>>();
        let error = scan_explicit_symbols(&top_level).expect_err("top-level item policy");
        assert_eq!(error.code, ErrorCode::PolicyExceeded);
        assert_eq!(
            error.operation_index,
            Some(MAX_STRUCTURED_DRAFT_ITEMS as u32)
        );

        let mut mixed = (0..MAX_STRUCTURED_DRAFT_ITEMS - 2)
            .map(|_| TransactionOp::RenameNode {
                node: existing,
                name: "x".into(),
            })
            .collect::<Vec<_>>();
        mixed.push(TransactionOp::InsertExpression {
            block,
            before: None,
            expression: ExpressionDraft {
                symbol: Some(DraftSymbol::generated(40_000)),
                operation: ExpressionKindDraft::Call {
                    function: existing,
                    arguments: vec![ValueDraft::FunctionParameter(existing); 3],
                },
            },
        });
        let error = scan_explicit_symbols(&mixed).expect_err("mixed item policy");
        assert_eq!(error.code, ErrorCode::PolicyExceeded);
        assert_eq!(
            error.operation_index,
            Some((MAX_STRUCTURED_DRAFT_ITEMS - 2) as u32)
        );

        let mut expression = ExpressionDraft {
            symbol: Some(DraftSymbol::generated(1)),
            operation: ExpressionKindDraft::ConstI64(1),
        };
        for depth in 0..=MAX_STRUCTURED_DRAFT_DEPTH {
            let inner_symbol = expression.symbol;
            let else_symbol = DraftSymbol::generated(10_000 + depth as u32);
            expression = ExpressionDraft {
                symbol: Some(DraftSymbol::generated(20_000 + depth as u32)),
                operation: ExpressionKindDraft::If {
                    condition: ValueDraft::OperationResult {
                        operation: existing,
                        output: 0,
                    },
                    result: SemanticType::I64.into(),
                    then_body: YieldingBodyDraft {
                        operations: vec![expression],
                        yield_value: ValueDraft::OperationResult {
                            operation: NodeTarget::Draft(inner_symbol.expect("bound expression")),
                            output: 0,
                        },
                    },
                    else_body: YieldingBodyDraft {
                        operations: vec![ExpressionDraft {
                            symbol: Some(else_symbol),
                            operation: ExpressionKindDraft::ConstI64(0),
                        }],
                        yield_value: ValueDraft::OperationResult {
                            operation: NodeTarget::Draft(else_symbol),
                            output: 0,
                        },
                    },
                },
            };
        }
        let too_deep = [TransactionOp::InsertExpression {
            block,
            before: None,
            expression,
        }];
        assert_eq!(
            scan_explicit_symbols(&too_deep)
                .expect_err("depth policy")
                .code,
            ErrorCode::PolicyExceeded
        );

        let oversized = [TransactionOp::InsertExpression {
            block,
            before: None,
            expression: ExpressionDraft {
                symbol: Some(DraftSymbol::generated(1)),
                operation: ExpressionKindDraft::Call {
                    function: existing,
                    arguments: vec![
                        ValueDraft::FunctionParameter(existing);
                        MAX_STRUCTURED_DRAFT_ITEMS + 1
                    ],
                },
            },
        }];
        assert_eq!(
            scan_explicit_symbols(&oversized)
                .expect_err("item policy")
                .code,
            ErrorCode::PolicyExceeded
        );

        for fine_grained in [
            TransactionOp::ReplaceOperation {
                operation: existing,
                replacement: OperationDraft::Call {
                    function: existing,
                    arguments: vec![
                        ValueDraft::FunctionParameter(existing);
                        MAX_STRUCTURED_DRAFT_ITEMS
                    ],
                },
            },
            TransactionOp::RefineHole {
                hole: existing,
                replacement: OperationDraft::Call {
                    function: existing,
                    arguments: vec![
                        ValueDraft::FunctionParameter(existing);
                        MAX_STRUCTURED_DRAFT_ITEMS
                    ],
                },
            },
        ] {
            let request = ApplyTransactionRequest {
                transaction: Transaction {
                    workspace: id,
                    base_revision: Revision::INITIAL,
                    idempotency_key: None,
                    mode: TransactionMode::Commit,
                    operations: vec![fine_grained],
                },
                response: TransactionResponseSpec::default(),
            };
            let workspace = Workspace::new(id).expect("workspace");
            let before = artifact::encode(workspace.head().expect("head")).expect("artifact");
            let error = workspace
                .prepare_transaction(&request)
                .expect_err("fine-grained call aggregate policy");
            assert_eq!(error.code, ErrorCode::PolicyExceeded);
            assert_eq!(workspace.head_revision(), Revision::INITIAL);
            assert_eq!(workspace.head().expect("head").next_serial(), 2);
            assert_eq!(
                artifact::encode(workspace.head().expect("head")).expect("artifact"),
                before
            );
        }

        let invalid_output = [TransactionOp::InsertExpression {
            block,
            before: None,
            expression: ExpressionDraft {
                symbol: Some(DraftSymbol::generated(1)),
                operation: ExpressionKindDraft::AddI64 {
                    lhs: ValueDraft::OperationResult {
                        operation: existing,
                        output: 1,
                    },
                    rhs: ValueDraft::OperationResult {
                        operation: existing,
                        output: 0,
                    },
                },
            },
        }];
        assert_eq!(
            scan_explicit_symbols(&invalid_output)
                .expect_err("output index")
                .code,
            ErrorCode::InvalidOperand
        );
    }

    #[test]
    fn mutual_function_bodies_resolve_local_calls_in_one_transaction() {
        let id = WorkspaceId::from_bytes([0x7c; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let local = |symbol| NodeTarget::Draft(DraftSymbol::generated(symbol));
        let call_body = |symbol, target| FunctionBodyDraft {
            operations: vec![ExpressionDraft {
                symbol: Some(DraftSymbol::generated(symbol)),
                operation: ExpressionKindDraft::Call {
                    function: local(target),
                    arguments: Vec::new(),
                },
            }],
            return_value: ValueDraft::OperationResult {
                operation: local(symbol),
                output: 0,
            },
        };
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: local(1),
                    name: "root".into(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(3),
                    module: local(2),
                    name: "a".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(call_body(5, 4)),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(4),
                    module: local(2),
                    name: "b".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(call_body(6, 3)),
                },
            ],
        };
        let receipt = commit(&mut workspace, &transaction).expect("one-transaction mutual calls");
        let function_a = binding(&receipt, 3);
        let function_b = binding(&receipt, 4);
        assert_eq!(function_a.serial(), 4);
        assert_eq!(function_b.serial(), 9);
        for (call_symbol, expected_target) in [(5, function_b), (6, function_a)] {
            let Node::Operation {
                operation:
                    OperationKind::Call {
                        function,
                        arguments,
                    },
                ..
            } = workspace
                .head()
                .expect("head")
                .node(binding(&receipt, call_symbol))
                .expect("call")
            else {
                panic!("call operation")
            };
            assert_eq!(*function, expected_target);
            assert!(arguments.is_empty());
        }
    }

    #[test]
    fn hole_refinement_can_use_supporting_values_created_before_it_atomically() {
        let id = WorkspaceId::from_bytes([0x75; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
        let forty = binding(&created, 6);
        let hole = binding(&created, 9);
        let block = prepared_operation_owner(workspace.head().expect("head"), hole);
        let support = DraftSymbol::generated(100);
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::InsertExpression {
                    block,
                    before: Some(hole),
                    expression: ExpressionDraft {
                        symbol: Some(support),
                        operation: ExpressionKindDraft::ConstI64(2),
                    },
                },
                TransactionOp::RefineHole {
                    hole: NodeTarget::Existing(hole),
                    replacement: OperationDraft::AddI64 {
                        lhs: ValueDraft::OperationResult {
                            operation: NodeTarget::Existing(forty),
                            output: 0,
                        },
                        rhs: ValueDraft::OperationResult {
                            operation: NodeTarget::Draft(support),
                            output: 0,
                        },
                    },
                },
            ],
        };
        let prepared = workspace
            .prepare_transaction(&ApplyTransactionRequest {
                transaction,
                response: TransactionResponseSpec {
                    return_symbols: vec![support],
                },
            })
            .expect("atomic support and refinement");
        assert_eq!(prepared.receipt.created_count, 1);
        assert!(prepared.receipt.complete_after);
        let support_id = binding(&prepared.receipt, 100);
        let Node::Block { operations, .. } = prepared.snapshot.node(block).expect("block") else {
            panic!("block kind");
        };
        let support_position = operations
            .iter()
            .position(|id| *id == support_id)
            .expect("support position");
        let hole_position = operations
            .iter()
            .position(|id| *id == hole)
            .expect("hole position");
        assert!(support_position < hole_position);
    }

    #[test]
    fn hole_refinement_rejects_wrong_targets_contracts_types_and_order() {
        let id = WorkspaceId::from_bytes([0x73; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
        let package = binding(&created, 1);
        let forty = binding(&created, 6);
        let boolean = binding(&created, 8);
        let hole = binding(&created, 9);
        let later = binding(&created, 10);
        let value = |operation| ValueDraft::OperationResult {
            operation: NodeTarget::Existing(operation),
            output: 0,
        };
        let cases = [
            (package, OperationDraft::ConstI64(1), ErrorCode::WrongKind),
            (
                forty,
                OperationDraft::ConstI64(1),
                ErrorCode::InvalidOperand,
            ),
            (
                hole,
                OperationDraft::Hole {
                    expected: SemanticType::I64.into(),
                },
                ErrorCode::InvalidOperand,
            ),
            (
                hole,
                OperationDraft::Return {
                    value: value(forty),
                },
                ErrorCode::InvalidOperand,
            ),
            (
                hole,
                OperationDraft::ConstBool(false),
                ErrorCode::TypeMismatch,
            ),
            (
                hole,
                OperationDraft::AddI64 {
                    lhs: value(forty),
                    rhs: value(boolean),
                },
                ErrorCode::TypeMismatch,
            ),
            (
                hole,
                OperationDraft::AddI64 {
                    lhs: value(forty),
                    rhs: value(later),
                },
                ErrorCode::InvalidOperand,
            ),
            (
                hole,
                OperationDraft::AddI64 {
                    lhs: value(forty),
                    rhs: value(hole),
                },
                ErrorCode::InvalidOperand,
            ),
        ];
        for (target, replacement, expected) in cases {
            let refine = Transaction {
                workspace: id,
                base_revision: Revision::new(1),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![TransactionOp::RefineHole {
                    hole: NodeTarget::Existing(target),
                    replacement,
                }],
            };
            assert_eq!(
                workspace
                    .prepare_transaction(&request(&refine))
                    .expect_err("invalid refinement")
                    .code,
                expected
            );
            assert_eq!(workspace.head_revision(), Revision::new(1));
            assert!(matches!(
                workspace.head().expect("head").node(hole).expect("hole"),
                Node::Operation {
                    operation: OperationKind::Hole { .. },
                    ..
                }
            ));
        }
    }

    #[test]
    fn nominal_declarations_resolve_forward_types_and_derive_exact_layouts() {
        let id = WorkspaceId::from_bytes([0x91; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let request = ApplyTransactionRequest {
            transaction: Transaction {
                workspace: id,
                base_revision: Revision::INITIAL,
                idempotency_key: None,
                mode: TransactionMode::ValidateOnly,
                operations: vec![
                    TransactionOp::CreatePackage {
                        symbol: DraftSymbol::generated(1),
                        name: "p".into(),
                    },
                    TransactionOp::CreateModule {
                        symbol: DraftSymbol::generated(2),
                        package: draft_symbol(1),
                        name: "m".into(),
                    },
                    TransactionOp::CreateProductType {
                        symbol: DraftSymbol::generated(3),
                        module: draft_symbol(2),
                        name: "Reading".into(),
                        fields: vec![
                            ProductFieldDraft {
                                symbol: DraftSymbol::generated(4),
                                name: "valid".into(),
                                ty: TypeDraft::Bool,
                            },
                            ProductFieldDraft {
                                symbol: DraftSymbol::generated(5),
                                name: "value".into(),
                                ty: TypeDraft::I64,
                            },
                        ],
                    },
                    TransactionOp::CreateSumType {
                        symbol: DraftSymbol::generated(6),
                        module: draft_symbol(2),
                        name: "Input".into(),
                        variants: vec![
                            SumVariantDraft {
                                symbol: DraftSymbol::generated(7),
                                name: "missing".into(),
                                payload: None,
                            },
                            SumVariantDraft {
                                symbol: DraftSymbol::generated(8),
                                name: "sample".into(),
                                payload: Some(TypeDraft::Nominal(draft_symbol(3))),
                            },
                        ],
                    },
                    TransactionOp::CreateFunction {
                        symbol: DraftSymbol::generated(9),
                        module: draft_symbol(2),
                        name: "pending".into(),
                        parameters: vec![FunctionParameterDraft {
                            symbol: DraftSymbol::generated(10),
                            name: "input".into(),
                            ty: TypeDraft::Nominal(draft_symbol(6)),
                        }],
                        result: TypeDraft::Nominal(draft_symbol(3)),
                        body: Some(FunctionBodyDraft {
                            operations: vec![draft_expression(
                                11,
                                ExpressionKindDraft::Hole {
                                    expected: TypeDraft::Nominal(draft_symbol(3)),
                                },
                            )],
                            return_value: draft_result(11),
                        }),
                    },
                ],
            },
            response: TransactionResponseSpec {
                return_symbols: vec![
                    DraftSymbol::generated(3),
                    DraftSymbol::generated(4),
                    DraftSymbol::generated(5),
                    DraftSymbol::generated(6),
                    DraftSymbol::generated(7),
                    DraftSymbol::generated(8),
                    DraftSymbol::generated(9),
                    DraftSymbol::generated(11),
                ],
            },
        };
        let prepared = workspace
            .prepare_transaction(&request)
            .expect("nominal validate-only");
        assert!(!prepared.receipt.published);
        assert_eq!(workspace.head().expect("head").next_serial(), 2);
        let reading = prepared.receipt.returned_bindings[0].1;
        let input = prepared.receipt.returned_bindings[3].1;
        let Node::Module {
            types, functions, ..
        } = prepared
            .snapshot
            .node(NodeId::new(id, 3).expect("module"))
            .expect("module")
        else {
            panic!("module kind")
        };
        assert_eq!(types, &[reading, input]);
        assert_eq!(functions.len(), 1);
        let layouts = crate::type_layout::derive_layouts(&prepared.snapshot).expect("layouts");
        let crate::type_layout::DerivedLayout::Representable(reading_layout) =
            layouts.get(&reading).expect("reading layout")
        else {
            panic!("representable")
        };
        assert_eq!(
            (
                reading_layout.size,
                reading_layout.align,
                reading_layout.cells
            ),
            (16, 8, 2)
        );
        let crate::type_layout::LayoutShape::Product { fields } = &reading_layout.shape else {
            panic!("product layout")
        };
        assert_eq!(
            fields.iter().map(|field| field.offset).collect::<Vec<_>>(),
            [0, 8]
        );
        let crate::type_layout::DerivedLayout::Representable(input_layout) =
            layouts.get(&input).expect("input layout")
        else {
            panic!("representable")
        };
        assert_eq!(input_layout.cells, 3);
    }

    #[test]
    fn by_value_cycles_and_duplicate_member_names_reject_without_identity_consumption() {
        let id = WorkspaceId::from_bytes([0x92; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let cyclic = structured_semantic_request(
            id,
            vec![
                TransactionOp::CreateProductType {
                    symbol: DraftSymbol::generated(3),
                    module: draft_symbol(2),
                    name: "A".into(),
                    fields: vec![ProductFieldDraft {
                        symbol: DraftSymbol::generated(4),
                        name: "b".into(),
                        ty: TypeDraft::Nominal(draft_symbol(5)),
                    }],
                },
                TransactionOp::CreateProductType {
                    symbol: DraftSymbol::generated(5),
                    module: draft_symbol(2),
                    name: "B".into(),
                    fields: vec![ProductFieldDraft {
                        symbol: DraftSymbol::generated(6),
                        name: "a".into(),
                        ty: TypeDraft::Nominal(draft_symbol(3)),
                    }],
                },
            ],
        );
        let error = workspace.prepare_transaction(&cyclic).expect_err("cycle");
        assert_eq!(error.code, ErrorCode::ByValueTypeCycle);
        assert_eq!(workspace.head().expect("head").next_serial(), 2);

        let duplicate = structured_semantic_request(
            id,
            vec![TransactionOp::CreateProductType {
                symbol: DraftSymbol::generated(3),
                module: draft_symbol(2),
                name: "D".into(),
                fields: vec![
                    ProductFieldDraft {
                        symbol: DraftSymbol::generated(4),
                        name: "same".into(),
                        ty: TypeDraft::I64,
                    },
                    ProductFieldDraft {
                        symbol: DraftSymbol::generated(5),
                        name: "same".into(),
                        ty: TypeDraft::Bool,
                    },
                ],
            }],
        );
        assert_eq!(
            workspace
                .prepare_transaction(&duplicate)
                .expect_err("duplicate")
                .code,
            ErrorCode::DuplicateName
        );
        assert_eq!(workspace.head().expect("head").next_serial(), 2);
    }

    #[test]
    fn nominal_operations_normalize_fields_and_match_arms_and_validate_payload_scope() {
        let id = WorkspaceId::from_bytes([0x94; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let operations = vec![
            TransactionOp::CreateProductType {
                symbol: DraftSymbol::generated(3),
                module: draft_symbol(2),
                name: "Pair".into(),
                fields: vec![
                    ProductFieldDraft {
                        symbol: DraftSymbol::generated(4),
                        name: "left".into(),
                        ty: TypeDraft::I64,
                    },
                    ProductFieldDraft {
                        symbol: DraftSymbol::generated(5),
                        name: "right".into(),
                        ty: TypeDraft::I64,
                    },
                ],
            },
            TransactionOp::CreateSumType {
                symbol: DraftSymbol::generated(6),
                module: draft_symbol(2),
                name: "Maybe".into(),
                variants: vec![
                    SumVariantDraft {
                        symbol: DraftSymbol::generated(7),
                        name: "none".into(),
                        payload: None,
                    },
                    SumVariantDraft {
                        symbol: DraftSymbol::generated(8),
                        name: "some".into(),
                        payload: Some(TypeDraft::I64),
                    },
                ],
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(9),
                module: draft_symbol(2),
                name: "main".into(),
                parameters: Vec::new(),
                result: TypeDraft::I64,
                body: Some(FunctionBodyDraft {
                    operations: vec![
                        draft_expression(20, ExpressionKindDraft::ConstI64(10)),
                        draft_expression(21, ExpressionKindDraft::ConstI64(20)),
                        draft_expression(
                            22,
                            ExpressionKindDraft::ConstructProduct {
                                product: draft_symbol(3),
                                fields: vec![
                                    ProductFieldValueDraft {
                                        field: draft_symbol(5),
                                        value: draft_result(21),
                                    },
                                    ProductFieldValueDraft {
                                        field: draft_symbol(4),
                                        value: draft_result(20),
                                    },
                                ],
                            },
                        ),
                        draft_expression(
                            23,
                            ExpressionKindDraft::ProjectField {
                                value: draft_result(22),
                                field: draft_symbol(4),
                            },
                        ),
                        draft_expression(
                            24,
                            ExpressionKindDraft::ConstructVariant {
                                variant: draft_symbol(8),
                                payload: Some(draft_result(23)),
                            },
                        ),
                        draft_expression(
                            25,
                            ExpressionKindDraft::MatchSum {
                                scrutinee: draft_result(24),
                                result: TypeDraft::I64,
                                arms: vec![
                                    MatchArmDraft {
                                        variant: draft_symbol(8),
                                        payload_symbol: Some(DraftSymbol::generated(30)),
                                        body: YieldingBodyDraft {
                                            operations: Vec::new(),
                                            yield_value: ValueDraft::BlockArgument(draft_symbol(
                                                30,
                                            )),
                                        },
                                    },
                                    MatchArmDraft {
                                        variant: draft_symbol(7),
                                        payload_symbol: None,
                                        body: YieldingBodyDraft {
                                            operations: vec![draft_expression(
                                                31,
                                                ExpressionKindDraft::ConstI64(0),
                                            )],
                                            yield_value: draft_result(31),
                                        },
                                    },
                                ],
                            },
                        ),
                    ],
                    return_value: draft_result(25),
                }),
            },
        ];
        let request = structured_semantic_request(id, operations);
        let mutate_expression =
            |request: &mut ApplyTransactionRequest,
             symbol: u32,
             mutate: &mut dyn FnMut(&mut ExpressionKindDraft)| {
                let TransactionOp::CreateFunction {
                    body: Some(body), ..
                } = request.transaction.operations.last_mut().expect("function")
                else {
                    panic!("function")
                };
                let expression = body
                    .operations
                    .iter_mut()
                    .find(|expression| expression.symbol == Some(DraftSymbol::generated(symbol)))
                    .expect("expression");
                mutate(&mut expression.operation);
            };
        let mut invalid = request.clone();
        mutate_expression(&mut invalid, 22, &mut |operation| {
            let ExpressionKindDraft::ConstructProduct { fields, .. } = operation else {
                panic!("product")
            };
            fields.pop();
        });
        assert_eq!(
            workspace
                .prepare_transaction(&invalid)
                .expect_err("missing field")
                .code,
            ErrorCode::InvalidOperand
        );
        let mut invalid = request.clone();
        mutate_expression(&mut invalid, 22, &mut |operation| {
            let ExpressionKindDraft::ConstructProduct { fields, .. } = operation else {
                panic!("product")
            };
            fields.push(fields[0].clone());
        });
        assert_eq!(
            workspace
                .prepare_transaction(&invalid)
                .expect_err("duplicate field")
                .code,
            ErrorCode::InvalidOperand
        );
        let mut invalid = request.clone();
        mutate_expression(&mut invalid, 20, &mut |operation| {
            *operation = ExpressionKindDraft::ConstBool(true)
        });
        assert_eq!(
            workspace
                .prepare_transaction(&invalid)
                .expect_err("wrong field type")
                .code,
            ErrorCode::TypeMismatch
        );
        let mut invalid = request.clone();
        mutate_expression(&mut invalid, 25, &mut |operation| {
            let ExpressionKindDraft::MatchSum { arms, .. } = operation else {
                panic!("match")
            };
            arms.pop();
        });
        assert_eq!(
            workspace
                .prepare_transaction(&invalid)
                .expect_err("missing arm")
                .code,
            ErrorCode::InvalidOperand
        );
        let mut invalid = request.clone();
        mutate_expression(&mut invalid, 25, &mut |operation| {
            let ExpressionKindDraft::MatchSum { arms, .. } = operation else {
                panic!("match")
            };
            arms[0].payload_symbol = None;
        });
        assert_eq!(
            workspace
                .prepare_transaction(&invalid)
                .expect_err("missing payload binding")
                .code,
            ErrorCode::InvalidDraftSymbol
        );
        assert_eq!(workspace.head().expect("head").next_serial(), 2);
        let prepared = workspace
            .prepare_transaction(&request)
            .expect("nominal operations");
        let product = prepared
            .snapshot
            .nodes()
            .find_map(|(operation_id, node)| match node {
                Node::Operation {
                    operation: OperationKind::ConstructProduct { product, fields },
                    ..
                } => Some((operation_id, *product, fields.clone())),
                _ => None,
            })
            .expect("product operation");
        let Node::ProductType {
            fields: declared, ..
        } = prepared.snapshot.node(product.1).expect("product")
        else {
            unreachable!()
        };
        assert_eq!(
            product
                .2
                .iter()
                .map(|binding| binding.field)
                .collect::<Vec<_>>(),
            *declared
        );
        let second_field_context = crate::query::execute(
            &prepared.snapshot,
            &crate::query::Query::RepairContext {
                target: crate::query::RepairTarget::Operand {
                    operation: product.0,
                    index: 1,
                },
                budget: crate::query::ContextBudget {
                    body_before: 0,
                    body_after: 0,
                    visible_values: 1,
                    incoming_uses: 1,
                    include_incompatible: false,
                },
            },
            None,
        )
        .expect("second product field context");
        let crate::query::QueryResult::RepairContext(second_field_context) = second_field_context
        else {
            panic!("repair context")
        };
        assert_eq!(
            second_field_context.use_mode,
            Some(crate::schema::OperandUse::Copy)
        );

        let arms = prepared
            .snapshot
            .nodes()
            .find_map(|(_, node)| match node {
                Node::Operation {
                    operation: OperationKind::MatchSum { arms, .. },
                    ..
                } => Some(arms.clone()),
                _ => None,
            })
            .expect("match operation");
        let sum = match prepared.snapshot.node(arms[0].variant).expect("variant") {
            Node::SumVariant { owner, .. } => *owner,
            _ => unreachable!(),
        };
        let Node::SumType { variants, .. } = prepared.snapshot.node(sum).expect("sum") else {
            unreachable!()
        };
        assert_eq!(
            arms.iter().map(|arm| arm.variant).collect::<Vec<_>>(),
            *variants
        );
        workspace.publish(prepared.snapshot).expect("publish");
    }

    #[test]
    fn nominal_hole_refinement_is_atomic_and_preserves_identity() {
        let id = WorkspaceId::from_bytes([0x95; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let request = structured_semantic_request(
            id,
            vec![
                TransactionOp::CreateProductType {
                    symbol: DraftSymbol::generated(3),
                    module: draft_symbol(2),
                    name: "Pair".into(),
                    fields: vec![
                        ProductFieldDraft {
                            symbol: DraftSymbol::generated(4),
                            name: "left".into(),
                            ty: TypeDraft::I64,
                        },
                        ProductFieldDraft {
                            symbol: DraftSymbol::generated(5),
                            name: "right".into(),
                            ty: TypeDraft::I64,
                        },
                    ],
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(6),
                    module: draft_symbol(2),
                    name: "make".into(),
                    parameters: Vec::new(),
                    result: TypeDraft::Nominal(draft_symbol(3)),
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            draft_expression(20, ExpressionKindDraft::ConstI64(1)),
                            draft_expression(21, ExpressionKindDraft::ConstI64(2)),
                            draft_expression(
                                22,
                                ExpressionKindDraft::Hole {
                                    expected: TypeDraft::Nominal(draft_symbol(3)),
                                },
                            ),
                        ],
                        return_value: draft_result(22),
                    }),
                },
            ],
        );
        let prepared = workspace
            .prepare_transaction(&request)
            .expect("incomplete product function");
        let prior = prepared.snapshot.clone();
        workspace
            .publish(prepared.snapshot)
            .expect("publish incomplete");
        let product = prior
            .nodes()
            .find_map(|(id, node)| matches!(node, Node::ProductType { .. }).then_some(id))
            .expect("product");
        let fields = match prior.node(product).expect("product") {
            Node::ProductType { fields, .. } => fields.clone(),
            _ => unreachable!(),
        };
        let hole = prior
            .nodes()
            .find_map(|(id, node)| {
                matches!(
                    node,
                    Node::Operation {
                        operation: OperationKind::Hole { .. },
                        ..
                    }
                )
                .then_some(id)
            })
            .expect("hole");
        let values = prior
            .nodes()
            .filter_map(|(id, node)| {
                matches!(
                    node,
                    Node::Operation {
                        operation: OperationKind::ConstI64(_),
                        ..
                    }
                )
                .then_some(id)
            })
            .collect::<Vec<_>>();
        let field_value = |field: NodeId, value: NodeId| ProductFieldValueDraft {
            field: NodeTarget::Existing(field),
            value: ValueDraft::OperationResult {
                operation: NodeTarget::Existing(value),
                output: 0,
            },
        };
        let first_page = crate::query::execute(
            &prior,
            &crate::query::Query::NominalType {
                declaration: product,
                page: crate::query::PageRequest {
                    after: None,
                    limit: 1,
                },
            },
            None,
        )
        .expect("nominal page");
        let crate::query::QueryResult::NominalType(first_page) = first_page else {
            panic!("nominal page")
        };
        assert_eq!(first_page.members.items.len(), 1);
        assert_eq!(first_page.members.total, Some(2));
        assert!(first_page.layout.representable);
        let cursor = first_page.members.next.expect("nominal continuation");
        let second_page = crate::query::execute(
            &prior,
            &crate::query::Query::NominalType {
                declaration: product,
                page: crate::query::PageRequest {
                    after: Some(cursor),
                    limit: 1,
                },
            },
            None,
        )
        .expect("nominal continuation");
        let crate::query::QueryResult::NominalType(second_page) = second_page else {
            panic!("nominal page")
        };
        assert_eq!(second_page.members.items.len(), 1);
        assert!(second_page.members.next.is_none());
        let context = crate::query::execute(
            &prior,
            &crate::query::Query::RepairContext {
                target: crate::query::RepairTarget::Hole(hole),
                budget: crate::query::ContextBudget {
                    body_before: 1,
                    body_after: 1,
                    visible_values: 8,
                    incoming_uses: 8,
                    include_incompatible: false,
                },
            },
            None,
        )
        .expect("nominal repair context");
        let crate::query::QueryResult::RepairContext(context) = context else {
            panic!("repair context")
        };
        assert_eq!(
            context
                .nominal_type
                .as_ref()
                .and_then(|nominal| nominal.members.total),
            Some(2)
        );
        assert!(
            context
                .legal_constructors
                .iter()
                .any(|constructor| constructor.code
                    == crate::schema::OperationCode::ConstructProduct
                    && constructor.members == fields)
        );
        let invalid = ApplyTransactionRequest {
            transaction: Transaction {
                workspace: id,
                base_revision: Revision::new(1),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![TransactionOp::RefineHole {
                    hole: NodeTarget::Existing(hole),
                    replacement: OperationDraft::ConstructProduct {
                        product: NodeTarget::Existing(product),
                        fields: vec![field_value(fields[0], values[0])],
                    },
                }],
            },
            response: TransactionResponseSpec::default(),
        };
        assert_eq!(
            workspace
                .prepare_transaction(&invalid)
                .expect_err("missing field")
                .code,
            ErrorCode::InvalidOperand
        );
        assert_eq!(workspace.head_revision(), Revision::new(1));
        let valid = ApplyTransactionRequest {
            transaction: Transaction {
                workspace: id,
                base_revision: Revision::new(1),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![TransactionOp::RefineHole {
                    hole: NodeTarget::Existing(hole),
                    replacement: OperationDraft::ConstructProduct {
                        product: NodeTarget::Existing(product),
                        fields: vec![
                            field_value(fields[1], values[1]),
                            field_value(fields[0], values[0]),
                        ],
                    },
                }],
            },
            response: TransactionResponseSpec::default(),
        };
        let mut validate_only = valid.clone();
        validate_only.transaction.mode = TransactionMode::ValidateOnly;
        let predicted = workspace
            .prepare_transaction(&validate_only)
            .expect("validate-only product refinement");
        assert!(!predicted.receipt.published);
        assert_eq!(workspace.head_revision(), Revision::new(1));
        let prepared = workspace
            .prepare_transaction(&valid)
            .expect("valid product refinement");
        assert!(matches!(
            prepared.snapshot.node(hole),
            Ok(Node::Operation {
                operation: OperationKind::ConstructProduct { .. },
                ..
            })
        ));
        let changes = crate::diff::between(&prior, &prepared.snapshot);
        assert!(changes.changes.iter().any(|change| change.node == hole
            && matches!(
                change.kind,
                crate::diff::ChangeKind::OperationRefined {
                    after: crate::schema::OperationCode::ConstructProduct,
                    ..
                }
            )));
        workspace
            .publish(prepared.snapshot)
            .expect("publish refinement");
    }

    #[test]
    fn nominal_type_references_block_declaration_deletion() {
        let id = WorkspaceId::from_bytes([0x93; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let mut transaction = structured_semantic_request(
            id,
            vec![
                TransactionOp::CreateProductType {
                    symbol: DraftSymbol::generated(3),
                    module: draft_symbol(2),
                    name: "Reading".into(),
                    fields: vec![ProductFieldDraft {
                        symbol: DraftSymbol::generated(4),
                        name: "value".into(),
                        ty: TypeDraft::I64,
                    }],
                },
                TransactionOp::CreateSumType {
                    symbol: DraftSymbol::generated(5),
                    module: draft_symbol(2),
                    name: "Input".into(),
                    variants: vec![SumVariantDraft {
                        symbol: DraftSymbol::generated(6),
                        name: "sample".into(),
                        payload: Some(TypeDraft::Nominal(draft_symbol(3))),
                    }],
                },
            ],
        );
        transaction.response.return_symbols = vec![DraftSymbol::generated(3)];
        let prepared = workspace
            .prepare_transaction(&transaction)
            .expect("declarations");
        let reading = prepared
            .receipt
            .returned_bindings
            .iter()
            .find(|(symbol, _)| *symbol == DraftSymbol::generated(3))
            .expect("reading binding")
            .1;
        workspace.publish(prepared.snapshot).expect("publish");
        let delete = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::DeleteOwnedSubtree {
                root: NodeTarget::Existing(reading),
            }],
        };
        assert_eq!(
            workspace
                .prepare_transaction(&request(&delete))
                .expect_err("referenced declaration")
                .code,
            ErrorCode::DeleteBlocked
        );
        assert_eq!(workspace.head_revision(), Revision::new(1));
    }

    #[test]
    fn stale_revisions_wrong_workspaces_and_no_changes_reject_deterministically() {
        let id = WorkspaceId::from_bytes([13; 16]);
        let other = WorkspaceId::from_bytes([14; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let first = commit(&mut workspace, &create_package_and_module(id)).expect("first commit");
        let package = first.returned_bindings[0].1;

        let stale = create_package_and_module(id);
        assert_eq!(
            workspace
                .prepare_transaction(&request(&stale))
                .expect_err("stale")
                .code,
            ErrorCode::RevisionConflict
        );
        let wrong = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RenameNode {
                node: NodeTarget::Existing(NodeId::new(other, package.serial()).expect("node")),
                name: "renamed".to_owned(),
            }],
        };
        assert_eq!(
            workspace
                .prepare_transaction(&request(&wrong))
                .expect_err("wrong workspace")
                .code,
            ErrorCode::WrongWorkspace
        );
        let no_change = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RenameNode {
                node: NodeTarget::Existing(package),
                name: "package".to_owned(),
            }],
        };
        assert_eq!(
            workspace
                .prepare_transaction(&request(&no_change))
                .expect_err("no change")
                .code,
            ErrorCode::NoChange
        );
    }

    #[test]
    fn preallocation_scan_covers_top_level_types_values_and_maintenance_targets() {
        let local = |value| NodeTarget::Draft(DraftSymbol::generated(value));
        let cases = vec![
            vec![TransactionOp::CreateModule {
                symbol: DraftSymbol::generated(1),
                package: local(99),
                name: "m".into(),
            }],
            vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "p".into(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(2),
                    module: local(99),
                    name: "f".into(),
                    parameters: Vec::new(),
                    result: TypeDraft::Nominal(local(98)),
                    body: None,
                },
            ],
            vec![TransactionOp::ReplaceOperand {
                operation: local(97),
                index: 0,
                value: ValueDraft::OperationResult {
                    operation: local(96),
                    output: 0,
                },
            }],
            vec![TransactionOp::RenameNode {
                node: local(95),
                name: "renamed".into(),
            }],
        ];
        for operations in cases {
            assert_eq!(
                validate_structured_request(&operations)
                    .expect_err("undeclared scan path")
                    .code,
                ErrorCode::InvalidDraftSymbol
            );
        }

        let wrong_kind = vec![
            TransactionOp::CreatePackage {
                symbol: DraftSymbol::generated(1),
                name: "p".into(),
            },
            TransactionOp::CreateModule {
                symbol: DraftSymbol::generated(2),
                package: local(1),
                name: "m".into(),
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: local(2),
                name: "f".into(),
                parameters: Vec::new(),
                result: TypeDraft::I64,
                body: Some(FunctionBodyDraft {
                    operations: vec![draft_expression(
                        4,
                        ExpressionKindDraft::Call {
                            function: local(1),
                            arguments: Vec::new(),
                        },
                    )],
                    return_value: draft_result(4),
                }),
            },
        ];
        let error = validate_structured_request(&wrong_kind).expect_err("wrong local category");
        assert_eq!(error.code, ErrorCode::WrongKind);
        assert_eq!(error.draft_symbol, Some(DraftSymbol::generated(1)));
    }

    #[test]
    fn preallocation_scan_rejects_non_region_if_and_for_targets() {
        let prefix = || {
            vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "p".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: draft_symbol(1),
                    name: "m".into(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(3),
                    module: draft_symbol(2),
                    name: "f".into(),
                    parameters: Vec::new(),
                    result: TypeDraft::I64,
                    body: Some(FunctionBodyDraft {
                        operations: vec![draft_expression(4, ExpressionKindDraft::ConstI64(0))],
                        return_value: draft_result(4),
                    }),
                },
            ]
        };
        let value = draft_result(4);
        let mut if_target = prefix();
        if_target.push(TransactionOp::ReplaceOperation {
            operation: draft_symbol(4),
            replacement: OperationDraft::If {
                condition: value.clone(),
                result: TypeDraft::I64,
                then_region: draft_symbol(3),
                else_region: draft_symbol(3),
            },
        });
        let mut for_target = prefix();
        for_target.push(TransactionOp::ReplaceOperation {
            operation: draft_symbol(4),
            replacement: OperationDraft::ForI64 {
                start: value.clone(),
                end_exclusive: value.clone(),
                step: 1,
                initial: value.clone(),
                carried: TypeDraft::I64,
                body_region: draft_symbol(3),
            },
        });

        for operations in [if_target, for_target] {
            let error = scan_explicit_symbols(&operations)
                .expect_err("non-region target must reject during the preallocation scan");
            assert_eq!(error.code, ErrorCode::WrongKind);
            assert_eq!(error.operation_index, Some(3));
            assert_eq!(error.draft_symbol, Some(DraftSymbol::generated(3)));
        }
    }

    #[test]
    fn later_nominal_declarations_and_permuted_match_arms_expand_identically() {
        let id = WorkspaceId::from_bytes([0xa4; 16]);
        let make = |permuted: bool| {
            let none = MatchArmDraft {
                variant: draft_symbol(11),
                payload_symbol: None,
                body: YieldingBodyDraft {
                    operations: vec![draft_expression(30, ExpressionKindDraft::ConstI64(0))],
                    yield_value: draft_result(30),
                },
            };
            let some = MatchArmDraft {
                variant: draft_symbol(12),
                payload_symbol: Some(DraftSymbol::generated(31)),
                body: YieldingBodyDraft {
                    operations: Vec::new(),
                    yield_value: ValueDraft::BlockArgument(draft_symbol(31)),
                },
            };
            let arms = if permuted {
                vec![some.clone(), none.clone()]
            } else {
                vec![none, some]
            };
            let fields = if permuted {
                vec![
                    ProductFieldValueDraft {
                        field: draft_symbol(6),
                        value: draft_result(20),
                    },
                    ProductFieldValueDraft {
                        field: draft_symbol(5),
                        value: draft_result(20),
                    },
                ]
            } else {
                vec![
                    ProductFieldValueDraft {
                        field: draft_symbol(5),
                        value: draft_result(20),
                    },
                    ProductFieldValueDraft {
                        field: draft_symbol(6),
                        value: draft_result(20),
                    },
                ]
            };
            ApplyTransactionRequest {
                transaction: Transaction {
                    workspace: id,
                    base_revision: Revision::INITIAL,
                    idempotency_key: None,
                    mode: TransactionMode::ValidateOnly,
                    operations: vec![
                        TransactionOp::CreatePackage {
                            symbol: DraftSymbol::generated(1),
                            name: "p".into(),
                        },
                        TransactionOp::CreateModule {
                            symbol: DraftSymbol::generated(2),
                            package: draft_symbol(1),
                            name: "m".into(),
                        },
                        TransactionOp::CreateFunction {
                            symbol: DraftSymbol::generated(3),
                            module: draft_symbol(2),
                            name: "forward".into(),
                            parameters: Vec::new(),
                            result: TypeDraft::I64,
                            body: Some(FunctionBodyDraft {
                                operations: vec![
                                    draft_expression(20, ExpressionKindDraft::ConstI64(7)),
                                    draft_expression(
                                        21,
                                        ExpressionKindDraft::ConstructProduct {
                                            product: draft_symbol(4),
                                            fields,
                                        },
                                    ),
                                    draft_expression(
                                        22,
                                        ExpressionKindDraft::ProjectField {
                                            value: draft_result(21),
                                            field: draft_symbol(5),
                                        },
                                    ),
                                    draft_expression(
                                        23,
                                        ExpressionKindDraft::ConstructVariant {
                                            variant: draft_symbol(12),
                                            payload: Some(draft_result(22)),
                                        },
                                    ),
                                    draft_expression(
                                        24,
                                        ExpressionKindDraft::MatchSum {
                                            scrutinee: draft_result(23),
                                            result: TypeDraft::I64,
                                            arms,
                                        },
                                    ),
                                ],
                                return_value: draft_result(24),
                            }),
                        },
                        TransactionOp::CreateProductType {
                            symbol: DraftSymbol::generated(4),
                            module: draft_symbol(2),
                            name: "Pair".into(),
                            fields: vec![
                                ProductFieldDraft {
                                    symbol: DraftSymbol::generated(5),
                                    name: "left".into(),
                                    ty: TypeDraft::I64,
                                },
                                ProductFieldDraft {
                                    symbol: DraftSymbol::generated(6),
                                    name: "right".into(),
                                    ty: TypeDraft::I64,
                                },
                            ],
                        },
                        TransactionOp::CreateSumType {
                            symbol: DraftSymbol::generated(10),
                            module: draft_symbol(2),
                            name: "Maybe".into(),
                            variants: vec![
                                SumVariantDraft {
                                    symbol: DraftSymbol::generated(11),
                                    name: "none".into(),
                                    payload: None,
                                },
                                SumVariantDraft {
                                    symbol: DraftSymbol::generated(12),
                                    name: "some".into(),
                                    payload: Some(TypeDraft::I64),
                                },
                            ],
                        },
                    ],
                },
                response: TransactionResponseSpec::default(),
            }
        };
        let workspace = Workspace::new(id).expect("workspace");
        let canonical = workspace
            .prepare_transaction(&make(false))
            .expect("canonical arms");
        let permuted = workspace
            .prepare_transaction(&make(true))
            .expect("permuted arms");
        assert_eq!(canonical.snapshot.hash(), permuted.snapshot.hash());
        assert_eq!(canonical.snapshot.nodes, permuted.snapshot.nodes);
    }

    #[test]
    fn draft_symbol_spelling_does_not_change_allocation_graph_or_execution() {
        let id = WorkspaceId::from_bytes([0xd1; 16]);
        let request = |names: [&str; 6]| {
            let symbols = names.map(DraftSymbol::new);
            let [package, module, function, one, two, sum] = symbols;
            ApplyTransactionRequest {
                transaction: Transaction {
                    workspace: id,
                    base_revision: Revision::INITIAL,
                    idempotency_key: None,
                    mode: TransactionMode::ValidateOnly,
                    operations: vec![
                        TransactionOp::CreatePackage {
                            symbol: package,
                            name: "package".into(),
                        },
                        TransactionOp::CreateModule {
                            symbol: module,
                            package: NodeTarget::Draft(package),
                            name: "module".into(),
                        },
                        TransactionOp::CreateFunction {
                            symbol: function,
                            module: NodeTarget::Draft(module),
                            name: "main".into(),
                            parameters: Vec::new(),
                            result: TypeDraft::I64,
                            body: Some(FunctionBodyDraft {
                                operations: vec![
                                    ExpressionDraft {
                                        symbol: Some(one),
                                        operation: ExpressionKindDraft::ConstI64(1),
                                    },
                                    ExpressionDraft {
                                        symbol: Some(two),
                                        operation: ExpressionKindDraft::ConstI64(2),
                                    },
                                    ExpressionDraft {
                                        symbol: Some(sum),
                                        operation: ExpressionKindDraft::AddI64 {
                                            lhs: ValueDraft::OperationResult {
                                                operation: NodeTarget::Draft(one),
                                                output: 0,
                                            },
                                            rhs: ValueDraft::OperationResult {
                                                operation: NodeTarget::Draft(two),
                                                output: 0,
                                            },
                                        },
                                    },
                                ],
                                return_value: ValueDraft::OperationResult {
                                    operation: NodeTarget::Draft(sum),
                                    output: 0,
                                },
                            }),
                        },
                        TransactionOp::SetEntryFunction {
                            package: NodeTarget::Draft(package),
                            function: NodeTarget::Draft(function),
                        },
                    ],
                },
                response: TransactionResponseSpec {
                    return_symbols: vec![function],
                },
            }
        };
        let first = Workspace::new(id)
            .expect("first workspace")
            .prepare_transaction(&request(["package", "module", "main", "one", "two", "sum"]))
            .expect("first proposal");
        let renamed = Workspace::new(id)
            .expect("renamed workspace")
            .prepare_transaction(&request(["p", "m", "entry", "a", "b", "answer"]))
            .expect("renamed proposal");
        assert_eq!(first.snapshot.nodes, renamed.snapshot.nodes);
        assert_eq!(first.snapshot.hash(), renamed.snapshot.hash());
        let entry = first.receipt.returned_bindings[0].1;
        assert_eq!(entry, renamed.receipt.returned_bindings[0].1);
        for prepared in [&first, &renamed] {
            let run = crate::interpret::compile_and_run(
                &prepared.snapshot,
                entry,
                &[],
                crate::interpret::RunPolicy {
                    fuel: 100,
                    maximum_frames: 16,
                },
            )
            .expect("canonical execution");
            assert_eq!(run.value, crate::interpret::RuntimeValue::I64(3));
        }
    }

    #[test]
    fn product_second_operand_is_copy_and_oversized_constructor_requirements_are_bounded() {
        let id = WorkspaceId::from_bytes([0xa5; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let fields = (0..65)
            .map(|index| ProductFieldDraft {
                symbol: DraftSymbol::generated(100 + index),
                name: format!("field_{index}"),
                ty: TypeDraft::I64,
            })
            .collect::<Vec<_>>();
        let parameters = (0..65)
            .map(|index| FunctionParameterDraft {
                symbol: DraftSymbol::generated(300 + index),
                name: format!("parameter_{index}"),
                ty: TypeDraft::I64,
            })
            .collect::<Vec<_>>();
        let request = structured_semantic_request(
            id,
            vec![
                TransactionOp::CreateProductType {
                    symbol: DraftSymbol::generated(3),
                    module: draft_symbol(2),
                    name: "Wide".into(),
                    fields,
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(4),
                    module: draft_symbol(2),
                    name: "wide_call".into(),
                    parameters,
                    result: TypeDraft::Nominal(draft_symbol(3)),
                    body: None,
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(5),
                    module: draft_symbol(2),
                    name: "repair".into(),
                    parameters: Vec::new(),
                    result: TypeDraft::Nominal(draft_symbol(3)),
                    body: Some(FunctionBodyDraft {
                        operations: vec![draft_expression(
                            6,
                            ExpressionKindDraft::Hole {
                                expected: TypeDraft::Nominal(draft_symbol(3)),
                            },
                        )],
                        return_value: draft_result(6),
                    }),
                },
            ],
        );
        let prepared = workspace
            .prepare_transaction(&request)
            .expect("wide product");
        let hole = prepared
            .snapshot
            .nodes()
            .find_map(|(id, node)| {
                matches!(
                    node,
                    Node::Operation {
                        operation: OperationKind::Hole { .. },
                        ..
                    }
                )
                .then_some(id)
            })
            .expect("hole");
        let context = crate::query::execute(
            &prepared.snapshot,
            &crate::query::Query::RepairContext {
                target: crate::query::RepairTarget::Hole(hole),
                budget: crate::query::ContextBudget {
                    body_before: 1,
                    body_after: 1,
                    visible_values: 1,
                    incoming_uses: 1,
                    include_incompatible: false,
                },
            },
            None,
        )
        .expect("context");
        let crate::query::QueryResult::RepairContext(context) = context else {
            panic!("context")
        };
        assert!(context.nominal_type.is_none());
        assert!(context.nominal_type_continuation.is_some());
        for constructor in &context.legal_constructors {
            assert!(constructor.operand_types.len() <= crate::query::MAX_CONTEXT_ITEMS as usize);
            assert!(constructor.operand_uses.len() <= crate::query::MAX_CONTEXT_ITEMS as usize);
            assert!(constructor.members.len() <= crate::query::MAX_CONTEXT_ITEMS as usize);
        }
        let product = context
            .legal_constructors
            .iter()
            .find(|constructor| constructor.code == crate::schema::OperationCode::ConstructProduct)
            .expect("product constructor");
        assert_eq!(product.operand_count, 65);
        assert_eq!(product.member_count, 65);
        assert!(!product.requirements_complete);
        assert!(product.nominal_type_continuation.is_some());
        let call = context
            .legal_constructors
            .iter()
            .find(|constructor| constructor.code == crate::schema::OperationCode::Call)
            .expect("call constructor");
        assert_eq!(call.operand_count, 65);
        assert!(!call.requirements_complete);
    }
}
