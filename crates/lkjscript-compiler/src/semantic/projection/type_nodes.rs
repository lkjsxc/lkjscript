use crate::semantic::schema::SemanticNodeKind as Kind;
use crate::source::{SourceNode, SyntaxKind};

pub(super) fn classify(name: &str, parent: Option<&SourceNode>, index: usize) -> Kind {
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
