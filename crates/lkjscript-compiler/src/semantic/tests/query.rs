use super::*;
use crate::semantic::schema::{FactRecord, ResponseResult};

#[test]
fn query_reports_only_correlated_compiler_facts() {
    let root = case_dir("query-facts").join("main.lkjscript");
    let source = concat!(
        "def/\nname/\nf\n/name\nfn/\nsig/\n->\nUnit\n/sig\n",
        "params/\n/params\nunit\n/fn\n/def\n",
        "def/\nname/\nshadow\n/name\nfn/\nsig/\n->\nUnit\n/sig\nparams/\n/params\n",
        "let/\nbind/\nf\nunit\n/bind\nf\n/let\n/fn\n/def\n",
        "main/\nsig/\n->\nUnit\n/sig\nf/\n/f\n/main\n",
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
            node.label.as_deref() == Some("f") && node.declaration.as_deref() == Some(&main.key)
        })
        .expect("global call node");
    let operation = format!(
        "{{\"kind\":\"query_node\",\"revision\":\"{revision}\",\"node\":{}}}",
        global_call.index
    );
    let queried =
        response(&crate::semantic::execute(&request(&root, &operation)).expect("query response"));
    let ResponseResult::QueryNode { query } = queried.result else {
        panic!("expected node query");
    };
    assert!(matches!(query.facts.binding, FactRecord::Available { .. }));
    assert!(matches!(
        query.facts.ownership,
        FactRecord::Unavailable { .. }
    ));

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
            node.label.as_deref() == Some("f") && node.declaration.as_deref() == Some(&shadow.key)
        })
        .expect("local binding node");
    let operation = format!(
        "{{\"kind\":\"query_node\",\"revision\":\"{revision}\",\"node\":{}}}",
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
