//! Embedded starter authorship plus the same durable behavior exercised for exact packages.
use super::*;

fn snapshot(public: &Native, template: &Path) -> (PathBuf, PathBuf) {
    let output = public.cli(&["build", "--deployment", path(template)], true);
    let deployment = compact_record(&output, "deployment");
    for (key, expected) in [
        ("admission", "static-only"),
        ("selection", "unchanged"),
        ("application-data", "untouched"),
        ("access", "owner-only"),
    ] {
        assert_eq!(compact_field(deployment, key), expected);
    }
    (
        PathBuf::from(compact_field(compact_record(&output, "output"), "path")),
        PathBuf::from(compact_field(deployment, "path")),
    )
}

fn value(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn find(public: &Native, class: &str, name: &str, parent: Option<&str>) -> String {
    let mut arguments = vec!["query", "find", class, name];
    if let Some(parent) = parent {
        arguments.extend(["--parent", parent]);
    }
    compact_field(compact_record(&public.cli(&arguments, true), "owner"), "id").to_owned()
}

fn check(public: &Native) {
    let result = public.cli(&["check"], true);
    let tests = compact_record(&result, "tests");
    assert_eq!(compact_field(tests, "passed"), "180"); // 105 local + 75 standard.
    assert_eq!(compact_field(tests, "failed"), "0");
    assert_eq!(compact_field(tests, "differential"), "equal");
}

fn refuses_start(public: &Native, deployment: &Path, environment: &[(&str, &str)]) {
    let output = support::output(
        Command::new(&public.executable)
            .args(["serve", "--deployment", path(deployment)])
            .current_dir(public.root.path())
            .env_clear()
            .env("PATH", "")
            .envs(environment.iter().copied()),
    )
    .unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("\"event\":\"ready\""));
}

fn fixture_address(mut descriptor: Value) -> Value {
    // The application's authority is exact; only the fixture's address is selected here.
    descriptor["listen"] = json!("127.0.0.1:0");
    descriptor["configuration"]["origin"]["value"] = json!("http://localhost");
    descriptor
}

