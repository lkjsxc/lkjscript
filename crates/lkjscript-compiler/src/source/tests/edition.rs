use super::*;
use crate::source::SourceEdition;

const MARKER: &str = "edition/\n2\n/edition\n";

fn edition2(body: &str) -> String {
    format!("{MARKER}{body}")
}

#[test]
fn exact_marker_is_structural_and_preserves_leading_trivia() {
    let source = format!("\n;; edition selection\n{MARKER}{}", unit_main("unit"));
    let tree = validate(&source, "src/main.lkjscript", &Limits::default()).expect("Edition 2");
    assert_eq!(tree.edition(), SourceEdition::Edition2);
    assert_eq!(
        tree.format_single_source().as_deref(),
        Some(source.as_str())
    );
    assert_eq!(tree.nodes()[0].kind(), NodeKind::EditionMarker);
    assert_eq!(tree.nodes()[0].children().len(), 1);
}

#[test]
fn marker_failures_are_strict_and_deterministic() {
    let missing = validate(
        "enum/\nname/\nChoice\n/name\n/enum\n",
        "src/main.lkjscript",
        &Limits::default(),
    )
    .expect_err("Edition 2-only declaration needs marker");
    assert_eq!(missing.code(), "LKJ-SRC-EDITION");

    let misordered = format!("{}{MARKER}", unit_main("unit"));
    let error = validate(&misordered, "src/main.lkjscript", &Limits::default())
        .expect_err("misordered marker");
    assert_eq!(error.code(), "LKJ-SRC-EDITION");
    assert!(error.message().contains("first semantic form"));

    let duplicate = format!("{MARKER}{MARKER}{}", unit_main("unit"));
    let error = validate(&duplicate, "src/main.lkjscript", &Limits::default())
        .expect_err("duplicate marker");
    assert_eq!(error.code(), "LKJ-SRC-EDITION");
    assert!(error.message().contains("duplicate"));
}

#[test]
fn malformed_marker_spellings_are_rejected() {
    let malformed = [
        "edition/\n1\n/edition\n",
        "edition/\n02\n/edition\n",
        "edition/\n;; no interior trivia\n2\n/edition\n",
        "edition/\n2\n;; no interior trivia\n/edition\n",
        "edition/\n2\n/edition",
        "edition/\r\n2\r\n/edition\r\n",
    ];
    for marker in malformed {
        let source = format!("{marker}{}", unit_main("unit"));
        let error = validate(&source, "src/main.lkjscript", &Limits::default())
            .expect_err("malformed marker");
        assert_eq!(
            error.category(),
            crate::source::DiagnosticCategory::SourceSyntax,
            "{marker:?}"
        );
    }
}

#[test]
fn marker_does_not_consume_declaration_limit_but_remains_inside_token_limit() {
    let source = edition2(&unit_main("unit"));
    let exact = Limits {
        max_toplevel_forms: 1,
        max_tokens_per_file: 10,
        ..Limits::default()
    };
    validate(&source, "src/main.lkjscript", &exact).expect("exact declaration and token budgets");
    let token_error = validate(
        &source,
        "src/main.lkjscript",
        &Limits {
            max_tokens_per_file: 9,
            ..Limits::default()
        },
    )
    .expect_err("marker tokens remain charged");
    assert_eq!(token_error.code(), "LKJ-SRC-LIMIT");

    let plus_one = format!(
        "{MARKER}import/\ndep.lkjscript\n/import\n{}",
        unit_main("unit")
    );
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
fn editions_separate_source_tree_revision_node_and_declaration_identity() {
    let body = unit_main("unit");
    let first = validate(&body, "src/main.lkjscript", &Limits::default()).expect("Edition 1");
    let second =
        validate(&edition2(&body), "src/main.lkjscript", &Limits::default()).expect("Edition 2");
    assert_ne!(
        first.source_identity("src/main.lkjscript"),
        second.source_identity("src/main.lkjscript")
    );
    assert_ne!(first.identity(), second.identity());
    assert_ne!(first.revision(), second.revision());
    assert_ne!(
        first.nodes()[0].id().revision(),
        second.nodes()[0].id().revision()
    );
    assert_ne!(
        first.declarations()[0].key(),
        second.declarations()[0].key()
    );
}

#[test]
fn edition2_formats_roundtrips_and_executes_existing_declarations() {
    let source = edition2(&unit_main("unit"));
    let first = validate(&source, "src/main.lkjscript", &Limits::default()).expect("parse");
    let formatted = first.format_single_source().expect("single source");
    let second = validate(&formatted, "src/main.lkjscript", &Limits::default()).expect("roundtrip");
    assert_eq!(first.revision(), second.revision());
    crate::compile_source(&formatted, "src/main.lkjscript", &Limits::default())
        .expect("Edition 2 existing declaration execution slice");
}

#[test]
fn exact_enum_declaration_shape_roundtrips_without_aliases() {
    let declaration = "enum/\nname/\nChoice\n/name\nvariants/\nvariant/\nname/\nOnly\n/name\nfields/\nvariant-field/\nname/\nvalue\n/name\ntype/\nI64\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\n";
    let source = edition2(&format!("{declaration}{}", unit_main("unit")));
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
        let source = edition2(malformed);
        let error = validate(&source, "src/main.lkjscript", &Limits::default())
            .expect_err("empty variants and field aliases reject");
        assert_eq!(error.code(), "LKJ-SRC-SYNTAX");
    }
}

#[test]
fn mixed_import_closures_reject_before_compilation_or_migration() -> std::io::Result<()> {
    for root_is_edition2 in [false, true] {
        let directory = TempDir::new("mixed-edition")?;
        let dependency = directory.0.join("dep.lkjscript");
        let root = directory.0.join("main.lkjscript");
        let dep_source = if root_is_edition2 {
            named_def("helper")
        } else {
            edition2(&named_def("helper"))
        };
        let root_body = format!("import/\n./dep.lkjscript\n/import\n{}", unit_main("unit"));
        let root_source = if root_is_edition2 {
            edition2(&root_body)
        } else {
            root_body
        };
        fs::write(dependency, dep_source)?;
        fs::write(&root, root_source)?;
        let error = load(&root, &Limits::default()).expect_err("mixed closure");
        assert_eq!(error.code(), "LKJ-SRC-MIXED-EDITION");
    }
    Ok(())
}
