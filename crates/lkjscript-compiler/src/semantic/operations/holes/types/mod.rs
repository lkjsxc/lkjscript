mod call;

pub(super) use call::parameter_type as call_parameter_type;

use crate::hir::Type;
use crate::source::{SourceNode, SyntaxKind};

pub(crate) fn canonical(ty: &Type) -> String {
    match ty {
        Type::Never => "never".into(),
        Type::Unit => "unit".into(),
        Type::Bool => "bool".into(),
        Type::I64 => "i64".into(),
        Type::F64 => "f64".into(),
        Type::Str => "string".into(),
        Type::Bytes => "bytes".into(),
        Type::ByteVector => "byte-vector".into(),
        Type::ByteSlice => "byte-slice".into(),
        Type::ByteSliceMut => "byte-slice-mut".into(),
        Type::Path => "path".into(),
        Type::Capability(kind) => format!("capability {}", kind.as_str()),
        Type::Symbol => "symbol".into(),
        Type::Resource(kind) => kind.as_str().into(),
        Type::Product(name) => format!("product {name}"),
        Type::Enum { id, arguments, .. }
            if id.bytes() == lkjscript_core::OPTION_ID && arguments.len() == 1 =>
        {
            format!("option {}", canonical(&arguments[0]))
        }
        Type::Enum { id, arguments, .. }
            if id.bytes() == lkjscript_core::RESULT_ID && arguments.len() == 2 =>
        {
            format!(
                "result {} {}",
                canonical(&arguments[0]),
                canonical(&arguments[1])
            )
        }
        Type::Enum {
            name, arguments, ..
        } => {
            let arguments = arguments
                .iter()
                .map(canonical)
                .collect::<Vec<_>>()
                .join(" ");
            format!("enum {name} {arguments}")
        }
        Type::Param(name) => name.clone(),
        Type::List(inner) => format!("list {}", canonical(inner)),
        Type::Fn { params, ret } => {
            let parameters = params.iter().map(canonical).collect::<Vec<_>>().join(" ");
            format!("fn inputs {parameters} output {}", canonical(ret))
        }
        Type::Forall { vars, body } => {
            format!("forall {} body {}", vars.join(" "), canonical(body))
        }
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
        "owned" | "ref" | "ref-mut" | "list" | "option" | "result" | "product" | "capability"
    ) || !crate::source::is_source_identifier(name)
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
        SyntaxKind::Unit => output.push("unit".into()),
        SyntaxKind::Symbol { name } => output.push(name.clone()),
        SyntaxKind::Call { name }
            if matches!(
                name.as_str(),
                "owned"
                    | "ref"
                    | "ref-mut"
                    | "list"
                    | "option"
                    | "result"
                    | "product"
                    | "capability"
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
    let [inputs, output] = node.children.as_slice() else {
        return None;
    };
    if !call_is(inputs, "inputs") || !call_is(output, "output") {
        return None;
    }
    let mut parameters = Vec::new();
    let mut index = 0;
    while index < inputs.children.len() {
        let (parameter, used) = parse_type_nodes(&inputs.children[index..])?;
        parameters.push(parameter);
        index = index.checked_add(used)?;
    }
    let (result, used) = parse_type_nodes(&output.children)?;
    (used == output.children.len()).then_some((parameters, result))
}

pub(super) fn type_form(node: &SourceNode) -> Option<Type> {
    if !call_is(node, "type") {
        return None;
    }
    let (ty, used) = parse_type_nodes(&node.children)?;
    (used == node.children.len()).then_some(ty)
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
        Type::ByteVector => OwnershipAccess::Move,
        Type::ByteSlice => OwnershipAccess::SharedBorrow,
        Type::ByteSliceMut => OwnershipAccess::MutableBorrow,
        Type::Resource(_) => OwnershipAccess::Unavailable,
        _ => OwnershipAccess::Copy,
    }
}
