//! Load a root `.lkjml` file and follow package-root / relative imports.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::Expr;
use crate::lex::lex;
use crate::limits_check::check_file_limits;
use crate::parse::parse_tokens;
use lkjscript2026_core::{Error, Limits, Result};

#[derive(Debug, Clone)]
pub struct SourceFile {
    #[allow(dead_code)]
    pub path: PathBuf,
    pub forms: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct Program {
    #[allow(dead_code)]
    pub root: PathBuf,
    pub files: Vec<SourceFile>,
}

pub fn load_program(path: &Path, limits: &Limits) -> Result<Program> {
    let entry = path
        .canonicalize()
        .map_err(|e| Error::msg(format!("cannot open {}: {e}", path.display())))?;
    let package_root = find_package_root(&entry)?;
    let mut loading = HashSet::new();
    let mut done = HashSet::new();
    let mut files = Vec::new();
    load_file(
        &entry,
        &package_root,
        limits,
        &mut loading,
        &mut done,
        &mut files,
    )?;
    Ok(Program {
        root: entry,
        files,
    })
}

fn find_package_root(entry: &Path) -> Result<PathBuf> {
    let mut cur = entry.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    loop {
        if cur.join("src").join("std").is_dir() {
            return Ok(cur);
        }
        if !cur.pop() {
            break;
        }
    }
    std::env::current_dir().map_err(|e| Error::msg(format!("cwd: {e}")))
}

fn load_file(
    path: &Path,
    package_root: &Path,
    limits: &Limits,
    loading: &mut HashSet<PathBuf>,
    done: &mut HashSet<PathBuf>,
    files: &mut Vec<SourceFile>,
) -> Result<()> {
    let canon = path
        .canonicalize()
        .map_err(|e| Error::msg(format!("cannot open {}: {e}", path.display())))?;
    if done.contains(&canon) {
        return Ok(());
    }
    if !loading.insert(canon.clone()) {
        return Err(Error::msg(format!(
            "cyclic import involving {}",
            canon.display()
        )));
    }
    let source = fs::read_to_string(&canon)
        .map_err(|e| Error::msg(format!("read {}: {e}", canon.display())))?;
    let label = canon.display().to_string();
    let tokens = lex(&source).map_err(|error| Error::msg(format!("{label}: {error}")))?;
    check_file_limits(&tokens, limits, &label)?;
    let forms = parse_tokens(&tokens)
        .map_err(|error| Error::msg(format!("{label}: {error}")))?;
    validate_top_level(&forms, limits, &label)?;
    let parent = canon.parent().unwrap_or_else(|| Path::new("."));
    for form in &forms {
        if let Expr::Call { name, args } = form {
            if name == "import" {
                let spec = import_path(args)?;
                let next = resolve_import(spec, parent, package_root)?;
                load_file(&next, package_root, limits, loading, done, files)?;
            }
        }
    }
    loading.remove(&canon);
    done.insert(canon.clone());
    files.push(SourceFile {
        path: canon,
        forms,
    });
    Ok(())
}

fn resolve_import(spec: &str, parent: &Path, package_root: &Path) -> Result<PathBuf> {
    let installed = std::env::var_os("LKJSCRIPT2026_ROOT").map(PathBuf::from);
    resolve_import_with_root(spec, parent, package_root, installed.as_deref())
}

fn resolve_import_with_root(
    spec: &str,
    parent: &Path,
    package_root: &Path,
    installed_root: Option<&Path>,
) -> Result<PathBuf> {
    if spec.contains("..") {
        return Err(Error::msg(format!(
            "import climb banned ({spec}); use package-root path"
        )));
    }
    if let Some(rest) = spec.strip_prefix("./") {
        return Ok(parent.join(rest));
    }
    if spec.starts_with('.') {
        return Err(Error::msg(format!(
            "import path must be package-root or ./relative ({spec})"
        )));
    }
    if let Some(rest) = spec.strip_prefix("std/") {
        return Ok(library_path(package_root, installed_root, "std", rest));
    }
    if let Some(rest) = spec.strip_prefix("lib/") {
        return Ok(library_path(package_root, installed_root, "lib", rest));
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

pub(crate) fn validate_top_level(forms: &[Expr], limits: &Limits, path: &str) -> Result<()> {
    let n = forms.len() as u32;
    if n > limits.max_toplevel_forms {
        return Err(Error::msg(format!(
            "{path}: too many top-level forms ({n} > {}); split via import",
            limits.max_toplevel_forms
        )));
    }
    for form in forms {
        match form {
            Expr::Call { name, .. } if name == "def" || name == "do" || name == "import" => {}
            _ => {
                return Err(Error::msg(format!(
                    "{path}: top-level must be def, do, or import"
                )));
            }
        }
    }
    Ok(())
}

fn import_path(args: &[Expr]) -> Result<&str> {
    match args {
        [Expr::LitStr(s)] => Ok(s.as_str()),
        [Expr::Symbol(s)] => Ok(s.as_str()),
        _ => Err(Error::msg("import expects a path string")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_climb() {
        let err = resolve_import_with_root(
            "../x.lkjml",
            Path::new("/a"),
            Path::new("/pkg"),
            None,
        )
        .unwrap_err();
        assert!(err.as_str().contains("climb"));
    }

    #[test]
    fn std_prefix() {
        let p = resolve_import_with_root(
            "std/list/nth.lkjml",
            Path::new("/a/b"),
            Path::new("/pkg"),
            None,
        )
        .unwrap();
        assert_eq!(p, PathBuf::from("/pkg/src/std/list/nth.lkjml"));
    }

    #[test]
    fn lib_prefix() {
        let p = resolve_import_with_root(
            "lib/edit/loop.lkjml",
            Path::new("/a/b"),
            Path::new("/pkg"),
            None,
        )
        .unwrap();
        assert_eq!(p, PathBuf::from("/pkg/src/lib/edit/loop.lkjml"));
    }

    #[test]
    fn examples_join() {
        let p = resolve_import_with_root(
            "examples/hello/main.lkjml",
            Path::new("/a/b"),
            Path::new("/pkg"),
            None,
        )
        .unwrap();
        assert_eq!(p, PathBuf::from("/pkg/examples/hello/main.lkjml"));
    }

    #[test]
    fn dot_relative() {
        let p = resolve_import_with_root(
            "./sib.lkjml",
            Path::new("/a/b"),
            Path::new("/pkg"),
            None,
        )
        .unwrap();
        assert_eq!(p, PathBuf::from("/a/b/sib.lkjml"));
    }

    #[test]
    fn installed_std_is_used_when_project_has_no_std() {
        let p = resolve_import_with_root(
            "std/list/nth.lkjml",
            Path::new("/project"),
            Path::new("/project"),
            Some(Path::new("/opt/lkjscript2026")),
        )
        .unwrap();
        assert_eq!(
            p,
            PathBuf::from("/opt/lkjscript2026/src/std/list/nth.lkjml")
        );
    }
}
