mod context;
mod control;
mod r#match;
mod session;
mod transactions;

pub(super) use super::{case_dir, request, response};
use crate::semantic::schema::{
    HoleContextResult, LegalActionsResult, ResponseResult, SnapshotResult,
};

pub(super) fn function_source(body: &str) -> String {
    format!(
        concat!(
            "def/\nname/\nf\n/name\nfn/\nsig/\ninputs/\ni64\ni64\n/inputs\noutput/\ni64\n/output\n/sig\n",
            "params/\nx\ni64\ny\ni64\n/params\n{body}/fn\n/def\n",
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nf/\n0\n1\n/f\n/main\n",
        ),
        body = body
    )
}

pub(super) fn hole(identity: &str, goal: Option<&str>) -> String {
    let goal = goal.map_or_else(String::new, |goal| {
        format!("goal/\nstring-literal/\n{goal}\n/string-literal\n/goal\n")
    });
    format!("hole/\nname/\n{identity}\n/name\n{goal}/hole\n")
}

pub(super) fn snapshot(root: &std::path::Path) -> (String, Box<SnapshotResult>) {
    let encoded = crate::semantic::execute(&request(root, "{\"kind\":\"snapshot\"}"))
        .expect("execute snapshot");
    let decoded = response(&encoded);
    let revision = decoded.revision.expect("snapshot revision");
    let ResponseResult::Snapshot { snapshot } = decoded.result else {
        panic!("expected snapshot")
    };
    (revision, snapshot)
}

pub(super) fn context(root: &std::path::Path) -> HoleContextResult {
    let (revision, snapshot) = snapshot(root);
    let node = snapshot
        .nodes
        .iter()
        .find(|node| node.kind == crate::semantic::schema::SemanticNodeKind::TypedHole)
        .expect("typed-hole node");
    let operation = format!(
        "{{\"kind\":\"hole-context\",\"revision\":{revision:?},\"node\":{}}}",
        node.index,
    );
    let encoded = crate::semantic::execute(&request(root, &operation)).expect("hole context");
    let ResponseResult::HoleContext { context } = response(&encoded).result else {
        panic!("expected hole context")
    };
    *context
}

pub(super) fn actions(root: &std::path::Path) -> LegalActionsResult {
    let (revision, snapshot) = snapshot(root);
    let node = snapshot
        .nodes
        .iter()
        .find(|node| node.kind == crate::semantic::schema::SemanticNodeKind::TypedHole)
        .expect("typed-hole node");
    let operation = format!(
        "{{\"kind\":\"legal-actions\",\"revision\":{revision:?},\"node\":{}}}",
        node.index,
    );
    let encoded = crate::semantic::execute(&request(root, &operation)).expect("legal actions");
    let ResponseResult::LegalActions { actions } = response(&encoded).result else {
        panic!("expected legal actions")
    };
    *actions
}
