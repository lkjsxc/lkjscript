//! Source loading owns module discovery; the host supplies bytes for requested paths.
use crate::ast::*;
use crate::diagnostics::BlockedRegion;
use crate::syntax;
use std::collections::BTreeMap;

pub struct SourceInput {
    pub path: String,
    pub text: String,
}
pub struct SourceBundle {
    pub root: String,
    pub library: Option<String>,
    pub files: Vec<SourceInput>,
}

/// Recovered syntax is retained for diagnostics and census only. An error in
/// this report prevents every production path from constructing checked K.
pub struct SourceReport {
    pub program: Program,
    pub diagnostics: Vec<Diagnostic>,
    pub blocked: Vec<BlockedRegion>,
}
impl SourceReport {
    pub fn into_result(self) -> Result<Program, Diagnostic> {
        match self.diagnostics.into_iter().next() {
            Some(error) => Err(error),
            None => Ok(self.program),
        }
    }
}

pub fn parse_bundle(bundle: &SourceBundle) -> Result<Program, Diagnostic> {
    parse_bundle_report(bundle).into_result()
}

pub fn parse_bundle_report(bundle: &SourceBundle) -> SourceReport {
    let mut files = BTreeMap::new();
    let mut duplicates = Vec::new();
    for file in &bundle.files {
        if files.contains_key(file.path.as_str()) {
            let mut error = Diagnostic::new(
                Span::default(),
                "E_SOURCE_DUPLICATE",
                &format!("source bundle repeats {}", file.path),
            );
            error.path = file.path.clone();
            duplicates.push(error);
        } else {
            files.insert(file.path.as_str(), file.text.as_str());
        }
    }
    let mut report = load_report(&bundle.root, bundle.library.as_deref(), &mut |path| {
        Ok(files.get(path).map(|s| s.to_string()))
    });
    if !duplicates.is_empty() {
        let mut diagnostics = duplicates;
        diagnostics.append(&mut report.diagnostics);
        report.diagnostics = diagnostics;
        report.blocked.push(BlockedRegion {
            domain: None,
            path: bundle.root.clone(),
            span: Span::default(),
            kind: "source bundle".to_string(),
            reason: "duplicate source inputs prevent an unambiguous compilation".to_string(),
        });
    }
    report
}

fn parent(path: &str) -> &str {
    path.rsplit_once('/').map(|(p, _)| p).unwrap_or("")
}
fn join(directory: &str, file: &str) -> String {
    if directory.is_empty() {
        file.to_string()
    } else {
        format!("{directory}/{file}")
    }
}
fn stem(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .strip_suffix(".rs")
        .unwrap_or(path)
}

fn read_source<F: FnMut(&str) -> std::result::Result<Option<String>, String>>(
    read: &mut F,
    path: &str,
    span: Span,
) -> Result<Option<String>, Diagnostic> {
    read(path).map_err(|message| {
        let mut e = Diagnostic::new(span, "E_IO", &message);
        e.path = path.to_string();
        e
    })
}

/// Both bundle compilation and the bootstrap CLI use this module interpretation.
/// `read` supplies bytes only; it does not resolve source names or select modules.
pub fn load<F: FnMut(&str) -> std::result::Result<Option<String>, String>>(
    root: &str,
    standard: Option<&str>,
    read: &mut F,
) -> Result<Program, Diagnostic> {
    load_report(root, standard, read).into_result()
}

