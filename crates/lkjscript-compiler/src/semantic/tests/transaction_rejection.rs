use super::*;
use crate::semantic::schema::{
    FilePrecondition, ProtocolErrorCode, ResponseResult, SnapshotResult, TransactionOperation,
};

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
        "def/\nname/\nf\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\n",
        "params/\n/params\nunit\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nf/\n/f\n/main\n",
    );
    std::fs::write(&root, source).expect("write stale source");
    let stale = response(
        &crate::semantic::execute(&request(
            &root,
            "{\"kind\":\"query-node\",\"revision\":\"00\",\"node\":0}",
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
fn semantic_boundary_limits_source_that_trusted_loader_accepts_unchanged() {
    const SOURCE_BYTES: usize = 16 * 1024 * 1024 + 1;
    let directory = case_dir("source-byte-boundary");
    let root = directory.join("main.lkjscript");
    let mut source = String::from(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n;;",
    );
    let trailing_bytes = SOURCE_BYTES
        .checked_sub(source.len() + 1)
        .expect("large boundary fixture prefix");
    source.extend(std::iter::repeat_n('x', trailing_bytes));
    source.push('\n');
    assert_eq!(source.len(), SOURCE_BYTES);
    std::fs::write(&root, &source).expect("write boundary source");

    crate::source::load(&root).expect("trusted loader is explicitly unrestricted");
    let limited = response(
        &crate::semantic::execute(&request(&root, "{\"kind\":\"snapshot\"}"))
            .expect("typed boundary response"),
    );
    let ResponseResult::Error { error, .. } = limited.result else {
        panic!("Semantic Source boundary must reject the over-policy source");
    };
    assert_eq!(error.code, ProtocolErrorCode::ResourceLimit);
    assert!(error.message.contains("aggregate-source-bytes"));
    assert_eq!(
        std::fs::read_to_string(&root).expect("source after boundary rejection"),
        source
    );
}

#[test]
fn staged_transaction_obeys_boundary_bytes_before_publication() {
    let directory = case_dir("staged-byte-policy");
    let root = directory.join("main.lkjscript");
    let source = concat!(
        "def/\nname/\nf\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\n",
        "params/\n/params\nunit\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nf/\n/f\n/main\n",
    );
    std::fs::write(&root, source).expect("write staged byte-policy source");
    let (_, snapshot) = take_snapshot(&root);
    let declaration = snapshot
        .declarations
        .iter()
        .find(|declaration| declaration.name == "f")
        .expect("function declaration");
    let file = snapshot.source_units.first().expect("source unit");
    let operation = TransactionOperation::RenameDeclaration {
        declaration_key: declaration.key.clone(),
        entity_fingerprint: declaration.fingerprint.clone(),
        new_name: "renamed-beyond-boundary".to_string(),
    };
    let precondition = FilePrecondition {
        path: file.path.clone(),
        bytes: file.bytes,
        sha256: file.sha256.clone(),
    };
    let tree = crate::source::load(&root).expect("trusted source load");
    let failure = match crate::semantic::transaction::stage(
        &tree,
        &[operation],
        &[precondition],
        crate::source::SourceBytePolicy::limited(file.bytes),
    ) {
        Ok(_) => panic!("staged aggregate must obey the request boundary policy"),
        Err(failure) => failure,
    };
    assert_eq!(failure.code, ProtocolErrorCode::ResourceLimit);
    assert!(failure.message.contains("aggregate-source-bytes"));
    assert_eq!(
        std::fs::read_to_string(&root).expect("source after rejected staging"),
        source
    );
}

#[test]
fn user_calls_reject_builtin_and_context_form_names() {
    let directory = case_dir("closed-user-call");
    let root = directory.join("main.lkjscript");
    std::fs::write(
        &root,
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n",
    )
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
    for name in ["add", "let", "sig"] {
        let operation = format!(
            concat!(
                "{{\"kind\":\"apply-transaction\",\"mode\":\"preview\",",
                "\"base_revision\":\"{}\",\"file_preconditions\":[],",
                "\"operations\":[{{\"kind\":\"replace-expression\",",
                "\"declaration_key\":\"{}\",\"entity_fingerprint\":\"{}\",",
                "\"node\":{},\"node_fingerprint\":\"{}\",",
                "\"expression\":{{\"kind\":\"user-call\",\"name\":{},",
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
