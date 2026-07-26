use std::path::Path;
use std::time::Instant;

use crate::source::{
    parse, DiagnosticCategory, Expr, SourceDiagnostic, SourceOrigin, SourceResult, SourceSpan,
    SyntaxKind,
};

use super::{LoadFrame, LoadState, PendingImport};

pub(super) fn load_frame(
    path: &Path,
    is_root: bool,
    reached_by: Option<(SourceOrigin, SourceSpan)>,
    stack: &[LoadFrame],
    state: &mut LoadState<'_>,
) -> SourceResult<Option<LoadFrame>> {
    super::containment::ensure_source_path(path)?;
    let loading_started = Instant::now();
    let diagnostic_origin = reached_by.as_ref().map_or_else(
        || SourceOrigin::in_memory("source.lkjscript"),
        |(origin, _)| origin.clone(),
    );
    super::containment::reject_obvious_non_regular(path, &diagnostic_origin)?;
    let mut file = super::containment::open_source_file(path).map_err(|error| {
        SourceDiagnostic::loading(
            diagnostic_origin.clone(),
            format!("cannot open source {path:?}: {error}"),
        )
    })?;
    let metadata = file.metadata().map_err(|error| {
        SourceDiagnostic::loading(
            diagnostic_origin.clone(),
            format!("inspect opened source {path:?}: {error}"),
        )
    })?;
    if !metadata.is_file() {
        return Err(SourceDiagnostic::loading(
            diagnostic_origin.clone(),
            format!("source is not a regular file: {path:?}"),
        ));
    }
    let canonical = super::containment::opened_source_path(
        &file,
        path,
        state.package_root,
        state.installed_root,
        &diagnostic_origin,
    )?;
    let origin =
        super::containment::source_origin(&canonical, state.package_root, state.installed_root)?;
    if state.done.contains(&canonical) {
        return Ok(None);
    }
    if state.loading.contains(&canonical) {
        return Err(super::imports::cycle_diagnostic(
            &canonical, reached_by, stack, state,
        )?);
    }

    let parent = canonical
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    super::directory::validate_source_directory(&parent, state.limits.max_dir_children)?;
    let source_bytes = super::read::read_bounded_source(
        &mut file,
        &metadata,
        &canonical,
        &origin,
        &mut state.budget,
    )?;
    let final_canonical = super::containment::opened_source_path(
        &file,
        path,
        state.package_root,
        state.installed_root,
        &origin,
    )?;
    if final_canonical != canonical {
        return Err(SourceDiagnostic::loading(
            origin.clone(),
            format!(
                "opened source descriptor path changed while reading: before={canonical:?}; after={final_canonical:?}"
            ),
        ));
    }
    let source = std::str::from_utf8(&source_bytes).map_err(|error| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("source is not UTF-8 {}: {error}", canonical.display()),
        )
    })?;
    state.metrics.source_loading = state
        .metrics
        .source_loading
        .saturating_add(loading_started.elapsed());

    let parsing_started = Instant::now();
    let parsed = parse::parse_file(source, origin.clone(), canonical.clone(), state.limits)?;
    if !is_root
        && parsed
            .forms
            .iter()
            .any(|form| matches!(form, Expr::Call { name, .. } if name == "main"))
    {
        return Err(SourceDiagnostic::new(
            "LKJ-SRC-IMPORTED-MAIN",
            DiagnosticCategory::Declaration,
            concat!(
                "imported file may contain only imports and function, product, trait, or ",
                "impl declarations; main is forbidden"
            ),
            origin,
            parsed
                .syntax
                .iter()
                .find(|form| matches!(&form.kind, SyntaxKind::Call { name } if name == "main"))
                .map_or(SourceSpan::zero(), |form| form.span),
        ));
    }
    state.metrics.parsing = state
        .metrics
        .parsing
        .saturating_add(parsing_started.elapsed());
    let mut imports = Vec::new();
    for (form, syntax) in parsed.forms.iter().zip(&parsed.syntax) {
        let Expr::Call { name, args } = form else {
            continue;
        };
        if name != "imports" {
            continue;
        }
        for (import, import_syntax) in args.iter().zip(&syntax.children) {
            let Expr::Call { name, args } = import else {
                continue;
            };
            if name != "import" {
                continue;
            }
            let spec = super::imports::import_path(args)
                .map_err(|message| SourceDiagnostic::generic(parsed.origin.clone(), message))?;
            imports.push(PendingImport {
                spec: spec.to_string(),
                span: import_syntax.span,
            });
        }
    }

    state.loading.insert(canonical.clone());
    Ok(Some(LoadFrame {
        canonical,
        parent,
        parsed,
        imports,
        next_import: 0,
        reached_by,
    }))
}
