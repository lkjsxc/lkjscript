use super::{cases, outcomes::DESCRIPTORS};
use crate::components::{Native, field, path};
use serde_json::{Value, json};

pub fn exercise(public: &Native, descriptor: &std::path::Path) {
    let proposal = json!({"roots":[90,80],"nodes":[]});
    let correct = json!({"case":"invalid","value":{"code":"missing-root","owner":-1,"target":90}});
    let valid = public.input(
        "outcome-recovery.json",
        &json!([proposal, correct]).to_string(),
    );
    public.compare(
        "outcome-single-negative",
        descriptor,
        &valid,
        &cases::verified(),
    );
    let false_claim = public.input(
        "outcome-single-false.json",
        &json!([
            {"roots":[],"nodes":[]}, {"case":"capacity","value":"nodes"}
        ])
        .to_string(),
    );
    public.compare(
        "outcome-single-false",
        descriptor,
        &false_claim,
        &cases::rejected("source-outcome"),
    );
    let mut malformed = json!([proposal, correct]);
    malformed[1]["value"]["owner"] = json!(false);
    let mut untaken = cases::empty_plan();
    untaken["unreachable"] = json!([false]);
    let over_capacity = json!({"roots":[],"nodes":vec![json!({"id":1,"successors":[]});4097]});
    let wrong = [
        malformed,
        json!([over_capacity,{"case":"valid","value":untaken}]),
        json!([proposal,{"case":"inconsistent","value":"not-a-claim"}]),
    ];
    for (index, wrong) in wrong.iter().enumerate() {
        let wrong = public.input(&format!("outcome-wrong-{index}.json"), &wrong.to_string());
        let absent = public
            .root
            .join(format!("outcome-wrong-result-{index}.json"));
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
            &format!("outcome-after-wrong-{index}"),
            descriptor,
            &valid,
            &cases::verified(),
        );
    }
    let mut restricted: Value = serde_json::from_str(DESCRIPTORS[0].1).unwrap();
    restricted["execution"] =
        json!({"instruction_fuel":1,"maximum_call_depth":4096,"maximum_value_stack":1000000});
    let restricted = public.input("outcome-refusal.deployment.json", &restricted.to_string());
    let absent = public.root.join("outcome-refusal-result.json");
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
    assert_eq!(field(&failure, "diagnostic", "class"), "resource");
    assert_eq!(
        field(&failure, "diagnostic", "code"),
        "normalized_instruction_steps"
    );
    assert!(!absent.exists());
    public.compare(
        "outcome-after-refusal",
        descriptor,
        &valid,
        &cases::verified(),
    );
}
