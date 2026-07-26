use std::path::{Component, Path, PathBuf};

use crate::source::{
    DiagnosticCategory, Expr, SourceDiagnostic, SourceOrigin, SourceResult, SourceSpan,
};

use super::containment::{ensure_source_path, source_origin};
use super::{LoadFrame, LoadState};

pub(super) fn cycle_diagnostic(
    canonical: &Path,
    reached_by: Option<(SourceOrigin, SourceSpan)>,
    stack: &[LoadFrame],
    state: &LoadState<'_>,
) -> SourceResult<SourceDiagnostic> {
    let (origin, span) = match reached_by {
        Some(reached_by) => reached_by,
        None => (
            source_origin(canonical, state.package_root, state.installed_root)?,
            SourceSpan::zero(),
        ),
    };
    let mut diagnostic = SourceDiagnostic::new(
        "LKJ-SRC-LOAD",
        DiagnosticCategory::SourceLoading,
        format!("cyclic import involving {}", canonical.display()),
        origin,
        span,
    );
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
) -> SourceResult<PathBuf> {
    let candidate = resolve_import_with_root(spec, parent, package_root, installed_root, origin)?;
    let canonical = candidate.canonicalize().map_err(|error| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!(
                "cannot open import {spec} ({}): {error}",
                candidate.display()
            ),
        )
    })?;

    let package_root = package_root.canonicalize().map_err(|error| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!(
                "canonicalize package root {}: {error}",
                package_root.display()
            ),
        )
    })?;
    let inside_package = canonical.starts_with(&package_root);
    let inside_install = installed_root.is_some_and(|root| canonical.starts_with(root));
    if !inside_package && !inside_install {
        return Err(SourceDiagnostic::loading(
            origin.clone(),
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
) -> SourceResult<PathBuf> {
    let spec_path = Path::new(spec);
    ensure_source_path(spec_path)?;
    if spec_path.is_absolute() {
        return Err(SourceDiagnostic::loading(
            origin.clone(),
            format!("absolute import path banned ({spec})"),
        ));
    }
    if spec_path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(SourceDiagnostic::loading(
            origin.clone(),
            format!("import climb banned ({spec}); use a package-root path"),
        ));
    }
    if spec.starts_with('.') {
        return Err(SourceDiagnostic::loading(
            origin.clone(),
            format!("import path must be an exact package-root module ID ({spec})"),
        ));
    }
    let _ = parent;
    let _ = installed_root;
    Ok(package_root.join(spec))
}

pub(super) fn import_path(args: &[Expr]) -> std::result::Result<&str, &'static str> {
    match args {
        [Expr::LitStr(spec)] => spec
            .split_once('#')
            .map(|(path, _)| path)
            .ok_or("import expects exact path#name-list encoding"),
        _ => Err("import expects exact path#name-list encoding"),
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
    )
}
