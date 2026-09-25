//! Copied-binary web starter: native editing and detached HTTP.
use super::native_http as http;
use super::*;

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
    // 19 unchanged shared UI tests + 13 app tests + 75 built-in standard tests.
    assert_eq!(compact_field(tests, "passed"), "107");
    assert_eq!(compact_field(tests, "failed"), "0");
    assert_eq!(compact_field(tests, "differential"), "equal");
}

fn encoded(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("%{byte:02X}"))
        .collect()
}

#[test]
fn web_starter_is_editable_rejects_bad_changes_and_serves_without_authoring_files() {
    let public = Native::template("web");
    check(&public);
    let initial = public.revision();
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let descriptor_bytes = std::fs::read(public.project.join("service.deployment.json")).unwrap();
    let descriptor: Value = serde_json::from_slice(&descriptor_bytes).unwrap();
    assert_eq!(descriptor["artifact"], "generated/application.lkja");
    assert_eq!(descriptor["target"], "serve");
    assert_eq!(descriptor["listen"], "127.0.0.1:0");
    assert!(descriptor["execution"]["instruction_fuel"].is_null());
    assert!(descriptor["secrets"].as_array().unwrap().is_empty());
    assert_eq!(descriptor["grants"].as_array().unwrap().len(), 1);
    assert_eq!(descriptor["grants"][0]["adapter"]["kind"], "byte_stream");

    // A new template does not acquire overwrite or relay authority.
    public.cli(&["new", path(&public.project), "--template", "web"], false);
    let rejected = public.root.path().join("rejected");
    public.cli(
        &[
            "new",
            path(&rejected),
            "--template",
            "web",
            "--relay-url",
            "ws://127.0.0.1:9",
        ],
        false,
    );
    assert!(!rejected.exists());
    assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    assert_eq!(
        std::fs::read(public.project.join("service.deployment.json")).unwrap(),
        descriptor_bytes
    );

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
    assert!(source.contains("(text \"My lkjscript app\")"));
    assert_eq!(
        compact_field(
            compact_record(&public.plan(&draft, true), "result"),
            "outcome"
        ),
        "unchanged"
    );
    let invalid = public.input(
        "wrong-type.lkjc",
        &source.replace("(text \"My lkjscript app\")", "(i64 7)"),
    );
    public.plan(&invalid, false);
    assert_eq!(public.revision(), initial);
    assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    let edited = public.input(
        "edited.lkjc",
        &source.replace("My lkjscript app", "Native <web> 🌱"),
    );
    public.apply(&edited, &public.plan(&edited, true), true);
    assert_ne!(public.revision(), initial);
    assert_eq!(
        find(&public, "declaration", "page-title", Some(&module)),
        title
    );
    check(&public);

    // The generated operator descriptor is usable without any manual field changes.
    let generated = public.root.path().join("generated");
    std::fs::create_dir(&generated).unwrap();
    let artifact = generated.join("application.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let artifact_bytes = std::fs::read(&artifact).unwrap();
    std::fs::remove_dir_all(&public.project).unwrap();
    for input in [draft, invalid, edited] {
        std::fs::remove_file(input).unwrap();
    }
    assert!(!public.project.exists());
    let server = http::Server::start(&public, "web", &descriptor, &[]);
    assert!(server.address.ip().is_loopback());
    assert_ne!(server.address.port(), 0);
    let headers = [("Host", "localhost")];
    let get = http::send(server.address, "GET", "/", &headers, "");
    assert_eq!(get.status, 200);
    assert!(get.body.contains("<title>Native &lt;web&gt; 🌱</title>"));
    assert!(get.body.contains("<h2>Hello, friend</h2>"));
    assert!(get.body.contains("<form method=\"get\""));
    assert!(get.body.contains("<body class=\"light\">"));
    assert!(!get.body.contains("<script"));
    for value in [
        "content-type: text/html; charset=utf-8",
        "x-content-type-options: nosniff",
        "default-src 'none'",
        "form-action 'self'",
        "frame-ancestors 'none'",
        "referrer-policy: same-origin",
        "cache-control: no-store",
    ] {
        assert!(get.headers.contains(value), "{value}");
    }
    assert!(!get.headers.contains("set-cookie:"));
    let injection = "\"><script>alert(1)</script>& 日本語🌱";
    let changed = http::send(
        server.address,
        "GET",
        &format!("/?name={}&theme=dark", encoded(injection)),
        &headers,
        "",
    );
    assert_eq!(changed.status, 200);
    assert!(changed.body.contains("<body class=\"dark\">"));
    assert!(
        changed
            .body
            .contains("Hello, &quot;&gt;&lt;script&gt;alert(1)&lt;/script&gt;&amp; 日本語🌱")
    );
    assert!(!changed.body.contains("<script"));
    for (route, expected, theme) in [
        (
            "/?name=first&name=second&theme=DARK",
            "Hello, first",
            "light",
        ),
        ("/?name=&theme=other", "Hello, ", "light"),
        ("/?name=a%2Bb+c&theme=dark", "Hello, a+b c", "dark"),
    ] {
        let response = http::send(server.address, "GET", route, &headers, "");
        assert_eq!(response.status, 200);
        assert!(response.body.contains(&format!("<h2>{expected}</h2>")));
        assert!(response.body.contains(&format!("<body class=\"{theme}\">")));
    }
    for (method, route) in [("GET", "/missing"), ("POST", "/"), ("PUT", "/")] {
        assert_eq!(
            http::send(server.address, method, route, &headers, "").status,
            404
        );
    }
    let again = http::send(server.address, "GET", "/", &headers, "");
    assert_eq!(again.body, get.body, "query form has no saved server state");
    server.stop();
    let restarted = http::Server::start(&public, "web-restarted", &descriptor, &[]);
    assert_eq!(
        http::send(restarted.address, "GET", "/", &headers, "").body,
        get.body
    );
    restarted.stop();
    assert!(!public.project.exists());
    assert_eq!(std::fs::read(&artifact).unwrap(), artifact_bytes);
}
