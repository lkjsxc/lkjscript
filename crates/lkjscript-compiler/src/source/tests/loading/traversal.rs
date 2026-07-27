use super::super::*;

fn exact_import(path: &str, name: &str) -> String {
    format!(
        concat!(
            "imports/\nimport/\nmodule/\n{}\n/module\n",
            "declarations/\n{}\n/declarations\n/import\n/imports\n"
        ),
        path, name
    )
}

#[test]
fn loader_uses_explicit_dependency_first_dfs_for_deep_import_chain() -> std::io::Result<()> {
    const DEPTH: usize = 1_500;
    const FILES_PER_GROUP: usize = 15;
    const GROUPS_PER_BUCKET: usize = 10;

    let directory = TempDir::new("deep-imports")?;
    let mut paths = Vec::with_capacity(DEPTH);
    let mut logical_paths = Vec::with_capacity(DEPTH);
    for index in 0..DEPTH {
        let group = index / FILES_PER_GROUP;
        let bucket = group / GROUPS_PER_BUCKET;
        let logical = format!("units/b{bucket:02}/g{group:03}/u{index:04}.lkjscript");
        let path = directory.0.join(&logical);
        fs::create_dir_all(path.parent().expect("unit parent"))?;
        paths.push(path);
        logical_paths.push(logical);
    }
    for (index, path) in paths.iter().enumerate() {
        let source = if let Some(next) = logical_paths.get(index + 1) {
            exact_import(next, "leaf")
        } else {
            named_def("leaf")
        };
        fs::write(path, source)?;
    }
    let root = directory.0.join("main.lkjscript");
    fs::write(
        &root,
        format!(
            "{}{}",
            exact_import(&logical_paths[0], "leaf"),
            unit_main("unit")
        ),
    )?;

    let tree = load(&root, &Limits::default())
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    assert_eq!(tree.files().len(), DEPTH + 1);
    assert_eq!(tree.files()[0].path, paths[DEPTH - 1].canonicalize()?);
    assert_eq!(tree.files()[DEPTH].path, root.canonicalize()?);
    Ok(())
}

#[test]
fn source_tree_validation_uses_an_explicit_stack_for_deep_directories() -> std::io::Result<()> {
    const DEPTH: usize = 1_500;
    let directory = TempDir::new("deep-source-tree")?;
    let mut deepest = directory.0.clone();
    for _ in 0..DEPTH {
        deepest.push("d");
        fs::create_dir(&deepest)?;
    }
    fs::write(deepest.join("leaf.lkjscript"), unit_main("unit"))?;
    validate_source_directory_tree(&directory.0, Limits::default().max_dir_children)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    Ok(())
}

#[test]
fn loader_cycle_diagnostic_retains_deterministic_related_import_spans() -> std::io::Result<()> {
    let directory = TempDir::new("cycle")?;
    let root = directory.0.join("main.lkjscript");
    let first = directory.0.join("first.lkjscript");
    let second = directory.0.join("second.lkjscript");
    fs::write(
        &root,
        format!(
            "{}{}",
            exact_import("first.lkjscript", "cycle"),
            unit_main("unit")
        ),
    )?;
    fs::write(&first, exact_import("second.lkjscript", "cycle"))?;
    fs::write(&second, exact_import("main.lkjscript", "cycle"))?;
    let error = load(&root, &Limits::default()).expect_err("import cycle");
    assert_eq!(error.code(), "LKJ-SRC-LOAD");
    assert_eq!(error.primary_span().start().line(), 2);
    assert_eq!(error.related_spans().len(), 2);
    let compact = error.render_compact_agent();
    assert!(compact.contains("related[0].label=earlier import in cycle"));
    assert!(compact.contains("related[1].label=earlier import in cycle"));
    assert_eq!(compact, error.render_compact_agent());
    Ok(())
}

#[test]
fn loader_retains_logical_origin_separate_from_canonical_host_path() -> std::io::Result<()> {
    let directory = TempDir::new("origin")?;
    let dependency = directory.0.join("dep.lkjscript");
    let root = directory.0.join("main.lkjscript");
    fs::write(&dependency, named_def("helper"))?;
    fs::write(
        &root,
        format!(
            "{}{}",
            exact_import("dep.lkjscript", "helper"),
            unit_main("unit")
        ),
    )?;
    let tree = load(&root, &Limits::default())
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    assert_eq!(tree.source_origins().len(), 2);
    for origin in tree.source_origins() {
        assert!(!origin.logical_path().starts_with('/'));
        let host = origin.host_containment_path().expect("loaded host origin");
        assert!(host.is_absolute());
        assert_eq!(host.canonicalize()?, host);
    }
    assert_eq!(tree.root_origin().logical_path(), "main.lkjscript");
    assert!(tree
        .declarations()
        .iter()
        .any(|declaration| declaration.kind() == DeclarationKind::Function));
    Ok(())
}

#[test]
fn loader_rejects_wide_entry_directories_and_accepts_imported_declarations() -> std::io::Result<()>
{
    let wide = TempDir::new("wide-entry")?;
    let entry = wide.0.join("main.lkjscript");
    fs::write(&entry, unit_main("unit"))?;
    for index in 0..16 {
        fs::write(wide.0.join(format!("asset-{index}")), "")?;
    }
    let error = load(&entry, &Limits::default())
        .expect_err("wide source directory")
        .to_string();
    assert!(error.contains("at least 17 entries (max 16)"));

    let declarations = TempDir::new("imported-declarations")?;
    let dependency = declarations.0.join("traits.lkjscript");
    fs::write(
        &dependency,
        "trait/\nname/\nmarked\n/name\n/trait\nproduct/\nname/\nitem\n/name\nfields/\n/fields\n/product\nimpl/\ntrait/\nmarked\n/trait\nfor/\nproduct\nitem\n/for\n/impl\n",
    )?;
    let root = declarations.0.join("main.lkjscript");
    fs::write(
        &root,
        format!(
            "{}{}",
            exact_import("traits.lkjscript", "marked"),
            unit_main("unit")
        ),
    )?;
    let tree = load(&root, &Limits::default())
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    assert_eq!(tree.source_origins().len(), 2);
    assert_eq!(tree.declarations().len(), 4);
    Ok(())
}
