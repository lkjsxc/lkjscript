use crate::diff;
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::{Snapshot, Workspace, require_kind};
use crate::ids::{
    ChangeDigest, IdempotencyKey, LocalHandle, NodeId, Revision, SnapshotHash, WorkspaceId,
};
use crate::query;
use crate::schema::{
    MatchArm, MatchArmOperationDraft, Node, NodeKind, OperationDraft, OperationKind,
    ProductFieldValue, ProductFieldValueDraft, SemanticType, TypeDraft, ValueDraft, ValueRef,
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
    Local(LocalHandle),
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
    pub return_handles: Vec<LocalHandle>,
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
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::CreatePackage => 1,
            Self::CreateModule => 2,
            Self::CreateFunction => 3,
            Self::DefineFunctionBody => 4,
            Self::InsertExpression => 5,
            Self::SetEntryFunction => 6,
            Self::RenameNode => 7,
            Self::ReplaceOperation => 8,
            Self::ReplaceOperand => 9,
            Self::DeleteOwnedSubtree => 10,
            Self::RefineHole => 11,
            Self::CreateProductType => 12,
            Self::CreateSumType => 13,
        }
    }
    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::CreatePackage),
            2 => Some(Self::CreateModule),
            3 => Some(Self::CreateFunction),
            4 => Some(Self::DefineFunctionBody),
            5 => Some(Self::InsertExpression),
            6 => Some(Self::SetEntryFunction),
            7 => Some(Self::RenameNode),
            8 => Some(Self::ReplaceOperation),
            9 => Some(Self::ReplaceOperand),
            10 => Some(Self::DeleteOwnedSubtree),
            11 => Some(Self::RefineHole),
            12 => Some(Self::CreateProductType),
            13 => Some(Self::CreateSumType),
            _ => None,
        }
    }
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
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::ConstUnit => 1,
            Self::ConstBool => 2,
            Self::ConstI64 => 3,
            Self::AddI64 => 4,
            Self::LtI64 => 5,
            Self::Call => 6,
            Self::Hole => 7,
            Self::If => 8,
            Self::ForI64 => 9,
            Self::ConstructProduct => 10,
            Self::ProjectField => 11,
            Self::ConstructVariant => 12,
            Self::MatchSum => 13,
        }
    }
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueDraftCode {
    FunctionParameter,
    OperationResult,
    BlockArgument,
}
impl ValueDraftCode {
    pub const ALL: [Self; 3] = [
        Self::FunctionParameter,
        Self::OperationResult,
        Self::BlockArgument,
    ];
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::FunctionParameter => 1,
            Self::OperationResult => 2,
            Self::BlockArgument => 3,
        }
    }
    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::FunctionParameter),
            2 => Some(Self::OperationResult),
            3 => Some(Self::BlockArgument),
            _ => None,
        }
    }
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::FunctionParameter => "function_parameter",
            Self::OperationResult => "operation_result",
            Self::BlockArgument => "block_argument",
        }
    }
}

