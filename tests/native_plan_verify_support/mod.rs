use crate::components::{Native, path};
use serde_json::{Value, json};

mod author;
mod batches;
pub use batches::{Batch, batches};
pub mod cases;
pub mod mutations;
mod outcome_cases;
mod outcome_refusals;
pub mod outcomes;
pub use author::author;

pub const VERIFIER: [(&str, &str); 6] = [
    (
        "plan-verify-data",
        include_str!("../../examples/dependency-plan/verification/data.lkjc"),
    ),
    (
        "plan-verify-members",
        include_str!("../../examples/dependency-plan/verification/members.lkjc"),
    ),
    (
        "plan-verify-edges",
        include_str!("../../examples/dependency-plan/verification/edges.lkjc"),
    ),
    (
        "plan-verify-walk",
        include_str!("../../examples/dependency-plan/verification/walk.lkjc"),
    ),
    (
        "plan-verify-stages",
        include_str!("../../examples/dependency-plan/verification/stages.lkjc"),
    ),
    (
        "plan-verify",
        include_str!("../../examples/dependency-plan/verification/verify.lkjc"),
    ),
];
pub const CONSUMER: [(&str, &str); 2] = [
    (
        "verified-plan",
        include_str!("../../examples/dependency-plan/verification/application.lkjc"),
    ),
    (
        "verified-plan-batch",
        include_str!("../../examples/dependency-plan/verification/batch.lkjc"),
    ),
];
pub const DESCRIPTORS: [(&str, &str); 6] = [
    (
        "flat",
        include_str!("../../examples/dependency-plan/verification/flat.deployment.json"),
    ),
    (
        "chunked",
        include_str!("../../examples/dependency-plan/verification/chunked.deployment.json"),
    ),
    (
        "batch-flat",
        include_str!("../../examples/dependency-plan/verification/batch-flat.deployment.json"),
    ),
    (
        "batch-chunked",
        include_str!("../../examples/dependency-plan/verification/batch-chunked.deployment.json"),
    ),
    (
        "verify",
        include_str!("../../examples/dependency-plan/verification/verify.deployment.json"),
    ),
    (
        "verify-batch",
        include_str!("../../examples/dependency-plan/verification/verify-batch.deployment.json"),
    ),
];

pub fn refusals(public: &Native, descriptor: &std::path::Path) {
    let empty_plan = cases::empty_plan();
    let proposal = json!({"roots":[],"nodes":[]});
    let valid = public.input(
        "verifier-recovery.json",
        &json!([proposal, empty_plan]).to_string(),
    );
    let mut wrong = json!([{"roots":[90],"nodes":[]}, cases::empty_plan()]);
    // Complete raw admission must reject even though source validation would fail first.
    wrong[1]["unreachable"] = json!([false]);
    let wrong = public.input("verifier-wrong-type.json", &wrong.to_string());
    let absent = public.root.join("verifier-wrong-type-result.json");
    public.cli(
        &public.root,
        &[
            "run",
            "--deployment",
            path(descriptor),
            "--arguments-file",
            path(&wrong),
            "--result-file",
            path(&absent),
        ],
        false,
    );
    assert!(!absent.exists());
    public.compare(
        "verifier-after-type",
        descriptor,
        &valid,
        &cases::verified(),
    );
    let mut restricted: Value = serde_json::from_str(DESCRIPTORS[4].1).unwrap();
    restricted["execution"] =
        json!({"instruction_fuel":1,"maximum_call_depth":4096,"maximum_value_stack":1000000});
    let restricted = public.input("verifier-refusal.deployment.json", &restricted.to_string());
    let absent = public.root.join("verifier-refusal-result.json");
    let failure = public.cli(
        &public.root,
        &[
            "run",
            "--deployment",
            path(&restricted),
            "--arguments-file",
            path(&valid),
            "--result-file",
            path(&absent),
        ],
        false,
    );
    assert_eq!(
        crate::components::field(&failure, "diagnostic", "class"),
        "resource"
    );
    assert_eq!(
        crate::components::field(&failure, "diagnostic", "code"),
        "normalized_instruction_steps"
    );
    assert!(!absent.exists());
    public.compare(
        "verifier-after-refusal",
        descriptor,
        &valid,
        &cases::verified(),
    );
}
