use crate::source::{
    format, DeclarationKind, SourceEdition, SourceNode, SyntaxKind,
    SEMANTIC_SOURCE_FOUNDATION_SCHEMA, SEMANTIC_SOURCE_FOUNDATION_SCHEMA_VERSION,
};

pub(crate) fn declaration_identity(node: &SourceNode) -> Option<(DeclarationKind, String)> {
    let SyntaxKind::Call { name } = &node.kind else {
        return None;
    };
    match name.as_str() {
        "main" => Some((DeclarationKind::Main, "$main".into())),
        "def" => declaration_name(node).map(|name| (DeclarationKind::Function, name)),
        "product" => declaration_name(node).map(|name| (DeclarationKind::Product, name)),
        "trait" => declaration_name(node).map(|name| (DeclarationKind::Trait, name)),
        "impl" => Some((
            DeclarationKind::Implementation,
            node.children
                .iter()
                .map(format::format_node_identity)
                .collect::<Vec<_>>()
                .join("|"),
        )),
        _ => None,
    }
}

fn declaration_name(node: &SourceNode) -> Option<String> {
    let first = node.children.first()?;
    let SyntaxKind::Call { name } = &first.kind else {
        return None;
    };
    if name != "name" || first.children.len() != 1 {
        return None;
    }
    match &first.children[0].kind {
        SyntaxKind::Str { value } => Some(value.clone()),
        SyntaxKind::Symbol { name } | SyntaxKind::Call { name } => Some(name.clone()),
        _ => None,
    }
}

pub(crate) fn declaration_key_bytes(
    edition: SourceEdition,
    logical_path: &str,
    kind: DeclarationKind,
    name: &str,
) -> Vec<u8> {
    let package = format!("edition{}-root", edition.number());
    let mut exact = Vec::new();
    for field in [
        SEMANTIC_SOURCE_FOUNDATION_SCHEMA.as_bytes(),
        SEMANTIC_SOURCE_FOUNDATION_SCHEMA_VERSION
            .to_be_bytes()
            .as_slice(),
        edition.number().to_be_bytes().as_slice(),
        package.as_bytes(),
        logical_path.as_bytes(),
        kind.as_str().as_bytes(),
        name.as_bytes(),
    ] {
        super::append_framed(&mut exact, field);
    }
    exact
}

pub(crate) fn declaration_key_human_identity(
    edition: SourceEdition,
    logical_path: &str,
    kind: DeclarationKind,
    name: &str,
) -> String {
    format!(
        "schema={};version={};edition={};package={};origin={};kind={};name={}",
        super::escape_compact(SEMANTIC_SOURCE_FOUNDATION_SCHEMA),
        SEMANTIC_SOURCE_FOUNDATION_SCHEMA_VERSION,
        edition.number(),
        super::escape_compact(&format!("edition{}-root", edition.number())),
        super::escape_compact(logical_path),
        super::escape_compact(kind.as_str()),
        super::escape_compact(name),
    )
}
