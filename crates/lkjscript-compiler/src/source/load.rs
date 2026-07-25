use std::collections::HashSet;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

use lkjscript_core::Limits;

use super::{
    finish_tree, parser, DiagnosticCategory, Expr, LoadMetrics, SourceDiagnostic, SourceFile,
    SourceFoundationBudget, SourceOrigin, SourceResult, SourceSpan, ValidatedSourceTree,
    FOUNDATION_MAX_SOURCE_TREE_ENTRIES,
};

struct LoadState<'a> {
    package_root: &'a Path,
    installed_root: Option<&'a Path>,
    limits: &'a Limits,
    loading: HashSet<PathBuf>,
    done: HashSet<PathBuf>,
    files: Vec<SourceFile>,
    budget: SourceFoundationBudget,
    metrics: LoadMetrics,
}

#[derive(Clone)]
struct PendingImport {
    spec: String,
    span: SourceSpan,
}

struct LoadFrame {
    canonical: PathBuf,
    parent: PathBuf,
    parsed: SourceFile,
    imports: Vec<PendingImport>,
    next_import: usize,
    reached_by: Option<(SourceOrigin, SourceSpan)>,
}

pub(super) fn load_with_metrics(
    path: &Path,
    limits: &Limits,
) -> SourceResult<(ValidatedSourceTree, LoadMetrics)> {
    ensure_source_path(path)?;
    let loading_started = Instant::now();
    let entry = path.canonicalize().map_err(|error| {
        SourceDiagnostic::loading(
            host_diagnostic_origin(path),
            format!("cannot resolve requested source {path:?}: {error}"),
        )
    })?;
    let package_root = find_package_root(&entry);
    let installed_root = std::env::var_os("LKJSCRIPT_ROOT")
        .map(PathBuf::from)
        .and_then(|root| root.canonicalize().ok());
    let mut state = LoadState {
        package_root: &package_root,
        installed_root: installed_root.as_deref(),
        limits,
        loading: HashSet::new(),
        done: HashSet::new(),
        files: Vec::new(),
        budget: SourceFoundationBudget::default(),
        metrics: LoadMetrics {
            source_loading: loading_started.elapsed(),
            ..LoadMetrics::default()
        },
    };
    let (root_path, root_origin) = load_files_depth_first(&entry, &mut state)?;
    let tree = finish_tree(root_path, root_origin, state.files)?;
    Ok((tree, state.metrics))
}

pub(super) fn ensure_source_path(path: &Path) -> SourceResult<()> {
    if path.extension().and_then(|extension| extension.to_str()) == Some(crate::SOURCE_EXTENSION) {
        return Ok(());
    }
    Err(SourceDiagnostic::loading(
        host_diagnostic_origin(path),
        format!(
            "source path must end in .{}: {path:?}",
            crate::SOURCE_EXTENSION
        ),
    ))
}

fn host_diagnostic_origin(path: &Path) -> SourceOrigin {
    path.to_str().map_or_else(
        || SourceOrigin::in_memory("source.lkjscript"),
        SourceOrigin::in_memory,
    )
}

fn find_package_root(entry: &Path) -> PathBuf {
    let entry_parent = entry.parent().unwrap_or_else(|| Path::new("."));
    let mut current = entry_parent.to_path_buf();
    loop {
        if current.join("src").join("std").is_dir() {
            return current;
        }
        if !current.pop() {
            return entry_parent.to_path_buf();
        }
    }
}

pub(super) fn source_origin(
    path: &Path,
    package_root: &Path,
    installed_root: Option<&Path>,
) -> SourceResult<SourceOrigin> {
    let relative = path
        .strip_prefix(package_root)
        .ok()
        .or_else(|| installed_root.and_then(|root| path.strip_prefix(root).ok()))
        .ok_or_else(|| {
            SourceDiagnostic::loading(
                SourceOrigin::in_memory("source.lkjscript"),
                format!("source path is outside canonical roots: {path:?}"),
            )
        })?;
    let mut pieces = Vec::new();
    for component in relative.components() {
        let Component::Normal(piece) = component else {
            return Err(SourceDiagnostic::loading(
                SourceOrigin::in_memory("source.lkjscript"),
                format!("host source path is not canonical: {path:?}"),
            ));
        };
        let piece = piece.to_str().ok_or_else(|| {
            SourceDiagnostic::loading(
                SourceOrigin::in_memory("source.lkjscript"),
                format!("host source path is not valid UTF-8: {path:?}"),
            )
        })?;
        pieces.push(piece);
    }
    if pieces.is_empty() {
        return Err(SourceDiagnostic::loading(
            SourceOrigin::in_memory("source.lkjscript"),
            format!("host source path has no logical source name: {path:?}"),
        ));
    }
    Ok(SourceOrigin {
        logical_path: pieces.join("/"),
        host_containment_path: Some(path.to_path_buf()),
    })
}

