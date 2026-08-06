use super::*;

#[test]
fn published_transaction_advances_session_revision() {
    let directory = case("transaction");
    let root = directory.join("main.lkjscript");
    std::fs::write(
        &root,
        concat!(
            "def/\nname/\nf\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\n",
            "params/\n/params\nunit\n/fn\n/def\n",
            "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nf/\n/f\n/main\n",
        ),
    )
    .expect("write transaction source");
    let snapshot = semantic_request(&root, "{\"kind\":\"snapshot\"}");
    let operation = execute_operation(&snapshot);
    let mut session = SemanticSession::new();
    let (_, first) = handle(&mut session, &session_request("snapshot", 0, &operation));
    let semantic = &first["response"]["response"];
    let source_revision = semantic["revision"].as_str().expect("source revision");
    let snapshot = &semantic["result"]["snapshot"];
    let declaration = snapshot["declarations"]
        .as_array()
        .expect("declarations")
        .iter()
        .find(|declaration| declaration["name"] == "f")
        .expect("function declaration");
    let preconditions = snapshot["source_units"]
        .as_array()
        .expect("source units")
        .iter()
        .map(|unit| {
            serde_json::json!({
                "path": unit["path"],
                "bytes": unit["bytes"],
                "sha256": unit["sha256"],
            })
        })
        .collect::<Vec<_>>();
    let transaction = serde_json::json!({
        "kind": "apply-transaction",
        "mode": "publish",
        "base_revision": source_revision,
        "file_preconditions": preconditions,
        "operations": [{
            "kind": "rename-declaration",
            "declaration_key": declaration["key"],
            "entity_fingerprint": declaration["fingerprint"],
            "new_name": "g",
        }],
    });
    let semantic = semantic_request(&root, &transaction.to_string());
    let operation = execute_operation(&semantic);
    let (_, published) = handle(&mut session, &session_request("publish", 1, &operation));
    assert_eq!(published["revision"], 2);
    assert_eq!(
        published["response"]["response"]["result"]["kind"],
        "apply-transaction"
    );
    let updated = std::fs::read_to_string(root).expect("published source");
    assert!(updated.contains("name/\ng\n/name"));
    assert!(updated.contains("g/\n/g"));
}
