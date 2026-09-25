//! Literal public authorship and exact offline suppliers for the editor workload.
use super::*;

pub(super) const SOURCE: &str = include_str!("../../docs/guides/examples/editor.lkjc");

fn apply(public: &Native, name: &str, source: &str, placeholder: &str) {
    let input = public.input(name, &source.replace(placeholder, &public.revision()));
    let plan = public.plan(&input, true);
    public.apply(&input, &plan, true);
}

fn supplier(name: &str, source: &str, tests: Option<&str>) -> Native {
    let library = Native::template("command");
    apply(&library, name, source, "LIBRARY_BASE");
    if let Some(tests) = tests {
        apply(&library, "tests.lkjc", tests, "LIBRARY_BASE");
    }
    library
}

fn import(public: &Native, library: &Native, prefix: &str, source: &mut String) {
    let transport = library.root.path().join("library.lkjp");
    let exported = library.cli(
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
    let package = compact_record(&exported, "package");
    public.cli(
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            compact_field(package, "transport"),
            "--input-file",
            path(&transport),
        ],
        true,
    );
    // Bind only metadata from the public export, longest placeholder first.
    for (suffix, field) in [
        ("PACKAGE_REVISION", "package-revision"),
        ("REVISION", "revision"),
        ("PACKAGE", "id"),
    ] {
        *source = source.replace(&format!("{prefix}_{suffix}"), compact_field(package, field));
    }
}

fn check_existing_get_consumer(ui: &Native) {
    let responses = supplier(
        "responses.lkjc",
        include_str!("../../docs/guides/examples/http-responses.lkjc"),
        None,
    );
    let site = Native::template("http");
    let mut source = include_str!("../../docs/guides/examples/ui-site.lkjc").to_owned();
    import(&site, ui, "UI", &mut source);
    import(&site, &responses, "RESPONSES", &mut source);
    apply(&site, "site.lkjc", &source, "SITE_BASE");
    let checked = site.cli(&["check"], true);
    assert_eq!(
        compact_field(compact_record(&checked, "tests"), "failed"),
        "0"
    );
    assert_eq!(
        compact_field(compact_record(&checked, "tests"), "differential"),
        "equal"
    );
    site.cli(
        &[
            "build",
            "--output",
            path(&site.root.path().join("site.lkja")),
        ],
        true,
    );
}

pub(super) fn author() -> (Native, Value) {
    assert!(
        !SOURCE.contains('<'),
        "the application contains no authored markup"
    );
    let ui = supplier(
        "ui.lkjc",
        include_str!("../../docs/guides/examples/ui.lkjc"),
        Some(include_str!("../../docs/guides/examples/ui-tests.lkjc")),
    );
    let forms = supplier(
        "forms.lkjc",
        include_str!("../../docs/guides/examples/form-codec.lkjc"),
        None,
    );
    let public = Native::template("http");
    let mut source = SOURCE.to_owned();
    import(&public, &ui, "UI", &mut source);
    import(&public, &forms, "FORMS", &mut source);
    apply(&public, "editor.lkjc", &source, "EDITOR_BASE");
    apply(
        &public,
        "editor-tests.lkjc",
        include_str!("../../docs/guides/examples/editor-tests.lkjc"),
        "EDITOR_BASE",
    );
    // The changed ordinary UI supplier must also preserve its unchanged GET consumer.
    check_existing_get_consumer(&ui);
    // Removing producers cannot invalidate the accepted exact implementation closure.
    drop(ui);
    drop(forms);
    let checked = public.cli(&["check"], true);
    assert_eq!(
        compact_field(compact_record(&checked, "tests"), "differential"),
        "equal"
    );
    assert_eq!(
        compact_field(compact_record(&checked, "tests"), "failed"),
        "0"
    );
    public.cli(
        &[
            "build",
            "--output",
            path(&public.root.path().join("editor.lkja")),
        ],
        true,
    );
    let mut descriptor: Value = serde_json::from_str(include_str!(
        "../../docs/guides/examples/editor.deployment.json"
    ))
    .unwrap();
    // Only fixture address selection differs from the operator-facing descriptor.
    descriptor["listen"] = json!("127.0.0.1:0");
    descriptor["configuration"]["origin"]["value"] = json!("http://localhost");
    compact_success_at(
        &public.executable,
        public.root.path(),
        &["data", "initialize", "--root", "data"],
    );
    std::fs::remove_dir_all(&public.project).unwrap();
    for name in ["editor.lkjc", "editor-tests.lkjc"] {
        std::fs::remove_file(public.root.path().join(name)).unwrap();
    }
    (public, descriptor)
}

pub(super) fn install_data_fixture(public: &Native) {
    let fixture = Native::template("command");
    apply(
        &fixture,
        "data-fixture.lkjc",
        include_str!("../fixtures/native-editor-data.lkjc"),
        "FIXTURE_BASE",
    );
    fixture.cli(&["check"], true);
    let bundle = fixture.root.path().join("fixture.lkja");
    fixture.cli(&["build", "--output", path(&bundle)], true);
    std::fs::copy(bundle, public.root.path().join("fixture.lkja")).unwrap();
}

pub(super) fn write_fixture(public: &Native, descriptor: &Value, mode: i64) {
    let mut command = descriptor.clone();
    command["artifact"] = json!("fixture.lkja");
    command["target"] = json!("replace");
    for field in ["listen", "http", "session", "worker"] {
        command[field] = Value::Null;
    }
    command["configuration"] = json!({});
    command["secrets"] = json!([]);
    command["grants"] = json!(
        descriptor["grants"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|grant| grant["requirement"] == "data")
            .cloned()
            .collect::<Vec<_>>()
    );
    let deployment = public.input("fixture.json", &command.to_string());
    let before = std::fs::read(public.root.path().join("data/HEAD")).unwrap();
    public.cli(
        &[
            "run",
            "--deployment",
            path(&deployment),
            "--arguments",
            &format!("[{mode}]"),
        ],
        true,
    );
    assert_ne!(
        std::fs::read(public.root.path().join("data/HEAD")).unwrap(),
        before
    );
}
