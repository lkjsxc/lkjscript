mod trivia;

use crate::semantic::schema::{
    ClosedBuiltinOperation, SemanticNodeKind as Kind, SemanticNodeValue as Value, TriviaRecord,
};
use crate::source::{SourceNode, SyntaxKind};

pub(crate) fn classify(
    node: &SourceNode,
    parent: Option<&SourceNode>,
    parent_kind: Option<Kind>,
    index: usize,
) -> (Kind, Option<Value>) {
    match &node.kind {
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
        SyntaxKind::Call { name } => classify_call(name, parent),
    }
}

pub(crate) fn trivia(node: &SourceNode) -> Vec<TriviaRecord> {
    trivia::records(node)
}

fn classify_text(value: &str, parent: Option<&SourceNode>) -> (Kind, Option<Value>) {
    match call_name(parent) {
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
        (Some(Kind::Parameters), index) if index.is_multiple_of(2) => Kind::ParameterName,
        (Some(Kind::Parameters), _) => type_atom(name, parent, index),
        (Some(Kind::TypeVariables), _) => Kind::TypeVariable,
        (Some(Kind::Bound), 0) => Kind::TypeVariable,
        (Some(Kind::Bound), _) => Kind::TraitName,
        (Some(Kind::ProductValue), 0) => Kind::ProductName,
        (Some(Kind::ProductValueField), 0) => Kind::FieldName,
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
        ) => type_atom(name, parent, index),
        _ => Kind::NameReference,
    };
    (
        kind,
        Some(Value::SourceName {
            name: name.to_string(),
        }),
    )
}

fn type_atom(name: &str, parent: Option<&SourceNode>, index: usize) -> Kind {
    if index > 0
        && parent
            .and_then(|node| node.children.get(index - 1))
            .is_some_and(
                |node| matches!(&node.kind, SyntaxKind::Symbol { name } if name == "Product"),
            )
    {
        return Kind::ProductName;
    }
    match name {
        "Unit" => Kind::TypeUnit,
        "Bool" => Kind::TypeBool,
        "I64" => Kind::TypeI64,
        "F64" => Kind::TypeF64,
        "Str" => Kind::TypeString,
        "Buf" => Kind::TypeBuffer,
        "Symbol" => Kind::TypeSymbol,
        "Handle" => Kind::TypeHandle,
        "Owned" => Kind::TypeOwned,
        "Ref" => Kind::TypeRef,
        "RefMut" => Kind::TypeRefMut,
        "Product" => Kind::TypeProduct,
        "List" => Kind::TypeList,
        "Option" => Kind::TypeOption,
        "Result" => Kind::TypeResult,
        "->" => Kind::ReturnArrow,
        _ => Kind::TypeVariable,
    }
}

fn classify_call(name: &str, parent: Option<&SourceNode>) -> (Kind, Option<Value>) {
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
        "fields" => Kind::ContextFields,
        "trait" if call_name(parent) == Some("impl") => Kind::ContextTrait,
        "trait" => Kind::MarkerTrait,
        "impl" => Kind::TraitImplementation,
        "name" => Kind::ContextName,
        "type" => Kind::ContextType,
        "for" => Kind::ContextFor,
        "field" if call_name(parent) == Some("fields") => Kind::ProductField,
        "field" if call_name(parent) == Some("product-value") => Kind::ProductValueField,
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
        _ => return classify_plain_call(name),
    };
    (kind, None)
}

fn classify_plain_call(name: &str) -> (Kind, Option<Value>) {
    if let Some(operation) = crate::hir::Operation::from_name(name) {
        (
            Kind::BuiltinCall,
            Some(Value::BuiltinOperation {
                operation: ClosedBuiltinOperation(operation),
            }),
        )
    } else {
        (
            Kind::UserFunctionCall,
            Some(Value::UserFunction {
                name: name.to_string(),
            }),
        )
    }
}

fn call_name(node: Option<&SourceNode>) -> Option<&str> {
    match node?.kind {
        SyntaxKind::Call { ref name } => Some(name),
        _ => None,
    }
}
