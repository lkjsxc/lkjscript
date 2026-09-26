//! Named draft selection is read-only; proposals retain exact owners and one base.
use super::*;

fn find(public: &Native, class: &str, name: &str, parent: Option<&str>) -> String {
    let mut args = vec!["query", "find", class, name];
    if let Some(parent) = parent {
        args.extend(["--parent", parent]);
    }
    compact_field(compact_record(&public.cli(&args, true), "owner"), "id").to_owned()
}

fn fixture() -> Native {
    let public = Native::new();
    let input = public.input(
        "create.lkjc",
        &format!("request base={}\n{COMPLETE}", public.revision()),
    );
    public.apply(&input, &public.plan(&input, true), true);
    public
}

fn draft(public: &Native, file: &str, selectors: &[&str]) -> String {
    let output = public.root.path().join(file);
    let mut args = vec!["change", "draft"];
    args.extend_from_slice(selectors);
    args.extend(["--output", path(&output)]);
    public.cli(&args, true);
    std::fs::read_to_string(output).unwrap()
}

#[test]
fn named_drafts_equal_exact_selections_and_keep_canonical_order() {
    let public = fixture();
    let module = find(&public, "module", "app", None);
    let hello = find(&public, "declaration", "hello", Some(&module));
    let target = find(&public, "target", "main", None);
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    for (index, (named, exact)) in [
        (vec!["--module", "app"], vec!["--owner", module.as_str()]),
        (
            vec!["--declaration", "app::hello"],
            vec!["--owner", hello.as_str()],
        ),
        (vec!["--target", "main"], vec!["--owner", target.as_str()]),
        (
            vec![
                "--target",
                "main",
                "--module",
                "app",
                "--declaration",
                "app::hello",
                "--owner",
                hello.as_str(),
                "--module",
                "app",
            ],
            vec!["--owner", module.as_str(), "--owner", target.as_str()],
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let named_file = format!("named-{index}.lkjc");
        let named_bytes = draft(&public, &named_file, &named);
        assert_eq!(
            named_bytes,
            draft(&public, &format!("exact-{index}.lkjc"), &exact)
        );
        let plan = public.plan(&public.root.path().join(named_file), true);
        assert_eq!(
            compact_field(compact_record(&plan, "result"), "outcome"),
            "unchanged"
        );
    }
    assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
}

#[test]
fn named_drafts_reject_missing_malformed_and_partial_selections_without_output() {
    let public = fixture();
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let output = public.root.path().join("absent.lkjc");
    for selectors in [
        vec!["--module", "missing"],
        vec!["--module", "main"],
        vec!["--target", "app"],
        vec!["--target", "missing"],
        vec!["--declaration", "app::missing"],
        vec!["--declaration", "missing::hello"],
        vec!["--declaration", "App::hello"],
        vec!["--declaration", "hello"],
        vec!["--declaration", "::hello"],
        vec!["--declaration", "app::"],
        vec!["--declaration", "app::hello::value"],
        vec!["--module", "app::hello"],
        vec!["--target", ""],
        vec!["--module", "app", "--target", "missing"],
    ] {
        let mut args = vec!["change", "draft"];
        args.extend(selectors);
        args.extend(["--output", path(&output)]);
        let result = public.cli(&args, false);
        assert_eq!(
            compact_field(compact_record(&result, "diagnostic"), "code"),
            "change_draft_selection"
        );
        assert!(!output.exists());
        assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    }
    public.cli(&["change", "draft", "--module"], false);
    public.cli(
        &[
            "change",
            "draft",
            "--module",
            "app",
            "--output",
            path(&output),
            "--bytes",
            "64",
        ],
        false,
    );
    assert!(!output.exists());
    let inside = public.project.join("draft.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--module",
            "app",
            "--output",
            path(&inside),
        ],
        false,
    );
    assert!(!inside.exists());
    std::fs::write(&output, "preserved").unwrap();
    public.cli(
        &[
            "change",
            "draft",
            "--module",
            "app",
            "--output",
            path(&output),
        ],
        false,
    );
    assert_eq!(std::fs::read_to_string(output).unwrap(), "preserved");
    assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
}

#[test]
fn named_drafts_keep_module_target_and_declaration_namespaces_distinct() {
    let public = fixture();
    let input = public.input(
        "namespaces.lkjc",
        &format!(
            r#"request base={}
declarations.begin
(units
  (module create main
    (constant create answer (visibility private) (type I64) (value (i64 7))))
  (module create other
    (function create hello (visibility private) (returns I64) (effect pure)
      (body (i64 9)))))
declarations.end
"#,
            public.revision()
        ),
    );
    public.apply(&input, &public.plan(&input, true), true);
    let module = find(&public, "module", "main", None);
    let target = find(&public, "target", "main", None);
    let other = find(&public, "module", "other", None);
    let hello = find(&public, "declaration", "hello", Some(&other));
    for (index, (selectors, owner)) in [
        (["--module", "main"], module),
        (["--target", "main"], target),
        (["--declaration", "other::hello"], hello),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            draft(&public, &format!("named-scope-{index}.lkjc"), &selectors),
            draft(
                &public,
                &format!("exact-scope-{index}.lkjc"),
                &["--owner", &owner]
            )
        );
    }
    let output = public.root.path().join("too-many.lkjc");
    let mut args = vec!["change", "draft", "--output", path(&output)];
    for _ in 0..10_001 {
        args.extend(["--module", "main"]);
    }
    let result = public.cli(&args, false);
    assert_eq!(
        compact_field(compact_record(&result, "diagnostic"), "code"),
        "change_draft_capacity"
    );
    assert!(!output.exists());
}

#[test]
fn named_drafts_do_not_retarget_an_old_proposal_after_rename() {
    let public = fixture();
    let module = find(&public, "module", "app", None);
    let hello = find(&public, "declaration", "hello", Some(&module));
    let original = draft(&public, "old.lkjc", &["--declaration", "app::hello"]);
    let renamed = public.input(
        "rename.lkjc",
        &format!(
            "request base={}\nrename.owner owner={} name=welcome\n",
            public.revision(),
            hello
        ),
    );
    public.apply(&renamed, &public.plan(&renamed, true), true);
    public.plan(&public.root.path().join("old.lkjc"), false);
    let fresh = draft(&public, "fresh.lkjc", &["--declaration", "app::welcome"]);
    assert!(original.contains(&format!("function edit {hello} hello")));
    assert!(fresh.contains(&format!("function edit {hello} welcome")));
    assert_eq!(fresh, draft(&public, "owner.lkjc", &["--owner", &hello]));
    let absent = public.root.path().join("old-name.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--declaration",
            "app::hello",
            "--output",
            path(&absent),
        ],
        false,
    );
    assert!(!absent.exists());
}