/// Load every independently discoverable file even after source errors. File
/// reads and parsing use the same path rules as ordinary compilation.
pub fn load_report<F: FnMut(&str) -> std::result::Result<Option<String>, String>>(
    root: &str,
    standard: Option<&str>,
    read: &mut F,
) -> SourceReport {
    let mut diagnostics = Vec::new();
    let mut blocked = Vec::new();
    let mut program = Program::default();
    program.scopes.push(ModuleScope {
        path: String::new(),
        parent: None,
        crate_root: 0,
    });
    let mut pending = Vec::new();
    if let Some(path) = standard {
        program.scopes.push(ModuleScope {
            path: "$std".to_string(),
            parent: None,
            crate_root: 1,
        });
        pending.push((path.to_string(), 1, None));
    }
    pending.push((root.to_string(), 0, None));
    let mut directories = BTreeMap::new();
    let mut loaded = BTreeMap::new();
    let mut next_module = 0;
    while let Some((path, scope, owner)) = pending.pop() {
        let text = match read_source(read, &path, Span::default()) {
            Ok(Some(text)) => text,
            result => {
                let mut error = match result {
                    Err(error) => error,
                    _ => Diagnostic::new(
                        Span::default(),
                        "E_MODULE_MISSING",
                        &format!("missing source file {path}"),
                    ),
                };
                error.path = path.clone();
                blocked.push(BlockedRegion {
                    domain: None,
                    path: path.clone(),
                    span: Span::default(),
                    kind: "file".to_string(),
                    reason: error.message.clone(),
                });
                diagnostics.push(error);
                continue;
            }
        };
        // A file can be mounted in multiple modules, but an ancestor inclusion cycle cannot terminate.
        let mut ancestor = Some(scope);
        let mut cycle = false;
        while let Some(a) = ancestor {
            if loaded.get(&a) == Some(&path) {
                cycle = true;
                break;
            }
            ancestor = program.scopes[a].parent;
        }
        if cycle {
            let mut error = Diagnostic::new(
                Span::default(),
                "E_MODULE_CYCLE",
                &format!("recursive module file {path}"),
            );
            error.path = path.clone();
            blocked.push(BlockedRegion {
                domain: None,
                path: path.clone(),
                span: Span::default(),
                kind: "file".to_string(),
                reason: error.message.clone(),
            });
            diagnostics.push(error);
            continue;
        }
        loaded.insert(scope, path.clone());
        let file = program.files.len();
        program.files.push(SourceFile {
            path: path.clone(),
            crate_root: program.scopes[scope].crate_root,
        });
        let base = if program.scopes[scope].parent.is_none() || stem(&path) == "mod" {
            parent(&path).to_string()
        } else {
            join(parent(&path), stem(&path))
        };
        directories.insert(scope, base);
        let mut parsed = syntax::parse_file_report(&mut program, &text, file, scope);
        diagnostics.append(&mut parsed.diagnostics);
        blocked.append(&mut parsed.blocked);
        let items = parsed.items;
        if let Some(index) = owner {
            let module: &mut Module = &mut program.modules[index];
            module.items = items;
            module.external = false;
        } else {
            program.items.extend(items);
        }
        while next_module < program.modules.len() {
            let index = next_module;
            next_module += 1;
            let module = &program.modules[index];
            let parent_scope = program.scopes[module.scope].parent.unwrap();
            let mut parent_path = directories.get(&parent_scope).cloned();
            let mut at = parent_scope;
            let mut inline = Vec::new();
            while parent_path.is_none() {
                let info = &program.scopes[at];
                inline.push(info.path.rsplit("::").next().unwrap().to_string());
                at = info.parent.unwrap();
                parent_path = directories.get(&at).cloned();
            }
            let mut directory = parent_path.unwrap();
            while let Some(part) = inline.pop() {
                directory = join(&directory, &part);
            }
            if !module.external {
                directories.insert(module.scope, join(&directory, &module.name));
                continue;
            }
            let direct = join(&directory, &format!("{}.rs", module.name));
            let nested = join(&directory, &format!("{}/mod.rs", module.name));
            let first = read_source(read, &direct, module.span);
            let second = read_source(read, &nested, module.span);
            let (first, second) = match (first, second) {
                (Ok(first), Ok(second)) => (first, second),
                (first, second) => {
                    for error in [first.err(), second.err()].into_iter().flatten() {
                        blocked.push(BlockedRegion {
                            domain: None,
                            path: error.path.clone(),
                            span: module.span,
                            kind: "module".to_string(),
                            reason: error.message.clone(),
                        });
                        diagnostics.push(error);
                    }
                    continue;
                }
            };
            let selected = match (first, second) {
                (Some(_), Some(_)) => {
                    let error = load_error(
                        &program,
                        module.span,
                        "E_MODULE_AMBIGUOUS",
                        &format!("both {direct} and {nested} define module {}", module.name),
                    );
                    blocked.push(BlockedRegion {
                        domain: None,
                        path: error.path.clone(),
                        span: module.span,
                        kind: "module".to_string(),
                        reason: error.message.clone(),
                    });
                    diagnostics.push(error);
                    continue;
                }
                (Some(_), None) => direct,
                (None, Some(_)) => nested,
                (None, None) => {
                    let error = load_error(
                        &program,
                        module.span,
                        "E_MODULE_MISSING",
                        &format!("module {} needs {direct} or {nested}", module.name),
                    );
                    blocked.push(BlockedRegion {
                        domain: None,
                        path: error.path.clone(),
                        span: module.span,
                        kind: "module".to_string(),
                        reason: error.message.clone(),
                    });
                    diagnostics.push(error);
                    continue;
                }
            };
            pending.push((selected, module.scope, Some(index)));
        }
    }
    if !diagnostics.is_empty() {
        blocked.push(BlockedRegion {
            domain: None,
            path: root.to_string(),
            span: Span::default(),
            kind: "semantic phase".to_string(),
            reason: "source errors prevent complete name resolution and semantic checking"
                .to_string(),
        });
    }
    SourceReport {
        program,
        diagnostics,
        blocked,
    }
}

fn load_error(program: &Program, span: Span, code: &str, message: &str) -> Diagnostic {
    let mut e = Diagnostic::new(span, code, message);
    e.path = program.files[span.file].path.clone();
    e
}
