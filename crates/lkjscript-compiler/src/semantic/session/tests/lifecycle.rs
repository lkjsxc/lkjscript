use super::*;
use crate::semantic::session::{
    MAX_SESSION_LIFETIME_FUEL, MAX_SESSION_REQUESTS, MAX_SESSION_REVISION,
};

fn write_source(root: &std::path::Path) {
    std::fs::write(
        root,
        concat!(
            "def/\nname/\nf\n/name\nfn/\nsig/\n->\nUnit\n/sig\n",
            "params/\n/params\nunit\n/fn\n/def\n",
            "main/\nsig/\n->\nUnit\n/sig\nf/\n/f\n/main\n",
        ),
    )
    .expect("write session source");
}

fn snapshot_request(root: &std::path::Path, profile: &str) -> String {
    execute_operation(&semantic_request(root, profile, "{\"kind\":\"snapshot\"}"))
}

#[test]
fn session_pins_selection_detects_external_change_and_refreshes() {
    let directory = case("lifecycle");
    let root = directory.join("main.lkjscript");
    write_source(&root);
    let other_directory = case("other-root");
    let other = other_directory.join("main.lkjscript");
    write_source(&other);
    let operation = snapshot_request(&root, "default");
    let mut session = SemanticSession::new();
    let (_, first) = handle(&mut session, &session_request("first", 0, &operation));
    assert_eq!(first["revision"], 1);
    assert_eq!(first["response"]["kind"], "execute");
    assert_eq!(first["response"]["session"]["cache_entries"], 0);
    assert_eq!(
        first["response"]["session"]["session_identity"]
            .as_str()
            .expect("session identity")
            .len(),
        64
    );
    assert_eq!(
        first["response"]["session"]["semantic_schema"],
        "lkjscript.semantic-source"
    );

    let read = session_request("read", 1, &operation);
    let (encoded, second) = handle(&mut session, &read);
    let (repeated, _) = handle(&mut session, &read);
    assert_eq!(encoded, repeated);
    assert_eq!(second["revision"], 1);

    let (_, stale) = handle(&mut session, &session_request("stale", 0, &operation));
    assert_eq!(stale["response"]["error"]["code"], "stale_session_revision");
    let profile = snapshot_request(&root, "build");
    let (_, mismatch) = handle(&mut session, &session_request("profile", 1, &profile));
    assert_eq!(
        mismatch["response"]["error"]["code"],
        "pinned_profile_mismatch"
    );
    let other_operation = snapshot_request(&other, "default");
    let (_, mismatch) = handle(&mut session, &session_request("root", 1, &other_operation));
    assert_eq!(
        mismatch["response"]["error"]["code"],
        "pinned_root_mismatch"
    );

    std::fs::write(
        &root,
        format!(
            "{};; external\n",
            std::fs::read_to_string(&root).expect("read source")
        ),
    )
    .expect("external edit");
    let (_, external) = handle(&mut session, &session_request("external", 1, &operation));
    assert_eq!(
        external["response"]["error"]["code"],
        "external_source_change"
    );
    let (_, refreshed) = handle(
        &mut session,
        &session_request("refresh", 1, "{\"kind\":\"refresh\"}"),
    );
    assert_eq!(refreshed["revision"], 2);
    assert_eq!(refreshed["response"]["kind"], "refresh");
    let (_, shutdown) = handle(
        &mut session,
        &session_request("shutdown", 2, "{\"kind\":\"shutdown\"}"),
    );
    assert_eq!(shutdown["response"]["kind"], "shutdown");
    assert!(session.closed);
}

#[test]
fn request_fuel_and_revision_bounds_return_closed_errors() {
    let directory = case("limits");
    let root = directory.join("main.lkjscript");
    write_source(&root);
    let operation = snapshot_request(&root, "default");
    let mut request_session = SemanticSession::new();
    handle(
        &mut request_session,
        &session_request("first", 0, &operation),
    );
    request_session.requests = MAX_SESSION_REQUESTS;
    let (_, limited) = handle(
        &mut request_session,
        &session_request("limited", 1, &operation),
    );
    assert_eq!(limited["response"]["error"]["code"], "resource_limit");

    let mut fuel_session = SemanticSession::new();
    handle(&mut fuel_session, &session_request("first", 0, &operation));
    fuel_session.lifetime_fuel = MAX_SESSION_LIFETIME_FUEL;
    let (_, limited) = handle(&mut fuel_session, &session_request("fuel", 1, &operation));
    assert_eq!(limited["response"]["error"]["code"], "resource_limit");

    let mut revision_session = SemanticSession::new();
    handle(
        &mut revision_session,
        &session_request("first", 0, &operation),
    );
    revision_session.revision = MAX_SESSION_REVISION;
    let (_, limited) = handle(
        &mut revision_session,
        &session_request("revision", MAX_SESSION_REVISION, "{\"kind\":\"refresh\"}"),
    );
    assert_eq!(limited["response"]["error"]["code"], "revision_overflow");
}