fn load_files_depth_first(
    entry: &Path,
    state: &mut LoadState<'_>,
) -> SourceResult<(PathBuf, SourceOrigin)> {
    let root = load_frame(entry, true, None, &[], state)?.ok_or_else(|| {
        SourceDiagnostic::generic(
            SourceOrigin::in_memory("source.lkjscript"),
            "root source was already loaded",
        )
    })?;
    let root_path = root.canonical.clone();
    let root_origin = root.parsed.origin.clone();
    let mut stack = vec![root];
    loop {
        let next = stack.last_mut().and_then(|frame| {
            let pending = frame.imports.get(frame.next_import)?.clone();
            frame.next_import += 1;
            Some((pending, frame.parent.clone(), frame.parsed.origin.clone()))
        });
        if let Some((pending, parent, origin)) = next {
            let loading_started = Instant::now();
            let next_path = resolve_import(
                &pending.spec,
                &parent,
                state.package_root,
                state.installed_root,
                &origin,
            )?;
            state.metrics.source_loading = state
                .metrics
                .source_loading
                .saturating_add(loading_started.elapsed());
            if let Some(frame) = load_frame(
                &next_path,
                false,
                Some((origin, pending.span)),
                &stack,
                state,
            )? {
                stack.push(frame);
            }
            continue;
        }

        let Some(frame) = stack.pop() else {
            break;
        };
        state.loading.remove(&frame.canonical);
        state.done.insert(frame.canonical);
        state.files.push(frame.parsed);
    }
    Ok((root_path, root_origin))
}

fn load_frame(
    path: &Path,
    is_root: bool,
    reached_by: Option<(SourceOrigin, SourceSpan)>,
    stack: &[LoadFrame],
    state: &mut LoadState<'_>,
) -> SourceResult<Option<LoadFrame>> {
    ensure_source_path(path)?;
    let loading_started = Instant::now();
    let diagnostic_origin = reached_by.as_ref().map_or_else(
        || SourceOrigin::in_memory("source.lkjscript"),
        |(origin, _)| origin.clone(),
    );
    reject_obvious_non_regular(path, &diagnostic_origin)?;
    let mut file = open_source_file(path).map_err(|error| {
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
    let canonical = opened_source_path(
        &file,
        path,
        state.package_root,
        state.installed_root,
        &diagnostic_origin,
    )?;
    let origin = source_origin(&canonical, state.package_root, state.installed_root)?;
    if state.done.contains(&canonical) {
        return Ok(None);
    }
    if state.loading.contains(&canonical) {
        return Err(cycle_diagnostic(&canonical, reached_by, stack, state)?);
    }

    let parent = canonical
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    validate_source_directory(&parent, state.limits.max_dir_children)?;
    let source_bytes =
        read_bounded_source(&mut file, &metadata, &canonical, &origin, &mut state.budget)?;
    let final_canonical = opened_source_path(
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
    let parsed = parser::parse_file(source, origin.clone(), canonical.clone(), state.limits)?;
    if !is_root
        && parsed
            .forms
            .iter()
            .any(|form| matches!(form, Expr::Call { name, .. } if name == "main"))
    {
        return Err(SourceDiagnostic::new(
            "LKJ-SRC-IMPORTED-MAIN",
            DiagnosticCategory::Declaration,
            "imported file may contain only imports and function, product, trait, or impl declarations; main is forbidden",
            origin,
            parsed
                .syntax
                .iter()
                .find(|form| matches!(&form.kind, super::SyntaxKind::Call { name } if name == "main"))
                .map_or(SourceSpan::zero(), |form| form.span),
        ));
    }
    state.metrics.parsing = state
        .metrics
        .parsing
        .saturating_add(parsing_started.elapsed());
    let imports = parsed
        .forms
        .iter()
        .zip(&parsed.syntax)
        .filter_map(|(form, syntax)| match form {
            Expr::Call { name, args } if name == "import" => Some((args, syntax.span)),
            _ => None,
        })
        .map(|(args, span)| {
            import_path(args)
                .map(|spec| PendingImport {
                    spec: spec.to_string(),
                    span,
                })
                .map_err(|message| SourceDiagnostic::generic(parsed.origin.clone(), message))
        })
        .collect::<SourceResult<Vec<_>>>()?;

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

#[cfg(target_os = "linux")]
fn reject_obvious_non_regular(_path: &Path, _origin: &SourceOrigin) -> SourceResult<()> {
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn reject_obvious_non_regular(path: &Path, origin: &SourceOrigin) -> SourceResult<()> {
    let metadata = fs::metadata(path).map_err(|error| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("inspect requested source {path:?}: {error}"),
        )
    })?;
    if metadata.is_file() {
        return Ok(());
    }
    Err(SourceDiagnostic::loading(
        origin.clone(),
        format!("source is not a regular file: {path:?}"),
    ))
}

#[cfg(target_os = "linux")]
pub(super) fn open_source_file(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    const O_NONBLOCK: i32 = 0x800;
    OpenOptions::new()
        .read(true)
        .custom_flags(O_NONBLOCK)
        .open(path)
}

#[cfg(not(target_os = "linux"))]
fn open_source_file(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().read(true).open(path)
}

#[cfg(target_os = "linux")]
pub(super) fn opened_source_path(
    file: &File,
    requested: &Path,
    package_root: &Path,
    installed_root: Option<&Path>,
    origin: &SourceOrigin,
) -> SourceResult<PathBuf> {
    use std::os::fd::AsRawFd;

    let descriptor_path = PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()));
    let canonical = descriptor_path.canonicalize().map_err(|error| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("resolve opened source descriptor for requested path {requested:?}: {error}"),
        )
    })?;
    let inside_package = canonical.starts_with(package_root);
    let inside_install = installed_root.is_some_and(|root| canonical.starts_with(root));
    if !inside_package && !inside_install {
        return Err(SourceDiagnostic::loading(
            origin.clone(),
            format!(
                "opened source escapes package roots: requested={requested:?}; actual={canonical:?}"
            ),
        ));
    }
    Ok(canonical)
}

