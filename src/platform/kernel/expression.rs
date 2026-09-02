//! Stable-ID Graph 7 expression records.

use super::contract::{GRAPH_CONTRACT_VERSION, MAXIMUM_CHILDREN, MAXIMUM_INLINE_TEXT_BYTES};
use super::digest::{BlobObjectDigest, TypeObjectDigest};
use super::name::Name;
use super::reference::{
    CaseReference, DeclarationReference, FieldReference, OperationReference, RequirementReference,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{BindingId, ExpressionId, ParameterId};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpressionRecord {
    pub contract_version: u16,
    pub id: ExpressionId,
    pub operation: ExpressionOperation,
}

impl ExpressionRecord {
    pub fn new(id: ExpressionId, operation: ExpressionOperation) -> Result<Self, Diagnostic> {
        let record = Self {
            contract_version: GRAPH_CONTRACT_VERSION,
            id,
            operation,
        };
        record.validate_local()?;
        Ok(record)
    }

    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
        if self.contract_version != GRAPH_CONTRACT_VERSION {
            return Err(expression_error(
                "kernel_expression_contract",
                format!(
                    "expression contract {} is not Graph Contract {GRAPH_CONTRACT_VERSION}",
                    self.contract_version
                ),
            ));
        }
        validate_operation(&self.operation)
    }

    pub fn children(&self) -> Vec<ExpressionChild> {
        expression_children(&self.operation)
    }

    pub fn type_roots(&self) -> Vec<TypeObjectDigest> {
        match &self.operation {
            ExpressionOperation::Call { type_arguments, .. }
            | ExpressionOperation::FunctionValue { type_arguments, .. } => type_arguments.clone(),
            ExpressionOperation::List { item_type, .. } => vec![*item_type],
            ExpressionOperation::Map {
                key_type,
                value_type,
                ..
            } => vec![*key_type, *value_type],
            _ => Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExpressionOperation {
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
        function: DeclarationReference,
        type_arguments: Vec<TypeObjectDigest>,
        arguments: Vec<ExpressionId>,
    },
    FunctionValue {
        function: DeclarationReference,
        type_arguments: Vec<TypeObjectDigest>,
    },
    Invoke {
        callee: ExpressionId,
        arguments: Vec<ExpressionId>,
    },
    Record {
        nominal_type: Option<DeclarationReference>,
        fields: Vec<RecordExpressionField>,
    },
    Variant {
        case: CaseReference,
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
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "storage", rename_all = "snake_case", deny_unknown_fields)]
pub enum TextValue {
    Inline {
        text: String,
    },
    Blob {
        digest: BlobObjectDigest,
        bytes: u64,
    },
}

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum LocalValueReference {
    FunctionParameter(ParameterId),
    OperationParameter(ParameterId),
    LexicalBinding(BindingId),
    MatchPayload(BindingId),
    TransactionBinding(BindingId),
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordExpressionField {
    pub selector: FieldSelector,
    pub value: ExpressionId,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum FieldSelector {
    Nominal(FieldReference),
    Structural(Name),
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MapExpressionEntry {
    pub key: ExpressionId,
    pub value: ExpressionId,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MatchExpressionArm {
    pub case: CaseReference,
    pub payload_binding: Option<BindingId>,
    pub body: ExpressionId,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ExpressionChild {
    pub expression: ExpressionId,
    pub role: ExpressionChildRole,
    pub ordinal: u32,
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExpressionChildRole {
    Condition,
    TrueBranch,
    FalseBranch,
    LetBody,
    SequenceItem,
    CallArgument,
    InvokeCallee,
    InvokeArgument,
    RecordField,
    VariantPayload,
    FieldValue,
    ListItem,
    MapKey,
    MapValue,
    MatchValue,
    MatchArmBody,
    CapabilityArgument,
    TransactionBody,
}

fn validate_operation(operation: &ExpressionOperation) -> Result<(), Diagnostic> {
    match operation {
        ExpressionOperation::Text { value } | ExpressionOperation::StaticText { value } => {
            validate_text(value)?;
        }
        ExpressionOperation::Let { bindings, .. } => {
            require_count("let bindings", bindings.len(), false)?;
            require_unique("let binding", bindings.iter().copied())?;
        }
        ExpressionOperation::Sequence { items } => {
            require_count("sequence items", items.len(), false)?;
        }
        ExpressionOperation::Call {
            type_arguments,
            arguments,
            ..
        } => {
            require_count("call type arguments", type_arguments.len(), true)?;
            require_count("call arguments", arguments.len(), true)?;
        }
        ExpressionOperation::FunctionValue { type_arguments, .. } => {
            require_count("function type arguments", type_arguments.len(), true)?;
        }
        ExpressionOperation::Invoke { arguments, .. } => {
            require_count("invoke arguments", arguments.len(), true)?;
        }
        ExpressionOperation::Record {
            nominal_type,
            fields,
        } => {
            require_count("record fields", fields.len(), false)?;
            let nominal = nominal_type.is_some();
            if fields
                .iter()
                .any(|field| nominal != matches!(field.selector, FieldSelector::Nominal(_)))
            {
                return Err(expression_error(
                    "kernel_expression_record_selector",
                    "record expression mixes nominal and structural field selectors",
                ));
            }
            let selectors = fields
                .iter()
                .map(|field| field.selector.clone())
                .collect::<BTreeSet<_>>();
            if selectors.len() != fields.len() {
                return Err(expression_error(
                    "kernel_expression_record_duplicate",
                    "record expression contains a duplicate field selector",
                ));
            }
            if fields
                .windows(2)
                .any(|pair| pair[0].selector >= pair[1].selector)
            {
                return Err(expression_error(
                    "kernel_expression_record_order",
                    "record fields must follow strict canonical selector order",
                ));
            }
        }
        ExpressionOperation::List { items, .. } => {
            require_count("list items", items.len(), true)?;
        }
        ExpressionOperation::Map { entries, .. } => {
            require_count("map entries", entries.len(), true)?;
        }
        ExpressionOperation::Match { arms, .. } => {
            require_count("match arms", arms.len(), false)?;
            if arms.windows(2).any(|pair| pair[0].case >= pair[1].case) {
                return Err(expression_error(
                    "kernel_expression_match_order",
                    "match arms must be strictly ordered by exact case reference",
                ));
            }
        }
        ExpressionOperation::CapabilityCall { arguments, .. } => {
            require_count("capability arguments", arguments.len(), true)?;
        }
        ExpressionOperation::Unit {}
        | ExpressionOperation::Bool { .. }
        | ExpressionOperation::I64 { .. }
        | ExpressionOperation::Local { .. }
        | ExpressionOperation::Constant { .. }
        | ExpressionOperation::If { .. }
        | ExpressionOperation::Variant { .. }
        | ExpressionOperation::Field { .. }
        | ExpressionOperation::Transaction { .. } => {}
    }
    Ok(())
}

fn validate_text(value: &TextValue) -> Result<(), Diagnostic> {
    match value {
        TextValue::Inline { text } if text.len() <= MAXIMUM_INLINE_TEXT_BYTES => Ok(()),
        TextValue::Inline { text } => Err(expression_error(
            "kernel_expression_inline_text_limit",
            format!(
                "inline text has {} bytes; values above {MAXIMUM_INLINE_TEXT_BYTES} require a blob",
                text.len()
            ),
        )),
        TextValue::Blob { bytes: 0, .. } => Err(expression_error(
            "kernel_expression_blob_length",
            "blob-backed text must contain at least one byte",
        )),
        TextValue::Blob { .. } => Ok(()),
    }
}

fn require_count(label: &str, count: usize, allow_zero: bool) -> Result<(), Diagnostic> {
    if (!allow_zero && count == 0) || count > MAXIMUM_CHILDREN {
        return Err(expression_error(
            "kernel_expression_child_count",
            format!("{label} count {count} is outside the Graph 7 bound"),
        ));
    }
    Ok(())
}

fn require_unique<T: Ord + Copy>(
    label: &str,
    values: impl Iterator<Item = T>,
) -> Result<(), Diagnostic> {
    let mut observed = BTreeSet::new();
    for value in values {
        if !observed.insert(value) {
            return Err(expression_error(
                "kernel_expression_duplicate_child",
                format!("duplicate {label}"),
            ));
        }
    }
    Ok(())
}

fn expression_children(operation: &ExpressionOperation) -> Vec<ExpressionChild> {
    let mut children = Vec::new();
    match operation {
        ExpressionOperation::If {
            condition,
            when_true,
            when_false,
        } => {
            push_child(&mut children, *condition, ExpressionChildRole::Condition, 0);
            push_child(
                &mut children,
                *when_true,
                ExpressionChildRole::TrueBranch,
                0,
            );
            push_child(
                &mut children,
                *when_false,
                ExpressionChildRole::FalseBranch,
                0,
            );
        }
        ExpressionOperation::Let { body, .. } => {
            push_child(&mut children, *body, ExpressionChildRole::LetBody, 0);
        }
        ExpressionOperation::Sequence { items } => {
            push_many(&mut children, items, ExpressionChildRole::SequenceItem)
        }
        ExpressionOperation::Call { arguments, .. } => {
            push_many(&mut children, arguments, ExpressionChildRole::CallArgument)
        }
        ExpressionOperation::Invoke { callee, arguments } => {
            push_child(&mut children, *callee, ExpressionChildRole::InvokeCallee, 0);
            push_many(
                &mut children,
                arguments,
                ExpressionChildRole::InvokeArgument,
            );
        }
        ExpressionOperation::Record { fields, .. } => {
            let values = fields.iter().map(|field| field.value).collect::<Vec<_>>();
            push_many(&mut children, &values, ExpressionChildRole::RecordField);
        }
        ExpressionOperation::Variant { payload, .. } => {
            if let Some(payload) = payload {
                push_child(
                    &mut children,
                    *payload,
                    ExpressionChildRole::VariantPayload,
                    0,
                );
            }
        }
        ExpressionOperation::Field { value, .. } => {
            push_child(&mut children, *value, ExpressionChildRole::FieldValue, 0);
        }
        ExpressionOperation::List { items, .. } => {
            push_many(&mut children, items, ExpressionChildRole::ListItem);
        }
        ExpressionOperation::Map { entries, .. } => {
            for (ordinal, entry) in entries.iter().enumerate() {
                let ordinal = u32::try_from(ordinal).unwrap_or(u32::MAX);
                push_child(
                    &mut children,
                    entry.key,
                    ExpressionChildRole::MapKey,
                    ordinal,
                );
                push_child(
                    &mut children,
                    entry.value,
                    ExpressionChildRole::MapValue,
                    ordinal,
                );
            }
        }
        ExpressionOperation::Match { value, arms } => {
            push_child(&mut children, *value, ExpressionChildRole::MatchValue, 0);
            let bodies = arms.iter().map(|arm| arm.body).collect::<Vec<_>>();
            push_many(&mut children, &bodies, ExpressionChildRole::MatchArmBody);
        }
        ExpressionOperation::CapabilityCall { arguments, .. } => push_many(
            &mut children,
            arguments,
            ExpressionChildRole::CapabilityArgument,
        ),
        ExpressionOperation::Transaction { body, .. } => {
            push_child(
                &mut children,
                *body,
                ExpressionChildRole::TransactionBody,
                0,
            );
        }
        ExpressionOperation::Unit {}
        | ExpressionOperation::Bool { .. }
        | ExpressionOperation::I64 { .. }
        | ExpressionOperation::Text { .. }
        | ExpressionOperation::StaticText { .. }
        | ExpressionOperation::Local { .. }
        | ExpressionOperation::Constant { .. }
        | ExpressionOperation::FunctionValue { .. } => {}
    }
    children
}

fn push_child(
    children: &mut Vec<ExpressionChild>,
    expression: ExpressionId,
    role: ExpressionChildRole,
    ordinal: u32,
) {
    children.push(ExpressionChild {
        expression,
        role,
        ordinal,
    });
}

fn push_many(
    children: &mut Vec<ExpressionChild>,
    values: &[ExpressionId],
    role: ExpressionChildRole,
) {
    children.extend(
        values
            .iter()
            .enumerate()
            .map(|(ordinal, expression)| ExpressionChild {
                expression: *expression,
                role,
                ordinal: u32::try_from(ordinal).unwrap_or(u32::MAX),
            }),
    );
}

fn expression_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}
