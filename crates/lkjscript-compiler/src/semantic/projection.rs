mod calls;
mod hole;
mod matches;
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
        SyntaxKind::EditionMarker => (
            Kind::EditionMarker,
            Some(Value::EditionIdentity { edition: 2 }),
        ),
        SyntaxKind::I64 { .. }
            if matches!(
                parent.map(|node| &node.kind),
                Some(SyntaxKind::EditionMarker)
            ) =>
        {
            (
                Kind::EditionNumber,
                Some(Value::EditionIdentity { edition: 2 }),
            )
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
        SyntaxKind::Symbol { name } => classify_symbol(name, parent, parent_kind, index),
        SyntaxKind::Call { name } if name == "hole" => (Kind::TypedHole, hole::value(node)),
        SyntaxKind::Call { name } => classify_call(node, name, parent),
    }
}

pub(crate) fn trivia(node: &SourceNode) -> Vec<TriviaRecord> {
    trivia::records(node)
}

fn classify_text(value: &str, parent: Option<&SourceNode>) -> (Kind, Option<Value>) {
    match type_nodes::call_name(parent) {
        Some("import") => (
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

fn classify_symbol(
    name: &str,
    parent: Option<&SourceNode>,
    parent_kind: Option<Kind>,
    index: usize,
) -> (Kind, Option<Value>) {
    let kind = match (parent_kind, index) {
        (Some(Kind::TypedHole), 0) => Kind::HoleIdentity,
        (Some(Kind::TypedHole), _) => Kind::HoleGoal,
        (Some(Kind::Parameters), index) if index.is_multiple_of(2) => Kind::ParameterName,
        (Some(Kind::Parameters), _) => type_nodes::classify(name, parent, index),
        (Some(Kind::TypeVariables), _) => Kind::TypeVariable,
        (Some(Kind::TypeEnum), _) => type_nodes::classify(name, parent, index),
        (Some(Kind::Bound), 0) => Kind::TypeVariable,
        (Some(Kind::Bound), _) => Kind::TraitName,
        (Some(Kind::ProductValue), 0) => Kind::ProductName,
        (Some(Kind::ContextVariant), 0) => Kind::VariantName,
        (Some(Kind::ProductValueField | Kind::VariantValueField), 0) => Kind::FieldName,
        (Some(Kind::FieldAccess), 1) | (Some(Kind::WithField), 1) => Kind::FieldName,
        (Some(Kind::Bind), 0) => Kind::BindingName,
        (Some(Kind::Set), 0) => Kind::MutableName,
        (Some(Kind::Quote), _) => Kind::QuotedName,
        (Some(Kind::Move | Kind::Borrow | Kind::BorrowMut), _) => Kind::PlaceName,
        (Some(Kind::ContextTrait), _) => Kind::TraitName,
        (
            Some(
                Kind::Signature
                | Kind::ContextType
                | Kind::ContextFor
                | Kind::EmptyList
                | Kind::None,
            ),
            _,
        ) => type_nodes::classify(name, parent, index),
        _ => Kind::NameReference,
    };
    (
        kind,
        Some(Value::SourceName {
            name: name.to_string(),
        }),
    )
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
        "main" => Kind::Main,
        "def" => Kind::FunctionDeclaration,
        "fn" => Kind::Function,
        "sig" => Kind::Signature,
        "params" => Kind::Parameters,
        "forall" => Kind::TypeVariables,
        "bounds" => Kind::Bounds,
        "bound" => Kind::Bound,
        "product" => Kind::Product,
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
        "Owned" => Kind::TypeOwned,
        "Ref" => Kind::TypeRef,
        "RefMut" => Kind::TypeRefMut,
        "List" => Kind::TypeList,
        "Option" => Kind::TypeOption,
        "Result" => Kind::TypeResult,
        "Product" => Kind::TypeProduct,
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
