//! Copied executable snapshots, independent HTTP observations and retained operational data.
use super::native_http as http;
use super::*;

fn snapshot(public: &Native, template: &Path) -> (PathBuf, PathBuf, Vec<CompactRecord>) {
    let records = public.cli(&["build", "--deployment", path(template)], true);
    let artifact = PathBuf::from(compact_field(compact_record(&records, "output"), "path"));
    let deployment = compact_record(&records, "deployment");
    assert_eq!(compact_field(deployment, "admission"), "static-only");
    assert_eq!(compact_field(deployment, "selection"), "unchanged");
    assert_eq!(compact_field(deployment, "application-data"), "untouched");
    assert_eq!(compact_field(deployment, "access"), "owner-only");
    (
        artifact,
        PathBuf::from(compact_field(deployment, "path")),
        records,
    )
}

#[cfg(unix)]
#[test]
fn deployment_snapshots_are_private_even_under_open_umask_and_reject_broad_reuse() {
    use std::os::unix::fs::PermissionsExt;
    let public = Native::template("command");
    let (template, original) = fixture(&public, "command.deployment.json");
    std::fs::set_permissions(&template, std::fs::Permissions::from_mode(0o600)).unwrap();
    // Set umask only in a child test driver; never mutate this parallel test process.
    let open_umask = |arguments: &[&str]| {
        let output = std::process::Command::new("/bin/sh")
            .args(["-c", "umask 000; exec \"$@\"", "build-permissions"])
            .arg(&public.executable)
            .args(["--project", path(&public.project)])
            .args(arguments)
            .current_dir(public.root.path())
            .env_clear()
            .env("PATH", "")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    };
    let first = open_umask(&["build", "--deployment", path(&template)]);
    assert!(String::from_utf8_lossy(&first.stdout).contains(" access=owner-only"));
    let paths = std::fs::read_dir(public.root.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("build-")
                && path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with(".deployment.json")
        })
        .collect::<Vec<_>>();
    assert_eq!(paths.len(), 1);
    let deployment = &paths[0];
    let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(deployment), 0o600);
    let contents = std::fs::read(deployment).unwrap();
    let (artifact, selected, reused) = snapshot(&public, &template);
    assert_eq!(&selected, deployment);
    assert_eq!(
        compact_field(compact_record(&reused, "deployment"), "visibility"),
        "reused-exact"
    );
    std::fs::remove_file(&artifact).unwrap();
    for exposed in [0o640, 0o604, 0o620, 0o601] {
        std::fs::set_permissions(deployment, std::fs::Permissions::from_mode(exposed)).unwrap();
        let rejected = public.cli(&["build", "--deployment", path(&template)], false);
        assert_eq!(
            compact_field(compact_record(&rejected, "diagnostic"), "code"),
            "deployment_build_permissions"
        );
        assert!(
            !artifact.exists(),
            "privacy rejection must precede artifact publication"
        );
        assert_eq!(
            mode(deployment),
            exposed,
            "build must not chmod an existing file"
        );
        assert_eq!(std::fs::read(deployment).unwrap(), contents);
    }
    std::fs::set_permissions(deployment, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(snapshot(&public, &template).1, *deployment);
    assert_eq!(mode(deployment), 0o600);
    let raw = public.root.path().join("ordinary.lkja");
    open_umask(&["build", "--output", path(&raw)]);
    assert_eq!(
        mode(&raw),
        0o666,
        "ordinary output policy remains unchanged"
    );
    assert_eq!(mode(&template), 0o600);
    assert_eq!(std::fs::read(&template).unwrap(), original);
}

