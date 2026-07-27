use lkjscript_core::ResourceCategory;

use super::*;

#[test]
fn valid_128_request_session_rolls_journal_with_deterministic_prefix() {
    let directory = case("ledger-rollover");
    let root = directory.join("main.lkjscript");
    std::fs::write(
        &root,
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n",
    )
    .expect("write source");
    let operation = execute_operation(&semantic_request(
        &root,
        "default",
        "{\"kind\":\"snapshot\"}",
    ));
    let run = || {
        let mut session = SemanticSession::new();
        let (_, first) = handle(&mut session, &session_request("first", 0, &operation));
        assert_eq!(first["revision"], 1);
        for request in 1..128 {
            let id = format!("stale-{request:03}");
            let (_, response) = handle(&mut session, &session_request(&id, 0, &operation));
            assert_eq!(
                response["response"]["error"]["code"],
                "stale-session-revision"
            );
        }
        let ledger = session.ledger.as_ref().expect("session ledger");
        let prefix = ledger.prefix();
        assert!(prefix.reservation_count() > 0);
        assert!(prefix.reservation_count() < lkjscript_core::MAX_BUDGET_JOURNAL_ENTRIES);
        assert_eq!(
            prefix.committed(ResourceCategory::SemanticSessionOutputBytes),
            ledger.used(ResourceCategory::SemanticSessionOutputBytes)
        );
        prefix
    };
    assert_eq!(run(), run());
}