impl ValueDraft {
    pub const fn code(self) -> ValueDraftCode {
        match self {
            Self::FunctionParameter(_) => ValueDraftCode::FunctionParameter,
            Self::OperationResult { .. } => ValueDraftCode::OperationResult,
            Self::BlockArgument(_) => ValueDraftCode::BlockArgument,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionParameterDraft {
    pub handle: LocalHandle,
    pub name: String,
    pub ty: TypeDraft,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProductFieldDraft {
    pub handle: LocalHandle,
    pub name: String,
    pub ty: TypeDraft,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SumVariantDraft {
    pub handle: LocalHandle,
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
    pub payload_handle: Option<LocalHandle>,
    pub body: YieldingBodyDraft,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpressionDraft {
    pub handle: LocalHandle,
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
        index_handle: LocalHandle,
        carried_handle: LocalHandle,
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

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TransactionOp {
    CreatePackage {
        handle: LocalHandle,
        name: String,
    },
    CreateModule {
        handle: LocalHandle,
        package: NodeTarget,
        name: String,
    },
    CreateProductType {
        handle: LocalHandle,
        module: NodeTarget,
        name: String,
        fields: Vec<ProductFieldDraft>,
    },
    CreateSumType {
        handle: LocalHandle,
        module: NodeTarget,
        name: String,
        variants: Vec<SumVariantDraft>,
    },
    CreateFunction {
        handle: LocalHandle,
        module: NodeTarget,
        name: String,
        parameters: Vec<FunctionParameterDraft>,
        result: TypeDraft,
        body: Option<FunctionBodyDraft>,
    },
    DefineFunctionBody {
        function: NodeTarget,
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
    pub const fn created_handle(&self) -> Option<LocalHandle> {
        match self {
            Self::CreatePackage { handle, .. }
            | Self::CreateModule { handle, .. }
            | Self::CreateProductType { handle, .. }
            | Self::CreateSumType { handle, .. }
            | Self::CreateFunction { handle, .. } => Some(*handle),
            Self::InsertExpression { expression, .. } => Some(expression.handle),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
enum CanonicalEdit {
    CreatePackage {
        handle: LocalHandle,
        name: String,
    },
    CreateModule {
        handle: LocalHandle,
        package: NodeTarget,
        name: String,
    },
    CreateProductType {
        handle: LocalHandle,
        module: NodeTarget,
        name: String,
    },
    CreateProductField {
        handle: LocalHandle,
        product: NodeTarget,
        name: String,
        ty: TypeDraft,
    },
    CreateSumType {
        handle: LocalHandle,
        module: NodeTarget,
        name: String,
    },
    CreateSumVariant {
        handle: LocalHandle,
        sum: NodeTarget,
        name: String,
        payload: Option<TypeDraft>,
    },
    CreateFunction {
        handle: LocalHandle,
        module: NodeTarget,
        name: String,
        result: TypeDraft,
    },
    CreateParameter {
        handle: LocalHandle,
        function: NodeTarget,
        name: String,
        ty: TypeDraft,
    },
    CreateRegion {
        handle: LocalHandle,
        owner: NodeTarget,
    },
    CreateBlock {
        handle: LocalHandle,
        region: NodeTarget,
    },
    CreateBlockArgument {
        handle: LocalHandle,
        block: NodeTarget,
        ty: TypeDraft,
    },
    CreateMatchPayloadArgument {
        handle: LocalHandle,
        block: NodeTarget,
        variant: NodeTarget,
    },
    CreateOperation {
        handle: LocalHandle,
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
    pub returned_bindings: Vec<(LocalHandle, NodeId)>,
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
    explicit_handles: BTreeSet<LocalHandle>,
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
                            NodeTarget::Local(_) => None,
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
                TransactionOp::CreateProductType { handle, fields, .. } => {
                    let declaration = NodeTarget::Local(*handle);
                    let members = fields
                        .iter()
                        .map(|field| NodeTarget::Local(field.handle))
                        .collect::<Vec<_>>();
                    for member in &members {
                        catalogue.field_owners.insert(*member, declaration);
                    }
                    catalogue.products.insert(declaration, members);
                }
                TransactionOp::CreateSumType {
                    handle, variants, ..
                } => {
                    let declaration = NodeTarget::Local(*handle);
                    let members = variants
                        .iter()
                        .map(|variant| NodeTarget::Local(variant.handle))
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
            if payload.is_some() != arm.payload_handle.is_some() {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "match payload handle presence does not match the variant payload",
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
}

#[derive(Clone)]
enum ExpandEvent {
    Source(usize),
    Edit(CanonicalEdit),
    FunctionBody {
        function: NodeTarget,
        body: FunctionBodyDraft,
    },
    YieldingBody {
        owner: NodeTarget,
        region: LocalHandle,
        arguments: Vec<(LocalHandle, TypeDraft)>,
        body: YieldingBodyDraft,
    },
    MatchArmBody {
        owner: NodeTarget,
        region: LocalHandle,
        variant: NodeTarget,
        payload_handle: Option<LocalHandle>,
        body: YieldingBodyDraft,
    },
    Expression {
        block: NodeTarget,
        before: Option<NodeTarget>,
        expression: ExpressionDraft,
    },
}

fn expand_transaction(
    base: &Snapshot,
    operations: &[TransactionOp],
) -> Result<ExpandedTransaction> {
    let explicit_handles = scan_explicit_handles(operations)?;
    let nominal_catalogue = StagedNominalCatalogue::build(base, operations);
    let mut synthetic = SyntheticHandles::new(&explicit_handles);
    let mut events = Vec::new();
    for (source, operation) in operations.iter().enumerate().rev() {
        match operation {
            TransactionOp::CreatePackage { handle, name } => {
                events.push(ExpandEvent::Edit(CanonicalEdit::CreatePackage {
                    handle: *handle,
                    name: name.clone(),
                }))
            }
            TransactionOp::CreateModule {
                handle,
                package,
                name,
            } => events.push(ExpandEvent::Edit(CanonicalEdit::CreateModule {
                handle: *handle,
                package: *package,
                name: name.clone(),
            })),
            TransactionOp::CreateProductType {
                handle,
                module,
                name,
                fields,
            } => {
                for field in fields.iter().rev() {
                    events.push(ExpandEvent::Edit(CanonicalEdit::CreateProductField {
                        handle: field.handle,
                        product: NodeTarget::Local(*handle),
                        name: field.name.clone(),
                        ty: field.ty,
                    }));
                }
                events.push(ExpandEvent::Edit(CanonicalEdit::CreateProductType {
                    handle: *handle,
                    module: *module,
                    name: name.clone(),
                }));
            }
            TransactionOp::CreateSumType {
                handle,
                module,
                name,
                variants,
            } => {
                for variant in variants.iter().rev() {
                    events.push(ExpandEvent::Edit(CanonicalEdit::CreateSumVariant {
                        handle: variant.handle,
                        sum: NodeTarget::Local(*handle),
                        name: variant.name.clone(),
                        payload: variant.payload,
                    }));
                }
                events.push(ExpandEvent::Edit(CanonicalEdit::CreateSumType {
                    handle: *handle,
                    module: *module,
                    name: name.clone(),
                }));
            }
            TransactionOp::CreateFunction {
                handle,
                module,
                name,
                parameters,
                result,
                body,
            } => {
                if let Some(body) = body {
                    events.push(ExpandEvent::FunctionBody {
                        function: NodeTarget::Local(*handle),
                        body: body.clone(),
                    });
                }
                for parameter in parameters.iter().rev() {
                    events.push(ExpandEvent::Edit(CanonicalEdit::CreateParameter {
                        handle: parameter.handle,
                        function: NodeTarget::Local(*handle),
                        name: parameter.name.clone(),
                        ty: parameter.ty,
                    }));
                }
                events.push(ExpandEvent::Edit(CanonicalEdit::CreateFunction {
                    handle: *handle,
                    module: *module,
                    name: name.clone(),
                    result: *result,
                }));
            }
            TransactionOp::DefineFunctionBody { function, body } => {
                events.push(ExpandEvent::FunctionBody {
                    function: *function,
                    body: body.clone(),
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
                value: *value,
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
    let mut current_source = 0;
    while let Some(event) = events.pop() {
        match event {
            ExpandEvent::Source(source) => current_source = source,
            ExpandEvent::Edit(edit) => edits.push(edit),
            ExpandEvent::FunctionBody { function, body } => {
                let region = synthetic.next(current_source)?;
                let block = synthetic.next(current_source)?;
                let terminator = synthetic.next(current_source)?;
                edits.push(CanonicalEdit::CreateRegion {
                    handle: region,
                    owner: function,
                });
                edits.push(CanonicalEdit::CreateBlock {
                    handle: block,
                    region: NodeTarget::Local(region),
                });
                events.push(ExpandEvent::Edit(CanonicalEdit::SetFunctionBody {
                    function,
                    region: NodeTarget::Local(region),
                }));
                events.push(ExpandEvent::Edit(CanonicalEdit::CreateOperation {
                    handle: terminator,
                    block: NodeTarget::Local(block),
                    before: None,
                    operation: OperationDraft::Return {
                        value: body.return_value,
                    },
                }));
                for expression in body.operations.into_iter().rev() {
                    events.push(ExpandEvent::Expression {
                        block: NodeTarget::Local(block),
                        before: None,
                        expression,
                    });
                }
            }
            ExpandEvent::YieldingBody {
                owner,
                region,
                arguments,
                body,
            } => {
                let block = synthetic.next(current_source)?;
                edits.push(CanonicalEdit::CreateRegion {
                    handle: region,
                    owner,
                });
                edits.push(CanonicalEdit::CreateBlock {
                    handle: block,
                    region: NodeTarget::Local(region),
                });
                for (handle, ty) in arguments {
                    edits.push(CanonicalEdit::CreateBlockArgument {
                        handle,
                        block: NodeTarget::Local(block),
                        ty,
                    });
                }
                let terminator = synthetic.next(current_source)?;
                events.push(ExpandEvent::Edit(CanonicalEdit::CreateOperation {
                    handle: terminator,
                    block: NodeTarget::Local(block),
                    before: None,
                    operation: OperationDraft::Yield {
                        value: body.yield_value,
                    },
                }));
                for expression in body.operations.into_iter().rev() {
                    events.push(ExpandEvent::Expression {
                        block: NodeTarget::Local(block),
                        before: None,
                        expression,
                    });
                }
            }
            ExpandEvent::MatchArmBody {
                owner,
                region,
                variant,
                payload_handle,
                body,
            } => {
                let block = synthetic.next(current_source)?;
                edits.push(CanonicalEdit::CreateRegion {
                    handle: region,
                    owner,
                });
                edits.push(CanonicalEdit::CreateBlock {
                    handle: block,
                    region: NodeTarget::Local(region),
                });
                if let Some(handle) = payload_handle {
                    edits.push(CanonicalEdit::CreateMatchPayloadArgument {
                        handle,
                        block: NodeTarget::Local(block),
                        variant,
                    });
                }
                let terminator = synthetic.next(current_source)?;
                events.push(ExpandEvent::Edit(CanonicalEdit::CreateOperation {
                    handle: terminator,
                    block: NodeTarget::Local(block),
                    before: None,
                    operation: OperationDraft::Yield {
                        value: body.yield_value,
                    },
                }));
                for expression in body.operations.into_iter().rev() {
                    events.push(ExpandEvent::Expression {
                        block: NodeTarget::Local(block),
                        before: None,
                        expression,
                    });
                }
            }
            ExpandEvent::Expression {
                block,
                before,
                expression,
            } => match expression.operation {
                ExpressionKindDraft::ConstUnit => edits.push(CanonicalEdit::CreateOperation {
                    handle: expression.handle,
                    block,
                    before,
                    operation: OperationDraft::ConstUnit,
                }),
                ExpressionKindDraft::ConstBool(value) => {
                    edits.push(CanonicalEdit::CreateOperation {
                        handle: expression.handle,
                        block,
                        before,
                        operation: OperationDraft::ConstBool(value),
                    })
                }
                ExpressionKindDraft::ConstI64(value) => {
                    edits.push(CanonicalEdit::CreateOperation {
                        handle: expression.handle,
                        block,
                        before,
                        operation: OperationDraft::ConstI64(value),
                    })
                }
                ExpressionKindDraft::AddI64 { lhs, rhs } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        handle: expression.handle,
                        block,
                        before,
                        operation: OperationDraft::AddI64 { lhs, rhs },
                    })
                }
                ExpressionKindDraft::LtI64 { lhs, rhs } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        handle: expression.handle,
                        block,
                        before,
                        operation: OperationDraft::LtI64 { lhs, rhs },
                    })
                }
                ExpressionKindDraft::Call {
                    function,
                    arguments,
                } => edits.push(CanonicalEdit::CreateOperation {
                    handle: expression.handle,
                    block,
                    before,
                    operation: OperationDraft::Call {
                        function,
                        arguments,
                    },
                }),
                ExpressionKindDraft::Hole { expected } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        handle: expression.handle,
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
                        handle: expression.handle,
                        block,
                        before,
                        operation: OperationDraft::If {
                            condition,
                            result,
                            then_region: NodeTarget::Local(then_region),
                            else_region: NodeTarget::Local(else_region),
                        },
                    });
                    events.push(ExpandEvent::YieldingBody {
                        owner: NodeTarget::Local(expression.handle),
                        region: else_region,
                        arguments: Vec::new(),
                        body: else_body,
                    });
                    events.push(ExpandEvent::YieldingBody {
                        owner: NodeTarget::Local(expression.handle),
                        region: then_region,
                        arguments: Vec::new(),
                        body: then_body,
                    });
                }
                ExpressionKindDraft::ForI64 {
                    start,
                    end_exclusive,
                    step,
                    initial,
                    carried,
                    index_handle,
                    carried_handle,
                    body,
                } => {
                    let body_region = synthetic.next(current_source)?;
                    edits.push(CanonicalEdit::CreateOperation {
                        handle: expression.handle,
                        block,
                        before,
                        operation: OperationDraft::ForI64 {
                            start,
                            end_exclusive,
                            step,
                            initial,
                            carried,
                            body_region: NodeTarget::Local(body_region),
                        },
                    });
                    events.push(ExpandEvent::YieldingBody {
                        owner: NodeTarget::Local(expression.handle),
                        region: body_region,
                        arguments: vec![(index_handle, TypeDraft::I64), (carried_handle, carried)],
                        body,
                    });
                }
                ExpressionKindDraft::ConstructProduct { product, fields } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        handle: expression.handle,
                        block,
                        before,
                        operation: OperationDraft::ConstructProduct { product, fields },
                    })
                }
                ExpressionKindDraft::ProjectField { value, field } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        handle: expression.handle,
                        block,
                        before,
                        operation: OperationDraft::ProjectField { value, field },
                    })
                }
                ExpressionKindDraft::ConstructVariant { variant, payload } => {
                    edits.push(CanonicalEdit::CreateOperation {
                        handle: expression.handle,
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
                    for arm in arms {
                        let region = synthetic.next(current_source)?;
                        canonical_arms.push(MatchArmOperationDraft {
                            variant: arm.variant,
                            region: NodeTarget::Local(region),
                        });
                        arm_events.push(ExpandEvent::MatchArmBody {
                            owner: NodeTarget::Local(expression.handle),
                            region,
                            variant: arm.variant,
                            payload_handle: arm.payload_handle,
                            body: arm.body,
                        });
                    }
                    edits.push(CanonicalEdit::CreateOperation {
                        handle: expression.handle,
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
        explicit_handles,
        nominal_catalogue,
    })
}

struct SyntheticHandles {
    used: BTreeSet<LocalHandle>,
    next: u32,
}
impl SyntheticHandles {
    fn new(explicit: &BTreeSet<LocalHandle>) -> Self {
        Self {
            used: explicit.clone(),
            next: u32::MAX,
        }
    }
    fn next(&mut self, source: usize) -> Result<LocalHandle> {
        while self.next != 0 && self.used.contains(&LocalHandle::new(self.next)) {
            self.next -= 1;
        }
        if self.next == 0 {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "private structured handle space exhausted",
            )
            .at_operation(source));
        }
        let handle = LocalHandle::new(self.next);
        self.used.insert(handle);
        self.next -= 1;
        Ok(handle)
    }
}

#[cfg(test)]
pub(crate) fn validate_structured_request(operations: &[TransactionOp]) -> Result<()> {
    scan_explicit_handles(operations).map(|_| ())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LocalHandleKind {
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
enum LocalReferenceKind {
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

impl LocalReferenceKind {
    fn accepts(self, actual: LocalHandleKind) -> bool {
        match self {
            Self::Any => true,
            Self::NominalType => matches!(
                actual,
                LocalHandleKind::ProductType | LocalHandleKind::SumType
            ),
            Self::Package => actual == LocalHandleKind::Package,
            Self::Module => actual == LocalHandleKind::Module,
            Self::ProductType => actual == LocalHandleKind::ProductType,
            Self::ProductField => actual == LocalHandleKind::ProductField,
            Self::SumVariant => actual == LocalHandleKind::SumVariant,
            Self::Function => actual == LocalHandleKind::Function,
            Self::Parameter => actual == LocalHandleKind::Parameter,
            Self::Region => actual == LocalHandleKind::Region,
            Self::BlockArgument => actual == LocalHandleKind::BlockArgument,
            Self::Operation => actual == LocalHandleKind::Operation,
        }
    }
}

fn scan_explicit_handles(operations: &[TransactionOp]) -> Result<BTreeSet<LocalHandle>> {
    enum Scan<'a> {
        Expression(&'a ExpressionDraft, usize, usize),
        Body(&'a [ExpressionDraft], ValueDraft, usize, usize),
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
        handles: &mut BTreeSet<LocalHandle>,
        kinds: &mut BTreeMap<LocalHandle, LocalHandleKind>,
        handle: LocalHandle,
        kind: LocalHandleKind,
        source: usize,
    ) -> Result<()> {
        if handle.get() == 0 {
            return Err(
                LkError::new(ErrorCode::InvalidHandle, "local handle zero is reserved")
                    .at_operation(source)
                    .for_handle(handle),
            );
        }
        if !handles.insert(handle) {
            return Err(LkError::new(
                ErrorCode::DuplicateHandle,
                "transaction-local handle is declared more than once",
            )
            .at_operation(source)
            .for_handle(handle));
        }
        kinds.insert(handle, kind);
        Ok(())
    }
    fn reference(
        target: NodeTarget,
        expected: LocalReferenceKind,
        source: usize,
        references: &mut Vec<(LocalHandle, LocalReferenceKind, usize)>,
    ) {
        if let NodeTarget::Local(handle) = target {
            references.push((handle, expected, source));
        }
    }
    fn type_reference(
        ty: TypeDraft,
        source: usize,
        references: &mut Vec<(LocalHandle, LocalReferenceKind, usize)>,
    ) {
        if let TypeDraft::Nominal(target) = ty {
            reference(target, LocalReferenceKind::NominalType, source, references);
        }
    }
    fn value_reference(
        value: ValueDraft,
        source: usize,
        references: &mut Vec<(LocalHandle, LocalReferenceKind, usize)>,
    ) -> Result<()> {
        validate_draft_value(value, source)?;
        match value {
            ValueDraft::FunctionParameter(target) => {
                reference(target, LocalReferenceKind::Parameter, source, references)
            }
            ValueDraft::BlockArgument(target) => reference(
                target,
                LocalReferenceKind::BlockArgument,
                source,
                references,
            ),
            ValueDraft::OperationResult { operation, .. } => {
                reference(operation, LocalReferenceKind::Operation, source, references)
            }
        }
        Ok(())
    }
    fn operation_references(
        operation: &OperationDraft,
        source: usize,
        budget: &mut DraftBudget,
        references: &mut Vec<(LocalHandle, LocalReferenceKind, usize)>,
    ) -> Result<()> {
        match operation {
            OperationDraft::ConstUnit
            | OperationDraft::ConstI64(_)
            | OperationDraft::ConstBool(_) => {}
            OperationDraft::AddI64 { lhs, rhs } | OperationDraft::LtI64 { lhs, rhs } => {
                value_reference(*lhs, source, references)?;
                value_reference(*rhs, source, references)?;
            }
            OperationDraft::Call {
                function,
                arguments,
            } => {
                reference(*function, LocalReferenceKind::Function, source, references);
                budget.add(arguments.len(), source)?;
                for value in arguments {
                    value_reference(*value, source, references)?;
                }
            }
            OperationDraft::Hole { expected } => type_reference(*expected, source, references),
            OperationDraft::If {
                condition,
                result,
                then_region,
                else_region,
            } => {
                value_reference(*condition, source, references)?;
                type_reference(*result, source, references);
                reference(*then_region, LocalReferenceKind::Region, source, references);
                reference(*else_region, LocalReferenceKind::Region, source, references);
            }
            OperationDraft::ForI64 {
                start,
                end_exclusive,
                initial,
                carried,
                body_region,
                ..
            } => {
                value_reference(*start, source, references)?;
                value_reference(*end_exclusive, source, references)?;
                value_reference(*initial, source, references)?;
                type_reference(*carried, source, references);
                reference(*body_region, LocalReferenceKind::Region, source, references);
            }
            OperationDraft::Return { value } | OperationDraft::Yield { value } => {
                value_reference(*value, source, references)?;
            }
            OperationDraft::ConstructProduct { product, fields } => {
                reference(
                    *product,
                    LocalReferenceKind::ProductType,
                    source,
                    references,
                );
                budget.add(fields.len(), source)?;
                for field in fields {
                    reference(
                        field.field,
                        LocalReferenceKind::ProductField,
                        source,
                        references,
                    );
                    value_reference(field.value, source, references)?;
                }
            }
            OperationDraft::ProjectField { value, field } => {
                value_reference(*value, source, references)?;
                reference(*field, LocalReferenceKind::ProductField, source, references);
            }
            OperationDraft::ConstructVariant { variant, payload } => {
                reference(*variant, LocalReferenceKind::SumVariant, source, references);
                if let Some(value) = payload {
                    value_reference(*value, source, references)?;
                }
            }
            OperationDraft::MatchSum {
                scrutinee,
                result,
                arms,
            } => {
                value_reference(*scrutinee, source, references)?;
                type_reference(*result, source, references);
                budget.add(arms.len(), source)?;
                for arm in arms {
                    reference(
                        arm.variant,
                        LocalReferenceKind::SumVariant,
                        source,
                        references,
                    );
                    reference(arm.region, LocalReferenceKind::Any, source, references);
                }
            }
        }
        Ok(())
    }

    let mut handles = BTreeSet::new();
    let mut kinds = BTreeMap::new();
    let mut references = Vec::<(LocalHandle, LocalReferenceKind, usize)>::new();
    let mut stack = Vec::new();
    let mut budget = DraftBudget(0);
    for (source, operation) in operations.iter().enumerate() {
        budget.add(1, source)?;
        match operation {
            TransactionOp::CreatePackage { handle, .. } => declare(
                &mut handles,
                &mut kinds,
                *handle,
                LocalHandleKind::Package,
                source,
            )?,
            TransactionOp::CreateModule {
                handle, package, ..
            } => {
                declare(
                    &mut handles,
                    &mut kinds,
                    *handle,
                    LocalHandleKind::Module,
                    source,
                )?;
                reference(
                    *package,
                    LocalReferenceKind::Package,
                    source,
                    &mut references,
                );
            }
            TransactionOp::CreateProductType {
                handle,
                module,
                fields,
                ..
            } => {
                declare(
                    &mut handles,
                    &mut kinds,
                    *handle,
                    LocalHandleKind::ProductType,
                    source,
                )?;
                reference(*module, LocalReferenceKind::Module, source, &mut references);
                budget.add(fields.len(), source)?;
                for field in fields {
                    declare(
                        &mut handles,
                        &mut kinds,
                        field.handle,
                        LocalHandleKind::ProductField,
                        source,
                    )?;
                    type_reference(field.ty, source, &mut references);
                }
            }
            TransactionOp::CreateSumType {
                handle,
                module,
                variants,
                ..
            } => {
                declare(
                    &mut handles,
                    &mut kinds,
                    *handle,
                    LocalHandleKind::SumType,
                    source,
                )?;
                reference(*module, LocalReferenceKind::Module, source, &mut references);
                if variants.is_empty() {
                    return Err(LkError::new(
                        ErrorCode::InvalidOperand,
                        "sum declarations require at least one variant",
                    )
                    .at_operation(source)
                    .for_handle(*handle));
                }
                budget.add(variants.len(), source)?;
                for variant in variants {
                    declare(
                        &mut handles,
                        &mut kinds,
                        variant.handle,
                        LocalHandleKind::SumVariant,
                        source,
                    )?;
                    if let Some(payload) = variant.payload {
                        type_reference(payload, source, &mut references);
                    }
                }
            }
            TransactionOp::CreateFunction {
                handle,
                module,
                parameters,
                result,
                body,
                ..
            } => {
                declare(
                    &mut handles,
                    &mut kinds,
                    *handle,
                    LocalHandleKind::Function,
                    source,
                )?;
                reference(*module, LocalReferenceKind::Module, source, &mut references);
                type_reference(*result, source, &mut references);
                budget.add(parameters.len(), source)?;
                for parameter in parameters {
                    declare(
                        &mut handles,
                        &mut kinds,
                        parameter.handle,
                        LocalHandleKind::Parameter,
                        source,
                    )?;
                    type_reference(parameter.ty, source, &mut references);
                }
                if let Some(body) = body {
                    budget.add(1, source)?;
                    stack.push(Scan::Body(&body.operations, body.return_value, 0, source));
                }
            }
            TransactionOp::DefineFunctionBody { function, body } => {
                reference(
                    *function,
                    LocalReferenceKind::Function,
                    source,
                    &mut references,
                );
                budget.add(1, source)?;
                stack.push(Scan::Body(&body.operations, body.return_value, 0, source));
            }
            TransactionOp::InsertExpression { expression, .. } => {
                stack.push(Scan::Expression(expression, 0, source))
            }
            TransactionOp::SetEntryFunction { package, function } => {
                reference(
                    *package,
                    LocalReferenceKind::Package,
                    source,
                    &mut references,
                );
                reference(
                    *function,
                    LocalReferenceKind::Function,
                    source,
                    &mut references,
                );
            }
            TransactionOp::RenameNode { node, .. } => {
                reference(*node, LocalReferenceKind::Any, source, &mut references)
            }
            TransactionOp::ReplaceOperation {
                operation,
                replacement,
            } => {
                reference(
                    *operation,
                    LocalReferenceKind::Operation,
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
                    LocalReferenceKind::Operation,
                    source,
                    &mut references,
                );
                value_reference(*value, source, &mut references)?;
            }
            TransactionOp::RefineHole { hole, replacement } => {
                reference(
                    *hole,
                    LocalReferenceKind::Operation,
                    source,
                    &mut references,
                );
                if matches!(replacement, OperationDraft::MatchSum { .. }) {
                    return Err(LkError::new(ErrorCode::InvalidOperand, "match_sum cannot be authored through a region-scaffolding maintenance operation").at_operation(source));
                }
                operation_references(replacement, source, &mut budget, &mut references)?;
            }
            TransactionOp::DeleteOwnedSubtree { root } => {
                reference(*root, LocalReferenceKind::Any, source, &mut references)
            }
        }
    }
    while let Some(event) = stack.pop() {
        match event {
            Scan::Body(expressions, terminal, depth, source) => {
                value_reference(terminal, source, &mut references)?;
                if depth > MAX_STRUCTURED_DRAFT_DEPTH {
                    return Err(LkError::new(
                        ErrorCode::PolicyExceeded,
                        "structured draft nesting exceeds request depth policy",
                    )
                    .at_operation(source));
                }
                for expression in expressions.iter().rev() {
                    stack.push(Scan::Expression(expression, depth, source));
                }
            }
            Scan::Expression(expression, depth, source) => {
                budget.add(1, source)?;
                declare(
                    &mut handles,
                    &mut kinds,
                    expression.handle,
                    LocalHandleKind::Operation,
                    source,
                )?;
                match &expression.operation {
                    ExpressionKindDraft::ConstUnit
                    | ExpressionKindDraft::ConstBool(_)
                    | ExpressionKindDraft::ConstI64(_) => {}
                    ExpressionKindDraft::AddI64 { lhs, rhs }
                    | ExpressionKindDraft::LtI64 { lhs, rhs } => {
                        value_reference(*lhs, source, &mut references)?;
                        value_reference(*rhs, source, &mut references)?;
                    }
                    ExpressionKindDraft::Call {
                        function,
                        arguments,
                    } => {
                        reference(
                            *function,
                            LocalReferenceKind::Function,
                            source,
                            &mut references,
                        );
                        budget.add(arguments.len(), source)?;
                        for value in arguments {
                            value_reference(*value, source, &mut references)?;
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
                        value_reference(*condition, source, &mut references)?;
                        type_reference(*result, source, &mut references);
                        budget.add(2, source)?;
                        stack.push(Scan::Body(
                            &else_body.operations,
                            else_body.yield_value,
                            depth + 1,
                            source,
                        ));
                        stack.push(Scan::Body(
                            &then_body.operations,
                            then_body.yield_value,
                            depth + 1,
                            source,
                        ));
                    }
                    ExpressionKindDraft::ForI64 {
                        start,
                        end_exclusive,
                        initial,
                        carried,
                        index_handle,
                        carried_handle,
                        body,
                        ..
                    } => {
                        value_reference(*start, source, &mut references)?;
                        value_reference(*end_exclusive, source, &mut references)?;
                        value_reference(*initial, source, &mut references)?;
                        type_reference(*carried, source, &mut references);
                        declare(
                            &mut handles,
                            &mut kinds,
                            *index_handle,
                            LocalHandleKind::BlockArgument,
                            source,
                        )?;
                        declare(
                            &mut handles,
                            &mut kinds,
                            *carried_handle,
                            LocalHandleKind::BlockArgument,
                            source,
                        )?;
                        budget.add(1, source)?;
                        stack.push(Scan::Body(
                            &body.operations,
                            body.yield_value,
                            depth + 1,
                            source,
                        ));
                    }
                    ExpressionKindDraft::ConstructProduct { product, fields } => {
                        reference(
                            *product,
                            LocalReferenceKind::ProductType,
                            source,
                            &mut references,
                        );
                        budget.add(fields.len(), source)?;
                        for field in fields {
                            reference(
                                field.field,
                                LocalReferenceKind::ProductField,
                                source,
                                &mut references,
                            );
                            value_reference(field.value, source, &mut references)?;
                        }
                    }
                    ExpressionKindDraft::ProjectField { value, field } => {
                        value_reference(*value, source, &mut references)?;
                        reference(
                            *field,
                            LocalReferenceKind::ProductField,
                            source,
                            &mut references,
                        );
                    }
                    ExpressionKindDraft::ConstructVariant { variant, payload } => {
                        reference(
                            *variant,
                            LocalReferenceKind::SumVariant,
                            source,
                            &mut references,
                        );
                        if let Some(value) = payload {
                            value_reference(*value, source, &mut references)?;
                        }
                    }
                    ExpressionKindDraft::MatchSum {
                        scrutinee,
                        result,
                        arms,
                    } => {
                        value_reference(*scrutinee, source, &mut references)?;
                        type_reference(*result, source, &mut references);
                        budget.add(arms.len(), source)?;
                        for arm in arms.iter().rev() {
                            reference(
                                arm.variant,
                                LocalReferenceKind::SumVariant,
                                source,
                                &mut references,
                            );
                            if let Some(handle) = arm.payload_handle {
                                declare(
                                    &mut handles,
                                    &mut kinds,
                                    handle,
                                    LocalHandleKind::BlockArgument,
                                    source,
                                )?;
                            }
                            budget.add(1, source)?;
                            stack.push(Scan::Body(
                                &arm.body.operations,
                                arm.body.yield_value,
                                depth + 1,
                                source,
                            ));
                        }
                    }
                }
            }
        }
    }
    for (handle, expected, source) in references {
        let Some(actual) = kinds.get(&handle).copied() else {
            return Err(LkError::new(
                ErrorCode::InvalidHandle,
                "structured draft references an undeclared local handle",
            )
            .at_operation(source)
            .for_handle(handle));
        };
        if !expected.accepts(actual) {
            return Err(LkError::new(
                ErrorCode::WrongKind,
                "transaction-local reference has the wrong declared category",
            )
            .at_operation(source)
            .for_handle(handle));
        }
    }
    Ok(handles)
}

fn validate_draft_value(value: ValueDraft, source: usize) -> Result<()> {
    if let ValueDraft::OperationResult { operation, output } = value
        && output != 0
    {
        let mut error = LkError::new(
            ErrorCode::InvalidOperand,
            "structured operation result output must be zero for the closed single-result schema",
        )
        .at_operation(source);
        if let NodeTarget::Local(handle) = operation {
            error = error.for_handle(handle);
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
        let (allocations, next_serial) = allocate_handles(
            base,
            &expanded.edits,
            &expanded.edit_sources,
            &expanded.explicit_handles,
        )?;
        validate_response_spec(&request.response, &allocations, &expanded.explicit_handles)?;
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
                return Err(error);
            }
            record_edit_provenance(operation, *source, &allocations, &mut provenance)?;
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
                    if error.local_handle.is_none() {
                        error.local_handle = error_handle_from_provenance(
                            &error,
                            &provenance,
                            &allocations,
                            &expanded.explicit_handles,
                        );
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
                if error.local_handle.is_none() {
                    error.local_handle = error_handle_from_provenance(
                        &error,
                        &provenance,
                        &allocations,
                        &expanded.explicit_handles,
                    );
                }
            }
            return Err(error);
        }
        let semantic_diff = diff::between(base, &candidate);
        let blockers_before = query::workspace_blockers(base);
        let blockers_after = query::workspace_blockers(&candidate);
        let returned_bindings = request
            .response
            .return_handles
            .iter()
            .map(|handle| allocated(&allocations, *handle).map(|node| (*handle, node)))
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

#[derive(Clone, Copy)]
struct NodeProvenance {
    source: usize,
    offending_use: bool,
}

fn record_edit_provenance(
    edit: &CanonicalEdit,
    source: usize,
    allocations: &BTreeMap<LocalHandle, NodeId>,
    provenance: &mut BTreeMap<NodeId, NodeProvenance>,
) -> Result<()> {
    let (target, offending_use) = match edit {
        CanonicalEdit::CreateOperation { handle, .. } => {
            (Some(allocated(allocations, *handle)?), true)
        }
        CanonicalEdit::CreatePackage { handle, .. }
        | CanonicalEdit::CreateModule { handle, .. }
        | CanonicalEdit::CreateProductType { handle, .. }
        | CanonicalEdit::CreateProductField { handle, .. }
        | CanonicalEdit::CreateSumType { handle, .. }
        | CanonicalEdit::CreateSumVariant { handle, .. }
        | CanonicalEdit::CreateFunction { handle, .. }
        | CanonicalEdit::CreateParameter { handle, .. }
        | CanonicalEdit::CreateRegion { handle, .. }
        | CanonicalEdit::CreateBlock { handle, .. }
        | CanonicalEdit::CreateBlockArgument { handle, .. }
        | CanonicalEdit::CreateMatchPayloadArgument { handle, .. } => {
            (Some(allocated(allocations, *handle)?), false)
        }
        CanonicalEdit::ReplaceOperation { operation, .. }
        | CanonicalEdit::ReplaceOperand { operation, .. } => {
            (Some(resolve_for_provenance(*operation, allocations)?), true)
        }
        CanonicalEdit::RefineHole { hole, .. } => {
            (Some(resolve_for_provenance(*hole, allocations)?), true)
        }
        CanonicalEdit::SetFunctionBody { function, .. } => {
            (Some(resolve_for_provenance(*function, allocations)?), false)
        }
        CanonicalEdit::SetEntryFunction { package, .. } => {
            (Some(resolve_for_provenance(*package, allocations)?), false)
        }
        CanonicalEdit::RenameNode { node, .. } => {
            (Some(resolve_for_provenance(*node, allocations)?), false)
        }
        CanonicalEdit::DeleteOwnedSubtree { root } => {
            (Some(resolve_for_provenance(*root, allocations)?), false)
        }
    };
    if let Some(target) = target {
        provenance.insert(
            target,
            NodeProvenance {
                source,
                offending_use,
            },
        );
    }
    Ok(())
}

fn resolve_for_provenance(
    target: NodeTarget,
    allocations: &BTreeMap<LocalHandle, NodeId>,
) -> Result<NodeId> {
    match target {
        NodeTarget::Existing(node) => Ok(node),
        NodeTarget::Local(handle) => allocated(allocations, handle),
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

fn error_handle_from_provenance(
    error: &LkError,
    provenance: &BTreeMap<NodeId, NodeProvenance>,
    allocations: &BTreeMap<LocalHandle, NodeId>,
    explicit_handles: &BTreeSet<LocalHandle>,
) -> Option<LocalHandle> {
    let (node, _) = preferred_error_provenance(error, provenance)?;
    allocations.iter().find_map(|(handle, allocated)| {
        (*allocated == node && explicit_handles.contains(handle)).then_some(*handle)
    })
}

fn validate_response_spec(
    response: &TransactionResponseSpec,
    allocations: &BTreeMap<LocalHandle, NodeId>,
    explicit_handles: &BTreeSet<LocalHandle>,
) -> Result<()> {
    if response.return_handles.len() > MAX_RETURNED_BINDINGS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "selected return handles exceed transaction response policy",
        ));
    }
    let mut previous = None;
    for handle in &response.return_handles {
        if previous.is_some_and(|prior| *handle <= prior) {
            return Err(LkError::new(
                ErrorCode::InvalidHandle,
                "selected return handles must be unique and strictly increasing",
            )
            .for_handle(*handle));
        }
        if !explicit_handles.contains(handle) || !allocations.contains_key(handle) {
            return Err(LkError::new(
                ErrorCode::InvalidHandle,
                "selected return handle is not declared by this transaction",
            )
            .for_handle(*handle));
        }
        previous = Some(*handle);
    }
    Ok(())
}

fn allocation_error(
    code: ErrorCode,
    message: impl Into<String>,
    source: usize,
    handle: LocalHandle,
    explicit_handles: &BTreeSet<LocalHandle>,
) -> LkError {
    let error = LkError::new(code, message).at_operation(source);
    if explicit_handles.contains(&handle) {
        error.for_handle(handle)
    } else {
        error
    }
}

fn allocate_handles(
    base: &Snapshot,
    operations: &[CanonicalEdit],
    edit_sources: &[usize],
    explicit_handles: &BTreeSet<LocalHandle>,
) -> Result<(BTreeMap<LocalHandle, NodeId>, u64)> {
    let mut allocations = BTreeMap::new();
    let mut next = base.next_serial;
    for (operation, source) in operations.iter().zip(edit_sources) {
        let Some(handle) = canonical_created_handle(operation) else {
            continue;
        };
        if allocations.contains_key(&handle) {
            return Err(allocation_error(
                ErrorCode::DuplicateHandle,
                "transaction-local handle is declared more than once",
                *source,
                handle,
                explicit_handles,
            ));
        }
        let id = NodeId::new(base.workspace(), next).map_err(|error| {
            allocation_error(
                ErrorCode::PolicyExceeded,
                format!("node identity allocation failed: {error}"),
                *source,
                handle,
                explicit_handles,
            )
        })?;
        next = next.checked_add(1).ok_or_else(|| {
            allocation_error(
                ErrorCode::PolicyExceeded,
                "node identity serial is exhausted",
                *source,
                handle,
                explicit_handles,
            )
        })?;
        allocations.insert(handle, id);
    }
    Ok((allocations, next))
}

fn canonical_created_handle(operation: &CanonicalEdit) -> Option<LocalHandle> {
    match operation {
        CanonicalEdit::CreatePackage { handle, .. }
        | CanonicalEdit::CreateModule { handle, .. }
        | CanonicalEdit::CreateProductType { handle, .. }
        | CanonicalEdit::CreateProductField { handle, .. }
        | CanonicalEdit::CreateSumType { handle, .. }
        | CanonicalEdit::CreateSumVariant { handle, .. }
        | CanonicalEdit::CreateFunction { handle, .. }
        | CanonicalEdit::CreateParameter { handle, .. }
        | CanonicalEdit::CreateRegion { handle, .. }
        | CanonicalEdit::CreateBlock { handle, .. }
        | CanonicalEdit::CreateBlockArgument { handle, .. }
        | CanonicalEdit::CreateMatchPayloadArgument { handle, .. }
        | CanonicalEdit::CreateOperation { handle, .. } => Some(*handle),
        _ => None,
    }
}

fn apply_operation(
    base: &Snapshot,
    nodes: &mut BTreeMap<NodeId, Node>,
    tombstones: &mut BTreeSet<u64>,
    allocations: &BTreeMap<LocalHandle, NodeId>,
    nominal_catalogue: &StagedNominalCatalogue,
    operation: &CanonicalEdit,
) -> Result<()> {
    match operation {
        CanonicalEdit::CreatePackage { handle, name } => {
            let id = allocated(allocations, *handle)?;
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
            handle,
            package,
            name,
        } => {
            let id = allocated(allocations, *handle)?;
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
            handle,
            module,
            name,
        } => {
            let id = allocated(allocations, *handle)?;
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
            handle,
            product,
            name,
            ty,
        } => {
            let id = allocated(allocations, *handle)?;
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
            handle,
            module,
            name,
        } => {
            let id = allocated(allocations, *handle)?;
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
            handle,
            sum,
            name,
            payload,
        } => {
            let id = allocated(allocations, *handle)?;
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
            handle,
            module,
            name,
            result,
        } => {
            let id = allocated(allocations, *handle)?;
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
            handle,
            function,
            name,
            ty,
        } => {
            let id = allocated(allocations, *handle)?;
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
        CanonicalEdit::CreateRegion { handle, owner } => {
            let id = allocated(allocations, *handle)?;
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
        CanonicalEdit::CreateBlock { handle, region } => {
            let id = allocated(allocations, *handle)?;
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
        CanonicalEdit::CreateBlockArgument { handle, block, ty } => {
            let id = allocated(allocations, *handle)?;
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
            handle,
            block,
            variant,
        } => {
            let id = allocated(allocations, *handle)?;
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
                        "nullary match arm cannot declare a payload handle",
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
            handle,
            block,
            before,
            operation,
        } => {
            let id = allocated(allocations, *handle)?;
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
            let value = resolve_value(*value, allocations, base.workspace())?;
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
    allocations: &BTreeMap<LocalHandle, NodeId>,
    workspace: WorkspaceId,
    nominal_catalogue: &StagedNominalCatalogue,
) -> Result<OperationKind> {
    Ok(match operation {
        OperationDraft::ConstUnit => OperationKind::ConstUnit,
        OperationDraft::ConstI64(value) => OperationKind::ConstI64(*value),
        OperationDraft::ConstBool(value) => OperationKind::ConstBool(*value),
        OperationDraft::AddI64 { lhs, rhs } => OperationKind::AddI64 {
            lhs: resolve_value(*lhs, allocations, workspace)?,
            rhs: resolve_value(*rhs, allocations, workspace)?,
        },
        OperationDraft::LtI64 { lhs, rhs } => OperationKind::LtI64 {
            lhs: resolve_value(*lhs, allocations, workspace)?,
            rhs: resolve_value(*rhs, allocations, workspace)?,
        },
        OperationDraft::Call {
            function,
            arguments,
        } => OperationKind::Call {
            function: resolve(*function, allocations, workspace)?,
            arguments: arguments
                .iter()
                .copied()
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
            condition: resolve_value(*condition, allocations, workspace)?,
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
            start: resolve_value(*start, allocations, workspace)?,
            end_exclusive: resolve_value(*end_exclusive, allocations, workspace)?,
            step: *step,
            initial: resolve_value(*initial, allocations, workspace)?,
            carried: resolve_type_draft(*carried, allocations, workspace)?,
            body_region: resolve(*body_region, allocations, workspace)?,
        },
        OperationDraft::Return { value } => OperationKind::Return {
            value: resolve_value(*value, allocations, workspace)?,
        },
        OperationDraft::Yield { value } => OperationKind::Yield {
            value: resolve_value(*value, allocations, workspace)?,
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
                        resolve_value(field.value, allocations, workspace)?,
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
            value: resolve_value(*value, allocations, workspace)?,
            field: resolve(*field, allocations, workspace)?,
        },
        OperationDraft::ConstructVariant { variant, payload } => OperationKind::ConstructVariant {
            variant: resolve(*variant, allocations, workspace)?,
            payload: payload
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
                scrutinee: resolve_value(*scrutinee, allocations, workspace)?,
                result: resolve_type_draft(*result, allocations, workspace)?,
                arms: resolved,
            }
        }
    })
}

fn resolve_type_draft(
    ty: TypeDraft,
    allocations: &BTreeMap<LocalHandle, NodeId>,
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
    value: ValueDraft,
    allocations: &BTreeMap<LocalHandle, NodeId>,
    workspace: WorkspaceId,
) -> Result<ValueRef> {
    Ok(match value {
        ValueDraft::FunctionParameter(parameter) => {
            ValueRef::FunctionParameter(resolve(parameter, allocations, workspace)?)
        }
        ValueDraft::BlockArgument(argument) => {
            ValueRef::BlockArgument(resolve(argument, allocations, workspace)?)
        }
        ValueDraft::OperationResult { operation, output } => ValueRef::OperationResult {
            operation: resolve(operation, allocations, workspace)?,
            output,
        },
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
    allocations: &BTreeMap<LocalHandle, NodeId>,
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
        NodeTarget::Local(handle) => allocations.get(&handle).copied().ok_or_else(|| {
            LkError::new(
                ErrorCode::InvalidHandle,
                "transaction references an undeclared local handle",
            )
            .for_handle(handle)
        }),
    }
}

fn allocated(allocations: &BTreeMap<LocalHandle, NodeId>, handle: LocalHandle) -> Result<NodeId> {
    allocations.get(&handle).copied().ok_or_else(|| {
        LkError::new(
            ErrorCode::InvalidHandle,
            "create operation has no staged node allocation",
        )
        .for_handle(handle)
    })
}

fn insert_new(nodes: &mut BTreeMap<NodeId, Node>, id: NodeId, node: Node) -> Result<()> {
    if nodes.insert(id, node).is_some() {
        return Err(LkError::new(
            ErrorCode::InvalidHandle,
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
        let mut return_handles: Vec<LocalHandle> = scan_explicit_handles(&transaction.operations)
            .expect("valid test handles")
            .into_iter()
            .collect();
        return_handles.sort();
        return_handles.truncate(MAX_RETURNED_BINDINGS);
        ApplyTransactionRequest {
            transaction: transaction.clone(),
            response: TransactionResponseSpec { return_handles },
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
                    handle: LocalHandle::new(1),
                    name: "package".to_owned(),
                },
                TransactionOp::CreateModule {
                    handle: LocalHandle::new(2),
                    package: NodeTarget::Local(LocalHandle::new(1)),
                    name: "module".to_owned(),
                },
            ],
        }
    }

    fn local_handle(value: u32) -> NodeTarget {
        NodeTarget::Local(LocalHandle::new(value))
    }
    fn draft_result(value: u32) -> ValueDraft {
        ValueDraft::OperationResult {
            operation: local_handle(value),
            output: 0,
        }
    }
    fn draft_expression(handle: u32, operation: ExpressionKindDraft) -> ExpressionDraft {
        ExpressionDraft {
            handle: LocalHandle::new(handle),
            operation,
        }
    }
    fn structured_semantic_request(
        id: WorkspaceId,
        mut operations: Vec<TransactionOp>,
    ) -> ApplyTransactionRequest {
        let mut all = vec![
            TransactionOp::CreatePackage {
                handle: LocalHandle::new(1),
                name: "package".into(),
            },
            TransactionOp::CreateModule {
                handle: LocalHandle::new(2),
                package: local_handle(1),
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

    #[test]
    fn negative_for_step_rejects_atomically() {
        let id = WorkspaceId::from_bytes([0xa1; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let request = structured_semantic_request(
            id,
            vec![TransactionOp::CreateFunction {
                handle: LocalHandle::new(3),
                module: local_handle(2),
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
                                index_handle: LocalHandle::new(7),
                                carried_handle: LocalHandle::new(8),
                                body: YieldingBodyDraft {
                                    operations: Vec::new(),
                                    yield_value: ValueDraft::BlockArgument(local_handle(8)),
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
                handle: LocalHandle::new(3),
                module: local_handle(2),
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
                handle: LocalHandle::new(3),
                module: local_handle(2),
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
                    handle: LocalHandle::new(3),
                    module: local_handle(2),
                    name: "producer".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![draft_expression(4, ExpressionKindDraft::ConstI64(1))],
                        return_value: draft_result(4),
                    }),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(5),
                    module: local_handle(2),
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
                        handle: LocalHandle::new(1),
                        name: "package".into(),
                    },
                    TransactionOp::CreateModule {
                        handle: LocalHandle::new(2),
                        package: local_handle(1),
                        name: "left".into(),
                    },
                    TransactionOp::CreateModule {
                        handle: LocalHandle::new(3),
                        package: local_handle(1),
                        name: "right".into(),
                    },
                    TransactionOp::CreateFunction {
                        handle: LocalHandle::new(4),
                        module: local_handle(2),
                        name: "callee".into(),
                        parameters: Vec::new(),
                        result: SemanticType::I64.into(),
                        body: Some(FunctionBodyDraft {
                            operations: vec![draft_expression(5, ExpressionKindDraft::ConstI64(7))],
                            return_value: draft_result(5),
                        }),
                    },
                    TransactionOp::CreateFunction {
                        handle: LocalHandle::new(6),
                        module: local_handle(3),
                        name: "caller".into(),
                        parameters: Vec::new(),
                        result: SemanticType::I64.into(),
                        body: Some(FunctionBodyDraft {
                            operations: vec![draft_expression(
                                7,
                                ExpressionKindDraft::Call {
                                    function: local_handle(4),
                                    arguments: Vec::new(),
                                },
                            )],
                            return_value: draft_result(7),
                        }),
                    },
                    TransactionOp::SetEntryFunction {
                        package: local_handle(1),
                        function: local_handle(6),
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
                    handle: LocalHandle::new(3),
                    module: NodeTarget::Existing(module),
                    name: "duplicate".to_owned(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: None,
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(4),
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
                handle: LocalHandle::new(5),
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
                handle: LocalHandle::new(3),
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
                handle: LocalHandle::new(4),
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
        let package = LocalHandle::new(1);
        let module = LocalHandle::new(2);
        let function = LocalHandle::new(3);
        let first_value = LocalHandle::new(6);
        let body_operations = (0..10_000_u32)
            .map(|offset| ExpressionDraft {
                handle: LocalHandle::new(6 + offset),
                operation: ExpressionKindDraft::ConstI64(i64::from(offset)),
            })
            .collect();
        let operations = vec![
            TransactionOp::CreatePackage {
                handle: package,
                name: "package".to_owned(),
            },
            TransactionOp::CreateModule {
                handle: module,
                package: NodeTarget::Local(package),
                name: "module".to_owned(),
            },
            TransactionOp::CreateFunction {
                handle: function,
                module: NodeTarget::Local(module),
                name: "main".to_owned(),
                parameters: Vec::new(),
                result: SemanticType::I64.into(),
                body: Some(FunctionBodyDraft {
                    operations: body_operations,
                    return_value: ValueDraft::OperationResult {
                        operation: NodeTarget::Local(first_value),
                        output: 0,
                    },
                }),
            },
            TransactionOp::SetEntryFunction {
                package: NodeTarget::Local(package),
                function: NodeTarget::Local(function),
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
        let local = NodeTarget::Local;
        let value = |handle| ValueDraft::OperationResult {
            operation: local(handle),
            output: 0,
        };
        Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    handle: LocalHandle::new(1),
                    name: "app".to_owned(),
                },
                TransactionOp::CreateModule {
                    handle: LocalHandle::new(2),
                    package: local(LocalHandle::new(1)),
                    name: "root".to_owned(),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(3),
                    module: local(LocalHandle::new(2)),
                    name: "main".to_owned(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            ExpressionDraft {
                                handle: LocalHandle::new(6),
                                operation: ExpressionKindDraft::ConstI64(40),
                            },
                            ExpressionDraft {
                                handle: LocalHandle::new(7),
                                operation: ExpressionKindDraft::ConstI64(2),
                            },
                            ExpressionDraft {
                                handle: LocalHandle::new(8),
                                operation: ExpressionKindDraft::ConstBool(true),
                            },
                            ExpressionDraft {
                                handle: LocalHandle::new(9),
                                operation: ExpressionKindDraft::Hole {
                                    expected: SemanticType::I64.into(),
                                },
                            },
                            ExpressionDraft {
                                handle: LocalHandle::new(10),
                                operation: ExpressionKindDraft::ConstI64(99),
                            },
                        ],
                        return_value: value(LocalHandle::new(9)),
                    }),
                },
                TransactionOp::SetEntryFunction {
                    package: local(LocalHandle::new(1)),
                    function: local(LocalHandle::new(3)),
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

    fn binding(receipt: &TransactionReceipt, handle: u32) -> NodeId {
        receipt
            .returned_bindings
            .iter()
            .find_map(|(candidate, node)| (candidate.get() == handle).then_some(*node))
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
                return_handles: vec![LocalHandle::new(2)],
            },
        };
        let prepared = workspace
            .prepare_transaction(&selected)
            .expect("selected receipt");
        assert_eq!(prepared.receipt.created_count, 2);
        assert_eq!(prepared.receipt.returned_bindings.len(), 1);
        assert_eq!(prepared.receipt.returned_bindings[0].0, LocalHandle::new(2));

        for return_handles in [
            vec![LocalHandle::new(1), LocalHandle::new(1)],
            vec![LocalHandle::new(2), LocalHandle::new(1)],
            vec![LocalHandle::new(3)],
        ] {
            let invalid = ApplyTransactionRequest {
                transaction: transaction.clone(),
                response: TransactionResponseSpec { return_handles },
            };
            assert_eq!(
                workspace
                    .prepare_transaction(&invalid)
                    .expect_err("invalid response projection")
                    .code,
                ErrorCode::InvalidHandle
            );
        }

        let mut too_many = Vec::new();
        for value in 0..=MAX_RETURNED_BINDINGS {
            too_many.push(LocalHandle::new(u32::try_from(value).expect("handle")));
        }
        let invalid = ApplyTransactionRequest {
            transaction: transaction.clone(),
            response: TransactionResponseSpec {
                return_handles: too_many,
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
                        handle: LocalHandle::new(1),
                        name: "p".into(),
                    },
                    TransactionOp::CreateModule {
                        handle: LocalHandle::new(2),
                        package: local_handle(1),
                        name: "m".into(),
                    },
                    TransactionOp::CreateProductType {
                        handle: LocalHandle::new(3),
                        module: local_handle(2),
                        name: "Pair".into(),
                        fields: vec![
                            ProductFieldDraft {
                                handle: LocalHandle::new(4),
                                name: "left".into(),
                                ty: TypeDraft::I64,
                            },
                            ProductFieldDraft {
                                handle: LocalHandle::new(5),
                                name: "right".into(),
                                ty: TypeDraft::I64,
                            },
                        ],
                    },
                    TransactionOp::CreateSumType {
                        handle: LocalHandle::new(6),
                        module: local_handle(2),
                        name: "Choice".into(),
                        variants: vec![
                            SumVariantDraft {
                                handle: LocalHandle::new(7),
                                name: "First".into(),
                                payload: None,
                            },
                            SumVariantDraft {
                                handle: LocalHandle::new(8),
                                name: "Second".into(),
                                payload: None,
                            },
                        ],
                    },
                    TransactionOp::CreateFunction {
                        handle: LocalHandle::new(9),
                        module: local_handle(2),
                        name: "main".into(),
                        parameters: Vec::new(),
                        result: TypeDraft::I64,
                        body: Some(FunctionBodyDraft {
                            operations: vec![
                                draft_expression(10, ExpressionKindDraft::ConstI64(1)),
                                draft_expression(
                                    11,
                                    ExpressionKindDraft::ConstructProduct {
                                        product: local_handle(3),
                                        fields: vec![
                                            ProductFieldValueDraft {
                                                field: local_handle(4),
                                                value: draft_result(10),
                                            },
                                            ProductFieldValueDraft {
                                                field: local_handle(5),
                                                value: draft_result(10),
                                            },
                                        ],
                                    },
                                ),
                                draft_expression(
                                    12,
                                    ExpressionKindDraft::ProjectField {
                                        value: draft_result(11),
                                        field: local_handle(4),
                                    },
                                ),
                                draft_expression(
                                    13,
                                    ExpressionKindDraft::ConstructVariant {
                                        variant: local_handle(7),
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
                    handle: LocalHandle::new(1),
                    name: "temporary".to_owned(),
                },
                TransactionOp::DeleteOwnedSubtree {
                    root: NodeTarget::Local(LocalHandle::new(1)),
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
                    handle: LocalHandle::new(12),
                    module: NodeTarget::Existing(module),
                    name: "callee".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: None,
                },
                TransactionOp::RefineHole {
                    hole: NodeTarget::Existing(hole),
                    replacement: OperationDraft::Call {
                        function: NodeTarget::Local(LocalHandle::new(12)),
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
        let local = |handle| NodeTarget::Local(LocalHandle::new(handle));
        let result = |handle| ValueDraft::OperationResult {
            operation: local(handle),
            output: 0,
        };
        let block_argument = |handle| ValueDraft::BlockArgument(local(handle));
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::ValidateOnly,
            operations: vec![
                TransactionOp::CreatePackage {
                    handle: LocalHandle::new(1),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    handle: LocalHandle::new(2),
                    package: local(1),
                    name: "root".into(),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(5),
                    module: local(2),
                    name: "main".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            ExpressionDraft {
                                handle: LocalHandle::new(6),
                                operation: ExpressionKindDraft::ConstI64(0),
                            },
                            ExpressionDraft {
                                handle: LocalHandle::new(7),
                                operation: ExpressionKindDraft::ConstI64(10),
                            },
                            ExpressionDraft {
                                handle: LocalHandle::new(8),
                                operation: ExpressionKindDraft::LtI64 {
                                    lhs: result(6),
                                    rhs: result(7),
                                },
                            },
                            ExpressionDraft {
                                handle: LocalHandle::new(9),
                                operation: ExpressionKindDraft::ForI64 {
                                    start: result(6),
                                    end_exclusive: result(7),
                                    step: 1,
                                    initial: result(6),
                                    carried: SemanticType::I64.into(),
                                    index_handle: LocalHandle::new(10),
                                    carried_handle: LocalHandle::new(11),
                                    body: YieldingBodyDraft {
                                        operations: vec![ExpressionDraft {
                                            handle: LocalHandle::new(12),
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
                                handle: LocalHandle::new(13),
                                operation: ExpressionKindDraft::If {
                                    condition: result(8),
                                    result: SemanticType::I64.into(),
                                    then_body: YieldingBodyDraft {
                                        operations: vec![ExpressionDraft {
                                            handle: LocalHandle::new(14),
                                            operation: ExpressionKindDraft::Call {
                                                function: local(20),
                                                arguments: vec![result(9)],
                                            },
                                        }],
                                        yield_value: result(14),
                                    },
                                    else_body: YieldingBodyDraft {
                                        operations: vec![ExpressionDraft {
                                            handle: LocalHandle::new(15),
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
                    handle: LocalHandle::new(20),
                    module: local(2),
                    name: "later".into(),
                    parameters: vec![FunctionParameterDraft {
                        handle: LocalHandle::new(21),
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
            return_handles: [1, 2, 5, 6, 9, 10, 11, 12, 13, 14, 15, 20, 21]
                .into_iter()
                .map(LocalHandle::new)
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
    fn structured_handles_reject_zero_duplicates_undeclared_and_private_selection_atomically() {
        let id = WorkspaceId::from_bytes([0x7b; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let base_transaction = |expression: ExpressionDraft| Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    handle: LocalHandle::new(1),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    handle: LocalHandle::new(2),
                    package: NodeTarget::Local(LocalHandle::new(1)),
                    name: "root".into(),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(3),
                    module: NodeTarget::Local(LocalHandle::new(2)),
                    name: "main".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![expression],
                        return_value: ValueDraft::OperationResult {
                            operation: NodeTarget::Local(LocalHandle::new(4)),
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
        let zero = base_transaction(ExpressionDraft {
            handle: LocalHandle::new(0),
            operation: ExpressionKindDraft::ConstI64(1),
        });
        let error = workspace
            .prepare_transaction(&unchecked(zero))
            .expect_err("zero");
        assert_eq!(error.code, ErrorCode::InvalidHandle);
        assert_eq!(error.operation_index, Some(2));
        assert_eq!(error.local_handle, Some(LocalHandle::new(0)));
        let duplicate = base_transaction(ExpressionDraft {
            handle: LocalHandle::new(3),
            operation: ExpressionKindDraft::ConstI64(1),
        });
        let error = workspace
            .prepare_transaction(&unchecked(duplicate))
            .expect_err("duplicate");
        assert_eq!(error.code, ErrorCode::DuplicateHandle);
        assert_eq!(error.operation_index, Some(2));
        assert_eq!(error.local_handle, Some(LocalHandle::new(3)));
        let undeclared = base_transaction(ExpressionDraft {
            handle: LocalHandle::new(4),
            operation: ExpressionKindDraft::AddI64 {
                lhs: ValueDraft::OperationResult {
                    operation: NodeTarget::Local(LocalHandle::new(99)),
                    output: 0,
                },
                rhs: ValueDraft::OperationResult {
                    operation: NodeTarget::Local(LocalHandle::new(4)),
                    output: 0,
                },
            },
        });
        let error = workspace
            .prepare_transaction(&unchecked(undeclared))
            .expect_err("undeclared");
        assert_eq!(error.code, ErrorCode::InvalidHandle);
        assert_eq!(error.operation_index, Some(2));
        assert_eq!(error.local_handle, Some(LocalHandle::new(99)));
        let valid = base_transaction(ExpressionDraft {
            handle: LocalHandle::new(4),
            operation: ExpressionKindDraft::ConstI64(1),
        });
        let private = ApplyTransactionRequest {
            transaction: valid,
            response: TransactionResponseSpec {
                return_handles: vec![LocalHandle::new(u32::MAX)],
            },
        };
        assert_eq!(
            workspace
                .prepare_transaction(&private)
                .expect_err("private binding")
                .code,
            ErrorCode::InvalidHandle
        );
        assert_eq!(workspace.head().expect("head").next_serial(), 2);
    }

    #[test]
    fn canonical_allocation_errors_remap_to_public_source_and_explicit_handle() {
        let id = WorkspaceId::from_bytes([0x7a; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let handle = LocalHandle::new(77);
        let edits = vec![
            CanonicalEdit::CreatePackage {
                handle,
                name: "first".into(),
            },
            CanonicalEdit::CreatePackage {
                handle,
                name: "duplicate".into(),
            },
        ];
        let error = allocate_handles(
            workspace.head().expect("head"),
            &edits,
            &[3, 8],
            &BTreeSet::from([handle]),
        )
        .expect_err("duplicate canonical allocation");
        assert_eq!(error.code, ErrorCode::DuplicateHandle);
        assert_eq!(error.operation_index, Some(8));
        assert_eq!(error.local_handle, Some(handle));
    }

    #[test]
    fn insert_expression_rejects_staged_block_and_anchor_with_public_source_atomically() {
        let id = WorkspaceId::from_bytes([0x7e; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let local = |handle| NodeTarget::Local(LocalHandle::new(handle));
        let staged_block = NodeId::new(id, 6).expect("predicted staged block");
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    handle: LocalHandle::new(1),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    handle: LocalHandle::new(2),
                    package: local(1),
                    name: "root".into(),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(3),
                    module: local(2),
                    name: "main".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![ExpressionDraft {
                            handle: LocalHandle::new(4),
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
                        handle: LocalHandle::new(5),
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
                        handle: LocalHandle::new(100),
                        operation: ExpressionKindDraft::ConstI64(1),
                    },
                },
                TransactionOp::InsertExpression {
                    block,
                    before: Some(predicted_anchor),
                    expression: ExpressionDraft {
                        handle: LocalHandle::new(101),
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
        let local = |handle| NodeTarget::Local(LocalHandle::new(handle));
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    handle: LocalHandle::new(1),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    handle: LocalHandle::new(2),
                    package: local(1),
                    name: "root".into(),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(3),
                    module: local(2),
                    name: "bad".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(FunctionBodyDraft {
                        operations: vec![ExpressionDraft {
                            handle: LocalHandle::new(4),
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
                    handle: LocalHandle::new(5),
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
        assert_eq!(error.local_handle, Some(LocalHandle::new(1)));
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
        let error = scan_explicit_handles(&top_level).expect_err("top-level item policy");
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
                handle: LocalHandle::new(40_000),
                operation: ExpressionKindDraft::Call {
                    function: existing,
                    arguments: vec![ValueDraft::FunctionParameter(existing); 3],
                },
            },
        });
        let error = scan_explicit_handles(&mixed).expect_err("mixed item policy");
        assert_eq!(error.code, ErrorCode::PolicyExceeded);
        assert_eq!(
            error.operation_index,
            Some((MAX_STRUCTURED_DRAFT_ITEMS - 2) as u32)
        );

        let mut expression = ExpressionDraft {
            handle: LocalHandle::new(1),
            operation: ExpressionKindDraft::ConstI64(1),
        };
        for depth in 0..=MAX_STRUCTURED_DRAFT_DEPTH {
            let inner_handle = expression.handle;
            let else_handle = LocalHandle::new(10_000 + depth as u32);
            expression = ExpressionDraft {
                handle: LocalHandle::new(20_000 + depth as u32),
                operation: ExpressionKindDraft::If {
                    condition: ValueDraft::OperationResult {
                        operation: existing,
                        output: 0,
                    },
                    result: SemanticType::I64.into(),
                    then_body: YieldingBodyDraft {
                        operations: vec![expression],
                        yield_value: ValueDraft::OperationResult {
                            operation: NodeTarget::Local(inner_handle),
                            output: 0,
                        },
                    },
                    else_body: YieldingBodyDraft {
                        operations: vec![ExpressionDraft {
                            handle: else_handle,
                            operation: ExpressionKindDraft::ConstI64(0),
                        }],
                        yield_value: ValueDraft::OperationResult {
                            operation: NodeTarget::Local(else_handle),
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
            scan_explicit_handles(&too_deep)
                .expect_err("depth policy")
                .code,
            ErrorCode::PolicyExceeded
        );

        let oversized = [TransactionOp::InsertExpression {
            block,
            before: None,
            expression: ExpressionDraft {
                handle: LocalHandle::new(1),
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
            scan_explicit_handles(&oversized)
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
                handle: LocalHandle::new(1),
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
            scan_explicit_handles(&invalid_output)
                .expect_err("output index")
                .code,
            ErrorCode::InvalidOperand
        );
    }

    #[test]
    fn mutual_function_bodies_resolve_local_calls_in_one_transaction() {
        let id = WorkspaceId::from_bytes([0x7c; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let local = |handle| NodeTarget::Local(LocalHandle::new(handle));
        let call_body = |handle, target| FunctionBodyDraft {
            operations: vec![ExpressionDraft {
                handle: LocalHandle::new(handle),
                operation: ExpressionKindDraft::Call {
                    function: local(target),
                    arguments: Vec::new(),
                },
            }],
            return_value: ValueDraft::OperationResult {
                operation: local(handle),
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
                    handle: LocalHandle::new(1),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    handle: LocalHandle::new(2),
                    package: local(1),
                    name: "root".into(),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(3),
                    module: local(2),
                    name: "a".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: Some(call_body(5, 4)),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(4),
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
        for (call_handle, expected_target) in [(5, function_b), (6, function_a)] {
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
                .node(binding(&receipt, call_handle))
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
        let support = LocalHandle::new(100);
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
                        handle: support,
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
                            operation: NodeTarget::Local(support),
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
                    return_handles: vec![support],
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
                        handle: LocalHandle::new(1),
                        name: "p".into(),
                    },
                    TransactionOp::CreateModule {
                        handle: LocalHandle::new(2),
                        package: local_handle(1),
                        name: "m".into(),
                    },
                    TransactionOp::CreateProductType {
                        handle: LocalHandle::new(3),
                        module: local_handle(2),
                        name: "Reading".into(),
                        fields: vec![
                            ProductFieldDraft {
                                handle: LocalHandle::new(4),
                                name: "valid".into(),
                                ty: TypeDraft::Bool,
                            },
                            ProductFieldDraft {
                                handle: LocalHandle::new(5),
                                name: "value".into(),
                                ty: TypeDraft::I64,
                            },
                        ],
                    },
                    TransactionOp::CreateSumType {
                        handle: LocalHandle::new(6),
                        module: local_handle(2),
                        name: "Input".into(),
                        variants: vec![
                            SumVariantDraft {
                                handle: LocalHandle::new(7),
                                name: "missing".into(),
                                payload: None,
                            },
                            SumVariantDraft {
                                handle: LocalHandle::new(8),
                                name: "sample".into(),
                                payload: Some(TypeDraft::Nominal(local_handle(3))),
                            },
                        ],
                    },
                    TransactionOp::CreateFunction {
                        handle: LocalHandle::new(9),
                        module: local_handle(2),
                        name: "pending".into(),
                        parameters: vec![FunctionParameterDraft {
                            handle: LocalHandle::new(10),
                            name: "input".into(),
                            ty: TypeDraft::Nominal(local_handle(6)),
                        }],
                        result: TypeDraft::Nominal(local_handle(3)),
                        body: Some(FunctionBodyDraft {
                            operations: vec![draft_expression(
                                11,
                                ExpressionKindDraft::Hole {
                                    expected: TypeDraft::Nominal(local_handle(3)),
                                },
                            )],
                            return_value: draft_result(11),
                        }),
                    },
                ],
            },
            response: TransactionResponseSpec {
                return_handles: vec![
                    LocalHandle::new(3),
                    LocalHandle::new(4),
                    LocalHandle::new(5),
                    LocalHandle::new(6),
                    LocalHandle::new(7),
                    LocalHandle::new(8),
                    LocalHandle::new(9),
                    LocalHandle::new(11),
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
                    handle: LocalHandle::new(3),
                    module: local_handle(2),
                    name: "A".into(),
                    fields: vec![ProductFieldDraft {
                        handle: LocalHandle::new(4),
                        name: "b".into(),
                        ty: TypeDraft::Nominal(local_handle(5)),
                    }],
                },
                TransactionOp::CreateProductType {
                    handle: LocalHandle::new(5),
                    module: local_handle(2),
                    name: "B".into(),
                    fields: vec![ProductFieldDraft {
                        handle: LocalHandle::new(6),
                        name: "a".into(),
                        ty: TypeDraft::Nominal(local_handle(3)),
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
                handle: LocalHandle::new(3),
                module: local_handle(2),
                name: "D".into(),
                fields: vec![
                    ProductFieldDraft {
                        handle: LocalHandle::new(4),
                        name: "same".into(),
                        ty: TypeDraft::I64,
                    },
                    ProductFieldDraft {
                        handle: LocalHandle::new(5),
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
                handle: LocalHandle::new(3),
                module: local_handle(2),
                name: "Pair".into(),
                fields: vec![
                    ProductFieldDraft {
                        handle: LocalHandle::new(4),
                        name: "left".into(),
                        ty: TypeDraft::I64,
                    },
                    ProductFieldDraft {
                        handle: LocalHandle::new(5),
                        name: "right".into(),
                        ty: TypeDraft::I64,
                    },
                ],
            },
            TransactionOp::CreateSumType {
                handle: LocalHandle::new(6),
                module: local_handle(2),
                name: "Maybe".into(),
                variants: vec![
                    SumVariantDraft {
                        handle: LocalHandle::new(7),
                        name: "none".into(),
                        payload: None,
                    },
                    SumVariantDraft {
                        handle: LocalHandle::new(8),
                        name: "some".into(),
                        payload: Some(TypeDraft::I64),
                    },
                ],
            },
            TransactionOp::CreateFunction {
                handle: LocalHandle::new(9),
                module: local_handle(2),
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
                                product: local_handle(3),
                                fields: vec![
                                    ProductFieldValueDraft {
                                        field: local_handle(5),
                                        value: draft_result(21),
                                    },
                                    ProductFieldValueDraft {
                                        field: local_handle(4),
                                        value: draft_result(20),
                                    },
                                ],
                            },
                        ),
                        draft_expression(
                            23,
                            ExpressionKindDraft::ProjectField {
                                value: draft_result(22),
                                field: local_handle(4),
                            },
                        ),
                        draft_expression(
                            24,
                            ExpressionKindDraft::ConstructVariant {
                                variant: local_handle(8),
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
                                        variant: local_handle(8),
                                        payload_handle: Some(LocalHandle::new(30)),
                                        body: YieldingBodyDraft {
                                            operations: Vec::new(),
                                            yield_value: ValueDraft::BlockArgument(local_handle(
                                                30,
                                            )),
                                        },
                                    },
                                    MatchArmDraft {
                                        variant: local_handle(7),
                                        payload_handle: None,
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
             handle: u32,
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
                    .find(|expression| expression.handle == LocalHandle::new(handle))
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
            arms[0].payload_handle = None;
        });
        assert_eq!(
            workspace
                .prepare_transaction(&invalid)
                .expect_err("missing payload binding")
                .code,
            ErrorCode::InvalidHandle
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
                    handle: LocalHandle::new(3),
                    module: local_handle(2),
                    name: "Pair".into(),
                    fields: vec![
                        ProductFieldDraft {
                            handle: LocalHandle::new(4),
                            name: "left".into(),
                            ty: TypeDraft::I64,
                        },
                        ProductFieldDraft {
                            handle: LocalHandle::new(5),
                            name: "right".into(),
                            ty: TypeDraft::I64,
                        },
                    ],
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(6),
                    module: local_handle(2),
                    name: "make".into(),
                    parameters: Vec::new(),
                    result: TypeDraft::Nominal(local_handle(3)),
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            draft_expression(20, ExpressionKindDraft::ConstI64(1)),
                            draft_expression(21, ExpressionKindDraft::ConstI64(2)),
                            draft_expression(
                                22,
                                ExpressionKindDraft::Hole {
                                    expected: TypeDraft::Nominal(local_handle(3)),
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
                    handle: LocalHandle::new(3),
                    module: local_handle(2),
                    name: "Reading".into(),
                    fields: vec![ProductFieldDraft {
                        handle: LocalHandle::new(4),
                        name: "value".into(),
                        ty: TypeDraft::I64,
                    }],
                },
                TransactionOp::CreateSumType {
                    handle: LocalHandle::new(5),
                    module: local_handle(2),
                    name: "Input".into(),
                    variants: vec![SumVariantDraft {
                        handle: LocalHandle::new(6),
                        name: "sample".into(),
                        payload: Some(TypeDraft::Nominal(local_handle(3))),
                    }],
                },
            ],
        );
        transaction.response.return_handles = vec![LocalHandle::new(3)];
        let prepared = workspace
            .prepare_transaction(&transaction)
            .expect("declarations");
        let reading = prepared
            .receipt
            .returned_bindings
            .iter()
            .find(|(handle, _)| *handle == LocalHandle::new(3))
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
        let local = |value| NodeTarget::Local(LocalHandle::new(value));
        let cases = vec![
            vec![TransactionOp::CreateModule {
                handle: LocalHandle::new(1),
                package: local(99),
                name: "m".into(),
            }],
            vec![
                TransactionOp::CreatePackage {
                    handle: LocalHandle::new(1),
                    name: "p".into(),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(2),
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
                ErrorCode::InvalidHandle
            );
        }

        let wrong_kind = vec![
            TransactionOp::CreatePackage {
                handle: LocalHandle::new(1),
                name: "p".into(),
            },
            TransactionOp::CreateModule {
                handle: LocalHandle::new(2),
                package: local(1),
                name: "m".into(),
            },
            TransactionOp::CreateFunction {
                handle: LocalHandle::new(3),
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
        assert_eq!(error.local_handle, Some(LocalHandle::new(1)));
    }

    #[test]
    fn preallocation_scan_rejects_non_region_if_and_for_targets() {
        let prefix = || {
            vec![
                TransactionOp::CreatePackage {
                    handle: LocalHandle::new(1),
                    name: "p".into(),
                },
                TransactionOp::CreateModule {
                    handle: LocalHandle::new(2),
                    package: local_handle(1),
                    name: "m".into(),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(3),
                    module: local_handle(2),
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
            operation: local_handle(4),
            replacement: OperationDraft::If {
                condition: value,
                result: TypeDraft::I64,
                then_region: local_handle(3),
                else_region: local_handle(3),
            },
        });
        let mut for_target = prefix();
        for_target.push(TransactionOp::ReplaceOperation {
            operation: local_handle(4),
            replacement: OperationDraft::ForI64 {
                start: value,
                end_exclusive: value,
                step: 1,
                initial: value,
                carried: TypeDraft::I64,
                body_region: local_handle(3),
            },
        });

        for operations in [if_target, for_target] {
            let error = scan_explicit_handles(&operations)
                .expect_err("non-region target must reject during the preallocation scan");
            assert_eq!(error.code, ErrorCode::WrongKind);
            assert_eq!(error.operation_index, Some(3));
            assert_eq!(error.local_handle, Some(LocalHandle::new(3)));
        }
    }

    #[test]
    fn later_nominal_declarations_and_permuted_match_arms_expand_identically() {
        let id = WorkspaceId::from_bytes([0xa4; 16]);
        let make = |permuted: bool| {
            let none = MatchArmDraft {
                variant: local_handle(11),
                payload_handle: None,
                body: YieldingBodyDraft {
                    operations: vec![draft_expression(30, ExpressionKindDraft::ConstI64(0))],
                    yield_value: draft_result(30),
                },
            };
            let some = MatchArmDraft {
                variant: local_handle(12),
                payload_handle: Some(LocalHandle::new(31)),
                body: YieldingBodyDraft {
                    operations: Vec::new(),
                    yield_value: ValueDraft::BlockArgument(local_handle(31)),
                },
            };
            let arms = if permuted {
                vec![some.clone(), none.clone()]
            } else {
                vec![none, some]
            };
            ApplyTransactionRequest {
                transaction: Transaction {
                    workspace: id,
                    base_revision: Revision::INITIAL,
                    idempotency_key: None,
                    mode: TransactionMode::ValidateOnly,
                    operations: vec![
                        TransactionOp::CreatePackage {
                            handle: LocalHandle::new(1),
                            name: "p".into(),
                        },
                        TransactionOp::CreateModule {
                            handle: LocalHandle::new(2),
                            package: local_handle(1),
                            name: "m".into(),
                        },
                        TransactionOp::CreateFunction {
                            handle: LocalHandle::new(3),
                            module: local_handle(2),
                            name: "forward".into(),
                            parameters: Vec::new(),
                            result: TypeDraft::I64,
                            body: Some(FunctionBodyDraft {
                                operations: vec![
                                    draft_expression(20, ExpressionKindDraft::ConstI64(7)),
                                    draft_expression(
                                        21,
                                        ExpressionKindDraft::ConstructProduct {
                                            product: local_handle(4),
                                            fields: vec![
                                                ProductFieldValueDraft {
                                                    field: local_handle(6),
                                                    value: draft_result(20),
                                                },
                                                ProductFieldValueDraft {
                                                    field: local_handle(5),
                                                    value: draft_result(20),
                                                },
                                            ],
                                        },
                                    ),
                                    draft_expression(
                                        22,
                                        ExpressionKindDraft::ProjectField {
                                            value: draft_result(21),
                                            field: local_handle(5),
                                        },
                                    ),
                                    draft_expression(
                                        23,
                                        ExpressionKindDraft::ConstructVariant {
                                            variant: local_handle(12),
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
                            handle: LocalHandle::new(4),
                            module: local_handle(2),
                            name: "Pair".into(),
                            fields: vec![
                                ProductFieldDraft {
                                    handle: LocalHandle::new(5),
                                    name: "left".into(),
                                    ty: TypeDraft::I64,
                                },
                                ProductFieldDraft {
                                    handle: LocalHandle::new(6),
                                    name: "right".into(),
                                    ty: TypeDraft::I64,
                                },
                            ],
                        },
                        TransactionOp::CreateSumType {
                            handle: LocalHandle::new(10),
                            module: local_handle(2),
                            name: "Maybe".into(),
                            variants: vec![
                                SumVariantDraft {
                                    handle: LocalHandle::new(11),
                                    name: "none".into(),
                                    payload: None,
                                },
                                SumVariantDraft {
                                    handle: LocalHandle::new(12),
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
    fn product_second_operand_is_copy_and_oversized_constructor_requirements_are_bounded() {
        let id = WorkspaceId::from_bytes([0xa5; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let fields = (0..65)
            .map(|index| ProductFieldDraft {
                handle: LocalHandle::new(100 + index),
                name: format!("field_{index}"),
                ty: TypeDraft::I64,
            })
            .collect::<Vec<_>>();
        let parameters = (0..65)
            .map(|index| FunctionParameterDraft {
                handle: LocalHandle::new(300 + index),
                name: format!("parameter_{index}"),
                ty: TypeDraft::I64,
            })
            .collect::<Vec<_>>();
        let request = structured_semantic_request(
            id,
            vec![
                TransactionOp::CreateProductType {
                    handle: LocalHandle::new(3),
                    module: local_handle(2),
                    name: "Wide".into(),
                    fields,
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(4),
                    module: local_handle(2),
                    name: "wide_call".into(),
                    parameters,
                    result: TypeDraft::Nominal(local_handle(3)),
                    body: None,
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(5),
                    module: local_handle(2),
                    name: "repair".into(),
                    parameters: Vec::new(),
                    result: TypeDraft::Nominal(local_handle(3)),
                    body: Some(FunctionBodyDraft {
                        operations: vec![draft_expression(
                            6,
                            ExpressionKindDraft::Hole {
                                expected: TypeDraft::Nominal(local_handle(3)),
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
