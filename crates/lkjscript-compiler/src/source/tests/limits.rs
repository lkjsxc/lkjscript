use super::*;

#[test]
fn all_existing_source_limits_remain_enforced_at_boundaries() {
    let accepted = unit_main("unit");
    let limits = Limits {
        max_tokens_per_file: 10,
        max_toplevel_forms: 1,
        max_nest_depth: 3,
        max_children: 2,
        ..Limits::default()
    };
    assert!(validate(&accepted, "limit.lkjscript", &limits).is_ok());
    for limits in [
        Limits {
            max_tokens_per_file: 9,
            ..limits
        },
        Limits {
            max_nest_depth: 2,
            ..limits
        },
        Limits {
            max_children: 1,
            ..limits
        },
    ] {
        let error =
            validate(&accepted, "limit.lkjscript", &limits).expect_err("source limit boundary");
        assert_eq!(error.category().as_str(), "resource-limit");
    }
    let imports = format!(
        "imports/\nimport/\nmodule/\na.lkjscript\n/module\ndeclarations/\na\n/declarations\n/import\nimport/\nmodule/\nb.lkjscript\n/module\ndeclarations/\nb\n/declarations\n/import\n/imports\n{}",
        named_def("extra")
    );
    let error =
        validate(&imports, "limit.lkjscript", &limits).expect_err("top-level form boundary");
    assert_eq!(error.category().as_str(), "resource-limit");
}

#[test]
fn match_structural_markers_have_a_separate_hard_depth_bound() {
    let nesting = 33;
    let mut source =
        String::from("main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nmatch/\n");
    source.push_str(&"fields/\n".repeat(nesting));
    source.push_str("x\n");
    source.push_str(&"/fields\n".repeat(nesting));
    source.push_str("arms/\narm/\nwildcard/\n/wildcard\n0\n/arm\n/arms\n/match\n/main\n");
    let error = validate(&source, "match-depth.lkjscript", &Limits::default())
        .expect_err("match marker depth plus one");
    assert_eq!(error.category().as_str(), "resource-limit", "{error}");
    assert!(
        error.message().contains("match marker depth exceeded"),
        "{error}"
    );
}

#[test]
fn source_file_safety_maximum_checks_metadata_and_actual_read_without_large_input() {
    let origin = SourceOrigin::in_memory("src/limit.lkjscript");
    let budget = SourceFoundationBudget::default();
    assert!(budget
        .check_metadata(&origin, FOUNDATION_MAX_SOURCE_FILE_BYTES)
        .is_ok());
    let metadata_error = budget
        .check_metadata(&origin, FOUNDATION_MAX_SOURCE_FILE_BYTES + 1)
        .expect_err("metadata file-byte maximum");
    assert_eq!(metadata_error.code(), "LKJ-SRC-LIMIT");
    assert!(metadata_error
        .message()
        .contains("category=source-file-bytes"));

    let mut read_budget = SourceFoundationBudget::default();
    let read_error = read_budget
        .record_read(&origin, FOUNDATION_MAX_SOURCE_FILE_BYTES + 1)
        .expect_err("actual read file-byte maximum");
    assert!(read_error.message().contains("category=source-file-bytes"));
}

#[test]
fn bounded_reader_stops_at_remaining_allowance_plus_one_without_large_input() {
    let origin = SourceOrigin::in_memory("src/bounded.lkjscript");
    let budget = SourceFoundationBudget::with_usage(0, FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES - 2);
    budget
        .check_metadata(&origin, 2)
        .expect("two aggregate bytes remain");
    let mut reader = Cursor::new(vec![b'x'; 4]);
    let error = super::load::read_bounded_bytes(
        &mut reader,
        2,
        Path::new("src/bounded.lkjscript"),
        &origin,
        &budget,
    )
    .expect_err("sentinel byte must detect growth");
    assert_eq!(reader.position(), 3);
    assert!(error.message().contains("metadata=2; read=3"));

    let budget = SourceFoundationBudget::default();
    let mut shortened = Cursor::new(vec![b'x']);
    let error = super::load::read_bounded_bytes(
        &mut shortened,
        2,
        Path::new("src/shortened.lkjscript"),
        &origin,
        &budget,
    )
    .expect_err("metadata/read shrink must be deterministic");
    assert_eq!(shortened.position(), 1);
    assert!(error.message().contains("metadata=2; read=1"));
}

#[test]
fn aggregate_source_safety_maximum_uses_checked_arithmetic_without_large_input() {
    let origin = SourceOrigin::in_memory("src/limit.lkjscript");
    let budget = SourceFoundationBudget::with_usage(1, FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES - 1);
    assert!(budget.check_metadata(&origin, 1).is_ok());
    let error = budget
        .check_metadata(&origin, 2)
        .expect_err("aggregate byte maximum");
    assert!(error.message().contains("category=aggregate-source-bytes"));

    let overflow = SourceFoundationBudget::with_usage(1, u64::MAX)
        .check_metadata(&origin, 1)
        .expect_err("aggregate checked addition");
    assert!(overflow
        .message()
        .contains("attempted=18446744073709551615"));
}

#[test]
fn source_unit_safety_maximum_uses_checked_arithmetic_without_many_files() {
    let origin = SourceOrigin::in_memory("src/limit.lkjscript");
    let boundary = SourceFoundationBudget::with_usage(FOUNDATION_MAX_SOURCE_UNITS - 1, 0);
    assert!(boundary.check_metadata(&origin, 0).is_ok());
    let error = SourceFoundationBudget::with_usage(FOUNDATION_MAX_SOURCE_UNITS, 0)
        .check_metadata(&origin, 0)
        .expect_err("source-unit maximum");
    assert!(error.message().contains("category=source-units"));
    let overflow = SourceFoundationBudget::with_usage(u64::MAX, 0)
        .check_metadata(&origin, 0)
        .expect_err("source-unit checked addition");
    assert!(overflow
        .message()
        .contains("attempted=18446744073709551615"));
}
