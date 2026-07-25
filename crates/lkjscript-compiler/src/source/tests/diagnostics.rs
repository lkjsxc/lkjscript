use super::*;

#[test]
fn exact_utf8_byte_line_column_spans_are_retained() {
    let source = unit_main("str/\néλ\n/str");
    let tree = validate(&source, "src/utf8.lkjscript", &Limits::default()).expect("validate");
    let string = tree
        .nodes()
        .iter()
        .find(|node| node.kind() == NodeKind::StringLiteral)
        .expect("string node");
    let span = string.span();
    let start = source.find("str/\n").expect("string open");
    let end = source.find("\n/str").expect("string close") + "\n/str".len();
    assert_eq!(span.start().byte() as usize, start);
    assert_eq!(span.end().byte() as usize, end);
    assert_eq!(span.start().line(), 6);
    assert_eq!(span.start().column(), 1);
    assert_eq!(span.end().line(), 8);
    assert_eq!(span.end().column(), 5);
}

#[test]
fn marker_diagnostics_have_stable_schema_spans_and_renderings() {
    let mismatched =
        validate("main/\n/wrong\n", "bad.lkjscript", &Limits::default()).expect_err("mismatch");
    assert_eq!(
        mismatched.schema(),
        "lkjscript.source-diagnostic-foundation"
    );
    assert_eq!(mismatched.schema_version(), 1);
    assert_eq!(mismatched.code(), "LKJ-SRC-UNMATCHED-MARKER");
    assert_eq!(mismatched.primary_span().start().line(), 2);
    assert_eq!(mismatched.related_spans().len(), 1);
    assert!(mismatched.render_human().contains("expected /main"));
    let compact = mismatched.render_compact_agent();
    assert!(compact.starts_with(
        "schema=lkjscript.source-diagnostic-foundation;version=1;code=LKJ-SRC-UNMATCHED-MARKER"
    ));
    assert!(compact.contains("related[0].label=opening marker main/"));
    assert!(compact.contains("related[0].origin=bad.lkjscript"));
    assert_eq!(compact, mismatched.render_compact_agent());

    let unexpected =
        validate("/main\n", "bad.lkjscript", &Limits::default()).expect_err("unexpected close");
    assert_eq!(unexpected.code(), "LKJ-SRC-UNMATCHED-MARKER");
    assert_eq!(unexpected.primary_span().byte_range(), 0..5);

    let unclosed =
        validate("main/\n", "bad.lkjscript", &Limits::default()).expect_err("unclosed open");
    assert_eq!(unclosed.code(), "LKJ-SRC-UNMATCHED-MARKER");
    assert_eq!(unclosed.primary_span().start().line(), 1);
}

#[test]
fn lexical_and_numeric_malformed_boundaries_are_rejected() {
    for source in [
        "  one\n",
        "one two\n",
        "\"hi\"\n",
        "main/\n/main\n",
        "def/\nname/\nx\n/name\nfn/\n/fn\n/def\n",
    ] {
        assert!(validate(source, "bad.lkjscript", &Limits::default()).is_err());
    }
    for spelling in [
        "+1", "1e3", "1.", ".", "-.", "+.", ".5", "-.5", "+.5", "--1", "+-1", "1_000", "0x10",
        "1.2.3", "NaN", "+inf", "inf",
    ] {
        let source = unit_main(spelling);
        assert!(
            validate(&source, "numeric.lkjscript", &Limits::default()).is_err(),
            "accepted {spelling}"
        );
    }
    assert!(validate(
        &unit_main("-9223372036854775808"),
        "min.lkjscript",
        &Limits::default()
    )
    .is_ok());
    assert!(validate(
        &unit_main("9223372036854775807"),
        "max.lkjscript",
        &Limits::default()
    )
    .is_ok());
    assert!(validate(
        &unit_main("9223372036854775808"),
        "overflow.lkjscript",
        &Limits::default()
    )
    .is_err());
}

#[test]
fn duplicate_same_unit_global_declarations_are_structured_errors() {
    let source = format!(
        "{}{}{}",
        named_def("same"),
        named_def("same"),
        unit_main("unit")
    );
    let error = validate(&source, "src/duplicate.lkjscript", &Limits::default())
        .expect_err("duplicate key");
    assert_eq!(error.code(), "LKJ-DECL-DUPLICATE");
    assert_eq!(error.related_spans().len(), 1);
    assert!(error
        .render_human()
        .contains("duplicate function declaration same"));
}

#[test]
fn duplicate_global_names_across_source_units_are_structured_errors() {
    let temp = TempDir::new("duplicate-global").expect("temp directory");
    let root = temp.0.join("main.lkjscript");
    fs::write(temp.0.join("a.lkjscript"), named_def("same")).expect("write a");
    fs::write(temp.0.join("b.lkjscript"), named_def("same")).expect("write b");
    fs::write(
        &root,
        format!(
            "import/\n./a.lkjscript\n/import\nimport/\n./b.lkjscript\n/import\n{}",
            unit_main("unit")
        ),
    )
    .expect("write root");

    let error = load(&root, &Limits::default()).expect_err("duplicate global");
    assert_eq!(error.code(), "LKJ-DECL-DUPLICATE");
    assert_eq!(error.related_spans().len(), 1);
    assert!(error
        .render_human()
        .contains("duplicate function declaration same"));
}
