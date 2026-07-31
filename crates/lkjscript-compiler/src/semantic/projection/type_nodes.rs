use crate::semantic::schema::SemanticNodeKind as Kind;
use crate::source::{SourceNode, SyntaxKind};

pub(super) fn call_name(node: Option<&SourceNode>) -> Option<&str> {
    match node?.kind {
        SyntaxKind::Call { ref name } => Some(name),
        _ => None,
    }
}

pub(super) fn enum_context(name: &str, parent: Option<&SourceNode>) -> bool {
    let valid = crate::source::is_source_identifier(name);
    let parent_name = match parent.map(|node| &node.kind) {
        Some(SyntaxKind::Call { name }) => Some(name.as_str()),
        _ => None,
    };
    valid
        && parent_name.is_some_and(|name| {
            matches!(
                name,
                "inputs" | "output" | "params" | "type" | "for" | "list" | "option" | "result"
            )
        })
}

pub(super) fn classify(name: &str, parent: Option<&SourceNode>, index: usize) -> Kind {
    if index > 0
        && parent
            .and_then(|node| node.children.get(index - 1))
            .is_some_and(
                |node| matches!(&node.kind, SyntaxKind::Symbol { name } if name == "product"),
            )
    {
        return Kind::ProductName;
    }
    match name {
        "never" => Kind::TypeNever,
        "unit" => Kind::TypeUnit,
        "bool" => Kind::TypeBool,
        "i64" => Kind::TypeI64,
        "f64" => Kind::TypeF64,
        "string" => Kind::TypeString,
        "bytes" => Kind::TypeBytes,
        "byte-vector" => Kind::TypeByteVector,
        "byte-slice" => Kind::TypeByteSlice,
        "byte-slice-mut" => Kind::TypeByteSliceMut,
        "path" => Kind::TypePath,
        "capability" => Kind::TypeCapability,
        "symbol" => Kind::TypeSymbol,
        resource if lkjscript_core::ResourceKind::parse(resource).is_some() => Kind::TypeResource,
        "product" => Kind::TypeProduct,
        "list" => Kind::TypeList,
        "option" => Kind::TypeOption,
        "result" => Kind::TypeResult,
        _ => Kind::TypeVariable,
    }
}
