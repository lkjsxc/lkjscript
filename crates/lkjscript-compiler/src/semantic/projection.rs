mod calls;
mod hole;
mod matches;
mod symbols;
mod trivia;
mod type_nodes;

use crate::semantic::schema::{SemanticNodeKind as Kind, SemanticNodeValue as Value, TriviaRecord};
use crate::source::{SourceNode, SyntaxKind};

pub(crate) fn classify(
    node: &SourceNode,
    parent: Option<&SourceNode>,
    parent_kind: Option<Kind>,
    index: usize,
) -> (Kind, Option<Value>) {
    match &node.kind {
        SyntaxKind::Unit
            if matches!(
                parent_kind,
                Some(
                    Kind::SignatureInputs
                        | Kind::SignatureOutput
                        | Kind::ContextType
                        | Kind::ContextFor
                )
            ) =>
        {
            (Kind::TypeUnit, None)
        }
        SyntaxKind::Unit => (Kind::UnitLiteral, None),
        SyntaxKind::Bool { value } => (Kind::BoolLiteral, Some(Value::Bool { value: *value })),
        SyntaxKind::I64 { value } => (Kind::I64Literal, Some(Value::I64 { value: *value })),
        SyntaxKind::F64 { value } => (
            Kind::F64Literal,
            Some(Value::F64 {
                canonical: crate::source::format_f64(*value),
            }),
        ),
        SyntaxKind::Str { value } => classify_text(value, parent),
        SyntaxKind::Bytes { value } => (
            Kind::BytesLiteral,
            Some(Value::Bytes {
                hexadecimal: value.iter().map(|byte| format!("{byte:02x}")).collect(),
            }),
        ),
        SyntaxKind::Symbol { name } => symbols::classify(name, parent, parent_kind, index),
        SyntaxKind::Call { name } if name == "hole" => (Kind::TypedHole, hole::value(node)),
        SyntaxKind::Call { name } => classify_call(node, name, parent),
    }
}

pub(crate) fn trivia(node: &SourceNode) -> Vec<TriviaRecord> {
    trivia::records(node)
}

fn classify_text(value: &str, parent: Option<&SourceNode>) -> (Kind, Option<Value>) {
    match type_nodes::call_name(parent) {
        Some("module") => (
            Kind::StringLiteral,
            Some(Value::ImportPath {
                path: value.to_string(),
            }),
        ),
        Some("name") => (
            Kind::StringLiteral,
            Some(Value::SourceName {
                name: value.to_string(),
            }),
        ),
        _ => (
            Kind::StringLiteral,
            Some(Value::Text {
                value: value.to_string(),
            }),
        ),
    }
}

fn classify_call(
    node: &SourceNode,
    name: &str,
    parent: Option<&SourceNode>,
) -> (Kind, Option<Value>) {
    if let Some(kind) = matches::call(node, name) {
        return (kind, None);
    }
    let kind = match name {
        "import" => Kind::Import,
        "module" => Kind::ImportModule,
        "declarations" => Kind::ImportDeclarations,
        "main" => Kind::Main,
        "def" => Kind::FunctionDeclaration,
        "fn" => Kind::Function,
        "sig" => Kind::Signature,
        "inputs" => Kind::SignatureInputs,
        "output" => Kind::SignatureOutput,
        "params" => Kind::Parameters,
        "forall" => Kind::TypeVariables,
        "bounds" => Kind::Bounds,
        "bound" => Kind::Bound,
        "product" if parent.is_none() => Kind::Product,
        "product" => Kind::TypeProduct,
        "enum" => Kind::EnumDeclaration,
        "variants" => Kind::ContextVariants,
        "variant" => Kind::EnumVariant,
        "fields" => Kind::ContextFields,
        "trait" if type_nodes::call_name(parent) == Some("impl") => Kind::ContextTrait,
        "trait" => Kind::MarkerTrait,
        "impl" => Kind::TraitImplementation,
        "name" => Kind::ContextName,
        "type" => Kind::ContextType,
        "for" => Kind::ContextFor,
        "field" if type_nodes::call_name(parent) == Some("fields") => Kind::ProductField,
        "field" if type_nodes::call_name(parent) == Some("product-value") => {
            Kind::ProductValueField
        }
        "field" => Kind::FieldAccess,
        "let" => Kind::Let,
        "bind" => Kind::Bind,
        "var" => Kind::Var,
        "set" => Kind::Set,
        "if" => Kind::If,
        "while" => Kind::While,
        "loop" => Kind::Loop,
        "return" => Kind::Return,
        "break" => Kind::Break,
        "continue" => Kind::Continue,
        "trap" => Kind::Trap,
        "exit" => Kind::Exit,
        "do" => Kind::Do,
        "quote" => Kind::Quote,
        "product-value" => Kind::ProductValue,
        "with-field" => Kind::WithField,
        "empty-list" => Kind::EmptyList,
        "none" => Kind::None,
        "goal" => Kind::HoleGoal,
        "move" => Kind::Move,
        "borrow" => Kind::Borrow,
        "borrow-mut" => Kind::BorrowMut,
        "list" => Kind::TypeList,
        "option" => Kind::TypeOption,
        "result" => Kind::TypeResult,
        _ if type_nodes::enum_context(name, parent) => {
            return (
                Kind::TypeEnum,
                Some(Value::SourceName {
                    name: name.to_string(),
                }),
            )
        }
        _ => return calls::plain(name),
    };
    (kind, None)
}
