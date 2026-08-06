use super::*;

#[test]
fn exact_utf8_byte_line_column_spans_are_retained() {
    let source = unit_main("string-literal/\néλ\n/string-literal");
    let tree = validate(&source, "src/utf8.lkjscript").expect("validate");
    let string = tree
        .nodes()
        .iter()
        .find(|node| node.kind() == NodeKind::StringLiteral)
        .expect("string node");
    let span = string.span();
    let start = source.find("string-literal/\n").expect("string open");
    let end = source.find("\n/string-literal").expect("string close") + "\n/string-literal".len();
    assert_eq!(usize::try_from(span.start().byte()).ok(), Some(start));
    assert_eq!(usize::try_from(span.end().byte()).ok(), Some(end));
    assert_eq!(span.start().line(), 9);
    assert_eq!(span.start().column(), 1);
    assert_eq!(span.end().line(), 11);
    assert_eq!(span.end().column(), 16);
}

#[test]
fn every_removed_spelling_has_a_deterministic_replacement_diagnostic() {
    for removed in lkjscript_contracts::REMOVED_SPELLINGS {
        let expression = if removed.old == "->" {
            removed.old.to_string()
        } else {
            format!("{}/\n/{}", removed.old, removed.old)
        };
        let source = unit_main(&expression);
        let error =
            validate(&source, "removed.lkjscript").expect_err("removed spelling must be rejected");
        let message = error.to_string();
        assert!(message.contains(removed.old), "{}: {message}", removed.old);
        assert!(
            message.contains(removed.replacement),
            "{}: {message}",
            removed.old
        );
    }
}

#[test]
fn marker_diagnostics_have_stable_schema_spans_and_renderings() {
    let mismatched = validate("main/\n/wrong\n", "bad.lkjscript").expect_err("mismatch");
    assert_eq!(mismatched.schema(), lkjscript_contracts::DIAGNOSTICS);
    assert_eq!(
        mismatched.contract(),
        lkjscript_contracts::DIAGNOSTICS_DIGEST
    );
    assert_eq!(mismatched.code(), "LKJ-SRC-UNMATCHED-MARKER");
    assert_eq!(mismatched.primary_span().start().line(), 2);
    assert_eq!(mismatched.related_spans().len(), 1);
    assert!(mismatched.render_human().contains("expected /main"));
    let compact = mismatched.render_compact_agent();
    assert!(compact.starts_with(&format!(
        "schema={};contract={};code=LKJ-SRC-UNMATCHED-MARKER",
        lkjscript_contracts::DIAGNOSTICS,
        lkjscript_contracts::DIAGNOSTICS_DIGEST
    )));
    assert!(compact.contains("related[0].label=opening marker main/"));
    assert!(compact.contains("related[0].origin=bad.lkjscript"));
    assert_eq!(compact, mismatched.render_compact_agent());

    let unexpected = validate("/main\n", "bad.lkjscript").expect_err("unexpected close");
    assert_eq!(unexpected.code(), "LKJ-SRC-UNMATCHED-MARKER");
    assert_eq!(unexpected.primary_span().byte_range(), 0..5);

    let unclosed = validate("main/\n", "bad.lkjscript").expect_err("unclosed open");
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
        assert!(validate(source, "bad.lkjscript").is_err());
    }
    for spelling in [
        "+1", "1e3", "1.", ".", "-.", "+.", ".5", "-.5", "+.5", "--1", "+-1", "1_000", "0x10",
        "1.2.3", "NaN", "+inf", "inf",
    ] {
        let source = unit_main(spelling);
        assert!(
            validate(&source, "numeric.lkjscript").is_err(),
            "accepted {spelling}"
        );
    }
    assert!(validate(&unit_main("-9223372036854775808"), "min.lkjscript").is_ok());
    assert!(validate(&unit_main("9223372036854775807"), "max.lkjscript").is_ok());
    assert!(validate(&unit_main("9223372036854775808"), "overflow.lkjscript").is_err());
}

#[test]
fn duplicate_same_unit_global_declarations_are_structured_errors() {
    let source = format!(
        "{}{}{}",
        named_def("same"),
        named_def("same"),
        unit_main("unit")
    );
    let error = validate(&source, "src/duplicate.lkjscript").expect_err("duplicate key");
    assert_eq!(error.code(), "LKJ-DECL-DUPLICATE");
    assert_eq!(error.related_spans().len(), 1);
    assert!(error
        .render_human()
        .contains("duplicate function declaration same"));
}

#[test]
fn equal_names_in_distinct_modules_are_valid_source_identities() {
    let temp = TempDir::new("duplicate-global").expect("temp directory");
    let root = temp.0.join("main.lkjscript");
    fs::write(temp.0.join("a.lkjscript"), named_def("same")).expect("write a");
    fs::write(temp.0.join("b.lkjscript"), named_def("same")).expect("write b");
    fs::write(
        &root,
        format!(
            "imports/\nimport/\nmodule/\na.lkjscript\n/module\ndeclarations/\nsame\n/declarations\n/import\nimport/\nmodule/\nb.lkjscript\n/module\ndeclarations/\nsame\n/declarations\n/import\n/imports\n{}",
            unit_main("unit")
        ),
    )
    .expect("write root");

    let tree = load(&root).expect("module-local names");
    let same = tree
        .declarations()
        .iter()
        .filter(|declaration| declaration.name() == "same")
        .count();
    assert_eq!(same, 2);
}
