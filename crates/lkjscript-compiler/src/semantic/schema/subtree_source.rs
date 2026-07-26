use crate::source::{SourceNode, SourceSpan, SyntaxKind};

use super::{
    SemanticNodeKind as Kind, SemanticNodeValue as Value, SemanticSubtreeRecord, TriviaAttachment,
};

impl SemanticSubtreeRecord {
    pub(crate) fn to_source(&self) -> Result<SourceNode, String> {
        let mut children = Vec::with_capacity(self.children.len());
        for child in &self.children {
            if child.node.parent != Some(self.node.index) {
                return Err("semantic subtree parent identity mismatch".into());
            }
            children.push(child.to_source()?);
        }
        let child_ids = self
            .children
            .iter()
            .map(|child| child.node.index)
            .collect::<Vec<_>>();
        if child_ids != self.node.children {
            return Err("semantic subtree child identity mismatch".into());
        }
        let (leading_trivia, before_close_trivia) = trivia(self)?;
        Ok(SourceNode {
            kind: source_kind(self.node.kind, self.node.value.as_ref())?,
            span: SourceSpan::zero(),
            leading_trivia,
            before_close_trivia,
            children,
        })
    }
}

fn source_kind(kind: Kind, value: Option<&Value>) -> Result<SyntaxKind, String> {
    if let Some(name) = marker(kind) {
        return no_value(value).map(|()| SyntaxKind::Call { name: name.into() });
    }
    match (kind, value) {
        (Kind::UnitLiteral, None) => Ok(SyntaxKind::Unit),
        (Kind::BoolLiteral, Some(Value::Bool { value })) => Ok(SyntaxKind::Bool { value: *value }),
        (Kind::I64Literal, Some(Value::I64 { value })) => Ok(SyntaxKind::I64 { value: *value }),
        (Kind::F64Literal, Some(Value::F64 { canonical })) => f64_kind(canonical),
        (Kind::StringLiteral, Some(Value::Text { value }))
        | (Kind::StringLiteral, Some(Value::SourceName { name: value }))
        | (Kind::StringLiteral, Some(Value::ImportPath { path: value })) => Ok(SyntaxKind::Str {
            value: value.clone(),
        }),
        (Kind::TypedHole, Some(Value::TypedHole { .. })) => Ok(call("hole")),
        (Kind::BuiltinCall, Some(Value::BuiltinOperation { operation })) => {
            Ok(call(operation.0.name()))
        }
        (Kind::UserFunctionCall, Some(Value::UserFunction { name })) => {
            if crate::hir::Operation::from_name(name).is_some() {
                return Err("user call uses a built-in identity".into());
            }
            Ok(call(name))
        }
        (kind, Some(Value::SourceName { name })) if symbol_kind(kind) => {
            Ok(SyntaxKind::Symbol { name: name.clone() })
        }
        (kind, None) if dual_type_marker(kind).is_some() => {
            Ok(call(dual_type_marker(kind).unwrap_or_default()))
        }
        _ => Err("semantic node kind/value combination is invalid".into()),
    }
}

fn marker(kind: Kind) -> Option<&'static str> {
    Some(match kind {
        Kind::Import => "import",
        Kind::Main => "main",
        Kind::FunctionDeclaration => "def",
        Kind::Function => "fn",
        Kind::Signature => "sig",
        Kind::Parameters => "params",
        Kind::TypeVariables => "forall",
        Kind::Bounds => "bounds",
        Kind::Bound => "bound",
        Kind::Product => "product",
        Kind::ProductField | Kind::ProductValueField | Kind::FieldAccess => "field",
        Kind::MarkerTrait | Kind::ContextTrait => "trait",
        Kind::TraitImplementation => "impl",
        Kind::ContextName => "name",
        Kind::ContextType => "type",
        Kind::ContextFields => "fields",
        Kind::ContextFor => "for",
        Kind::Let => "let",
        Kind::Bind => "bind",
        Kind::Var => "var",
        Kind::Set => "set",
        Kind::If => "if",
        Kind::While => "while",
        Kind::Do => "do",
        Kind::Quote => "quote",
        Kind::ProductValue => "product-value",
        Kind::WithField => "with-field",
        Kind::EmptyList => "empty-list",
        Kind::None => "none",
        Kind::HoleGoal => "goal",
        Kind::Move => "move",
        Kind::Borrow => "borrow",
        Kind::BorrowMut => "borrow-mut",
        _ => return None,
    })
}

fn symbol_kind(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::NameReference
            | Kind::HoleIdentity
            | Kind::ParameterName
            | Kind::BindingName
            | Kind::MutableName
            | Kind::FieldName
            | Kind::ProductName
            | Kind::TraitName
            | Kind::QuotedName
            | Kind::PlaceName
            | Kind::TypeUnit
            | Kind::TypeBool
            | Kind::TypeI64
            | Kind::TypeF64
            | Kind::TypeString
            | Kind::TypeBuffer
            | Kind::TypeSymbol
            | Kind::TypeHandle
            | Kind::TypeOwned
            | Kind::TypeRef
            | Kind::TypeRefMut
            | Kind::TypeProduct
            | Kind::TypeList
            | Kind::TypeOption
            | Kind::TypeResult
            | Kind::TypeVariable
            | Kind::ReturnArrow
    )
}

fn dual_type_marker(kind: Kind) -> Option<&'static str> {
    Some(match kind {
        Kind::TypeOwned => "Owned",
        Kind::TypeRef => "Ref",
        Kind::TypeRefMut => "RefMut",
        Kind::TypeProduct => "Product",
        Kind::TypeList => "List",
        Kind::TypeOption => "Option",
        Kind::TypeResult => "Result",
        _ => return None,
    })
}

fn f64_kind(canonical: &str) -> Result<SyntaxKind, String> {
    let value = canonical
        .parse::<f64>()
        .map_err(|_| "invalid semantic F64")?;
    if !value.is_finite() || crate::source::format_f64(value) != canonical {
        return Err("noncanonical semantic F64".into());
    }
    Ok(SyntaxKind::F64 { value })
}

fn trivia(record: &SemanticSubtreeRecord) -> Result<(Vec<String>, Vec<String>), String> {
    let mut leading = None;
    let mut before_close = None;
    for item in &record.node.trivia {
        let slot = match item.attachment {
            TriviaAttachment::Leading => &mut leading,
            TriviaAttachment::BeforeClose => &mut before_close,
            TriviaAttachment::SourceUnitTrailing => {
                return Err("source-unit trailing trivia on semantic node".into())
            }
        };
        if slot.replace(item.lines.clone()).is_some() {
            return Err("duplicate semantic trivia attachment".into());
        }
    }
    Ok((
        leading.unwrap_or_default(),
        before_close.unwrap_or_default(),
    ))
}

fn no_value(value: Option<&Value>) -> Result<(), String> {
    value.map_or(Ok(()), |_| {
        Err("structural semantic node has a value".into())
    })
}

fn call(name: &str) -> SyntaxKind {
    SyntaxKind::Call { name: name.into() }
}
