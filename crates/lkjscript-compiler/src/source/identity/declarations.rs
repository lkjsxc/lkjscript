use std::collections::HashMap;

use crate::source::{
    parse, DeclarationKey, DeclarationKind, DeclarationSummary, DiagnosticCategory, NodeId,
    SourceDiagnostic, SourceFile, SourceOrigin, SourceResult, SourceSpan,
};

use super::{declaration_identity, declaration_key_bytes, declaration_key_human_identity};

pub(crate) fn build_declarations(
    files: &[SourceFile],
    ordered: &[usize],
    top_ids: &HashMap<(usize, usize), NodeId>,
) -> SourceResult<Vec<DeclarationSummary>> {
    let mut declarations = Vec::new();
    let mut exact_keys: HashMap<Vec<u8>, (SourceOrigin, SourceSpan)> = HashMap::new();
    let mut global_names: HashMap<String, (DeclarationKind, SourceOrigin, SourceSpan)> =
        HashMap::new();
    for file_index in ordered {
        let file = &files[*file_index];
        for (form_index, form) in file.syntax.iter().enumerate() {
            let Some((kind, name)) = declaration_identity(form) else {
                continue;
            };
            if matches!(
                kind,
                DeclarationKind::Function | DeclarationKind::Product | DeclarationKind::Trait
            ) && !parse::is_source_identifier(&name)
            {
                return Err(SourceDiagnostic::new(
                    "LKJ-DECL-NAME",
                    DiagnosticCategory::Declaration,
                    format!(
                        "{} declaration name {name:?} is not a spellable source identifier",
                        kind.as_str()
                    ),
                    file.origin.clone(),
                    form.span,
                ));
            }
            if matches!(
                kind,
                DeclarationKind::Function | DeclarationKind::Product | DeclarationKind::Trait
            ) {
                if let Some((first_kind, first_origin, first_span)) = global_names.get(&name) {
                    let message = if *first_kind == kind {
                        format!("duplicate {} declaration {name}", kind.as_str())
                    } else if matches!(
                        (*first_kind, kind),
                        (DeclarationKind::Trait, DeclarationKind::Product)
                            | (DeclarationKind::Product, DeclarationKind::Trait)
                    ) {
                        format!("product {name} collides with a trait")
                    } else {
                        format!(
                            "duplicate global declaration {name}: {} conflicts with {}",
                            kind.as_str(),
                            first_kind.as_str()
                        )
                    };
                    return Err(SourceDiagnostic::new(
                        "LKJ-DECL-DUPLICATE",
                        DiagnosticCategory::Declaration,
                        message,
                        file.origin.clone(),
                        form.span,
                    )
                    .with_related(
                        "first declaration",
                        first_origin.clone(),
                        *first_span,
                    ));
                }
                global_names.insert(name.clone(), (kind, file.origin.clone(), form.span));
            }
            let exact = declaration_key_bytes(&file.origin.logical_path, kind, &name);
            if let Some((first_origin, first_span)) = exact_keys.get(&exact) {
                return Err(SourceDiagnostic::new(
                    "LKJ-DECL-DUPLICATE",
                    DiagnosticCategory::Declaration,
                    format!("duplicate {} declaration {name}", kind.as_str()),
                    file.origin.clone(),
                    form.span,
                )
                .with_related(
                    "first declaration",
                    first_origin.clone(),
                    *first_span,
                ));
            }
            exact_keys.insert(exact.clone(), (file.origin.clone(), form.span));
            let node = top_ids
                .get(&(*file_index, form_index))
                .copied()
                .ok_or_else(|| {
                    SourceDiagnostic::generic(file.origin.clone(), "missing dense declaration node")
                })?;
            declarations.push(DeclarationSummary {
                key: DeclarationKey {
                    digest: lkjscript_core::sha256(&exact),
                    exact_identity: exact,
                    canonical_identity: declaration_key_human_identity(
                        &file.origin.logical_path,
                        kind,
                        &name,
                    ),
                },
                kind,
                name,
                origin: file.origin.clone(),
                span: form.span,
                node,
            });
        }
    }
    declarations.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(declarations)
}
