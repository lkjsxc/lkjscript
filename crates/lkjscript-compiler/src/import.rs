//! Load a root `.lkjscript` file and follow contained package or relative imports.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::ast::Expr;
use crate::ensure_source_path;
use crate::lex::lex;
use crate::limits_check::check_file_limits;
use crate::parse::parse_tokens;
use lkjscript_core::{Error, Limits, Result};

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
    ensure_source_path(path)?;
    let entry = path
        .canonicalize()
        .map_err(|error| Error::msg(format!("cannot open {}: {error}", path.display())))?;
    let package_root = find_package_root(&entry);
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

pub fn validate_source_tree(root: &Path, limits: &Limits) -> Result<()> {
    validate_source_directory_recursive(root, limits.max_dir_children)
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

fn load_file(
    path: &Path,
    package_root: &Path,
    limits: &Limits,
    loading: &mut HashSet<PathBuf>,
    done: &mut HashSet<PathBuf>,
    files: &mut Vec<SourceFile>,
) -> Result<()> {
    ensure_source_path(path)?;
    let canonical = path
        .canonicalize()
        .map_err(|error| Error::msg(format!("cannot open {}: {error}", path.display())))?;
    if done.contains(&canonical) {
        return Ok(());
    }
    if !loading.insert(canonical.clone()) {
        return Err(Error::msg(format!(
            "cyclic import involving {}",
            canonical.display()
        )));
    }

    let parent = canonical.parent().unwrap_or_else(|| Path::new("."));
    validate_source_directory(parent, limits.max_dir_children)?;
    let source = fs::read_to_string(&canonical)
        .map_err(|error| Error::msg(format!("read {}: {error}", canonical.display())))?;
    let label = canonical.display().to_string();
    let tokens = lex(&source).map_err(|error| Error::msg(format!("{label}: {error}")))?;
    check_file_limits(&tokens, limits, &label)?;
    let forms = parse_tokens(&tokens)
        .map_err(|error| Error::msg(format!("{label}: {error}")))?;
    validate_top_level(&forms, limits, &label)?;

    for form in &forms {
        if let Expr::Call { name, args } = form {
            if name == "import" {
                let spec = import_path(args)?;
                let next = resolve_import(spec, parent, package_root)?;
                load_file(&next, package_root, limits, loading, done, files)?;
            }
        }
    }

    loading.remove(&canonical);
    done.insert(canonical.clone());
    files.push(SourceFile {
        path: canonical,
        forms,
    });
    Ok(())
}

fn resolve_import(spec: &str, parent: &Path, package_root: &Path) -> Result<PathBuf> {
    let installed_root = std::env::var_os("LKJSCRIPT_ROOT").map(PathBuf::from);
    let candidate =
        resolve_import_with_root(spec, parent, package_root, installed_root.as_deref())?;
    let canonical = candidate.canonicalize().map_err(|error| {
        Error::msg(format!("cannot open import {spec} ({}): {error}", candidate.display()))
    })?;

    let package_root = package_root.canonicalize().map_err(|error| {
        Error::msg(format!(
            "canonicalize package root {}: {error}",
            package_root.display()
        ))
    })?;
    let inside_package = canonical.starts_with(&package_root);
    let inside_install = installed_root
        .as_deref()
        .and_then(|root| root.canonicalize().ok())
        .is_some_and(|root| canonical.starts_with(root));
    if !inside_package && !inside_install {
        return Err(Error::msg(format!(
            "import escapes package roots: {spec} -> {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

fn resolve_import_with_root(
    spec: &str,
    parent: &Path,
    package_root: &Path,
    installed_root: Option<&Path>,
) -> Result<PathBuf> {
    let spec_path = Path::new(spec);
    ensure_source_path(spec_path)?;
    if spec_path.is_absolute() {
        return Err(Error::msg(format!("absolute import path banned ({spec})")));
    }
    if spec_path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(Error::msg(format!(
            "import climb banned ({spec}); use a package-root path"
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
    if let Some(rest) = spec.strip_prefix("examples/") {
        return Ok(library_path(
            package_root,
            installed_root,
            "examples",
            rest,
        ));
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

fn validate_source_directory_recursive(dir: &Path, max: u32) -> Result<()> {
    let entries = source_directory_entries(dir, max)?;
    for entry in entries {
        let kind = entry.file_type().map_err(|error| {
            Error::msg(format!("inspect source entry {}: {error}", entry.path().display()))
        })?;
        if kind.is_dir() {
            validate_source_directory_recursive(&entry.path(), max)?;
        }
    }
    Ok(())
}

fn validate_source_directory(dir: &Path, max: u32) -> Result<()> {
    source_directory_entries(dir, max).map(|_| ())
}

fn source_directory_entries(dir: &Path, max: u32) -> Result<Vec<fs::DirEntry>> {
    let entries = fs::read_dir(dir)
        .map_err(|error| Error::msg(format!("read source directory {}: {error}", dir.display())))?;
    let mut children = Vec::new();
    for entry in entries {
        let entry = entry
            .map_err(|error| Error::msg(format!("read entry in {}: {error}", dir.display())))?;
        let name = entry.file_name();
        if name == OsStr::new(".git") || name == OsStr::new("target") {
            continue;
        }
        children.push(entry);
    }
    children.sort_by_key(|entry| entry.file_name());
    if children.len() as u32 > max {
        return Err(Error::msg(format!(
            "lkjscript source directory {} has {} entries (max {max}); split it",
            dir.display(),
            children.len()
        )));
    }
    Ok(children)
}

pub(crate) fn validate_top_level(forms: &[Expr], limits: &Limits, path: &str) -> Result<()> {
    let count = forms.len() as u32;
    if count > limits.max_toplevel_forms {
        return Err(Error::msg(format!(
            "{path}: too many top-level forms ({count} > {}); split via import",
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
        [Expr::LitStr(path)] | [Expr::Symbol(path)] => Ok(path.as_str()),
        _ => Err(Error::msg("import expects one path string")),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use lkjscript_core::Limits;

    use super::{load_program, resolve_import_with_root, validate_source_tree};

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> std::io::Result<Self> {
            let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "lkjscript-import-{label}-{}-{id}",
                std::process::id()
            ));
            fs::create_dir(&path)?;
            Ok(Self(path))
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn rejects_climb() {
        let error = resolve_import_with_root(
            "../x.lkjscript",
            Path::new("/a"),
            Path::new("/pkg"),
            None,
        )
        .err()
        .map(|error| error.to_string());
        assert!(error.as_deref().is_some_and(|text| text.contains("climb")));
    }

    #[test]
    fn rejects_absolute_and_legacy_imports() {
        assert!(resolve_import_with_root(
            "/x.lkjscript",
            Path::new("/a"),
            Path::new("/pkg"),
            None,
        )
        .is_err());
        assert!(resolve_import_with_root(
            "std/list/nth.lkjml",
            Path::new("/a"),
            Path::new("/pkg"),
            None,
        )
        .is_err());
    }

    #[test]
    fn package_prefixes_and_relative_paths_resolve() {
        let std_path = resolve_import_with_root(
            "std/list/nth.lkjscript",
            Path::new("/a/b"),
            Path::new("/pkg"),
            None,
        )
        .ok();
        let lib_path = resolve_import_with_root(
            "lib/lkjedit/loop.lkjscript",
            Path::new("/a/b"),
            Path::new("/pkg"),
            None,
        )
        .ok();
        let example_path = resolve_import_with_root(
            "examples/hello/main.lkjscript",
            Path::new("/a/b"),
            Path::new("/pkg"),
            None,
        )
        .ok();
        let relative_path = resolve_import_with_root(
            "./sib.lkjscript",
            Path::new("/a/b"),
            Path::new("/pkg"),
            None,
        )
        .ok();

        assert_eq!(std_path, Some(PathBuf::from("/pkg/src/std/list/nth.lkjscript")));
        assert_eq!(lib_path, Some(PathBuf::from("/pkg/src/lib/lkjedit/loop.lkjscript")));
        assert_eq!(
            example_path,
            Some(PathBuf::from("/pkg/src/examples/hello/main.lkjscript"))
        );
        assert_eq!(relative_path, Some(PathBuf::from("/a/b/sib.lkjscript")));
    }

    #[test]
    fn installed_std_is_used_when_project_has_no_std() {
        let path = resolve_import_with_root(
            "std/list/nth.lkjscript",
            Path::new("/project"),
            Path::new("/project"),
            Some(Path::new("/opt/lkjscript")),
        )
        .ok();
        assert_eq!(
            path,
            Some(PathBuf::from(
                "/opt/lkjscript/src/std/list/nth.lkjscript"
            ))
        );
    }

    #[test]
    fn source_tree_accepts_sixteen_and_rejects_seventeen(
    ) -> std::io::Result<()> {
        let accepted = TempDir::new("sixteen")?;
        for index in 0..16 {
            fs::write(
                accepted.0.join(format!("source-{index}.lkjscript")),
                "",
            )?;
        }
        assert!(validate_source_tree(&accepted.0, &Limits::default()).is_ok());

        let rejected = TempDir::new("seventeen")?;
        for index in 0..16 {
            fs::write(
                rejected.0.join(format!("source-{index}.lkjscript")),
                "",
            )?;
        }
        fs::write(rejected.0.join(".hidden.lkjscript"), "")?;
        assert!(validate_source_tree(&rejected.0, &Limits::default()).is_err());
        Ok(())
    }

    #[test]
    fn compilation_rejects_a_wide_entry_directory() -> std::io::Result<()> {
        let directory = TempDir::new("wide-entry")?;
        let entry = directory.0.join("main.lkjscript");
        fs::write(&entry, "do/\nnil\n/do\n")?;
        for index in 0..16 {
            fs::write(directory.0.join(format!("asset-{index}")), "")?;
        }
        let error = load_program(&entry, &Limits::default())
            .err()
            .map(|error| error.to_string());
        assert!(error
            .as_deref()
            .is_some_and(|text| text.contains("17 entries (max 16)")));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn compilation_rejects_a_symlink_import_escape() -> std::io::Result<()> {
        use std::os::unix::fs::symlink;

        let package = TempDir::new("package")?;
        let outside = TempDir::new("outside")?;
        fs::create_dir_all(package.0.join("src/std"))?;
        fs::write(outside.0.join("escaped.lkjscript"), "do/\nnil\n/do\n")?;
        symlink(
            outside.0.join("escaped.lkjscript"),
            package.0.join("escaped.lkjscript"),
        )?;
        let entry = package.0.join("main.lkjscript");
        fs::write(
            &entry,
            "import/\n./escaped.lkjscript\n/import\ndo/\nnil\n/do\n",
        )?;

        let error = load_program(&entry, &Limits::default())
            .err()
            .map(|error| error.to_string());
        assert!(error
            .as_deref()
            .is_some_and(|text| text.contains("escapes package roots")));
        Ok(())
    }
}
