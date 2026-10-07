//! Real ordinary applications share code, not configuration, grants or data.
use super::native_http as http;
use super::*;
use rustix::process::Signal;
use serde_json::json;

#[path = "shared_runtime/faults.rs"]
mod faults;
#[path = "shared_runtime/process.rs"]
pub(super) mod process;
use process::Group;

fn build(public: &Native, output: &str) {
    public.cli(&["check"], true);
    public.cli(
        &["build", "--output", path(&public.root.path().join(output))],
        true,
    );
}

fn descriptor(public: &Native) -> Value {
    serde_json::from_slice(&std::fs::read(public.project.join("service.deployment.json")).unwrap())
        .unwrap()
}

fn initialize(public: &Native, root: &str) {
    let result = support::output(
        Command::new(&public.executable)
            .args(["data", "initialize", "--root", root])
            .current_dir(public.root.path())
            .env_clear()
            .env("PATH", ""),
    )
    .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
}

fn note(body: &str) -> &str {
    body.split_once("<textarea ")
        .unwrap()
        .1
        .split_once(">\n")
        .unwrap()
        .1
        .split_once("</textarea>")
        .unwrap()
        .0
}

fn editor_descriptor(base: &Value, label: &str, artifact: &str, variable: &str) -> Value {
    let mut value = base.clone();
    value["artifact"] = json!(artifact);
    value["listen"] = json!("127.0.0.1:0");
    value["configuration"]["origin"]["value"] = json!(format!("http://{label}.localhost"));
    value["secrets"][0]["variable"] = json!(variable);
    value["grants"][3]["adapter"]["root"] = json!(format!("{label}.lkjdata"));
    value
}

