//! Independent public regressions: ownership is checked before any representation is chosen.
use super::*;

const ABSTRACT: &str = include_str!("../fixtures/owned-parameters-abstract.lkjc");

#[test]
fn native_owned_parameters_work_without_concrete_carriers_or_dependencies() {
    let public = Native::new();
    let input = public.input(
        "abstract.lkjc",
        &format!("request base={}\n{ABSTRACT}", public.revision()),
    );
    let plan = public.plan(&input, true);
    public.apply(&input, &plan, true);
    public.cli(&["check"], true);
    let status = public.cli(&["status"], true);
    assert_eq!(
        compact_field(compact_record(&status, "summary"), "dependencies"),
        "0"
    );
    // This library has no standard import, concrete memory type, implementation, or caller.
    let artifact = public.root.path().join("abstract.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let transport = public.root.path().join("abstract.lkjp");
    public.cli(
        &[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            path(&transport),
        ],
        true,
    );
    let draft = public.root.path().join("abstract-draft.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--module",
            "abstract-memory",
            "--output",
            path(&draft),
        ],
        true,
    );
    let text = std::fs::read_to_string(&draft).unwrap();
    assert_eq!(text.matches("(constraint owned)").count(), 5);
    assert!(!text.contains("ByteBuffer"));
    assert!(!text.contains("OwnedI64Cell"));
    let unchanged = public.plan(&draft, true);
    assert_eq!(
        compact_field(compact_record(&unchanged, "result"), "outcome"),
        "unchanged"
    );
}

#[test]
fn native_owned_parameters_reject_symbolic_copy_loans_aliases_and_containers() {
    let public = Native::new();
    let before = public.revision();
    const DISCARD: &str = r#"
      (function create discard (visibility private)
        (type-parameter create T (constraint owned))
        (parameter create value (type T) (use consume))
        (returns Unit) (effect pure) (body (unit)))
    "#;
    let cases = [
        (
            "twice",
            r#"
          (function create bad (visibility private)
            (type-parameter create T (constraint owned))
            (parameter create value (type T) (use consume))
            (returns Unit) (effect pure)
            (body (sequence (call discard (types T) (local value))
                            (call discard (types T) (local value)))))"#,
        ),
        (
            "loan-result",
            r#"
          (function create bad (visibility private)
            (type-parameter create T (constraint owned))
            (parameter create value (type T) (use borrow))
            (returns T) (effect pure) (body (local value)))"#,
        ),
        (
            "consume-loan",
            r#"
          (function create bad (visibility private)
            (type-parameter create T (constraint owned))
            (parameter create value (type T) (use borrow))
            (returns Unit) (effect pure)
            (body (call discard (types T) (local value))))"#,
        ),
        (
            "borrow-consume-alias",
            r#"
          (function create pair (visibility private)
            (type-parameter create T (constraint owned))
            (parameter create a (type T) (use borrow))
            (parameter create b (type T) (use consume))
            (returns Unit) (effect pure) (body (unit)))
          (function create bad (visibility private)
            (type-parameter create T (constraint owned))
            (parameter create value (type T) (use consume))
            (returns Unit) (effect pure)
            (body (call pair (types T) (local value) (local value))))"#,
        ),
        (
            "consume-borrow-alias",
            r#"
          (function create pair (visibility private)
            (type-parameter create T (constraint owned))
            (parameter create a (type T) (use consume))
            (parameter create b (type T) (use borrow))
            (returns Unit) (effect pure) (body (unit)))
          (function create bad (visibility private)
            (type-parameter create T (constraint owned))
            (parameter create value (type T) (use consume))
            (returns Unit) (effect pure)
            (body (call pair (types T) (local value) (local value))))"#,
        ),
        (
            "branch-move",
            r#"
          (function create bad (visibility private)
            (type-parameter create T (constraint owned))
            (parameter create choose (type Bool))
            (parameter create value (type T) (use consume))
            (returns T) (effect pure)
            (body (sequence
              (if (local choose) (call discard (types T) (local value)) (unit))
              (local value))))"#,
        ),
        (
            "missing-annotation",
            r#"
          (function create bad (visibility private)
            (type-parameter create T (constraint owned))
            (parameter create value (type T) (use consume))
            (returns T) (effect pure)
            (body (let (binding alias (local value)) (in (local alias)))))"#,
        ),
        (
            "ordinary-argument",
            r#"
          (function create ordinary (visibility private)
            (type-parameter create U)
            (parameter create value (type U))
            (returns U) (effect pure) (body (local value)))
          (function create bad (visibility private)
            (type-parameter create T (constraint owned))
            (parameter create value (type T) (use consume))
            (returns T) (effect pure)
            (body (call ordinary (types T) (local value))))"#,
        ),
        (
            "capture-safe-argument",
            r#"
          (function create ordinary (visibility private)
            (type-parameter create U (constraint capture-safe))
            (parameter create value (type U))
            (returns U) (effect pure) (body (local value)))
          (function create bad (visibility private)
            (type-parameter create T (constraint owned))
            (parameter create value (type T) (use consume))
            (returns T) (effect pure)
            (body (call ordinary (types T) (local value))))"#,
        ),
        (
            "empty-owned-container",
            r#"
          (function create bad (visibility private)
            (type-parameter create T (constraint owned))
            (returns (list T)) (effect pure) (body (list T)))"#,
        ),
    ];
    for (name, body) in cases {
        let input = public.input(
            &format!("{name}.lkjc"),
            &format!("request base={before}\ndeclarations.begin\n(units (module create rejection {DISCARD} {body}))\ndeclarations.end\n"),
        );
        let failure = public.plan(&input, false);
        let code = compact_field(compact_record(&failure, "diagnostic"), "code");
        assert!(
            code.starts_with("kernel_buffer")
                || code.starts_with("kernel_owned")
                || code.starts_with("kernel_affine"),
            "expected ownership admission, not syntax failure, for {name}: {code}"
        );
        assert_eq!(public.revision(), before, "{name}");
    }
}

#[test]
fn native_owned_constraint_change_rechecks_previously_valid_ordinary_body() {
    let public = Native::new();
    let input = public.input(
        "ordinary.lkjc",
        &format!(
            r#"request base={}
declarations.begin
(units (module create changing
  (function create duplicate (visibility public)
    (type-parameter create T)
    (parameter create value (type T))
    (returns T) (effect pure)
    (body (sequence (local value) (local value))))))
declarations.end
"#,
            public.revision()
        ),
    );
    let plan = public.plan(&input, true);
    public.apply(&input, &plan, true);
    public.cli(&["check"], true);
    let before = public.revision();
    let draft = public.root.path().join("changing.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--module",
            "changing",
            "--output",
            path(&draft),
        ],
        true,
    );
    let text = std::fs::read_to_string(&draft).unwrap();
    assert_eq!(text.matches("(constraint none)").count(), 1);
    assert_eq!(text.matches("(use unrestricted)").count(), 1);
    // Preserve the existing type-parameter identity and body. Change only its contract.
    let changed = text
        .replacen("(constraint none)", "(constraint owned)", 1)
        .replacen("(use unrestricted)", "(use consume)", 1);
    let input = public.input("changing-owned.lkjc", &changed);
    let failure = public.plan(&input, false);
    let code = compact_field(compact_record(&failure, "diagnostic"), "code");
    assert!(
        code.starts_with("kernel_buffer")
            || code.starts_with("kernel_owned")
            || code.starts_with("kernel_affine"),
        "ownership change must invalidate the formerly reusable ordinary body: {code}"
    );
    assert_eq!(public.revision(), before);
    public.cli(&["check"], true);
}