#[cfg(not(target_os = "linux"))]
fn opened_source_path(
    _file: &File,
    requested: &Path,
    _package_root: &Path,
    _installed_root: Option<&Path>,
    origin: &SourceOrigin,
) -> SourceResult<PathBuf> {
    Err(SourceDiagnostic::loading(
        origin.clone(),
        format!(
            "host source loading requires descriptor-derived containment on the Current Linux acceptance target: {requested:?}"
        ),
    ))
}

fn read_bounded_source(
    file: &mut File,
    metadata: &Metadata,
    canonical: &Path,
    origin: &SourceOrigin,
    budget: &mut SourceFoundationBudget,
) -> SourceResult<Vec<u8>> {
    let expected_bytes = metadata.len();
    budget.check_metadata(origin, expected_bytes)?;
    let source_bytes = read_bounded_bytes(file, expected_bytes, canonical, origin, budget)?;
    let final_metadata = file.metadata().map_err(|error| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("reinspect opened source {}: {error}", canonical.display()),
        )
    })?;
    let actual_bytes = u64::try_from(source_bytes.len()).map_err(|_| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!(
                "source byte length is not representable: {}",
                canonical.display()
            ),
        )
    })?;
    if !final_metadata.is_file()
        || final_metadata.len() != expected_bytes
        || actual_bytes != expected_bytes
    {
        return Err(SourceDiagnostic::loading(
            origin.clone(),
            format!(
                "source size changed while reading {}: metadata={expected_bytes}; read={actual_bytes}; final-metadata={}",
                canonical.display(),
                final_metadata.len()
            ),
        ));
    }
    budget.record_read(origin, actual_bytes)?;
    Ok(source_bytes)
}

pub(super) fn read_bounded_bytes<R: Read>(
    reader: &mut R,
    expected_bytes: u64,
    path: &Path,
    origin: &SourceOrigin,
    budget: &SourceFoundationBudget,
) -> SourceResult<Vec<u8>> {
    let allowance = budget.remaining_read_allowance(origin)?;
    let read_limit = allowance.checked_add(1).ok_or_else(|| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("bounded source read limit overflow: {path:?}"),
        )
    })?;
    let mut source_bytes = Vec::new();
    reader
        .take(read_limit)
        .read_to_end(&mut source_bytes)
        .map_err(|error| {
            SourceDiagnostic::loading(origin.clone(), format!("read source {path:?}: {error}"))
        })?;
    let actual_bytes = u64::try_from(source_bytes.len()).map_err(|_| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("source byte length is not representable: {path:?}"),
        )
    })?;
    if actual_bytes != expected_bytes {
        return Err(SourceDiagnostic::loading(
            origin.clone(),
            format!(
                "source size changed while reading {path:?}: metadata={expected_bytes}; read={actual_bytes}"
            ),
        ));
    }
    Ok(source_bytes)
}

