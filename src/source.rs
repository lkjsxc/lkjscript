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

/// Recovered syntax is retained for diagnostics only. An error in
/// this report prevents every production path from constructing checked IR.
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
    let mut files: BTreeMap<String, usize> = BTreeMap::new();
    let mut duplicates = Vec::new();
    for index in 0..bundle.files.len() {
        let file = &bundle.files[index];
        if files.contains_key(&file.path) {
            let mut error = Diagnostic::new(
                Span::default(),
                "E_SOURCE_DUPLICATE",
                &format!("source bundle repeats {}", file.path),
            );
            error.path = file.path.clone();
            duplicates.push(error);
        } else {
            files.insert(file.path.clone(), index);
        }
    }
    let library = match &bundle.library {
        Some(path) => Some(path.as_str()),
        None => None,
    };
    let mut report = load_report(&bundle.root, library, &mut |path: &str| {
        let text = match files.get(&path.to_string()) {
            Some(index) => Some(bundle.files[*index].text.as_bytes().to_vec()),
            None => None,
        };
        Ok(text)
    });
    if !duplicates.is_empty() {
        let mut diagnostics = duplicates;
        for diagnostic in report.diagnostics {
            diagnostics.push(diagnostic);
        }
        report.diagnostics = diagnostics;
        report.blocked.push(BlockedRegion {
            path: bundle.root.clone(),
            span: Span::default(),
            kind: "source bundle".to_string(),
            reason: "duplicate source inputs prevent an unambiguous compilation".to_string(),
        });
    }
    report
}

fn parent(path: &str) -> &str {
    match path.rfind('/') {
        Some(0) => &path[..1],
        Some(at) => &path[..at],
        None => "",
    }
}
fn join(directory: &str, file: &str) -> String {
    if directory.is_empty() {
        file.to_string()
    } else if directory == "/" {
        format!("/{file}")
    } else {
        format!("{directory}/{file}")
    }
}
fn stem(path: &str) -> &str {
    let start = match path.rfind('/') {
        Some(at) => at + 1,
        None => 0,
    };
    let name = &path[start..];
    if name.ends_with(".rs") {
        &name[..name.len() - 3]
    } else {
        path
    }
}

fn read_source<F: FnMut(&str) -> std::result::Result<Option<Vec<u8>>, String>>(
    read: &mut F,
    path: &str,
    span: Span,
) -> Result<Option<String>, Diagnostic> {
    let bytes = read(path).map_err(|message| {
        let mut e = Diagnostic::new(span, "E_IO", &message);
        e.path = path.to_string();
        e
    })?;
    match bytes {
        Some(bytes) => match String::from_utf8(bytes) {
            Ok(text) => Ok(Some(text)),
            Err(_) => {
                let mut error = Diagnostic::new(span, "E_IO", "source is not valid UTF-8");
                error.path = path.to_string();
                Err(error)
            }
        },
        None => Ok(None),
    }
}

fn include_path(source: &str, requested: &str) -> Result<String, String> {
    if requested.is_empty() || requested.starts_with('/') {
        return Err(
            "an embedded file needs a nonempty path beneath the source directory".to_string(),
        );
    }
    for byte in requested.bytes() {
        if byte == 0u8 {
            return Err("an embedded file path cannot contain a NUL byte".to_string());
        }
    }
    let directory = parent(source);
    let absolute = directory.starts_with('/');
    let mut parts: Vec<String> = Vec::new();
    for part in directory.split('/') {
        if part != "" && part != "." {
            if part == ".." {
                if parts.pop().is_none() {
                    return Err("embedded file escapes the source directory".to_string());
                }
            } else {
                parts.push(part.to_string());
            }
        }
    }
    let floor = if !parts.is_empty() && parts[0] == "$std" {
        1usize
    } else {
        0usize
    };
    for part in requested.split('/') {
        if part != "" && part != "." {
            if part == ".." {
                if parts.len() == floor {
                    return Err("embedded file escapes the source directory".to_string());
                }
                parts.pop();
            } else {
                parts.push(part.to_string());
            }
        }
    }
    let mut path = if absolute {
        "/".to_string()
    } else {
        String::new()
    };
    for part in parts {
        if !path.is_empty() && path != "/" {
            path.push('/');
        }
        path.push_str(&part);
    }
    Ok(path)
}

