//! Native graph edits retain identities while invalidating the selected implementation's consumers.
use super::*;

const LIBRARY: &str = include_str!("../fixtures/owned-witness-library.lkjc");
const CELL: &str = include_str!("../fixtures/owned-witness-cell.lkjc");
const CONSUMER: &str = r#"declarations.begin
(units
  (module create verification
    (test create selected-scalar (visibility private)
      (actual (let
        (binding value (type OwnedI64Cell)
          (implementation-call abstraction::produce (types OwnedI64Cell)
            (implementations concrete@cell::Scalar) (i64 128)))
        (in (implementation-call abstraction::consume (types OwnedI64Cell)
          (implementations concrete@cell::Scalar) (local value)))))
      (expected (i64 128)))))
declarations.end
"#;

#[test]
fn native_owned_implementation_edit_reselects_tests_and_invalidates_prepared_results() {
    let public = Native::new();
    let input = public.input(
        "create.lkjc",
        &format!(
            "request base={}\n{LIBRARY}\n{CELL}\n{CONSUMER}",
            public.revision()
        ),
    );
    let plan = public.plan(&input, true);
    public.apply(&input, &plan, true);
    public.cli(&["check"], true);
    let original = public.revision();
    let draft = public.root.path().join("cell-draft.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--module",
            "cell",
            "--output",
            path(&draft),
        ],
        true,
    );
    let unchanged = public.plan(&draft, true);
    assert_eq!(
        compact_field(compact_record(&unchanged, "result"), "outcome"),
        "unchanged"
    );
    let source = std::fs::read_to_string(&draft).unwrap();
    let read = draft_reference(&source, "read");
    let alternate = draft_reference(&source, "alternate-read");
    let from = format!("(method method_10000000000000000000000000000003 {read})");
    let to = format!("(method method_10000000000000000000000000000003 {alternate})");
    assert_eq!(source.matches(&from).count(), 1);
    let changed = public.input("remap.lkjc", &source.replacen(&from, &to, 1));
    let remap = public.plan(&changed, true);
    assert_eq!(
        compact_field(compact_record(&remap, "summary"), "updated"),
        "1"
    );
    assert_eq!(
        compact_field(compact_record(&remap, "validation"), "tests-selected"),
        "1"
    );
    assert_eq!(
        compact_field(compact_record(&remap, "validation"), "tests-passed"),
        "0"
    );
    assert_eq!(public.revision(), original);

    // Planning selects tests; it does not execute them. Change only the mapping,
    // preserving its declaration identity and every generic function body.
    public.apply(&changed, &remap, true);
    assert_ne!(public.revision(), original);
    let failure = public.cli(&["check"], false);
    assert_eq!(
        compact_field(compact_record(&failure, "diagnostic"), "code"),
        "normalized_test_failed"
    );

    // The unchanged selected witness now dispatches to the alternative method.
    // Repair the expected result through a second native edit and require both
    // evaluators to observe exactly 99, not the formerly prepared result 128.
    let expected = public.root.path().join("verification-draft.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--module",
            "verification",
            "--output",
            path(&expected),
        ],
        true,
    );
    let source = std::fs::read_to_string(&expected).unwrap();
    assert_eq!(source.matches("(expected (i64 128))").count(), 1);
    let corrected = public.input(
        "expected-99.lkjc",
        &source.replacen("(expected (i64 128))", "(expected (i64 99))", 1),
    );
    let plan = public.plan(&corrected, true);
    public.apply(&corrected, &plan, true);
    let checked = public.cli(&["check"], true);
    let tests = compact_record(&checked, "tests");
    assert_eq!(compact_field(tests, "passed"), "1");
    assert_eq!(compact_field(tests, "failed"), "0");
    assert_eq!(compact_field(tests, "differential"), "equal");
}
