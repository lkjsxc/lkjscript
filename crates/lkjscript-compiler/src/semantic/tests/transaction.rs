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

fn preconditions(snapshot: &SnapshotResult) -> String {
    snapshot
        .source_units
        .iter()
        .map(|file| {
            format!(
                "{{\"path\":{},\"bytes\":{},\"sha256\":\"{}\"}}",
                serde_json::to_string(&file.path).expect("path JSON"),
                file.bytes,
                file.sha256
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
fn cross_import_rename_and_expression_replacement_are_atomic() {
    let directory = case_dir("transactions");
    let root = directory.join("main.lkjscript");
    let library = directory.join("lib.lkjscript");
    let original_root = concat!(
        "import/\nlib.lkjscript\n/import\n;; inc stays in a comment\n",
        "def/\nname/\ntext\n/name\nfn/\nsig/\n->\nStr\n/sig\n",
        "params/\n/params\nstr/\ninc stays in a string\n/str\n/fn\n/def\n",
        "def/\nname/\nshadow\n/name\nfn/\nsig/\n->\nUnit\n/sig\nparams/\n/params\n",
        "let/\nbind/\ninc\nunit\n/bind\ninc\n/let\n/fn\n/def\n",
        "main/\nsig/\n->\nI64\n/sig\ninc/\n2\n/inc\n/main\n",
    );
    let original_library = concat!(
        "def/\nname/\ninc\n/name\nfn/\nsig/\nI64\n->\nI64\n/sig\n",
        "params/\nx\nI64\n/params\n+/\nx\n1\n/+\n/fn\n/def\n",
    );
    std::fs::write(&root, original_root).expect("write root");
    std::fs::write(&library, original_library).expect("write library");
    let (revision, snapshot) = take_snapshot(&root);
    let declaration = snapshot
        .declarations
        .iter()
        .find(|declaration| declaration.name == "inc")
        .expect("inc declaration");
    let operation = format!(
        concat!(
            "{{\"kind\":\"apply_transaction\",\"mode\":\"preview\",",
            "\"base_revision\":\"{revision}\",\"file_preconditions\":[{}],",
            "\"operations\":[{{\"kind\":\"rename_declaration\",",
            "\"declaration_key\":\"{}\",\"entity_fingerprint\":\"{}\",",
            "\"new_name\":\"increment\"}}]}}",
        ),
        preconditions(&snapshot),
        declaration.key,
        declaration.fingerprint,
        revision = revision
    );
    let collision = operation.replace("new_name\":\"increment", "new_name\":\"text");
    let rejected = response(
        &crate::semantic::execute(&request(&root, &collision)).expect("collision response"),
    );
    assert!(matches!(rejected.result, ResponseResult::Error { .. }));
    assert_eq!(
        std::fs::read_to_string(&root).expect("root after collision"),
        original_root
    );
    assert_eq!(
        std::fs::read_to_string(&library).expect("library after collision"),
        original_library
    );
    let preview =
        response(&crate::semantic::execute(&request(&root, &operation)).expect("preview rename"));
    assert!(matches!(
        preview.result,
        ResponseResult::ApplyTransaction { .. }
    ));
    assert_eq!(
        std::fs::read_to_string(&root).expect("root unchanged"),
        original_root
    );
    let publish = operation.replace("\"mode\":\"preview\"", "\"mode\":\"publish\"");
    let published =
        response(&crate::semantic::execute(&request(&root, &publish)).expect("publish rename"));
    assert!(matches!(
        published.result,
        ResponseResult::ApplyTransaction { .. }
    ));
    let renamed_root = std::fs::read_to_string(&root).expect("renamed root");
    let renamed_library = std::fs::read_to_string(&library).expect("renamed library");
    assert!(renamed_root.contains("increment/\n2\n/increment"));
    assert!(renamed_root.contains(";; inc stays in a comment"));
    assert!(renamed_root.contains("inc stays in a string"));
    assert!(renamed_root.contains("bind/\ninc\nunit\n/bind\ninc\n/let"));
    assert!(renamed_library.contains("name/\nincrement\n/name"));

    let (revision, snapshot) = take_snapshot(&root);
    let main = snapshot
        .declarations
        .iter()
        .find(|declaration| declaration.name == "$main")
        .expect("main declaration");
    let call = snapshot
        .nodes
        .iter()
        .find(|node| {
            node.label.as_deref() == Some("increment")
                && node.declaration.as_deref() == Some(&main.key)
        })
        .expect("main call");
    let replace = |kind: &str, value: &str| {
        format!(
            concat!(
                "{{\"kind\":\"apply_transaction\",\"mode\":\"preview\",",
                "\"base_revision\":\"{revision}\",\"file_preconditions\":[{}],",
                "\"operations\":[{{\"kind\":\"replace_expression\",",
                "\"declaration_key\":\"{}\",\"entity_fingerprint\":\"{}\",",
                "\"node\":{},\"node_fingerprint\":\"{}\",",
                "\"expression\":{{\"kind\":\"{kind}\",\"value\":{value}}}}}]}}",
            ),
            preconditions(&snapshot),
            main.key,
            main.fingerprint,
            call.index,
            call.fingerprint,
            revision = revision,
            kind = kind,
            value = value
        )
    };
    let before_failure = std::fs::read(&root).expect("bytes before failed replacement");
    let failed = response(
        &crate::semantic::execute(&request(&root, &replace("string", "\"wrong\"")))
            .expect("typed replacement failure response"),
    );
    assert!(matches!(failed.result, ResponseResult::Error { .. }));
    assert_eq!(
        std::fs::read(&root).expect("bytes after failure"),
        before_failure
    );
    let replaced = response(
        &crate::semantic::execute(&request(&root, &replace("i64", "5")))
            .expect("replacement preview"),
    );
    let transaction = match replaced.result {
        ResponseResult::ApplyTransaction { transaction } => transaction,
        other => panic!("expected replacement transaction, got {other:?}"),
    };
    assert_eq!(transaction.identities[0].relation, "replaced_expression");
    assert_eq!(
        transaction.identities[0].old_node,
        transaction.identities[0].new_node
    );
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