fn cycle_diagnostic(
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

fn resolve_import(
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
    if let Some(rest) = spec.strip_prefix("./") {
        return Ok(parent.join(rest));
    }
    if spec.starts_with('.') {
        return Err(SourceDiagnostic::loading(
            origin.clone(),
            format!("import path must be package-root or ./relative ({spec})"),
        ));
    }
    if let Some(rest) = spec.strip_prefix("std/") {
        return Ok(library_path(package_root, installed_root, "std", rest));
    }
    if let Some(rest) = spec.strip_prefix("lib/") {
        return Ok(library_path(package_root, installed_root, "lib", rest));
    }
    if let Some(rest) = spec.strip_prefix("examples/") {
        return Ok(library_path(package_root, installed_root, "examples", rest));
    }
    Ok(package_root.join(spec))
}

fn library_path(
    package_root: &Path,
    installed_root: Option<&Path>,
    library: &str,
    rest: &str,
) -> PathBuf {
    let local = package_root.join("src").join(library);
    if local.is_dir() || installed_root.is_none() {
        return local.join(rest);
    }
    installed_root
        .unwrap_or(package_root)
        .join("src")
        .join(library)
        .join(rest)
}

pub fn validate_source_directory_tree(root: &Path, max: u32) -> SourceResult<()> {
    let mut pending = vec![root.to_path_buf()];
    let mut traversed_entries = 0_u64;
    while let Some(directory) = pending.pop() {
        let entries = source_directory_entries(&directory, max)?;
        traversed_entries = traversed_entries
            .checked_add(u64::try_from(entries.len()).map_err(|_| {
                SourceDiagnostic::loading(
                    host_diagnostic_origin(&directory),
                    "source-tree entry count is not representable",
                )
            })?)
            .ok_or_else(|| source_tree_entry_limit_error(&directory, u64::MAX))?;
        if traversed_entries > FOUNDATION_MAX_SOURCE_TREE_ENTRIES {
            return Err(source_tree_entry_limit_error(&directory, traversed_entries));
        }
        for entry in entries.into_iter().rev() {
            let kind = entry.file_type().map_err(|error| {
                SourceDiagnostic::loading(
                    host_diagnostic_origin(&entry.path()),
                    format!("inspect source entry {:?}: {error}", entry.path()),
                )
            })?;
            if kind.is_dir() {
                pending.push(entry.path());
            }
        }
    }
    Ok(())
}

fn source_tree_entry_limit_error(path: &Path, attempted: u64) -> SourceDiagnostic {
    SourceDiagnostic::new(
        "LKJ-SRC-LIMIT",
        DiagnosticCategory::ResourceLimit,
        format!(
            "Semantic Source Foundation V1 resource limit: category=source-tree-entries; attempted={attempted}; limit={FOUNDATION_MAX_SOURCE_TREE_ENTRIES}"
        ),
        host_diagnostic_origin(path),
        SourceSpan::zero(),
    )
}

fn validate_source_directory(dir: &Path, max: u32) -> SourceResult<()> {
    source_directory_entries(dir, max).map(|_| ())
}

fn source_directory_entries(dir: &Path, max: u32) -> SourceResult<Vec<fs::DirEntry>> {
    let origin = host_diagnostic_origin(dir);
    let entries = fs::read_dir(dir).map_err(|error| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("read source directory {}: {error}", dir.display()),
        )
    })?;
    let max_entries = usize::try_from(max).map_err(|_| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("source directory entry limit is not representable: {max}"),
        )
    })?;
    let implementation_max =
        usize::try_from(FOUNDATION_MAX_SOURCE_TREE_ENTRIES).unwrap_or(usize::MAX);
    let effective_max = max_entries.min(implementation_max);
    let mut children = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            SourceDiagnostic::loading(
                origin.clone(),
                format!("read entry in {}: {error}", dir.display()),
            )
        })?;
        if children.len() == effective_max {
            let attempted = u64::try_from(effective_max)
                .unwrap_or(u64::MAX)
                .saturating_add(1);
            if effective_max < max_entries {
                return Err(source_tree_entry_limit_error(dir, attempted));
            }
            return Err(SourceDiagnostic::new(
                "LKJ-SRC-LIMIT",
                DiagnosticCategory::ResourceLimit,
                format!(
                    "lkjscript source directory {} has at least {attempted} entries (max {max}); split it",
                    dir.display()
                ),
                origin,
                SourceSpan::zero(),
            ));
        }
        children.push(entry);
    }
    children.sort_by_key(fs::DirEntry::file_name);
    Ok(children)
}

fn import_path(args: &[Expr]) -> std::result::Result<&str, &'static str> {
    match args {
        [Expr::LitStr(path)] | [Expr::Symbol(path)] => Ok(path.as_str()),
        _ => Err("import expects one path string"),
    }
}

#[cfg(test)]
pub(super) fn resolve_for_test(
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
