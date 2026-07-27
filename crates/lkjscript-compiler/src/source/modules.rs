use std::collections::{BTreeMap, BTreeSet};

use crate::source::{DeclarationSummary, Expr, SourceDiagnostic, SourceFile, SourceResult};

mod rewrite;

#[derive(Default)]
struct Interface {
    declarations: BTreeSet<String>,
    exports: BTreeSet<String>,
}

pub(super) fn scope(
    files: &mut [SourceFile],
    declarations: &mut [DeclarationSummary],
) -> SourceResult<()> {
    let mut interfaces = BTreeMap::new();
    for file in files.iter() {
        let module = file.origin.logical_path.clone();
        if interfaces
            .insert(module.clone(), interface(file)?)
            .is_some()
        {
            return Err(error(file, format!("duplicate module identity: {module}")));
        }
    }
    let mut bindings = Vec::with_capacity(files.len());
    for file in files.iter() {
        bindings.push(module_bindings(file, &interfaces)?);
    }
    for declaration in declarations {
        if let Some(replacement) = files
            .iter()
            .position(|file| file.origin.logical_path == declaration.origin.logical_path())
            .and_then(|index| bindings[index].get(declaration.name()))
        {
            declaration.name = replacement.clone();
        }
    }
    for (file, bindings) in files.iter_mut().zip(&bindings) {
        file.forms = file
            .forms
            .iter()
            .filter(|form| !metadata(form))
            .cloned()
            .map(|mut form| {
                rewrite::top_level(&mut form, bindings);
                form
            })
            .collect();
    }
    Ok(())
}

pub(crate) fn public_names(file: &SourceFile) -> SourceResult<BTreeSet<String>> {
    interface(file).map(|interface| interface.exports)
}

fn interface(file: &SourceFile) -> SourceResult<Interface> {
    let mut result = Interface::default();
    for form in &file.forms {
        if let Expr::Call { name, args } = form {
            if matches!(name.as_str(), "def" | "product" | "enum" | "trait") {
                let mut names = BTreeSet::new();
                rewrite::declaration_names(name, args, &mut names);
                result.declarations.extend(names.iter().cloned());
                if args
                    .iter()
                    .any(|arg| matches!(arg, Expr::Symbol(name) if name == "public"))
                {
                    result.exports.extend(names);
                }
            }
        }
    }
    for export in &result.exports {
        if !result.declarations.contains(export) {
            return Err(error(
                file,
                format!("export is not declared by this module: {export}"),
            ));
        }
    }
    Ok(result)
}

fn module_bindings(
    file: &SourceFile,
    interfaces: &BTreeMap<String, Interface>,
) -> SourceResult<BTreeMap<String, String>> {
    let module = &file.origin.logical_path;
    let own = interfaces
        .get(module)
        .ok_or_else(|| error(file, "module interface is absent"))?;
    let mut result: BTreeMap<_, _> = own
        .declarations
        .iter()
        .map(|name| {
            (
                name.clone(),
                super::module_names::internal_name(module, name),
            )
        })
        .collect();
    for imports in &file.forms {
        let Expr::Call { name, args } = imports else {
            continue;
        };
        if name != "imports" {
            continue;
        }
        for import in args {
            let (target, names) =
                import_fields(import).ok_or_else(|| error(file, "malformed import metadata"))?;
            let target_interface = interfaces
                .get(target)
                .ok_or_else(|| error(file, format!("imported module is not loaded: {target}")))?;
            let mut prior = "";
            for name in names {
                if name <= prior {
                    return Err(error(
                        file,
                        format!("import names must be sorted and unique: {target}"),
                    ));
                }
                prior = name;
                if !target_interface.exports.contains(name) {
                    return Err(error(
                        file,
                        format!("imported name is private or absent: {target}:{name}"),
                    ));
                }
                if result
                    .insert(
                        name.to_string(),
                        super::module_names::internal_name(target, name),
                    )
                    .is_some()
                {
                    return Err(error(
                        file,
                        format!("import collides with a local or imported name: {name}"),
                    ));
                }
            }
        }
    }
    Ok(result)
}

fn import_fields(import: &Expr) -> Option<(&str, Vec<&str>)> {
    let Expr::Call { name, args } = import else {
        return None;
    };
    if name != "import" {
        return None;
    }
    let [Expr::Call {
        name: module,
        args: path,
    }, Expr::Call {
        name: declarations,
        args: names,
    }] = args.as_slice()
    else {
        return None;
    };
    let [Expr::LitStr(path)] = path.as_slice() else {
        return None;
    };
    if module != "module" || declarations != "declarations" {
        return None;
    }
    let names = names
        .iter()
        .map(|name| match name {
            Expr::Symbol(name) => Some(name.as_str()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some((path, names))
}

fn metadata(expr: &Expr) -> bool {
    matches!(expr, Expr::Call { name, .. } if name == "imports")
}

fn error(file: &SourceFile, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic::generic(file.origin.clone(), message)
}
