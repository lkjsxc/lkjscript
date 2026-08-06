use super::*;
use crate::semantic::schema::{FactRecord, ResponseResult};

#[test]
fn high_wire_node_identity_rejects_without_narrowing_or_aliasing() {
    let root = case_dir("query-high-node").join("main.lkjscript");
    std::fs::write(
        &root,
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n",
    )
    .expect("write query source");
    let snapshot = response(
        &crate::semantic::execute(&request(&root, "{\"kind\":\"snapshot\"}"))
            .expect("query snapshot"),
    );
    let revision = snapshot.revision.expect("query revision");
    let high = u64::from(u32::MAX) + 1;
    let operation =
        format!("{{\"kind\":\"query-node\",\"revision\":\"{revision}\",\"node\":{high}}}");
    let queried = response(
        &crate::semantic::execute(&request(&root, &operation))
            .expect("high node identity reaches semantic validation"),
    );
    let ResponseResult::Error { error, .. } = queried.result else {
        panic!("high node identity unexpectedly resolved");
    };
    assert_eq!(
        error.code,
        crate::semantic::schema::ProtocolErrorCode::UnknownNode
    );
}

#[test]
fn query_reports_only_correlated_compiler_facts() {
    let root = case_dir("query-facts").join("main.lkjscript");
    let source = concat!(
        "def/\nname/\nf\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\n",
        "params/\n/params\nunit\n/fn\n/def\n",
        "def/\nname/\nshadow\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nparams/\n/params\n",
        "let/\nbind/\nf\nunit\n/bind\nf\n/let\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nf/\n/f\n/main\n",
    );
    std::fs::write(&root, source).expect("write query source");
    let snapshot = response(
        &crate::semantic::execute(&request(&root, "{\"kind\":\"snapshot\"}"))
            .expect("query snapshot"),
    );
    let revision = snapshot.revision.expect("query revision");
    let ResponseResult::Snapshot { snapshot } = snapshot.result else {
        panic!("expected query snapshot");
    };
    let main = snapshot
        .declarations
        .iter()
        .find(|declaration| declaration.name == "$main")
        .expect("main declaration");
    let global_call = snapshot
        .nodes
        .iter()
        .find(|node| {
            matches!(
                &node.value,
                Some(crate::semantic::schema::SemanticNodeValue::UserFunction { name })
                    if name == "f"
            ) && node.declaration.as_deref() == Some(&main.key)
        })
        .expect("global call node");
    let operation = format!(
        "{{\"kind\":\"query-node\",\"revision\":\"{revision}\",\"node\":{}}}",
        global_call.index
    );
    let queried =
        response(&crate::semantic::execute(&request(&root, &operation)).expect("query response"));
    let ResponseResult::QueryNode { query } = queried.result else {
        panic!("expected node query");
    };
    assert!(matches!(query.facts.binding, FactRecord::Available { .. }));
    let FactRecord::Available {
        source_revision,
        cardinality,
        producer,
        ..
    } = &query.facts.binding
    else {
        panic!("binding must be available");
    };
    assert_eq!(source_revision, &revision);
    assert_eq!(producer.component, "lkjscript-compiler");
    assert_eq!(
        *cardinality,
        crate::semantic::schema::RelationCardinality::One
    );
    let FactRecord::Unavailable {
        source_revision,
        cardinality,
        reason,
        ..
    } = &query.facts.ownership_place_loan
    else {
        panic!("ownership correlation must be unavailable");
    };
    assert_eq!(source_revision, &revision);
    assert_eq!(
        *cardinality,
        crate::semantic::schema::RelationCardinality::Zero
    );
    assert_eq!(
        *reason,
        crate::semantic::schema::UnavailableReason::NoExactSourceCorrelation
    );

    let shadow = snapshot
        .declarations
        .iter()
        .find(|declaration| declaration.name == "shadow")
        .expect("shadow declaration");
    let local = snapshot
        .nodes
        .iter()
        .rev()
        .find(|node| {
            node.kind == crate::semantic::schema::SemanticNodeKind::NameReference
                && matches!(
                    &node.value,
                    Some(crate::semantic::schema::SemanticNodeValue::SourceName { name })
                        if name == "f"
                )
                && node.declaration.as_deref() == Some(&shadow.key)
        })
        .expect("local binding node");
    let operation = format!(
        "{{\"kind\":\"query-node\",\"revision\":\"{revision}\",\"node\":{}}}",
        local.index
    );
    let queried = response(
        &crate::semantic::execute(&request(&root, &operation)).expect("local query response"),
    );
    let ResponseResult::QueryNode { query } = queried.result else {
        panic!("expected local node query");
    };
    assert!(matches!(
        query.facts.binding,
        FactRecord::Unavailable { .. }
    ));
}
