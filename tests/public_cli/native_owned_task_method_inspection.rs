//! Public agent-facing projections must retain task kind and exact requirements.
use super::*;

#[test]
fn native_owned_task_methods_inspection_reports_task_kind_and_exact_requirement() {
    let public = Native::template("command");
    author(
        &public,
        include_str!("../fixtures/owned-task-method-library.lkjc"),
    );
    let draft = std::fs::read_to_string(unchanged(&public, "task-method")).unwrap();
    let id = draft
        .lines()
        .find(|line| line.trim_start().starts_with("(owned-contract edit "))
        .unwrap()
        .split_whitespace()
        .nth(2)
        .unwrap();
    let records = public.cli(&["inspect", "owner", "owned_contract", id], true);
    let methods = records
        .iter()
        .filter(|r| r.operation == "owned.method")
        .collect::<Vec<_>>();
    assert_eq!(methods.len(), 2);
    assert!(methods.iter().all(|r| compact_field(r, "kind") == "task"));
    assert_eq!(
        methods
            .iter()
            .map(|r| compact_field(r, "requirements"))
            .collect::<Vec<_>>(),
        ["0", "1"]
    );
    let requirements = records
        .iter()
        .filter(|r| r.operation == "owned.method-requirement")
        .collect::<Vec<_>>();
    assert_eq!(requirements.len(), 1);
    assert_eq!(
        compact_field(requirements[0], "method"),
        "method_80000000000000000000000000000002"
    );
    let requirement = compact_field(requirements[0], "requirement");
    assert!(
        requirement.starts_with("pkg_") && requirement.contains("/req_"),
        "{requirement}"
    );
}