fn expand_includes<F: FnMut(&str) -> Result<Option<Vec<u8>>, String>>(
    program: &mut Program,
    first: usize,
    read: &mut F,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut index = first;
    while index < program.exprs.len() {
        let expression = program.exprs[index].clone();
        if let Expr::Macro {
            name,
            args,
            separator,
        } = expression.kind
        {
            if name == "include_str" || name == "include_bytes" {
                let span = expression.span;
                let requested: Option<String> = if args.len() == 1 && separator == "," {
                    match &program.exprs[args[0]].kind {
                        Expr::String(path) => Some(path.clone()),
                        _ => None,
                    }
                } else {
                    None
                };
                let result: Result<String, String> = match requested {
                    Some(requested) => include_path(&program.files[span.file].path, &requested),
                    None => Err(format!("{name}! requires one literal file path")),
                };
                let path = match result {
                    Ok(path) => path,
                    Err(message) => {
                        diagnostics.push(load_error(program, span, "E_INCLUDE_PATH", &message));
                        index += 1;
                        continue;
                    }
                };
                match read(&path) {
                    Ok(Some(bytes)) => {
                        if name == "include_bytes" {
                            program.exprs[index].kind = Expr::Bytes(bytes);
                        } else {
                            match String::from_utf8(bytes) {
                                Ok(text) => program.exprs[index].kind = Expr::String(text),
                                Err(_) => diagnostics.push(load_error(
                                    program,
                                    span,
                                    "E_INCLUDE_UTF8",
                                    &format!("embedded file {path} is not valid UTF-8"),
                                )),
                            }
                        }
                    }
                    Ok(None) => diagnostics.push(load_error(
                        program,
                        span,
                        "E_INCLUDE_MISSING",
                        &format!("missing embedded file {path}"),
                    )),
                    Err(message) => diagnostics.push(load_error(
                        program,
                        span,
                        "E_IO",
                        &format!("cannot read embedded file {path}: {message}"),
                    )),
                }
            }
        }
        index += 1;
    }
}

/// Both bundle compilation and the bootstrap CLI use this module interpretation.
/// `read` supplies bytes only; it does not resolve source names or select modules.
pub fn load<F: FnMut(&str) -> std::result::Result<Option<Vec<u8>>, String>>(
    root: &str,
    standard: Option<&str>,
    read: &mut F,
) -> Result<Program, Diagnostic> {
    load_report(root, standard, read).into_result()
}

/// Load every independently discoverable file even after source errors. File
/// reads and parsing use the same path rules as ordinary compilation.
pub fn load_report<F: FnMut(&str) -> std::result::Result<Option<Vec<u8>>, String>>(
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
    let mut pending: Vec<(String, ModuleId, Option<usize>)> = Vec::new();
    if let Some(path) = standard {
        program.scopes.push(ModuleScope {
            path: "$std".to_string(),
            parent: None,
            crate_root: 1,
        });
        pending.push((path.to_string(), 1, None));
    }
    pending.push((root.to_string(), 0, None));
    let mut directories: BTreeMap<ModuleId, String> = BTreeMap::new();
    let mut loaded: BTreeMap<ModuleId, String> = BTreeMap::new();
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
        let first_expression = program.exprs.len();
        let parsed = syntax::parse_file_report(&mut program, &text, file, scope);
        expand_includes(&mut program, first_expression, read, &mut diagnostics);
        for diagnostic in parsed.diagnostics {
            diagnostics.push(diagnostic);
        }
        for region in parsed.blocked {
            blocked.push(region);
        }
        let items = parsed.items;
        if let Some(index) = owner {
            let module: &mut Module = &mut program.modules[index];
            module.items = items;
            module.external = false;
        } else {
            for item in items {
                program.items.push(item);
            }
        }
        while next_module < program.modules.len() {
            let index = next_module;
            next_module += 1;
            let module = &program.modules[index];
            let parent_scope = program.scopes[module.scope].parent.unwrap();
            let mut parent_path = directories.get(&parent_scope).cloned();
            let mut at = parent_scope;
            let mut inline: Vec<String> = Vec::new();
            while parent_path.is_none() {
                let info = &program.scopes[at];
                let start = match info.path.rfind("::") {
                    Some(position) => position + 2,
                    None => 0,
                };
                inline.push(info.path[start..].to_string());
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
                    for candidate in vec![first.err(), second.err()] {
                        if let Some(error) = candidate {
                            blocked.push(BlockedRegion {
                                path: error.path.clone(),
                                span: module.span,
                                kind: "module".to_string(),
                                reason: error.message.clone(),
                            });
                            diagnostics.push(error);
                        }
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