#[test]
fn web_editor_starter_is_editable_offline_and_preserves_conditional_saves() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("lkjscript");
    copy_executable(&binary(), &executable);
    let project = root.path().join("project");
    let public = Native {
        root,
        executable,
        project,
    };
    let created = public.cli(
        &["new", path(&public.project), "--template", "web-editor"],
        true,
    );
    let next: Vec<_> = created
        .iter()
        .filter(|record| record.operation == "next")
        .collect();
    assert_eq!(
        next.iter()
            .map(|record| compact_field(record, "order"))
            .collect::<Vec<_>>(),
        ["1", "2", "3", "4", "5", "6"]
    );
    assert_eq!(compact_field(next[3], "kind"), "data-initialize");
    assert_eq!(
        compact_field(next[3], "root"),
        path(&public.project.join("notes.lkjdata"))
    );
    assert_eq!(compact_field(next[4], "kind"), "secret-environment");
    assert_eq!(
        compact_field(next[4], "variable"),
        "LKJSCRIPT_EDITOR_AUTHORIZATION"
    );
    assert_eq!(compact_field(next[5], "operation"), "serve");
    let initial = public.revision();
    let graph_head = std::fs::read(public.project.join("HEAD")).unwrap();
    let original = std::fs::read(public.project.join("service.deployment.json")).unwrap();
    let descriptor: Value = serde_json::from_slice(&original).unwrap();
    assert_eq!(descriptor["artifact"], "generated/application.lkja");
    assert_eq!(descriptor["target"], "editor");
    assert_eq!(descriptor["listen"], "127.0.0.1:8080");
    assert_eq!(
        descriptor["configuration"]["origin"]["value"],
        "http://127.0.0.1:8080"
    );
    assert_eq!(descriptor["grants"][3]["adapter"]["root"], "notes.lkjdata");
    assert_eq!(
        descriptor["secrets"][0]["variable"],
        "LKJSCRIPT_EDITOR_AUTHORIZATION"
    );
    assert!(!public.project.join("notes.lkjdata").exists());
    assert!(!public.project.join("generated/application.lkja").exists());
    let data = public.root.path().join("notes.lkjdata");
    assert!(!data.exists());
    check(&public);

    public.cli(
        &["new", path(&public.project), "--template", "web-editor"],
        false,
    );
    let rejected = public.root.path().join("rejected");
    public.cli(
        &[
            "new",
            path(&rejected),
            "--template",
            "web-editor",
            "--relay-url",
            "ws://127.0.0.1:9",
        ],
        false,
    );
    assert!(!rejected.exists());
    assert_eq!(
        std::fs::read(public.project.join("HEAD")).unwrap(),
        graph_head
    );

    // The actual emitted operator template builds without data, a secret or a listener.
    let template = public.input("operator.json", std::str::from_utf8(&original).unwrap());
    std::fs::create_dir(public.root.path().join("generated")).unwrap();
    let (old_artifact, old_deployment) = snapshot(&public, &template);
    let old_artifact_bytes = std::fs::read(&old_artifact).unwrap();
    let old_deployment_bytes = std::fs::read(&old_deployment).unwrap();
    let ordinary = public.root.path().join("generated/application.lkja");
    public.cli(&["build", "--output", path(&ordinary)], true);
    assert_eq!(std::fs::read(&ordinary).unwrap(), old_artifact_bytes);
    assert!(!data.exists());
    let mut observed = value(&old_deployment);
    observed["artifact"] = descriptor["artifact"].clone();
    assert_eq!(observed, descriptor, "no other operator setting may change");

    let module = find(&public, "module", "editor", None);
    let page = find(&public, "declaration", "page", Some(&module));
    let draft_file = public.root.path().join("page.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--owner",
            &page,
            "--output",
            path(&draft_file),
        ],
        true,
    );
    let source = std::fs::read_to_string(&draft_file).unwrap();
    assert_eq!(source.matches("(text \"Native notes\")").count(), 1);
    let wrong = public.input(
        "wrong.lkjc",
        &source.replace("(text \"Native notes\")", "(i64 7)"),
    );
    public.plan(&wrong, false);
    assert_eq!(public.revision(), initial);
    assert_eq!(
        std::fs::read(public.project.join("HEAD")).unwrap(),
        graph_head
    );
    let edit = public.input(
        "edit.lkjc",
        &source.replace("(text \"Native notes\")", "(text \"My native notes 🌱\")"),
    );
    let plan = public.plan(&edit, true);
    public.apply(&edit, &plan, true);
    assert_ne!(public.revision(), initial);
    assert_eq!(find(&public, "declaration", "page", Some(&module)), page);
    check(&public);
    let (new_artifact, new_deployment) = snapshot(&public, &template);
    assert_ne!(new_artifact, old_artifact);
    assert_ne!(new_deployment, old_deployment);
    assert_eq!(std::fs::read(&template).unwrap(), original);
    assert_eq!(std::fs::read(&old_artifact).unwrap(), old_artifact_bytes);
    assert_eq!(
        std::fs::read(&old_deployment).unwrap(),
        old_deployment_bytes
    );
    assert!(!data.exists());

    // Neither a missing secret nor a missing store may initialize replacement operational data.
    refuses_start(&public, &new_deployment, &[]);
    refuses_start(&public, &new_deployment, AUTH_ENVIRONMENT);
    assert!(!data.exists());
    std::fs::remove_dir_all(&public.project).unwrap();
    for input in [draft_file, wrong, edit] {
        std::fs::remove_file(input).unwrap();
    }
    compact_success_at(
        &public.executable,
        public.root.path(),
        &["data", "initialize", "--root", "notes.lkjdata"],
    );
    let empty_head = std::fs::read(data.join("HEAD")).unwrap();
    refuses_start(&public, &new_deployment, &[]);
    assert_eq!(std::fs::read(data.join("HEAD")).unwrap(), empty_head);

    // Both immutable builds still work after removing the authoring graph and edit proposals.
    for (label, file, title) in [
        (
            "old-starter",
            &old_deployment,
            "<title>Native notes</title>",
        ),
        (
            "new-starter",
            &new_deployment,
            "<title>My native notes 🌱</title>",
        ),
    ] {
        let server = http::Server::start(
            &public,
            label,
            &fixture_address(value(file)),
            AUTH_ENVIRONMENT,
        );
        let response = http::send(server.address, "GET", "/", &headers(), "");
        assert_eq!(response.status, 200);
        assert!(response.body.contains(title));
        assert!(!response.body.contains("<script"));
        assert_eq!(draft(&response.body), "");
        server.stop();
    }
    assert_eq!(std::fs::read(data.join("HEAD")).unwrap(), empty_head);
    assert_eq!(std::fs::read(&old_artifact).unwrap(), old_artifact_bytes);
    assert_eq!(
        std::fs::read(&old_deployment).unwrap(),
        old_deployment_bytes
    );
    let descriptor = fixture_address(value(&new_deployment));
    exercise_editor(public, descriptor);
}
