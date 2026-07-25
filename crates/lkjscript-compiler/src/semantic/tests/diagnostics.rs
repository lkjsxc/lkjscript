use super::*;
use crate::semantic::schema::{DiagnosticCode, ResponseResult};

fn hir_diagnostics(source: &str, label: &str) -> Vec<crate::semantic::schema::DiagnosticRecord> {
    let root = case_dir(label).join("main.lkjscript");
    std::fs::write(&root, source).expect("write diagnostic source");
    let snapshot = response(
        &crate::semantic::execute(&request(&root, "{\"kind\":\"snapshot\"}"))
            .expect("diagnostic snapshot"),
    );
    let revision = snapshot.revision.expect("diagnostic revision");
    let operation =
        format!("{{\"kind\":\"diagnostics\",\"revision\":\"{revision}\",\"analysis\":\"hir\"}}");
    let result = response(
        &crate::semantic::execute(&request(&root, &operation)).expect("diagnostics response"),
    );
    let ResponseResult::Diagnostics { result } = result.result else {
        panic!("expected diagnostics result");
    };
    assert!(result.complete);
    result.diagnostics
}

#[test]
fn six_foundation_diagnostic_codes_have_closed_complete_records() {
    let unknown = hir_diagnostics(
        "main/\nsig/\n->\nUnit\n/sig\nmissing/\n/missing\n/main\n",
        "unknown",
    );
    assert_eq!(unknown[0].code, DiagnosticCode::UnknownName);
    let arity = hir_diagnostics(
        concat!(
            "def/\nname/\nf\n/name\nfn/\nsig/\n->\nUnit\n/sig\n",
            "params/\n/params\nunit\n/fn\n/def\n",
            "main/\nsig/\n->\nUnit\n/sig\nf/\nunit\n/f\n/main\n",
        ),
        "arity",
    );
    assert_eq!(arity[0].code, DiagnosticCode::CallArity);
    assert_eq!(arity[0].expected.as_deref(), Some("0"));
    let mismatch = hir_diagnostics("main/\nsig/\n->\nI64\n/sig\nunit\n/main\n", "mismatch");
    assert_eq!(mismatch[0].code, DiagnosticCode::TypeMismatch);
    assert_eq!(mismatch[0].expected.as_deref(), Some("I64"));
    assert_eq!(mismatch[0].actual.as_deref(), Some("Unit"));
    for diagnostic in unknown.iter().chain(&arity).chain(&mismatch) {
        assert_eq!(diagnostic.schema, "lkjscript.diagnostic");
        assert_eq!(diagnostic.version, 1);
        assert!(!diagnostic.human_rendering.is_empty());
        assert!(!diagnostic.agent_rendering.is_empty());
        assert!(diagnostic.repairs.is_empty());
    }
}

#[test]
fn source_and_stale_failures_use_registered_codes() {
    let root = case_dir("source-diagnostic").join("main.lkjscript");
    std::fs::write(&root, "main/\nsig/\n->\nUnit\n/sig\nunit\n/main-wrong\n")
        .expect("write unmatched source");
    let failure = response(
        &crate::semantic::execute(&request(&root, "{\"kind\":\"snapshot\"}"))
            .expect("source failure response"),
    );
    let ResponseResult::Error {
        diagnostic: Some(diagnostic),
        ..
    } = failure.result
    else {
        panic!("expected source diagnostic");
    };
    assert_eq!(diagnostic.code, DiagnosticCode::UnmatchedMarker);

    std::fs::write(&root, "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n")
        .expect("write valid stale source");
    let stale = response(
        &crate::semantic::execute(&request(
            &root,
            "{\"kind\":\"query_node\",\"revision\":\"00\",\"node\":0}",
        ))
        .expect("stale diagnostic response"),
    );
    let ResponseResult::Error {
        diagnostic: Some(diagnostic),
        ..
    } = stale.result
    else {
        panic!("expected stale diagnostic");
    };
    assert_eq!(diagnostic.code, DiagnosticCode::StaleEdit);
    assert_eq!(diagnostic.repairs.len(), 1);
}
