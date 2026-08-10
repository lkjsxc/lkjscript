use std::path::{Component, Path, PathBuf};

use crate::source::{
    DiagnosticCategory, Expr, SourceDiagnostic, SourceOrigin, SourceResult, SourceSpan,
};

use super::containment::source_origin;
use super::{LoadFrame, LoadState};

pub(super) fn cycle_diagnostic(
    canonical: &Path,
    reached_by: Option<(SourceOrigin, SourceSpan)>,
    stack: &[LoadFrame],
    state: &LoadState<'_>,
) -> SourceResult<SourceDiagnostic> {
    let message = format!("cyclic import involving {}", canonical.display());
    let mut diagnostic = match reached_by {
        Some((origin, span)) => loading_diagnostic(&origin, Some(span), message),
        None => {
            let origin = source_origin(canonical, state.package_root, state.installed_root)?;
            loading_diagnostic(&origin, None, message)
        }
    };
    if let Some(cycle_start) = stack.iter().position(|frame| frame.canonical == canonical) {
        for frame in &stack[cycle_start + 1..] {
            if let Some((edge_origin, edge_span)) = &frame.reached_by {
                diagnostic = diagnostic.with_related(
                    "earlier import in cycle",
                    edge_origin.clone(),
                    *edge_span,
                );
            }
        }
    }
    Ok(diagnostic)
}

pub(super) fn resolve_import(
    spec: &str,
    parent: &Path,
    package_root: &Path,
    installed_root: Option<&Path>,
    origin: &SourceOrigin,
    span: SourceSpan,
) -> SourceResult<PathBuf> {
    let candidate = resolve_import_with_root(
        spec,
        parent,
        package_root,
        installed_root,
        origin,
        Some(span),
    )?;
    let canonical = candidate.canonicalize().map_err(|error| {
        loading_diagnostic(
            origin,
            Some(span),
            format!(
                "cannot open import {spec} ({}): {error}",
                candidate.display()
            ),
        )
    })?;

    let package_root = package_root.canonicalize().map_err(|error| {
        loading_diagnostic(
            origin,
            Some(span),
            format!(
                "canonicalize package root {}: {error}",
                package_root.display()
            ),
        )
    })?;
    let inside_package = canonical.starts_with(&package_root);
    let inside_install = installed_root.is_some_and(|root| canonical.starts_with(root));
    if !inside_package && !inside_install {
        return Err(loading_diagnostic(
            origin,
            Some(span),
            format!(
                "import escapes package roots: {spec} -> {}",
                canonical.display()
            ),
        ));
    }
    Ok(canonical)
}

fn resolve_import_with_root(
    spec: &str,
    parent: &Path,
    package_root: &Path,
    installed_root: Option<&Path>,
    origin: &SourceOrigin,
    span: Option<SourceSpan>,
) -> SourceResult<PathBuf> {
    let spec_path = Path::new(spec);
    if spec_path
        .extension()
        .and_then(|extension| extension.to_str())
        != Some(crate::SOURCE_EXTENSION)
    {
        return Err(loading_diagnostic(
            origin,
            span,
            format!(
                "source path must end in .{}: {spec_path:?}",
                crate::SOURCE_EXTENSION
            ),
        ));
    }
    if spec_path.is_absolute() {
        return Err(loading_diagnostic(
            origin,
            span,
            format!("absolute import path banned ({spec})"),
        ));
    }
    if spec_path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(loading_diagnostic(
            origin,
            span,
            format!("import climb banned ({spec}); use a package-root path"),
        ));
    }
    if spec.starts_with('.') {
        return Err(loading_diagnostic(
            origin,
            span,
            format!("import path must be an exact package-root module ID ({spec})"),
        ));
    }
    let _ = parent;
    let _ = installed_root;
    Ok(package_root.join(spec))
}

fn loading_diagnostic(
    origin: &SourceOrigin,
    span: Option<SourceSpan>,
    message: impl Into<String>,
) -> SourceDiagnostic {
    match span {
        Some(span) => SourceDiagnostic::new(
            "LKJ-SRC-LOAD",
            DiagnosticCategory::SourceLoading,
            message,
            origin.clone(),
            span,
        ),
        None => SourceDiagnostic::loading(origin.clone(), message),
    }
}

pub(super) fn import_path(args: &[Expr]) -> std::result::Result<&str, &'static str> {
    match args {
        [Expr::Call { name: module, args }, Expr::Call {
            name: declarations, ..
        }] if module == "module" && declarations == "declarations" => match args.as_slice() {
            [Expr::LitStr(path)] => Ok(path),
            _ => Err("module requires one exact path text value"),
        },
        _ => Err("import expects structured module and declarations fields"),
    }
}

#[cfg(test)]
pub(crate) fn resolve_for_test(
    spec: &str,
    parent: &Path,
    package_root: &Path,
    installed_root: Option<&Path>,
) -> SourceResult<PathBuf> {
    resolve_import_with_root(
        spec,
        parent,
        package_root,
        installed_root,
        &SourceOrigin::in_memory("test.lkjscript"),
        None,
    )
}
