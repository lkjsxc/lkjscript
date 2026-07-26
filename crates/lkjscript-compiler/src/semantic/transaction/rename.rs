use crate::semantic::codec::error;
use crate::semantic::schema::{ProtocolError, ProtocolErrorCode};
use crate::semantic::transaction::ResolvedOperation;
use crate::source::{DeclarationKind, SourceFile, SyntaxKind, ValidatedSourceTree};

pub(crate) fn resolve(
    tree: &ValidatedSourceTree,
    key: &str,
    fingerprint: &str,
    new_name: &str,
) -> Result<ResolvedOperation, ProtocolError> {
    let declaration = crate::semantic::operations::entity::find(tree, key)?;
    if matches!(
        declaration.kind(),
        DeclarationKind::Main | DeclarationKind::Enum | DeclarationKind::Implementation
    ) {
        return Err(error(
            ProtocolErrorCode::InvalidOperation,
            "this declaration kind cannot be renamed",
        ));
    }
    if !crate::source::is_source_identifier(new_name) {
        return Err(error(
            ProtocolErrorCode::InvalidOperation,
            format!("invalid source identifier {new_name:?}"),
        ));
    }
    let nodes = crate::semantic::tree::source_nodes(tree);
    let node = nodes
        .get(declaration.node().index() as usize)
        .ok_or_else(|| {
            error(
                ProtocolErrorCode::UnknownNode,
                "declaration source node is unavailable",
            )
        })?;
    if crate::semantic::tree::fingerprint(node) != fingerprint {
        return Err(error(
            ProtocolErrorCode::PreconditionFailed,
            "entity fingerprint is stale",
        ));
    }
    if tree.declarations().iter().any(|candidate| {
        candidate.origin().logical_path() == declaration.origin().logical_path()
            && candidate.name() == new_name
            && matches!(
                candidate.kind(),
                DeclarationKind::Function
                    | DeclarationKind::Product
                    | DeclarationKind::Enum
                    | DeclarationKind::Trait
            )
    }) {
        return Err(error(
            ProtocolErrorCode::InvalidOperation,
            format!("declaration name {new_name:?} collides"),
        ));
    }
    Ok(ResolvedOperation::Rename {
        key: key.to_string(),
        old_name: declaration.name().to_string(),
        new_name: new_name.to_string(),
        module: declaration.origin().logical_path().to_string(),
        declaration_node: declaration.node().index(),
    })
}

pub(crate) fn apply(
    files: &mut [SourceFile],
    operation: &ResolvedOperation,
    kind: DeclarationKind,
) -> Result<(), ProtocolError> {
    let ResolvedOperation::Rename {
        old_name,
        new_name,
        module,
        declaration_node,
        ..
    } = operation
    else {
        return Ok(());
    };
    let declaration = super::nodes::node_mut(files, *declaration_node).ok_or_else(|| {
        error(
            ProtocolErrorCode::UnknownNode,
            "resolved declaration disappeared while staging",
        )
    })?;
    let name = declaration
        .children
        .first_mut()
        .and_then(|marker| marker.children.first_mut())
        .ok_or_else(|| {
            error(
                ProtocolErrorCode::InvalidOperation,
                "declaration has no mutable semantic name",
            )
        })?;
    match &mut name.kind {
        SyntaxKind::Str { value } | SyntaxKind::Symbol { name: value } => *value = new_name.clone(),
        _ => {
            return Err(error(
                ProtocolErrorCode::InvalidOperation,
                "declaration name has unsupported source kind",
            ))
        }
    }
    for file in files {
        let owns_declaration = file.origin.logical_path() == module;
        let imports_declaration = rename_module_metadata(file, module, old_name, new_name);
        if owns_declaration || imports_declaration {
            for form in &mut file.syntax {
                super::references::rename_references(form, kind, old_name, new_name, false, false);
            }
        }
    }
    Ok(())
}

fn rename_module_metadata(
    file: &mut SourceFile,
    module: &str,
    old_name: &str,
    new_name: &str,
) -> bool {
    file.syntax
        .iter_mut()
        .filter(|form| matches!(&form.kind, SyntaxKind::Call { name } if name == "imports"))
        .flat_map(|imports| &mut imports.children)
        .any(|import| rename_one_import(import, module, old_name, new_name))
}

fn rename_one_import(
    import: &mut crate::source::SourceNode,
    module: &str,
    old_name: &str,
    new_name: &str,
) -> bool {
    let Some(crate::source::SourceNode {
        kind: SyntaxKind::Str { value },
        ..
    }) = import.children.first_mut()
    else {
        return false;
    };
    let Some((path, encoded_names)) = value.split_once('#') else {
        return false;
    };
    if path != module {
        return false;
    }
    let path = path.to_string();
    let mut renamed = false;
    let mut names: Vec<_> = encoded_names.split(',').map(str::to_string).collect();
    for name in &mut names {
        if name == old_name {
            *name = new_name.to_string();
            renamed = true;
        }
    }
    names.sort();
    *value = format!("{path}#{}", names.join(","));
    renamed
}