#[test]
fn shared_runtime_edits_versions_and_keeps_secrets_configuration_and_data_private() {
    let public = Native::template("web-editor");
    let base = descriptor(&public);
    build(&public, "old.lkja");
    let draft = public.root.path().join("page.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--declaration",
            "editor::page",
            "--output",
            path(&draft),
        ],
        true,
    );
    let source = std::fs::read_to_string(&draft).unwrap();
    assert_eq!(source.matches("(text \"Native notes\")").count(), 1);
    let edited = public.input(
        "edited.lkjc",
        &source.replace("(text \"Native notes\")", "(text \"Shared next version\")"),
    );
    let plan = public.plan(&edited, true);
    public.apply(&edited, &plan, true);
    build(&public, "new.lkja");
    let old_bytes = std::fs::read(public.root.path().join("old.lkja")).unwrap();
    assert_ne!(
        old_bytes,
        std::fs::read(public.root.path().join("new.lkja")).unwrap()
    );
    std::fs::remove_dir_all(&public.project).unwrap();
    std::fs::remove_file(draft).unwrap();
    std::fs::remove_file(edited).unwrap();

    // Public, disposable fixture credentials; neither defaults nor real account secrets.
    let alpha_auth = "Basic YWxwaGE6Zml4dHVyZQ==";
    let beta_auth = "Basic YmV0YTpmaXh0dXJl";
    let environment = [
        ("SHARED_FIXTURE_ALPHA", alpha_auth),
        ("SHARED_FIXTURE_BETA", beta_auth),
    ];
    let mut descriptors = vec![
        editor_descriptor(&base, "alpha", "old.lkja", "SHARED_FIXTURE_ALPHA"),
        editor_descriptor(&base, "beta", "old.lkja", "SHARED_FIXTURE_BETA"),
        editor_descriptor(&base, "next", "new.lkja", "SHARED_FIXTURE_ALPHA"),
        editor_descriptor(&base, "limited", "old.lkja", "SHARED_FIXTURE_ALPHA"),
    ];
    descriptors[3]["execution"]["instruction_fuel"] = json!(1);
    for label in ["alpha", "beta", "next", "limited"] {
        initialize(&public, &format!("{label}.lkjdata"));
    }
    let group = Group::start(&public, &descriptors, &environment);
    let observed = &group.ready["shared_runtime"];
    assert_eq!(observed["instances"], 4);
    assert_eq!(observed["contract_version"], 3);
    let mut counts = observed["programs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|program| program["instances"].as_u64().unwrap())
        .collect::<Vec<_>>();
    counts.sort();
    assert_eq!(counts, [1, 3]);
    let ready = group.ready.to_string();
    assert!(!ready.contains(alpha_auth) && !ready.contains(beta_auth));
    for (index, label, credential, title) in [
        (0, "alpha", alpha_auth, "<title>Native notes</title>"),
        (1, "beta", beta_auth, "<title>Native notes</title>"),
        (2, "next", alpha_auth, "<title>Shared next version</title>"),
    ] {
        let host = format!("{label}.localhost");
        let origin = format!("http://{host}");
        let headers = [
            ("Host", host.as_str()),
            ("Authorization", credential),
            ("Origin", origin.as_str()),
            ("Content-Type", "application/x-www-form-urlencoded"),
        ];
        let reply = http::send(group.address(index), "GET", "/", &headers, "");
        assert_eq!(reply.status, 200);
        assert!(reply.body.contains(title));
        assert_eq!(note(&reply.body), "");
        let text = format!("{label}-private");
        let saved = http::send(
            group.address(index),
            "POST",
            "/",
            &headers,
            &format!("base=0&text={text}&intent=save"),
        );
        assert_eq!(saved.status, 303);
        assert!(saved.headers.contains("location: /"));
        assert!(saved.body.is_empty());
        let reply = http::send(group.address(index), "GET", "/", &headers, "");
        assert_eq!(note(&reply.body), text);
        let wrong = if index == 1 { alpha_auth } else { beta_auth };
        assert_eq!(
            http::send(
                group.address(index),
                "GET",
                "/",
                &[("Host", &host), ("Authorization", wrong)],
                ""
            )
            .status,
            401
        );
        assert_eq!(
            http::send(
                group.address(index),
                "GET",
                "/",
                &[("Host", "foreign.localhost"), ("Authorization", credential)],
                ""
            )
            .status,
            400
        );
    }
    let limited = http::send(
        group.address(3),
        "GET",
        "/",
        &[("Host", "limited.localhost"), ("Authorization", alpha_auth)],
        "",
    );
    assert!(limited.status >= 400);
    assert!(limited.headers.contains("x-lkjscript-failure-code"));
    for (index, host, credential, text) in [
        (0, "alpha.localhost", alpha_auth, "alpha-private"),
        (1, "beta.localhost", beta_auth, "beta-private"),
        (2, "next.localhost", alpha_auth, "next-private"),
    ] {
        let reply = http::send(
            group.address(index),
            "GET",
            "/",
            &[("Host", host), ("Authorization", credential)],
            "",
        );
        assert_eq!(reply.status, 200);
        assert_eq!(note(&reply.body), text);
    }
    let stopped = group.stop(Signal::TERM);
    assert!(
        stopped["instances"][3]["receipt"]["runtime"]["resident"]["failed"]
            .as_u64()
            .unwrap()
            >= 1
    );
    assert_eq!(
        std::fs::read(public.root.path().join("old.lkja")).unwrap(),
        old_bytes
    );
    // A separate-process baseline must give the same persisted state and authority.
    for (index, host, credential, text) in [
        (0, "alpha.localhost", alpha_auth, "alpha-private"),
        (1, "beta.localhost", beta_auth, "beta-private"),
    ] {
        let server = http::Server::start(
            &public,
            &format!("separate-{index}"),
            &descriptors[index],
            &environment,
        );
        let reply = http::send(
            server.address,
            "GET",
            "/",
            &[("Host", host), ("Authorization", credential)],
            "",
        );
        assert_eq!(reply.status, 200);
        assert_eq!(note(&reply.body), text);
        server.stop();
    }
    eprintln!(
        "shared ordinary editor: four instances, two exact versions, private grants/data, joined stop"
    );
}

#[test]
fn shared_runtime_http_and_interactive_share_code_and_join_connected_peer() {
    use std::io::Write;
    use std::net::TcpStream;
    use std::time::Duration;
    let (public, http_descriptor) = super::resident_termination::author("http");
    let (_, mut interactive) = super::resident_termination::author("live");
    interactive["artifact"] = json!("termination.lkja");
    let group = Group::start(&public, &[http_descriptor, interactive], &[]);
    assert_eq!(
        group.ready["shared_runtime"]["programs"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let mut peer = TcpStream::connect_timeout(&group.address(1), Duration::from_secs(5)).unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    peer.set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    peer.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").unwrap();
    let mut line = String::new();
    BufReader::new(&mut peer).read_line(&mut line).unwrap();
    assert!(line.starts_with("HTTP/1.1 101 "), "{line}");
    assert_eq!(
        http::send(
            group.address(0),
            "GET",
            "/health",
            &[("Host", "localhost")],
            ""
        )
        .body,
        "7"
    );
    let stopped = group.stop(Signal::INT);
    assert_eq!(
        stopped["instances"][1]["receipt"]["sessions"]["completed_sessions"],
        1
    );
    drop(peer);
}
