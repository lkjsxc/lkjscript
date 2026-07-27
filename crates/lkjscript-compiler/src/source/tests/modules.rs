use super::*;

fn import(path: &str, names: &[&str]) -> String {
    format!(
        concat!(
            "imports/\nimport/\nmodule/\n{path}\n/module\n",
            "declarations/\n{}\n/declarations\n/import\n/imports\n"
        ),
        names.join("\n"),
        path = path
    )
}

fn exported_def(export: Option<&str>, name: &str, body: &str) -> String {
    let visibility = export.map_or("", |_| "public\n");
    format!(
        "def/\nname/\n{name}\n/name\n{visibility}fn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nparams/\n/params\n{body}\n/fn\n/def\n"
    )
}

#[test]
fn declarations_are_private_until_explicitly_exported() -> std::io::Result<()> {
    let directory = TempDir::new("private-module")?;
    fs::write(
        directory.0.join("library.lkjscript"),
        exported_def(None, "helper", "unit"),
    )?;
    let root = directory.0.join("main.lkjscript");
    fs::write(
        &root,
        format!(
            "{}{}",
            import("library.lkjscript", &["helper"]),
            unit_main("helper/\n/helper")
        ),
    )?;
    let error = crate::compile_path(&root, &Limits::default()).expect_err("private import");
    assert!(error.to_string().contains("private or absent"));
    Ok(())
}

#[test]
fn exact_import_list_does_not_expose_other_public_names() -> std::io::Result<()> {
    let directory = TempDir::new("exact-import")?;
    let library = concat!(
        "def/\nname/\nallowed\n/name\npublic\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nparams/\n/params\nunit\n/fn\n/def\n",
        "def/\nname/\nsecret\n/name\npublic\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nparams/\n/params\nunit\n/fn\n/def\n"
    );
    fs::write(directory.0.join("library.lkjscript"), library)?;
    let root = directory.0.join("main.lkjscript");
    fs::write(
        &root,
        format!(
            "{}{}",
            import("library.lkjscript", &["allowed"]),
            unit_main("secret/\n/secret")
        ),
    )?;
    let error = crate::compile_path(&root, &Limits::default()).expect_err("unimported name");
    assert!(error.to_string().contains("unknown call secret"));
    Ok(())
}

#[test]
fn equal_private_names_coexist_in_distinct_loaded_modules() -> std::io::Result<()> {
    let directory = TempDir::new("module-local-equality")?;
    fs::write(
        directory.0.join("a.lkjscript"),
        exported_def(Some("helper"), "helper", "unit"),
    )?;
    fs::write(
        directory.0.join("b.lkjscript"),
        exported_def(Some("helper"), "helper", "unit"),
    )?;
    fs::write(
        directory.0.join("left.lkjscript"),
        format!(
            "{}{}",
            import("a.lkjscript", &["helper"]),
            exported_def(Some("left"), "left", "helper/\n/helper")
        ),
    )?;
    fs::write(
        directory.0.join("right.lkjscript"),
        format!(
            "{}{}",
            import("b.lkjscript", &["helper"]),
            exported_def(Some("right"), "right", "helper/\n/helper")
        ),
    )?;
    let root = directory.0.join("main.lkjscript");
    let imports = format!(
        "{}{}",
        import("left.lkjscript", &["left"]),
        import("right.lkjscript", &["right"])
    );
    fs::write(
        &root,
        format!(
            "{imports}{}",
            unit_main("do/\nleft/\n/left\nright/\n/right\n/do")
        ),
    )?;
    crate::compile_path(&root, &Limits::default()).expect("module-local duplicate names");
    Ok(())
}

#[test]
fn wildcard_import_is_rejected_by_the_source_grammar() {
    let source = format!(
        "{}{}",
        import("library.lkjscript", &["*"]),
        unit_main("unit")
    );
    let error = validate(&source, "main.lkjscript", &Limits::default()).expect_err("wildcard");
    assert_eq!(error.code(), "LKJ-SRC-SYNTAX");
}
