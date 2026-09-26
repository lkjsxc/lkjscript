//! Literal-only native edits retain graph identity and exact review in a real detached web app.
#[path = "native_literal_list.rs"]
mod paged_list;
use super::native_http as http;
use super::*;

fn find(public: &Native, class: &str, name: &str, parent: Option<&str>) -> String {
    let mut arguments = vec!["query", "find", class, name];
    if let Some(parent) = parent {
        arguments.extend(["--parent", parent]);
    }
    compact_field(compact_record(&public.cli(&arguments, true), "owner"), "id").to_owned()
}

#[test]
fn native_literal_edit_keeps_review_identity_and_old_and_new_detached_web_snapshots() {
    let public = Native::template("web");
    let module = find(&public, "module", "web", None);
    let function = find(&public, "declaration", "controls", Some(&module));
    let initial = public.revision();
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let draft = public.root.path().join("controls.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--owner",
            &function,
            "--output",
            path(&draft),
        ],
        true,
    );
    let source = std::fs::read_to_string(&draft).unwrap();
    assert!(source.contains("Apply dark theme"));
    assert_eq!(
        compact_field(
            compact_record(&public.plan(&draft, true), "result"),
            "outcome"
        ),
        "unchanged"
    );

    let invalid = public.input(
        "wrong-type.lkjc",
        &source.replace("(text \"Apply dark theme\")", "(i64 7)"),
    );
    public.plan(&invalid, false);
    assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    let desired = public.input(
        "desired.lkjc",
        &source.replace("Apply dark theme", "Use the dark theme"),
    );
    let tampered = public.input(
        "tampered.lkjc",
        &source.replace("Apply dark theme", "Unreviewed button"),
    );
    let complete = public.root.path().join("complete.lkjplan");
    let review = public.cli(
        &[
            "change",
            "plan",
            "--input-file",
            path(&desired),
            "--output",
            path(&complete),
        ],
        true,
    );
    let summary = compact_record(&review, "summary");
    for (key, value) in [
        ("created", "0"),
        ("updated", "1"),
        ("deleted", "0"),
        ("retirements", "0"),
    ] {
        assert_eq!(compact_field(summary, key), value, "{key}");
    }
    let complete_bytes = std::fs::read(&complete).unwrap();
    // The old whole-body path used >80 KB and retired 16 expressions for this one label.
    // Keep all proof records, not just a short summary, while preventing that churn returning.
    assert!(complete_bytes.len() < 40_000, "{}", complete_bytes.len());
    let repeat = public.root.path().join("repeat.lkjplan");
    let repeated = public.cli(
        &[
            "change",
            "plan",
            "--input-file",
            path(&desired),
            "--output",
            path(&repeat),
        ],
        true,
    );
    assert_eq!(std::fs::read(&repeat).unwrap(), complete_bytes);
    assert_eq!(
        compact_field(compact_record(&review, "plan"), "token"),
        compact_field(compact_record(&repeated, "plan"), "token")
    );
    public.apply(&tampered, &review, false);
    assert_eq!(public.revision(), initial);

    let before = public.root.path().join("before.lkja");
    public.cli(&["build", "--output", path(&before)], true);
    let before_bytes = std::fs::read(&before).unwrap();
    let descriptor_bytes = std::fs::read(public.project.join("service.deployment.json")).unwrap();
    let mut descriptor: Value = serde_json::from_slice(&descriptor_bytes).unwrap();
    public.apply(&desired, &review, true);
    let updated = public.revision();
    assert_ne!(updated, initial);
    assert_eq!(
        find(&public, "declaration", "controls", Some(&module)),
        function
    );
    public.apply(&desired, &review, false);
    assert_eq!(public.revision(), updated);
    assert_eq!(std::fs::read(&before).unwrap(), before_bytes);
    assert_eq!(
        std::fs::read(public.project.join("service.deployment.json")).unwrap(),
        descriptor_bytes
    );
    let check = public.cli(&["check"], true);
    let tests = compact_record(&check, "tests");
    assert_eq!(compact_field(tests, "passed"), "107");
    assert_eq!(compact_field(tests, "failed"), "0");
    assert_eq!(compact_field(tests, "differential"), "equal");
    let after = public.root.path().join("after.lkja");
    public.cli(&["build", "--output", path(&after)], true);
    assert_ne!(std::fs::read(&after).unwrap(), before_bytes);
    std::fs::rename(
        &public.project,
        public.root.path().join("retained-authoring-project"),
    )
    .unwrap();
    for (artifact, label, absent) in [
        ("before.lkja", "Apply dark theme", "Use the dark theme"),
        ("after.lkja", "Use the dark theme", "Apply dark theme"),
    ] {
        descriptor["artifact"] = serde_json::json!(artifact);
        let server = http::Server::start(&public, artifact, &descriptor, &[]);
        let response = http::send(
            server.address,
            "GET",
            "/?name=Native&theme=dark",
            &[("Host", "localhost")],
            "",
        );
        assert_eq!(response.status, 200);
        assert!(response.body.contains(label));
        assert!(!response.body.contains(absent));
        assert!(response.body.contains("<body class=\"dark\">"));
        assert!(response.body.contains("<h2>Hello, Native</h2>"));
        assert!(!response.body.contains("<script"));
        server.stop();
    }
    assert!(!public.project.exists());
    assert_eq!(std::fs::read(&before).unwrap(), before_bytes);
    eprintln!(
        "native literal web proof: plan_bytes={} created=0 updated=1 deleted=0 retirements=0 graph_tests=107 differential=equal detached_snapshots=2",
        complete_bytes.len()
    );
}
