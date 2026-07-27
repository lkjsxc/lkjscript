use super::*;

#[test]
fn bounded_local_session_serves_current_contract_hole_context() {
    let directory = case_dir("hole-session");
    let root = directory.join("main.lkjscript");
    std::fs::write(&root, function_source(&hole("body", None))).expect("write source");
    let (source_revision, snapshot) = snapshot(&root);
    let hole = snapshot
        .nodes
        .iter()
        .find(|node| node.kind == crate::semantic::schema::SemanticNodeKind::TypedHole)
        .expect("hole node");
    let semantic = serde_json::json!({
        "schema": crate::semantic::SCHEMA,
        "contract": crate::semantic::CONTRACT.to_hex(),
        "profile": "sandbox",
        "root": root.to_string_lossy(),
        "operation": {
            "kind": "hole-context",
            "revision": source_revision,
            "node": hole.index,
        },
    });
    let envelope = serde_json::to_vec(&serde_json::json!({
        "schema": "lkjscript.semantic-session",
        "contract": crate::semantic::session::CONTRACT.to_hex(),
        "request_id": "hole",
        "revision": 0,
        "request": { "kind": "execute", "request": semantic },
    }))
    .expect("encode session request");
    let mut framed = Vec::new();
    framed.extend_from_slice(&(envelope.len() as u64).to_be_bytes());
    framed.extend_from_slice(&envelope);
    let mut output = Vec::new();
    crate::semantic::session::serve(&mut std::io::Cursor::new(framed), &mut output)
        .expect("serve hole context");
    let length = u64::from_be_bytes(output[..8].try_into().expect("response header"));
    assert_eq!(length as usize, output.len() - 8);
    let response: serde_json::Value =
        serde_json::from_slice(&output[8..]).expect("decode session response");
    assert_eq!(response["response"]["kind"], "execute");
    assert_eq!(
        response["response"]["session"]["semantic_contract"],
        crate::semantic::CONTRACT.to_hex()
    );
    assert_eq!(
        response["response"]["response"]["result"]["kind"],
        "hole-context"
    );
    assert_eq!(
        response["response"]["response"]["result"]["context"]["expected_type"]["canonical"],
        "i64"
    );
    assert!(
        response["response"]["response"]["charges"]["hole_search_work"]
            .as_u64()
            .is_some_and(|charge| charge > 0)
    );
    assert!(response["response"]["session"]["limits"]["session_nodes"]
        .as_u64()
        .is_some_and(|limit| limit > 0));
}
