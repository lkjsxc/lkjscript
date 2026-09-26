//! The same literal-edit path composes two independently authored and transported libraries.
use super::*;

struct Library {
    public: Native,
    file: PathBuf,
    package: String,
    revision: String,
    package_revision: String,
    transport: String,
}

fn library(source: &str) -> Library {
    let public = Native::template("command");
    // The command template already declares its exact standard dependency. All native
    // declarations below remain the maintained literal source, not host-generated syntax.
    let source = source.replace("LIBRARY_BASE", &public.revision()).replace(
        "add.dependency package=pkg_10000000000000000000000000000001 semantic-revision=STANDARD_REVISION package-revision=STANDARD_PACKAGE_REVISION\n", "");
    let input = public.input("library.lkjc", &source);
    let plan = public.plan(&input, true);
    public.apply(&input, &plan, true);
    let file = public.root.path().join("library.lkjp");
    let exported = public.cli(
        &[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            path(&file),
        ],
        true,
    );
    let package = compact_record(&exported, "package");
    Library {
        public,
        file,
        package: compact_field(package, "id").to_owned(),
        revision: compact_field(package, "revision").to_owned(),
        package_revision: compact_field(package, "package-revision").to_owned(),
        transport: compact_field(package, "transport").to_owned(),
    }
}

#[test]
fn native_literal_edit_preserves_imported_list_app_and_both_default_policies() {
    let html = library(include_str!("../../docs/guides/examples/html.lkjc"));
    let responses = library(include_str!(
        "../../docs/guides/examples/http-responses.lkjc"
    ));
    let public = Native::template("http");
    let mut source = include_str!("../../docs/guides/examples/list-site.lkjc")
        .replace("SITE_BASE", &public.revision());
    for (prefix, library) in [("HTML", &html), ("RESPONSES", &responses)] {
        public.cli(
            &[
                "package",
                "dependency",
                "stage",
                "--transport",
                &library.transport,
                "--input-file",
                path(&library.file),
            ],
            true,
        );
        source = source
            .replace(
                &format!("{prefix}_PACKAGE_REVISION"),
                &library.package_revision,
            )
            .replace(&format!("{prefix}_REVISION"), &library.revision)
            .replace(&format!("{prefix}_PACKAGE"), &library.package);
    }
    let input = public.input("list-site.lkjc", &source);
    let initial = public.plan(&input, true);
    public.apply(&input, &initial, true);
    let module = find(&public, "module", "catalog", None);
    let function = find(&public, "declaration", "page", Some(&module));
    let draft = public.root.path().join("page.lkjc");
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
    assert_eq!(
        source.matches("(i64 10)").count(),
        1,
        "only the page fallback, never the parser radix"
    );
    let input = public.input("page-edited.lkjc", &source.replace("(i64 10)", "(i64 25)"));
    let complete = public.root.path().join("page.lkjplan");
    let review = public.cli(
        &[
            "change",
            "plan",
            "--input-file",
            path(&input),
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
    let plan_bytes = std::fs::read(&complete).unwrap().len();
    assert!(plan_bytes < 40_000, "{plan_bytes}");
    let before = public.root.path().join("before.lkja");
    public.cli(&["build", "--output", path(&before)], true);
    let before_bytes = std::fs::read(&before).unwrap();
    public.apply(&input, &review, true);
    assert_eq!(
        find(&public, "declaration", "page", Some(&module)),
        function
    );
    let checked = public.cli(&["check"], true);
    let tests = compact_record(&checked, "tests");
    assert_eq!(compact_field(tests, "passed"), "111");
    assert_eq!(compact_field(tests, "failed"), "0");
    assert_eq!(compact_field(tests, "differential"), "equal");
    let after = public.root.path().join("after.lkja");
    public.cli(&["build", "--output", path(&after)], true);
    let mut descriptor: Value = serde_json::from_slice(
        &std::fs::read(public.project.join("service.deployment.json")).unwrap(),
    )
    .unwrap();
    descriptor["target"] = serde_json::json!("paged-web");
    for library in [&html, &responses] {
        std::fs::rename(
            &library.public.project,
            library
                .public
                .root
                .path()
                .join("retained-authoring-project"),
        )
        .unwrap();
        std::fs::rename(
            &library.file,
            library.public.root.path().join("retained-transport.lkjp"),
        )
        .unwrap();
    }
    std::fs::rename(
        &public.project,
        public.root.path().join("retained-authoring-project"),
    )
    .unwrap();
    let query = (0..30)
        .map(|index| format!("item={index}"))
        .collect::<Vec<_>>()
        .join("&");
    for (artifact, default) in [("before.lkja", 10), ("after.lkja", 25)] {
        descriptor["artifact"] = serde_json::json!(artifact);
        let server = http::Server::start(&public, artifact, &descriptor, &[]);
        for (suffix, expected_status, count) in [
            ("", 200, default),
            ("&count=2", 200, 2),
            ("&count=101", 400, 0),
        ] {
            let reply = http::send(
                server.address,
                "GET",
                &format!("/?{query}{suffix}"),
                &[("Host", "localhost")],
                "",
            );
            assert_eq!(reply.status, expected_status);
            if expected_status == 200 {
                assert_eq!(reply.body.matches("<li>").count(), count);
                assert!(
                    reply
                        .body
                        .contains(&format!("Showing {count} of 30 items; start 0"))
                );
                assert!(reply.headers.contains("x-content-type-options: nosniff"));
            } else {
                assert!(reply.body.contains("count must not exceed 100"));
            }
        }
        server.stop();
    }
    assert_eq!(std::fs::read(&before).unwrap(), before_bytes);
    eprintln!(
        "native literal list proof: plan_bytes={plan_bytes} created=0 updated=1 deleted=0 retirements=0 graph_tests=111 differential=equal detached_snapshots=2 http_observations=6"
    );
}
