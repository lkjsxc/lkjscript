use std::collections::HashMap;

use crate::source::{
    parse, DeclarationKey, DeclarationKind, DeclarationSummary, DiagnosticCategory,
    SourceDiagnostic, SourceFile, SourceOrigin, SourceResult, SourceSpan,
};

use super::{declaration_identity, declaration_key_bytes};

pub(crate) fn build_declarations(
    files: &[SourceFile],
    ordered: &[usize],
) -> SourceResult<Vec<DeclarationSummary>> {
    let mut declarations = Vec::new();
    let mut exact_keys: HashMap<Vec<u8>, (SourceOrigin, SourceSpan)> = HashMap::new();
    for file_index in ordered {
        let file = &files[*file_index];
        let mut module_names: HashMap<String, (DeclarationKind, SourceOrigin, SourceSpan)> =
            HashMap::new();
        for form in &file.syntax {
            let Some((kind, name)) = declaration_identity(form) else {
                continue;
            };
            if matches!(
                kind,
                DeclarationKind::Function
                    | DeclarationKind::Product
                    | DeclarationKind::Enum
                    | DeclarationKind::Trait
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
                DeclarationKind::Function
                    | DeclarationKind::Product
                    | DeclarationKind::Enum
                    | DeclarationKind::Trait
            ) {
                if let Some((first_kind, first_origin, first_span)) = module_names.get(&name) {
                    let message = if *first_kind == kind {
                        format!("duplicate {} declaration {name}", kind.as_str())
                    } else if matches!(
                        (*first_kind, kind),
                        (DeclarationKind::Trait, DeclarationKind::Product)
                            | (DeclarationKind::Product, DeclarationKind::Trait)
                    ) {
                        format!("product {name} collides with a trait")
                    } else if *first_kind != DeclarationKind::Function
                        && kind != DeclarationKind::Function
                    {
                        format!(
                            "nominal type {name} collides between {} and {} declarations",
                            first_kind.as_str(),
                            kind.as_str()
                        )
                    } else {
                        format!(
                            "duplicate module declaration {name}: {} conflicts with {}",
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
                module_names.insert(name.clone(), (kind, file.origin.clone(), form.span));
            }
            let exact =
                declaration_key_bytes(&file.origin.logical_path, kind, &name).map_err(|error| {
                    SourceDiagnostic::generic(
                        file.origin.clone(),
                        format!("cannot encode declaration identity: {error:?}"),
                    )
                })?;
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
            declarations.push(DeclarationSummary {
                key: DeclarationKey {
                    digest: lkjscript_core::sha256(&exact),
                    exact_identity: exact,
                },
                kind,
                name,
                origin: file.origin.clone(),
            });
        }
    }
    declarations.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(declarations)
}
