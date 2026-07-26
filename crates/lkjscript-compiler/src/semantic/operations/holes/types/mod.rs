mod call;

pub(super) use call::parameter_type as call_parameter_type;

use crate::hir::Type;
use crate::source::{SourceNode, SyntaxKind};

pub(crate) fn canonical(ty: &Type) -> String {
    match ty {
        Type::Never => "Never".into(),
        Type::Unit => "Unit".into(),
        Type::Bool => "Bool".into(),
        Type::I64 => "I64".into(),
        Type::F64 => "F64".into(),
        Type::Str => "Str".into(),
        Type::Buf => "Buf".into(),
        Type::Symbol => "Symbol".into(),
        Type::Handle => "Handle".into(),
        Type::Product(name) => format!("Product {name}"),
        Type::Enum {
            name, arguments, ..
        } => {
            let arguments = arguments
                .iter()
                .map(canonical)
                .collect::<Vec<_>>()
                .join(",");
            format!("Enum {name}[{arguments}]")
        }
        Type::Param(name) => name.clone(),
        Type::Owned(inner) => format!("Owned {}", canonical(inner)),
        Type::Ref(inner) => format!("Ref {}", canonical(inner)),
        Type::RefMut(inner) => format!("RefMut {}", canonical(inner)),
        Type::List(inner) => format!("List {}", canonical(inner)),
        Type::Option(inner) => format!("Option {}", canonical(inner)),
        Type::Result(ok, error) => format!("Result {} {}", canonical(ok), canonical(error)),
        Type::Fn { params, ret } => {
            let mut parts: Vec<_> = params.iter().map(canonical).collect();
            parts.push("->".into());
            parts.push(canonical(ret));
            parts.join(" ")
        }
        Type::Forall { vars, body } => format!("forall {} . {}", vars.join(" "), canonical(body)),
    }
}

pub(super) fn parse_type_nodes(nodes: &[SourceNode]) -> Option<(Type, usize)> {
    if let Some(enum_type) = nodes.first().and_then(parse_enum_node) {
        return Some((enum_type, 1));
    }
    let mut atoms = Vec::new();
    let mut boundaries = Vec::new();
    for node in nodes {
        collect_type_atoms(node, &mut atoms)?;
        boundaries.push(atoms.len());
    }
    let (ty, atom_end) = crate::types::parse_one(&atoms, 0).ok()?;
    let used = boundaries
        .iter()
        .position(|boundary| *boundary == atom_end)?
        + 1;
    Some((ty, used))
}

fn parse_enum_node(node: &SourceNode) -> Option<Type> {
    let SyntaxKind::Call { name } = &node.kind else {
        return None;
    };
    if matches!(
        name.as_str(),
        "Owned" | "Ref" | "RefMut" | "List" | "Option" | "Result" | "Product"
    ) || !crate::source::is_source_identifier(name)
        || !name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
    {
        return None;
    }
    let mut arguments = Vec::new();
    let mut index = 0;
    while index < node.children.len() {
        let (argument, used) = parse_type_nodes(&node.children[index..])?;
        arguments.push(argument);
        index = index.checked_add(used)?;
    }
    Some(Type::Enum {
        id: crate::hir::EnumId::UNRESOLVED,
        name: name.clone(),
        arguments,
    })
}

fn collect_type_atoms(node: &SourceNode, output: &mut Vec<String>) -> Option<()> {
    match &node.kind {
        SyntaxKind::Symbol { name } => output.push(name.clone()),
        SyntaxKind::Call { name }
            if matches!(
                name.as_str(),
                "Owned" | "Ref" | "RefMut" | "List" | "Option" | "Result" | "Product"
            ) =>
        {
            output.push(name.clone());
            for child in &node.children {
                collect_type_atoms(child, output)?;
            }
        }
        _ => return None,
    }
    Some(())
}

pub(super) fn signature(node: &SourceNode) -> Option<(Vec<Type>, Type)> {
    if !call_is(node, "sig") {
        return None;
    }
    let arrow = node
        .children
        .iter()
        .position(|child| type_atom(child).as_deref() == Some("->"))?;
    let mut parameters = Vec::new();
    let mut index = 0;
    while index < arrow {
        let (parameter, used) = parse_type_nodes(&node.children[index..arrow])?;
        parameters.push(parameter);
        index = index.checked_add(used)?;
    }
    let (result, used) = parse_type_nodes(&node.children[arrow + 1..])?;
    (arrow + 1 + used == node.children.len()).then_some((parameters, result))
}

pub(super) fn type_form(node: &SourceNode) -> Option<Type> {
    if !call_is(node, "type") {
        return None;
    }
    let (ty, used) = parse_type_nodes(&node.children)?;
    (used == node.children.len()).then_some(ty)
}

pub(super) fn type_atom(node: &SourceNode) -> Option<String> {
    match &node.kind {
        SyntaxKind::Symbol { name } => Some(name.clone()),
        _ => None,
    }
}

pub(super) fn call_is(node: &SourceNode, expected: &str) -> bool {
    matches!(&node.kind, SyntaxKind::Call { name } if name == expected)
}

pub(super) fn source_name(node: &SourceNode) -> Option<&str> {
    match &node.kind {
        SyntaxKind::Symbol { name } | SyntaxKind::Str { value: name } => Some(name),
        SyntaxKind::Call { name } if name == "name" => node.children.first().and_then(source_name),
        _ => None,
    }
}

pub(super) fn ownership(ty: &Type) -> crate::semantic::schema::OwnershipAccess {
    use crate::semantic::schema::OwnershipAccess;
    match ty {
        Type::Never => OwnershipAccess::Unavailable,
        Type::Owned(_) => OwnershipAccess::Move,
        Type::Ref(_) => OwnershipAccess::SharedBorrow,
        Type::RefMut(_) => OwnershipAccess::MutableBorrow,
        Type::Buf | Type::Handle => OwnershipAccess::Unavailable,
        _ => OwnershipAccess::Copy,
    }
}
