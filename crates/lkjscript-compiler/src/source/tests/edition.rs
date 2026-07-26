use super::*;

const REMOVED_MARKER: &str = "edition/\n2\n/edition\n";

#[test]
fn canonical_source_is_marker_free_and_roundtrips() {
    let source = format!("\n;; canonical source\n{}", unit_main("unit"));
    let tree =
        validate(&source, "src/main.lkjscript", &Limits::default()).expect("canonical source");
    assert_eq!(
        tree.format_single_source().as_deref(),
        Some(source.as_str())
    );
    assert!(tree
        .nodes()
        .iter()
        .all(|node| node.kind() != NodeKind::Call || node.label() != Some("edition")));
    crate::compile_source(&source, "src/main.lkjscript", &Limits::default())
        .expect("canonical source compiles");
}

#[test]
fn removed_edition_form_is_rejected_not_selected() {
    let source = format!("{REMOVED_MARKER}{}", unit_main("unit"));
    let error = validate(&source, "src/main.lkjscript", &Limits::default())
        .expect_err("removed edition form");
    assert_eq!(error.code(), "LKJ-SRC-SYNTAX");
    assert!(!error.message().contains("select"));
}

#[test]
fn every_top_level_form_consumes_the_declaration_limit() {
    let source = unit_main("unit");
    validate(
        &source,
        "src/main.lkjscript",
        &Limits {
            max_toplevel_forms: 1,
            ..Limits::default()
        },
    )
    .expect("one declaration fits");
    let plus_one = format!("{}{source}", named_def("helper"));
    let error = validate(
        &plus_one,
        "src/main.lkjscript",
        &Limits {
            max_toplevel_forms: 1,
            ..Limits::default()
        },
    )
    .expect_err("one declaration over limit");
    assert_eq!(error.code(), "LKJ-SRC-LIMIT");
}

#[test]
fn source_identity_changes_with_exact_bytes_and_logical_path() {
    let source = unit_main("unit");
    let first = validate(&source, "src/main.lkjscript", &Limits::default()).expect("first");
    let changed = validate(
        &format!(";; comment\n{source}"),
        "src/main.lkjscript",
        &Limits::default(),
    )
    .expect("changed bytes");
    let moved = validate(&source, "src/moved.lkjscript", &Limits::default()).expect("moved");
    assert_ne!(first.revision(), changed.revision());
    assert_ne!(first.identity(), moved.identity());
    assert!(first.declarations()[0]
        .key()
        .canonical_identity()
        .starts_with(&format!(
            "contract={};package=root;",
            lkjscript_contracts::SOURCE_DIGEST
        )));
}

#[test]
fn exact_enum_declaration_shape_roundtrips_without_aliases() {
    let declaration = "enum/\nname/\nChoice\n/name\nvariants/\nvariant/\nname/\nOnly\n/name\nfields/\nvariant-field/\nname/\nvalue\n/name\ntype/\nI64\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\n";
    let source = format!("{declaration}{}", unit_main("unit"));
    let tree = validate(&source, "src/main.lkjscript", &Limits::default())
        .expect("exact enum declaration");
    assert_eq!(
        tree.format_single_source().as_deref(),
        Some(source.as_str())
    );
    assert_eq!(
        tree.declarations()
            .iter()
            .find(|item| item.kind() == DeclarationKind::Enum)
            .expect("enum declaration")
            .name(),
        "Choice"
    );

    for malformed in [
        "enum/\nname/\nChoice\n/name\nvariants/\n/variants\n/enum\n",
        "enum/\nname/\nChoice\n/name\nvariants/\nvariant/\nname/\nOnly\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\nI64\n/type\n/field\n/fields\n/variant\n/variants\n/enum\n",
    ] {
        let error = validate(malformed, "src/main.lkjscript", &Limits::default())
            .expect_err("empty variants and field aliases reject");
        assert_eq!(error.code(), "LKJ-SRC-SYNTAX");
    }
}

#[test]
fn canonical_import_closure_loads_without_language_selection() -> std::io::Result<()> {
    let directory = TempDir::new("canonical-closure")?;
    let dependency = directory.0.join("dep.lkjscript");
    let root = directory.0.join("main.lkjscript");
    fs::write(dependency, named_def("helper"))?;
    fs::write(
        &root,
        format!(
            "imports/\nimport/\ndep.lkjscript#helper\n/import\n/imports\n{}",
            unit_main("unit")
        ),
    )?;
    let tree = load(&root, &Limits::default()).expect("canonical closure");
    assert_eq!(tree.source_origins().len(), 2);
    Ok(())
}
