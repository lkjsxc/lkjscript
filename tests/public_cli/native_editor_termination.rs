//! Process termination preserves committed ordinary native application data.
use super::*;
use rustix::process::Signal;

#[test]
fn resident_editor_sigterm_preserves_committed_data() {
    let public = Native::template("web-editor");
    public.cli(&["check"], true);
    let artifact = public.root.path().join("editor.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let mut descriptor: Value = serde_json::from_slice(
        &std::fs::read(public.project.join("service.deployment.json")).unwrap(),
    )
    .unwrap();
    descriptor["artifact"] = json!("editor.lkja");
    descriptor["listen"] = json!("127.0.0.1:0");
    descriptor["configuration"]["origin"]["value"] = json!("http://localhost");
    let data = public.root.path().join("notes.lkjdata");
    assert!(!data.exists());
    compact_success_at(
        &public.executable,
        public.root.path(),
        &["data", "initialize", "--root", path(&data)],
    );
    // Only disposable test authorship is removed. The newly initialized store is retained.
    std::fs::remove_dir_all(&public.project).unwrap();
    let head = || std::fs::read(data.join("HEAD")).unwrap();

    let first = http::Server::start(&public, "termination-first", &descriptor, AUTH_ENVIRONMENT);
    let empty = http::send(first.address, "GET", "/", &headers(), "");
    assert_eq!(empty.status, 200);
    assert_eq!(revision(&empty.body), "0");
    assert_eq!(draft(&empty.body), "");
    let text = "committed before SIGTERM 🌱";
    assert_eq!(
        http::send(first.address, "POST", "/", &headers(), &encoded("0", text)).status,
        303
    );
    let committed = head();
    first.stop_with_signal(Signal::TERM);
    assert_eq!(head(), committed);

    let second = http::Server::start(&public, "termination-second", &descriptor, AUTH_ENVIRONMENT);
    let restored = http::send(second.address, "GET", "/", &headers(), "");
    assert_eq!(restored.status, 200);
    assert_eq!(revision(&restored.body), "1");
    assert_eq!(draft(&restored.body), text);
    assert_eq!(head(), committed);
    assert_eq!(
        http::send(
            second.address,
            "POST",
            "/",
            &headers(),
            &encoded("0", "stale overwrite must not happen")
        )
        .status,
        409
    );
    assert_eq!(head(), committed);
    let changed = "committed after restart 🌱";
    assert_eq!(
        http::send(
            second.address,
            "POST",
            "/",
            &headers(),
            &encoded("1", changed)
        )
        .status,
        303
    );
    let updated = head();
    assert_ne!(updated, committed);
    second.stop_with_signal(Signal::TERM);
    assert_eq!(head(), updated);

    let final_read =
        http::Server::start(&public, "termination-final", &descriptor, AUTH_ENVIRONMENT);
    let restored = http::send(final_read.address, "GET", "/", &headers(), "");
    assert_eq!(restored.status, 200);
    assert_eq!(revision(&restored.body), "2");
    assert_eq!(draft(&restored.body), changed);
    final_read.stop();
    assert_eq!(head(), updated);
    assert!(!public.project.exists());
    eprintln!(
        "copied native editor: two SIGTERM joins, revisions 1/2 retained, stale save 409, final SIGINT join"
    );
}
