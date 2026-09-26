//! A complete native declaration collection cannot select the same accepted owner twice.
use super::*;

#[test]
fn native_literal_edit_rejects_cross_scope_overlap_before_plan_or_publication() {
    let public = Native::new();
    let source = format!(
        r#"request base={}
declarations.begin
(units
  (module create overlap
    (function create pair (visibility public)
      (returns (record (left I64) (right I64))) (effect pure)
      (body (record structural (field left (i64 1)) (field right (i64 2)))))))
declarations.end
"#,
        public.revision()
    );
    let create = public.input("create.lkjc", &source);
    let plan = public.plan(&create, true);
    public.apply(&create, &plan, true);
    let module = find(&public, "module", "overlap", None);
    let function = find(&public, "declaration", "pair", Some(&module));
    let base = public.revision();
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let first = format!(
        r#"(module edit {module} overlap
      (function edit {function} pair (visibility public)
        (returns (record (left I64) (right I64))) (effect pure)
        (body (record structural (field left (i64 10)) (field right (i64 2))))))"#
    );
    let single = public.input(
        "single.lkjc",
        &format!("request base={base}\ndeclarations.begin\n(units {first})\ndeclarations.end\n"),
    );
    let review = public.plan(&single, true);
    for separated in [false, true] {
        for (left, right) in [(1, 20), (1, 2), (10, 2)] {
            let boundary = if separated {
                ")\ndeclarations.end\ndeclarations.begin\n(units"
            } else {
                ""
            };
            let source = format!(
                r#"request base={base}
declarations.begin
(units {first}
  {boundary}
  (function edit {function} pair (in {module}) (visibility public)
    (returns (record (left I64) (right I64))) (effect pure)
    (body (record structural (field left (i64 {left})) (field right (i64 {right}))))))
declarations.end
"#
            );
            let input = public.input("overlap.lkjc", &source);
            let output = public.root.path().join("rejected.lkjplan");
            let result = public.cli(
                &[
                    "change",
                    "plan",
                    "--input-file",
                    path(&input),
                    "--output",
                    path(&output),
                ],
                false,
            );
            assert_eq!(
                compact_field(compact_record(&result, "diagnostic"), "code"),
                "change_unit_duplicate"
            );
            assert!(!output.exists());
            public.apply(&input, &review, false);
            assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
            assert_eq!(public.revision(), base);
        }
    }
    public.apply(&single, &review, true);
    assert_ne!(public.revision(), base);
    assert_eq!(
        find(&public, "declaration", "pair", Some(&module)),
        function
    );
    eprintln!(
        "native overlap proof: six ambiguous selections rejected in plan and apply; no output or head changes; exact valid review still applies"
    );
}
