use super::*;
use crate::semantic::schema::{ResponseResult, SnapshotResult};

fn take_snapshot(root: &std::path::Path) -> (String, SnapshotResult) {
    let output = crate::semantic::execute(&request(root, "{\"kind\":\"snapshot\"}"))
        .expect("snapshot request");
    let decoded = response(&output);
    let revision = decoded.revision.expect("snapshot revision");
    let ResponseResult::Snapshot { snapshot } = decoded.result else {
        panic!("expected snapshot response");
    };
    (revision, *snapshot)
}

#[test]
fn stale_revision_and_collision_leave_sources_identical() {
    let directory = case_dir("stale");
    let root = directory.join("main.lkjscript");
    let source = concat!(
        "def/\nname/\nf\n/name\nfn/\nsig/\n->\nUnit\n/sig\n",
        "params/\n/params\nunit\n/fn\n/def\n",
        "main/\nsig/\n->\nUnit\n/sig\nf/\n/f\n/main\n",
    );
    std::fs::write(&root, source).expect("write stale source");
    let stale = response(
        &crate::semantic::execute(&request(
            &root,
            "{\"kind\":\"query_node\",\"revision\":\"00\",\"node\":0}",
        ))
        .expect("stale response"),
    );
    assert!(matches!(stale.result, ResponseResult::Error { .. }));
    assert_eq!(
        std::fs::read_to_string(root).expect("unchanged stale source"),
        source
    );
}

#[test]
fn user_calls_reject_builtin_and_context_form_names() {
    let directory = case_dir("closed-user-call");
    let root = directory.join("main.lkjscript");
    std::fs::write(&root, "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n")
        .expect("write user-call source");
    let (revision, snapshot) = take_snapshot(&root);
    let main = snapshot
        .declarations
        .iter()
        .find(|declaration| declaration.name == "$main")
        .expect("main declaration");
    let node = snapshot
        .nodes
        .iter()
        .find(|node| node.declaration.as_deref() == Some(&main.key) && node.children.is_empty())
        .expect("replaceable expression");
    for name in ["+", "let", "sig"] {
        let operation = format!(
            concat!(
                "{{\"kind\":\"apply_transaction\",\"mode\":\"preview\",",
                "\"base_revision\":\"{}\",\"file_preconditions\":[],",
                "\"operations\":[{{\"kind\":\"replace_expression\",",
                "\"declaration_key\":\"{}\",\"entity_fingerprint\":\"{}\",",
                "\"node\":{},\"node_fingerprint\":\"{}\",",
                "\"expression\":{{\"kind\":\"user_call\",\"name\":{},",
                "\"arguments\":[]}}}}]}}"
            ),
            revision,
            main.key,
            main.fingerprint,
            node.index,
            node.fingerprint,
            serde_json::to_string(name).expect("name JSON"),
        );
        let rejected = response(
            &crate::semantic::execute(&request(&root, &operation)).expect("protocol response"),
        );
        assert!(matches!(rejected.result, ResponseResult::Error { .. }));
    }
}
