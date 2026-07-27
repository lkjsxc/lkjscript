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
    if !crate::source::is_source_identifier(new_name)
        || lkjscript_contracts::removed_spelling(new_name).is_some()
    {
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
    let [module_field, declarations_field] = import.children.as_mut_slice() else {
        return false;
    };
    let [crate::source::SourceNode {
        kind: SyntaxKind::Str { value: path },
        ..
    }] = module_field.children.as_slice()
    else {
        return false;
    };
    if !matches!(&module_field.kind, SyntaxKind::Call { name } if name == "module")
        || path != module
        || !matches!(&declarations_field.kind, SyntaxKind::Call { name } if name == "declarations")
    {
        return false;
    }
    let mut renamed = false;
    for declaration in &mut declarations_field.children {
        let SyntaxKind::Symbol { name } = &mut declaration.kind else {
            return false;
        };
        if name == old_name {
            *name = new_name.to_string();
            renamed = true;
        }
    }
    declarations_field.children.sort_by(|left, right| {
        let left = match &left.kind {
            SyntaxKind::Symbol { name } => name,
            _ => unreachable!("validated import declaration"),
        };
        let right = match &right.kind {
            SyntaxKind::Symbol { name } => name,
            _ => unreachable!("validated import declaration"),
        };
        left.cmp(right)
    });
    renamed
}
