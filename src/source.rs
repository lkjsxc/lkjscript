//! Source loading owns module discovery; the host supplies bytes for requested paths.
use crate::ast::*;
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

pub fn parse_bundle(bundle: &SourceBundle) -> Result<Program, Diagnostic> {
    let mut files = BTreeMap::new();
    for file in &bundle.files {
        if files
            .insert(file.path.as_str(), file.text.as_str())
            .is_some()
        {
            let mut error = Diagnostic::new(
                Span::default(),
                "E_SOURCE_DUPLICATE",
                &format!("source bundle repeats {}", file.path),
            );
            error.path = file.path.clone();
            return Err(error);
        }
    }
    load(&bundle.root, bundle.library.as_deref(), &mut |path| {
        Ok(files.get(path).map(|s| s.to_string()))
    })
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
        let text = read_source(read, &path, Span::default())?.ok_or_else(|| {
            Diagnostic::new(
                Span::default(),
                "E_MODULE_MISSING",
                &format!("missing source file {path}"),
            )
        })?;
        // A file can be mounted in multiple modules, but an ancestor inclusion cycle cannot terminate.
        let mut ancestor = Some(scope);
        while let Some(a) = ancestor {
            if loaded.get(&a) == Some(&path) {
                return Err(Diagnostic::new(
                    Span::default(),
                    "E_MODULE_CYCLE",
                    &format!("recursive module file {path}"),
                ));
            }
            ancestor = program.scopes[a].parent;
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
        let items = syntax::parse_file(&mut program, &text, file, scope).map_err(|mut e| {
            e.path = path.clone();
            e
        })?;
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
            let first = read_source(read, &direct, module.span)?;
            let second = read_source(read, &nested, module.span)?;
            let selected = match (first, second) {
                (Some(_), Some(_)) => {
                    return Err(load_error(
                        &program,
                        module.span,
                        "E_MODULE_AMBIGUOUS",
                        &format!("both {direct} and {nested} define module {}", module.name),
                    ));
                }
                (Some(_), None) => direct,
                (None, Some(_)) => nested,
                (None, None) => {
                    return Err(load_error(
                        &program,
                        module.span,
                        "E_MODULE_MISSING",
                        &format!("module {} needs {direct} or {nested}", module.name),
                    ));
                }
            };
            pending.push((selected, module.scope, Some(index)));
        }
    }
    Ok(program)
}

fn load_error(program: &Program, span: Span, code: &str, message: &str) -> Diagnostic {
    let mut e = Diagnostic::new(span, code, message);
    e.path = program.files[span.file].path.clone();
    e
}
