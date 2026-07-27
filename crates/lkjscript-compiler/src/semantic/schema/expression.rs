mod matching;
mod measure;

pub(crate) use matching::{MatchExpressionArm, MatchPatternExpression, MatchPatternField};

use serde::{Deserialize, Serialize};

use super::{ClosedBuiltinOperation, TypeExpression};
use crate::source::{SourceNode, SourceSpan};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum Expression {
    Unit {},
    Bool {
        value: bool,
    },
    I64 {
        value: i64,
    },
    F64 {
        value: String,
    },
    String {
        value: String,
    },
    NameReference {
        name: String,
    },
    TypedHole {
        identity: String,
        goal: Option<String>,
    },
    EmptyList {
        element: TypeExpression,
    },
    None {
        value_type: TypeExpression,
    },
    Let {
        bindings: Vec<ExpressionBinding>,
        body: Box<Expression>,
    },
    Var {
        name: String,
        value_type: TypeExpression,
        initial: Box<Expression>,
        body: Box<Expression>,
    },
    Set {
        name: String,
        value: Box<Expression>,
    },
    If {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
    },
    While {
        condition: Box<Expression>,
        body: Vec<Expression>,
    },
    Loop {
        result_type: TypeExpression,
        body: Vec<Expression>,
    },
    Return {
        value: Box<Expression>,
    },
    Break {
        value: Box<Expression>,
    },
    Continue {},
    Trap {
        value: Box<Expression>,
    },
    Exit {
        code: Box<Expression>,
    },
    Do {
        expressions: Vec<Expression>,
    },
    Quote {
        name: String,
    },
    ProductValue {
        product: String,
        fields: Vec<ExpressionField>,
    },
    VariantValue {
        value_type: TypeExpression,
        variant: String,
        fields: Vec<ExpressionField>,
    },
    Match {
        scrutinee: Box<Expression>,
        arms: Vec<MatchExpressionArm>,
    },
    Field {
        value: Box<Expression>,
        field: String,
    },
    WithField {
        value: Box<Expression>,
        field: String,
        replacement: Box<Expression>,
    },
    Move {
        name: String,
    },
    Borrow {
        name: String,
    },
    BorrowMut {
        name: String,
    },
    BuiltinCall {
        operation: ClosedBuiltinOperation,
        arguments: Vec<Expression>,
    },
    UserCall {
        name: String,
        arguments: Vec<Expression>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExpressionBinding {
    pub name: String,
    pub value: Expression,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExpressionField {
    pub name: String,
    pub value: Expression,
}

impl Expression {
    pub(crate) fn to_source(&self, span: SourceSpan) -> Result<SourceNode, String> {
        super::expression_source::to_source(self, span)
    }
}

#[derive(Default)]
pub(crate) struct ExpressionCounts {
    pub nodes: u64,
    pub depth: u32,
    pub string_bytes: u64,
}
