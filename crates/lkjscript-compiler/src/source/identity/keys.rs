use crate::source::{constructor_identity, DeclarationKind, SourceNode, SyntaxKind};

use super::IdentityEncodingError;

pub(crate) fn declaration_identity(node: &SourceNode) -> Option<(DeclarationKind, String)> {
    let SyntaxKind::Call { name } = &node.kind else {
        return None;
    };
    match name.as_str() {
        "main" => Some((DeclarationKind::Main, "$main".into())),
        "def" => declaration_name(node).map(|name| (DeclarationKind::Function, name)),
        "product" => declaration_name(node).map(|name| (DeclarationKind::Product, name)),
        "enum" => declaration_name(node).map(|name| (DeclarationKind::Enum, name)),
        "trait" => declaration_name(node).map(|name| (DeclarationKind::Trait, name)),
        "impl" => Some((
            DeclarationKind::Implementation,
            node.children
                .iter()
                .map(constructor_identity::constructor_identity)
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
    logical_path: &str,
    kind: DeclarationKind,
    name: &str,
) -> Result<Vec<u8>, IdentityEncodingError> {
    let mut exact = Vec::new();
    let contract = lkjscript_contracts::SOURCE_DIGEST.as_bytes();
    for field in [
        contract.as_slice(),
        b"declaration".as_slice(),
        b"root".as_slice(),
        logical_path.as_bytes(),
        kind.as_str().as_bytes(),
        name.as_bytes(),
    ] {
        super::append_framed(&mut exact, field)?;
    }
    Ok(exact)
}

pub(crate) fn product_field_identity(
    parent: [u8; 32],
    name: &str,
    source_order: u64,
) -> Result<[u8; 32], IdentityEncodingError> {
    let mut exact = Vec::new();
    let order = source_order.to_be_bytes();
    for field in [
        b"lkjscript.product-field\0canonical-platform-contract".as_slice(),
        parent.as_slice(),
        name.as_bytes(),
        order.as_slice(),
    ] {
        super::append_framed(&mut exact, field)?;
    }
    Ok(lkjscript_core::sha256(&exact))
}

pub(crate) fn enum_member_identity(parent: [u8; 32], kind: &str, name: &str) -> [u8; 32] {
    let mut exact = [0_u8; 160];
    // Member identity is independent of both text syntax and workspace protocol.
    // This neutral domain is local to semantic constructor identity.
    exact[..32].copy_from_slice(&lkjscript_core::sha256(
        b"lkjscript.identity.enum-member.v1",
    ));
    exact[32..64].copy_from_slice(&lkjscript_core::sha256(b"enum-member"));
    exact[64..96].copy_from_slice(&parent);
    exact[96..128].copy_from_slice(&lkjscript_core::sha256(kind.as_bytes()));
    exact[128..].copy_from_slice(&lkjscript_core::sha256(name.as_bytes()));
    lkjscript_core::sha256(&exact)
}
