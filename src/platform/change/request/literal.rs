//! Same-kind scalar updates within a live function's existing expression ownership tree.

use super::{AuthoredExpressionOperation, AuthoredLowerer, DeclarationSelector};
use crate::platform::binary64::Binary64;
use crate::platform::change::{CanonicalBaseRead, WitnessBaseRead};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DeclarationPayload, ExpressionOperation, OwnerKey, OwnerRecord, TextValue,
};
use crate::platform::semantic_id::ExpressionId;
use crate::platform::witness::aggregation_children;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredLiteralValue {
    Bool { value: bool },
    I64 { value: i64 },
    F64 { value: Binary64 },
    Text { value: String },
    StaticText { value: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredLiteralUpdate {
    pub expression: ExpressionId,
    pub value: AuthoredLiteralValue,
}

impl AuthoredLiteralValue {
    pub(crate) fn from_authored(operation: &AuthoredExpressionOperation) -> Option<Self> {
        use AuthoredExpressionOperation as A;
        Some(match operation {
            A::Bool { value } => Self::Bool { value: *value },
            A::I64 { value } => Self::I64 { value: *value },
            A::F64 { value } => Self::F64 { value: *value },
            A::Text { value } => Self::Text {
                value: value.clone(),
            },
            A::StaticText { value } => Self::StaticText {
                value: value.clone(),
            },
            _ => return None,
        })
    }

    pub(crate) fn authored(&self) -> AuthoredExpressionOperation {
        use AuthoredExpressionOperation as A;
        match self {
            Self::Bool { value } => A::Bool { value: *value },
            Self::I64 { value } => A::I64 { value: *value },
            Self::F64 { value } => A::F64 { value: *value },
            Self::Text { value } => A::Text {
                value: value.clone(),
            },
            Self::StaticText { value } => A::StaticText {
                value: value.clone(),
            },
        }
    }

    fn accepts(&self, operation: &ExpressionOperation) -> bool {
        matches!(
            (self, operation),
            (Self::Bool { .. }, ExpressionOperation::Bool { .. })
                | (Self::I64 { .. }, ExpressionOperation::I64 { .. })
                | (Self::F64 { .. }, ExpressionOperation::F64 { .. })
                | (Self::Text { .. }, ExpressionOperation::Text { .. })
                | (
                    Self::StaticText { .. },
                    ExpressionOperation::StaticText { .. }
                )
        )
    }

    fn operation(&self) -> ExpressionOperation {
        match self {
            Self::Bool { value } => ExpressionOperation::Bool { value: *value },
            Self::I64 { value } => ExpressionOperation::I64 { value: *value },
            Self::F64 { value } => ExpressionOperation::F64 { value: *value },
            Self::Text { value } => ExpressionOperation::Text {
                value: TextValue::Inline {
                    text: value.clone(),
                },
            },
            Self::StaticText { value } => ExpressionOperation::StaticText {
                value: TextValue::Inline {
                    text: value.clone(),
                },
            },
        }
    }
}

pub(super) fn lower<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    function: &DeclarationSelector,
    literals: &[AuthoredLiteralUpdate],
) -> Result<(), Diagnostic> {
    if literals.is_empty() {
        return Err(error(
            "change_literal_empty",
            "literal update requires a nonempty selection",
        ));
    }
    lowerer.budget.check_canonical_edit_counts(
        u64::try_from(literals.len()).unwrap_or(u64::MAX),
        0,
        0,
        0,
        "function literal selection admission",
    )?;
    let function = lowerer.resolve_declaration(function)?;
    let record = live(lowerer, OwnerKey::Declaration(function))?;
    let OwnerRecord::Declaration(declaration) = record else {
        return Err(error(
            "change_literal_function",
            "literal update requires a function",
        ));
    };
    let DeclarationPayload::Function(function) = declaration.payload else {
        return Err(error(
            "change_literal_function",
            "literal update requires a function",
        ));
    };
    let mut selected = BTreeMap::new();
    for literal in literals {
        if selected
            .insert(literal.expression, &literal.value)
            .is_some()
        {
            return Err(error(
                "change_literal_duplicate",
                "literal update repeats an expression",
            ));
        }
    }
    let mut pending = vec![OwnerKey::Expression(function.body)];
    let mut seen = BTreeSet::new();
    charge(lowerer, 1)?;
    // Read the actual candidate, not a stale ownership index or a request-supplied path.
    // Finish all membership/kind checks before changing any literal in this operation.
    while let Some(owner) = pending.pop() {
        if !seen.insert(owner) {
            return Err(error(
                "change_literal_structure",
                "function body repeats an owned identity",
            ));
        }
        let record = live(lowerer, owner)?;
        match &record {
            OwnerRecord::Expression(expression) => {
                if let Some(value) = selected.remove(&expression.id)
                    && !value.accepts(&expression.operation)
                {
                    return Err(error(
                        "change_literal_kind",
                        "literal update must preserve its scalar kind",
                    ));
                }
            }
            OwnerRecord::Binding(_) => {}
            _ => {
                return Err(error(
                    "change_literal_structure",
                    "function body contains a foreign owner kind",
                ));
            }
        }
        let children = aggregation_children(&record)?;
        charge(lowerer, children.len())?;
        pending.extend(children.into_iter().map(|(_, child)| child));
    }
    if !selected.is_empty() {
        return Err(error(
            "change_literal_foreign",
            "selected literal is not live within this function",
        ));
    }
    for literal in literals {
        let record = lowerer.candidate_mut(OwnerKey::Expression(literal.expression))?;
        let OwnerRecord::Expression(expression) = record else {
            return Err(error(
                "change_literal_kind",
                "selected identity is not an expression",
            ));
        };
        expression.operation = literal.value.operation();
    }
    Ok(())
}

fn live<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
) -> Result<OwnerRecord, Diagnostic> {
    lowerer.require_owner(owner)?;
    let working = lowerer.owners.get(&owner).ok_or_else(|| {
        error(
            "change_literal_owner",
            "literal traversal lost its candidate owner",
        )
    })?;
    if working.original.is_none() {
        return Err(error(
            "change_literal_created",
            "literal update requires existing accepted identities",
        ));
    }
    lowerer.check_budget("function literal canonical reads")?;
    Ok(working.record.clone())
}

fn charge<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    count: usize,
) -> Result<(), Diagnostic> {
    lowerer.work.ownership_steps = lowerer
        .work
        .ownership_steps
        .saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
    lowerer.check_budget("function literal ownership traversal")
}

fn error(code: &'static str, message: &'static str) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}