fn value(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn fixture(public: &Native, name: &str) -> (PathBuf, Vec<u8>) {
    let original = std::fs::read(public.project.join(name)).unwrap();
    let template = public.input("operator.json", std::str::from_utf8(&original).unwrap());
    std::fs::create_dir(public.root.path().join("generated")).unwrap();
    (template, original)
}

#[test]
fn command_snapshots_are_exact_reusable_and_preserve_conflicts_and_legacy_outputs() {
    let public = Native::template("command");
    let (template, original) = fixture(&public, "command.deployment.json");
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let expected = public.cli(&["run", "main"], true);
    let (artifact, deployment, first) = snapshot(&public, &template);
    assert_eq!(
        compact_field(compact_record(&first, "output"), "visibility"),
        "created"
    );
    assert_eq!(
        compact_field(compact_record(&first, "deployment"), "visibility"),
        "created"
    );
    assert_eq!(deployment.parent().unwrap(), template.parent().unwrap());
    assert_eq!(
        artifact.parent().unwrap(),
        public.root.path().join("generated")
    );
    let mut observed = value(&deployment);
    observed["artifact"] = value(&template)["artifact"].clone();
    assert_eq!(
        observed,
        value(&template),
        "only the artifact value changes; omitted fields stay omitted"
    );
    let artifact_bytes = std::fs::read(&artifact).unwrap();
    let deployment_bytes = std::fs::read(&deployment).unwrap();
    let (same_artifact, same_deployment, reused) = snapshot(&public, &template);
    assert_eq!((&same_artifact, &same_deployment), (&artifact, &deployment));
    for record in ["output", "deployment"] {
        assert_eq!(
            compact_field(compact_record(&reused, record), "visibility"),
            "reused-exact"
        );
    }
    // Exercise independent public processes, not only the file publisher's threads.
    std::fs::remove_file(&artifact).unwrap();
    std::fs::remove_file(&deployment).unwrap();
    let barrier = std::sync::Barrier::new(4);
    let simultaneous = std::thread::scope(|scope| {
        let handles = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    snapshot(&public, &template)
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    for record in ["output", "deployment"] {
        assert_eq!(
            simultaneous
                .iter()
                .filter(|(_, _, records)| compact_field(
                    compact_record(records, record),
                    "visibility"
                ) == "created")
                .count(),
            1
        );
    }
    for (observed_artifact, observed_deployment, _) in simultaneous {
        assert_eq!(
            (observed_artifact, observed_deployment),
            (artifact.clone(), deployment.clone())
        );
    }
    // A generated snapshot is itself a valid template, with no filename proliferation.
    assert_eq!(snapshot(&public, &deployment).1, deployment);
    let reference = public.root.path().join("reference.lkja");
    public.cli(&["build", "--output", path(&reference)], true);
    assert_eq!(std::fs::read(&reference).unwrap(), artifact_bytes);
    public.cli(&["build", "--output", path(&artifact)], false);
    assert_eq!(std::fs::read(&artifact).unwrap(), artifact_bytes);
    for arguments in [
        vec!["build"],
        vec![
            "build",
            "--deployment",
            path(&template),
            "--output",
            path(&reference),
        ],
        vec![
            "build",
            "--deployment",
            path(&template),
            "--deployment",
            path(&template),
        ],
        vec!["build", "--deployment"],
    ] {
        public.cli(&arguments, false);
    }
    // Refuse a conflicting descriptor before publishing even a missing artifact.
    std::fs::remove_file(&artifact).unwrap();
    std::fs::write(&deployment, b"conflicting operator file").unwrap();
    public.cli(&["build", "--deployment", path(&template)], false);
    assert!(!artifact.exists());
    assert_eq!(
        std::fs::read(&deployment).unwrap(),
        b"conflicting operator file"
    );
    std::fs::write(&deployment, &deployment_bytes).unwrap();
    snapshot(&public, &template);
    std::fs::write(&artifact, b"corrupt artifact").unwrap();
    public.cli(&["build", "--deployment", path(&template)], false);
    assert_eq!(std::fs::read(&artifact).unwrap(), b"corrupt artifact");
    assert_eq!(std::fs::read(&deployment).unwrap(), deployment_bytes);
    std::fs::write(&artifact, &artifact_bytes).unwrap();
    assert_eq!(std::fs::read(&template).unwrap(), original);
    assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    assert!(
        !public
            .root
            .path()
            .join("generated/application.lkja")
            .exists()
    );
    std::fs::remove_dir_all(&public.project).unwrap();
    let actual = public.cli(&["run", "--deployment", path(&deployment)], true);
    assert_eq!(
        compact_field(compact_record(&actual, "execution"), "value"),
        compact_field(compact_record(&expected, "execution"), "value")
    );
}

#[test]
fn persistent_data_survives_native_rebuild_and_stays_separate_from_outputs() {
    let public = Native::template("command");
    let source = include_str!("../fixtures/native-editor-data.lkjc")
        .replace("FIXTURE_BASE", &public.revision());
    let input = public.input("store.lkjc", &source);
    public.apply(&input, &public.plan(&input, true), true);
    let (template, _) = fixture(&public, "command.deployment.json");
    let editor: Value = serde_json::from_str(include_str!(
        "../../docs/guides/examples/editor.deployment.json"
    ))
    .unwrap();
    let mut descriptor = value(&template);
    descriptor["target"] = serde_json::json!("replace");
    let mut grant = editor["grants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|grant| grant["requirement"] == "data")
        .unwrap()
        .clone();
    grant["adapter"]["root"] = serde_json::json!("state.lkjdata");
    descriptor["grants"] = serde_json::json!([grant]);
    descriptor["secrets"] =
        serde_json::json!([{"name":"unused","variable":"LKJSCRIPT_BUILD_NO_SECRET"}]);
    let no_resources = public.input("no-resources.json", &descriptor.to_string());
    let (artifact, _, _) = snapshot(&public, &no_resources);
    let data = public.root.path().join("state.lkjdata");
    assert!(
        !data.exists(),
        "static build must neither open nor initialize the declared data store"
    );
    // The public helper clears all environment variables. Missing secrets did not block building.
    let mut overlap = descriptor.clone();
    overlap["artifact"] = serde_json::json!("state.lkjdata/unused.lkja");
    let overlap = public.input("overlap.json", &overlap.to_string());
    public.cli(&["build", "--deployment", path(&overlap)], false);
    assert!(!data.exists());
    descriptor["secrets"] = serde_json::json!([]);
    let selected = public.input("selected.json", &descriptor.to_string());
    let (same_artifact, old, _) = snapshot(&public, &selected);
    assert_eq!(
        same_artifact, artifact,
        "a configuration-only change reuses the exact program"
    );
    compact_success_at(
        &public.executable,
        public.root.path(),
        &["data", "initialize", "--root", "state.lkjdata"],
    );
    public.cli(
        &["run", "--deployment", path(&old), "--arguments", "[5]"],
        true,
    );
    let saved = std::fs::read(data.join("HEAD")).unwrap();
    let verified = compact_success_at(
        &public.executable,
        public.root.path(),
        &["data", "verify", "--root", "state.lkjdata"],
    );
    let store = super::super::compact_field(compact_record(&verified, "data"), "store")
        .unwrap()
        .to_owned();
    let module = find(&public, "module", "fixture", None);
    let function = find(&public, "declaration", "value", Some(&module));
    let draft = public.root.path().join("value.lkjc");
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
    assert!(source.contains("fixture note"));
    let edited = public.input(
        "next.lkjc",
        &source.replace("fixture note", "second program"),
    );
    public.apply(&edited, &public.plan(&edited, true), true);
    public.cli(&["check"], true);
    let (next_artifact, new, _) = snapshot(&public, &selected);
    assert_ne!(next_artifact, artifact);
    assert_eq!(value(&new)["grants"], value(&old)["grants"]);
    assert_eq!(
        std::fs::read(data.join("HEAD")).unwrap(),
        saved,
        "building neither migrates nor resets data"
    );
    std::fs::remove_dir_all(&public.project).unwrap();
    for deployment in [&new, &old] {
        let before = std::fs::read(data.join("HEAD")).unwrap();
        public.cli(
            &[
                "run",
                "--deployment",
                path(deployment),
                "--arguments",
                "[5]",
            ],
            true,
        );
        assert_ne!(std::fs::read(data.join("HEAD")).unwrap(), before);
        let verified = compact_success_at(
            &public.executable,
            public.root.path(),
            &["data", "verify", "--root", "state.lkjdata"],
        );
        assert_eq!(
            super::super::compact_field(compact_record(&verified, "data"), "store"),
            Some(store.as_str())
        );
    }
    assert_eq!(value(&selected), descriptor);
}

fn find(public: &Native, class: &str, name: &str, parent: Option<&str>) -> String {
    let mut arguments = vec!["query", "find", class, name];
    if let Some(parent) = parent {
        arguments.extend(["--parent", parent]);
    }
    compact_field(compact_record(&public.cli(&arguments, true), "owner"), "id").to_owned()
}

#[test]
fn web_snapshot_rebuild_does_not_replace_running_old_program_or_require_its_graph() {
    let public = Native::template("web");
    let (template, original) = fixture(&public, "service.deployment.json");
    let (old_artifact, old_deployment, _) = snapshot(&public, &template);
    let old_bytes = std::fs::read(&old_artifact).unwrap();
    let old = http::Server::start_file(&public, "old-snapshot", &old_deployment, &[]);
    let headers = [("Host", "localhost")];
    let old_page = http::send(old.address, "GET", "/", &headers, "");
    assert!(old_page.body.contains("<title>My lkjscript app</title>"));
    let module = find(&public, "module", "web", None);
    let title = find(&public, "declaration", "page-title", Some(&module));
    let draft = public.root.path().join("title.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--owner",
            &title,
            "--output",
            path(&draft),
        ],
        true,
    );
    let source = std::fs::read_to_string(&draft).unwrap();
    let edited = public.input(
        "edited.lkjc",
        &source.replace("My lkjscript app", "Snapshot <two> 🌱"),
    );
    public.apply(&edited, &public.plan(&edited, true), true);
    assert_eq!(
        find(&public, "declaration", "page-title", Some(&module)),
        title
    );
    let checked = public.cli(&["check"], true);
    assert_eq!(
        compact_field(compact_record(&checked, "tests"), "passed"),
        "107"
    );
    assert_eq!(
        compact_field(compact_record(&checked, "tests"), "differential"),
        "equal"
    );
    let (new_artifact, new_deployment, _) = snapshot(&public, &template);
    assert_ne!(new_artifact, old_artifact);
    assert_ne!(new_deployment, old_deployment);
    assert_eq!(snapshot(&public, &template).1, new_deployment);
    let new = http::Server::start_file(&public, "new-snapshot", &new_deployment, &[]);
    let new_page = http::send(
        new.address,
        "GET",
        "/?name=Japanese%20%E6%97%A5%E6%9C%AC%E8%AA%9E&theme=dark",
        &headers,
        "",
    );
    assert_eq!(new_page.status, 200);
    assert!(
        new_page
            .body
            .contains("<title>Snapshot &lt;two&gt; 🌱</title>")
    );
    assert!(new_page.body.contains("Hello, Japanese 日本語"));
    assert!(!new_page.body.contains("<script"));
    assert_eq!(
        http::send(old.address, "GET", "/", &headers, "").body,
        old_page.body
    );
    assert_eq!(std::fs::read(&old_artifact).unwrap(), old_bytes);
    assert_eq!(std::fs::read(&template).unwrap(), original);
    old.stop();
    new.stop();
    std::fs::remove_dir_all(&public.project).unwrap();
    for (label, selected, title) in [
        ("old-restart", old_deployment, "My lkjscript app"),
        ("new-restart", new_deployment, "Snapshot &lt;two&gt; 🌱"),
    ] {
        let server = http::Server::start_file(&public, label, &selected, &[]);
        assert!(
            http::send(server.address, "GET", "/", &headers, "")
                .body
                .contains(&format!("<title>{title}</title>"))
        );
        server.stop();
    }
    assert!(!public.project.exists());
}

#[test]
fn duplicate_configuration_rejects_before_build_publication_or_runtime_loading() {
    let public = Native::template("command");
    let (template, original) = fixture(&public, "command.deployment.json");
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let mut base: Value = serde_json::from_slice(&original).unwrap();
    base["configuration"] = serde_json::json!("CONFIGURATION_FIXTURE");
    let raw = base.to_string();
    for key in ["alpha", r"\u0061lpha", r"a\u006cpha"] {
        for second in ["first", "second"] {
            let configuration = format!(
                r#"{{"alpha":{{"kind":"text","value":"first"}},"{key}":{{"kind":"text","value":"{second}"}}}}"#
            );
            let input = raw.replace("\"CONFIGURATION_FIXTURE\"", &configuration);
            std::fs::write(&template, &input).unwrap();
            for operation in ["build", "run"] {
                let rejected = public.cli(&[operation, "--deployment", path(&template)], false);
                assert_eq!(
                    compact_field(compact_record(&rejected, "diagnostic"), "code"),
                    "deployment_json",
                    "{operation}: {key}: {second}"
                );
            }
            // Resident commands have a standalone JSON envelope, not project compact records.
            let served = support::output(
                Command::new(&public.executable)
                    .args(["serve", "--deployment", path(&template)])
                    .current_dir(public.root.path())
                    .env_clear()
                    .env("PATH", ""),
            )
            .unwrap();
            assert!(!served.status.success());
            let failure: Value = serde_json::from_slice(&served.stdout).unwrap();
            assert_eq!(failure["error"]["code"], "deployment_json");
            assert_eq!(std::fs::read_to_string(&template).unwrap(), input);
            assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
            assert_eq!(
                std::fs::read_dir(public.root.path().join("generated"))
                    .unwrap()
                    .count(),
                0
            );
            assert!(
                !std::fs::read_dir(public.root.path()).unwrap().any(|entry| {
                    entry
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                        .starts_with("build-")
                })
            );
        }
    }
    std::fs::write(&template, original).unwrap();
    snapshot(&public, &template);
}

#[test]
fn static_admission_rejects_mismatched_authority_without_publishing() {
    let public = Native::template("http");
    let (template, original) = fixture(&public, "service.deployment.json");
    let base: Value = serde_json::from_slice(&original).unwrap();
    for mode in 0..4 {
        let mut invalid = base.clone();
        match mode {
            0 => invalid["target"] = Value::String("absent-target".to_owned()),
            1 => invalid["grants"] = serde_json::json!([]),
            2 => invalid["grants"][0]["adapter"] = serde_json::json!({"kind":"wall_clock"}),
            _ => invalid["runtime"] = Value::Null,
        }
        std::fs::write(&template, invalid.to_string()).unwrap();
        public.cli(&["build", "--deployment", path(&template)], false);
        assert_eq!(
            std::fs::read_dir(public.root.path().join("generated"))
                .unwrap()
                .count(),
            0
        );
        assert!(
            !std::fs::read_dir(public.root.path())
                .unwrap()
                .any(|entry| entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("build-"))
        );
    }
    std::fs::write(&template, original).unwrap();
    snapshot(&public, &template);
}
