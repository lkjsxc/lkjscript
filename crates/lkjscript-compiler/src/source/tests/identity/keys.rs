use super::super::*;

#[test]
fn declaration_keys_ignore_order_offsets_and_nonsemantic_trivia() {
    let first = format!("{}{}{}", named_def("a"), named_def("b"), unit_main("unit"));
    let second = format!(
        ";; leading\n\n{}\n{}{}",
        named_def("b"),
        named_def("a"),
        unit_main("unit")
    );
    let first = validate(&first, "src/keys.lkjscript", &Limits::default()).expect("first");
    let second = validate(&second, "src/keys.lkjscript", &Limits::default()).expect("second");
    for name in ["a", "b", "$main"] {
        let left = first
            .declarations()
            .iter()
            .find(|declaration| declaration.name() == name)
            .expect("first declaration");
        let right = second
            .declarations()
            .iter()
            .find(|declaration| declaration.name() == name)
            .expect("second declaration");
        assert_eq!(left.key(), right.key());
    }
    assert_ne!(first.revision(), second.revision());
}

#[test]
fn exact_source_spelling_and_line_endings_change_revisions_and_reject_stale_nodes() {
    let numeric_one =
        validate(&unit_main("1.0"), "src/alias.lkjscript", &Limits::default()).expect("1.0");
    let numeric_two = validate(
        &unit_main("1.00"),
        "src/alias.lkjscript",
        &Limits::default(),
    )
    .expect("1.00");
    assert_ne!(numeric_one.revision(), numeric_two.revision());
    assert_eq!(
        numeric_one.format_single_source(),
        numeric_two.format_single_source()
    );
    let stale_numeric = numeric_two
        .node(numeric_one.nodes()[0].id())
        .expect_err("numeric alias NodeId must be stale");
    assert_eq!(stale_numeric.actual_revision(), numeric_one.revision());
    assert_eq!(stale_numeric.expected_revision(), numeric_two.revision());

    let lf = unit_main("unit");
    let crlf = lf.replace('\n', "\r\n");
    let lf = validate(&lf, "src/endings.lkjscript", &Limits::default()).expect("LF");
    let crlf = validate(&crlf, "src/endings.lkjscript", &Limits::default()).expect("CRLF");
    assert_ne!(lf.revision(), crlf.revision());
    assert_eq!(lf.format_single_source(), crlf.format_single_source());
    assert!(crlf.node(lf.nodes()[0].id()).is_err());
}

#[test]
fn declaration_key_framing_prevents_delimiter_and_path_false_collisions() {
    let left_path = "src/a.lkjscript";
    let left_name = "x.lkjscript;kind=trait;name=y";
    let right_path = "src/a.lkjscript;kind=function;name=x.lkjscript";
    let right_name = "y";
    let old_left = format!(
        "origin={left_path};kind={};name={left_name}",
        DeclarationKind::Function.as_str()
    );
    let old_right = format!(
        "origin={right_path};kind={};name={right_name}",
        DeclarationKind::Trait.as_str()
    );
    assert_eq!(old_left, old_right, "adversarial delimiter setup");
    assert_ne!(
        super::declaration_key_bytes(left_path, DeclarationKind::Function, left_name),
        super::declaration_key_bytes(right_path, DeclarationKind::Trait, right_name)
    );

    let human = super::declaration_key_human_identity(
        "src/a=b;path.lkjscript",
        DeclarationKind::Function,
        "callable=name",
    );
    assert!(human.starts_with(&format!(
        "contract={};package=root;",
        lkjscript_contracts::SOURCE_DIGEST
    )));
    assert!(human.contains("origin=src/a%3Db%3Bpath.lkjscript"));
    assert!(human.contains("name=callable%3Dname"));
    assert!(!human.contains("edition="));
    assert!(!human.contains("version="));
}

#[test]
fn declaration_names_must_be_spellable_source_identifiers_before_keying() {
    let sources = [
        named_def("uncallable;name"),
        "product/\nname/\nuncallable;name\n/name\nfields/\n/fields\n/product\n".into(),
        "trait/\nname/\nuncallable;name\n/name\n/trait\n".into(),
    ];
    for source in sources {
        let error = validate(&source, "src/name.lkjscript", &Limits::default())
            .expect_err("uncallable declaration name");
        assert_eq!(error.code(), "LKJ-DECL-NAME");
        assert!(error
            .message()
            .contains("not a spellable source identifier"));
    }

    let callable = validate(
        &named_def("callable=name"),
        "src/name.lkjscript",
        &Limits::default(),
    )
    .expect("spellable equals name");
    assert_eq!(callable.declarations()[0].name(), "callable=name");
}

#[test]
fn distinct_source_units_cannot_share_one_logical_origin() {
    let origin = SourceOrigin::in_memory("src/duplicate-origin.lkjscript");
    let first = parser::parse_file(
        &named_def("first"),
        origin.clone(),
        PathBuf::from("host-a.lkjscript"),
        &Limits::default(),
    )
    .expect("first source");
    let second = parser::parse_file(
        &named_def("second"),
        origin.clone(),
        PathBuf::from("host-b.lkjscript"),
        &Limits::default(),
    )
    .expect("second source");
    let error = finish_tree(
        PathBuf::from("host-a.lkjscript"),
        origin,
        vec![first, second],
    )
    .expect_err("duplicate logical origin");
    assert_eq!(error.code(), "LKJ-SRC-LOAD");
    assert_eq!(error.related_spans().len(), 1);
    assert!(error.message().contains("duplicate logical origin"));
}
