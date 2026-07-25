use super::*;

#[test]
fn accepts_only_canonical_source_extension() {
    assert!(ensure_source_path(Path::new("main.lkjscript")).is_ok());
    assert!(ensure_source_path(Path::new("main.lkjml")).is_err());
    assert!(ensure_source_path(Path::new("main")).is_err());
}

#[test]
fn public_in_memory_apis_require_canonical_relative_lkjscript_paths() {
    let source = unit_main("unit");
    for rejected in [
        "../escape.lkjscript",
        "./aliased.lkjscript",
        "src//aliased.lkjscript",
        "/absolute.lkjscript",
        "legacy.lkjml",
    ] {
        assert!(
            validate_source(&source, rejected, &Limits::default()).is_err(),
            "validate_source accepted {rejected}"
        );
        assert!(
            compile_source(&source, rejected, &Limits::default()).is_err(),
            "compile_source accepted {rejected}"
        );
    }
    validate_source(&source, "src/canonical.lkjscript", &Limits::default())
        .expect("validate canonical logical path");
    compile_source(&source, "src/canonical.lkjscript", &Limits::default())
        .expect("compile canonical logical path");
}
