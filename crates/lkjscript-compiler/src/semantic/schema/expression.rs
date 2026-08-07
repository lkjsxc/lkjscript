mod matching;
mod measure;

pub(crate) use matching::{MatchExpressionArm, MatchPatternExpression, MatchPatternField};

use serde::{Deserialize, Serialize};

use super::{ClosedBuiltinOperation, TypeExpression};
use crate::source::{SourceNode, SourceSpan};

#[derive(Debug, Serialize)]
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

#[derive(Deserialize)]
#[serde(
    remote = "Expression",
    tag = "kind",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
enum ExpressionDef {
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

impl<'de> Deserialize<'de> for Expression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::stack::grow(|| ExpressionDef::deserialize(deserializer))
    }
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

impl Clone for Expression {
    fn clone(&self) -> Self {
        crate::stack::grow(|| match self {
            Self::Unit {} => Self::Unit {},
            Self::Bool { value } => Self::Bool { value: *value },
            Self::I64 { value } => Self::I64 { value: *value },
            Self::F64 { value } => Self::F64 {
                value: value.clone(),
            },
            Self::String { value } => Self::String {
                value: value.clone(),
            },
            Self::NameReference { name } => Self::NameReference { name: name.clone() },
            Self::TypedHole { identity, goal } => Self::TypedHole {
                identity: identity.clone(),
                goal: goal.clone(),
            },
            Self::EmptyList { element } => Self::EmptyList {
                element: element.clone(),
            },
            Self::None { value_type } => Self::None {
                value_type: value_type.clone(),
            },
            Self::Let { bindings, body } => Self::Let {
                bindings: bindings.clone(),
                body: body.clone(),
            },
            Self::Var {
                name,
                value_type,
                initial,
                body,
            } => Self::Var {
                name: name.clone(),
                value_type: value_type.clone(),
                initial: initial.clone(),
                body: body.clone(),
            },
            Self::Set { name, value } => Self::Set {
                name: name.clone(),
                value: value.clone(),
            },
            Self::If {
                condition,
                then_branch,
                else_branch,
            } => Self::If {
                condition: condition.clone(),
                then_branch: then_branch.clone(),
                else_branch: else_branch.clone(),
            },
            Self::While { condition, body } => Self::While {
                condition: condition.clone(),
                body: body.clone(),
            },
            Self::Loop { result_type, body } => Self::Loop {
                result_type: result_type.clone(),
                body: body.clone(),
            },
            Self::Return { value } => Self::Return {
                value: value.clone(),
            },
            Self::Break { value } => Self::Break {
                value: value.clone(),
            },
            Self::Continue {} => Self::Continue {},
            Self::Trap { value } => Self::Trap {
                value: value.clone(),
            },
            Self::Exit { code } => Self::Exit { code: code.clone() },
            Self::Do { expressions } => Self::Do {
                expressions: expressions.clone(),
            },
            Self::Quote { name } => Self::Quote { name: name.clone() },
            Self::ProductValue { product, fields } => Self::ProductValue {
                product: product.clone(),
                fields: fields.clone(),
            },
            Self::VariantValue {
                value_type,
                variant,
                fields,
            } => Self::VariantValue {
                value_type: value_type.clone(),
                variant: variant.clone(),
                fields: fields.clone(),
            },
            Self::Match { scrutinee, arms } => Self::Match {
                scrutinee: scrutinee.clone(),
                arms: arms.clone(),
            },
            Self::Field { value, field } => Self::Field {
                value: value.clone(),
                field: field.clone(),
            },
            Self::WithField {
                value,
                field,
                replacement,
            } => Self::WithField {
                value: value.clone(),
                field: field.clone(),
                replacement: replacement.clone(),
            },
            Self::Move { name } => Self::Move { name: name.clone() },
            Self::Borrow { name } => Self::Borrow { name: name.clone() },
            Self::BorrowMut { name } => Self::BorrowMut { name: name.clone() },
            Self::BuiltinCall {
                operation,
                arguments,
            } => Self::BuiltinCall {
                operation: *operation,
                arguments: arguments.clone(),
            },
            Self::UserCall { name, arguments } => Self::UserCall {
                name: name.clone(),
                arguments: arguments.clone(),
            },
        })
    }
}

impl Drop for Expression {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        take_expression_children(self, &mut pending);
        while let Some(mut expression) = pending.pop() {
            take_expression_children(&mut expression, &mut pending);
        }
    }
}

fn take_expression_children(expression: &mut Expression, pending: &mut Vec<Expression>) {
    match expression {
        Expression::Let { bindings, body } => {
            pending.extend(
                std::mem::take(bindings)
                    .into_iter()
                    .map(|binding| binding.value),
            );
            take_boxed_expression(body, pending);
        }
        Expression::Var { initial, body, .. } => {
            take_boxed_expression(initial, pending);
            take_boxed_expression(body, pending);
        }
        Expression::Set { value, .. }
        | Expression::Return { value }
        | Expression::Break { value }
        | Expression::Trap { value }
        | Expression::Exit { code: value }
        | Expression::Field { value, .. } => take_boxed_expression(value, pending),
        Expression::If {
            condition,
            then_branch,
            else_branch,
        } => {
            take_boxed_expression(condition, pending);
            take_boxed_expression(then_branch, pending);
            take_boxed_expression(else_branch, pending);
        }
        Expression::While { condition, body } => {
            take_boxed_expression(condition, pending);
            pending.append(body);
        }
        Expression::Loop { body, .. } | Expression::Do { expressions: body } => {
            pending.append(body);
        }
        Expression::ProductValue { fields, .. } | Expression::VariantValue { fields, .. } => {
            pending.extend(std::mem::take(fields).into_iter().map(|field| field.value));
        }
        Expression::Match { scrutinee, arms } => {
            take_boxed_expression(scrutinee, pending);
            pending.extend(std::mem::take(arms).into_iter().map(|arm| arm.body));
        }
        Expression::WithField {
            value, replacement, ..
        } => {
            take_boxed_expression(value, pending);
            take_boxed_expression(replacement, pending);
        }
        Expression::BuiltinCall { arguments, .. } | Expression::UserCall { arguments, .. } => {
            pending.append(arguments);
        }
        _ => {}
    }
}

fn take_boxed_expression(value: &mut Box<Expression>, pending: &mut Vec<Expression>) {
    pending.push(std::mem::replace(value.as_mut(), Expression::Unit {}));
}

#[derive(Default)]
pub(crate) struct ExpressionCounts {
    pub nodes: u64,
    pub depth: u64,
    pub string_bytes: u64,
}
