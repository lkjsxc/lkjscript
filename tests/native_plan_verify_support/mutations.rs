use super::cases::{rejected, verified};
use serde_json::{Value, json};

pub type Mutation = (String, Value, Value);
pub fn suite() -> Vec<Mutation> {
    let proposal = crate::components::cases::example();
    let original = crate::plan::oracle::expected(&proposal)["value"].clone();
    let mut output = Vec::new();
    let mut add = |name: &str, code: &str, edit: fn(&mut Value)| {
        let mut plan = original.clone();
        edit(&mut plan);
        assert_ne!(plan, original, "mutation must change the claim: {name}");
        output.push((
            name.to_owned(),
            json!({"proposal":proposal,"plan":plan}),
            rejected(code),
        ));
    };
    add("missing-component", "component-members", |p| {
        p["components"].as_array_mut().unwrap().pop();
    });
    add("empty-component", "component-members", |p| {
        p["components"][0]["members"] = json!([]);
    });
    add("missing-member", "component-members", |p| {
        p["components"][1]["members"] = json!([20]);
    });
    add("duplicate-member", "component-members", |p| {
        p["components"][1]["members"] = json!([20, 20, 40]);
    });
    add("cross-component-duplicate", "component-members", |p| {
        p["components"][1]["members"] = json!([20, 40, 60]);
    });
    add("unknown-signed-member", "component-members", |p| {
        p["components"][0]["members"] = json!([i64::MIN]);
    });
    add("negative-one-is-not-a-sentinel", "component-members", |p| {
        p["components"][0]["members"] = json!([-1]);
    });
    add("member-order", "component-members", |p| {
        p["components"][1]["members"] = json!([40, 20]);
    });
    add("component-order", "component-members", |p| {
        p["components"].as_array_mut().unwrap().swap(0, 1);
    });
    add("extra-component", "component-members", |p| {
        let row = p["components"][0].clone();
        p["components"].as_array_mut().unwrap().push(row);
    });
    add("false-singleton-cycle", "component-cyclic", |p| {
        p["components"][0]["cyclic"] = json!(true);
    });
    add("hidden-self-loop", "component-cyclic", |p| {
        p["components"][3]["cyclic"] = json!(false);
    });
    add("hidden-multi-node-cycle", "component-cyclic", |p| {
        p["components"][1]["cyclic"] = json!(false);
    });
    add("missing-dependency", "component-dependencies", |p| {
        p["components"][0]["dependencies"] = json!([20]);
    });
    add("duplicate-dependency", "component-dependencies", |p| {
        p["components"][0]["dependencies"] = json!([20, 30, 20]);
    });
    add("dependency-order", "component-dependencies", |p| {
        p["components"][0]["dependencies"] = json!([30, 20]);
    });
    add("internal-component-edge", "component-dependencies", |p| {
        p["components"][0]["dependencies"] = json!([10, 20, 30]);
    });
    add("unknown-dependency", "component-dependencies", |p| {
        p["components"][0]["dependencies"] = json!([777, 30]);
    });
    add(
        "nonrepresentative-dependency",
        "component-dependencies",
        |p| {
            p["components"][0]["dependencies"] = json!([40, 30]);
        },
    );
    add("extra-dependency", "component-dependencies", |p| {
        p["components"][4]["dependencies"] = json!([20]);
    });
    add("reachable-order", "reachability", |p| {
        p["reachable"] = json!([20, 10, 30, 40]);
    });
    add("reachable-duplicate", "reachability", |p| {
        p["reachable"] = json!([10, 20, 30, 40, 40]);
    });
    add("reachable-missing", "reachability", |p| {
        p["reachable"] = json!([10, 20, 30]);
    });
    add("unreachable-moved", "reachability", |p| {
        p["reachable"] = json!([10, 20, 30, 40, 50]);
        p["unreachable"] = json!([60]);
    });
    add("unreachable-order", "reachability", |p| {
        p["unreachable"] = json!([60, 50]);
    });
    add("missing-stage-member", "component-stages", |p| {
        p["stages"][0] = json!([50, 60]);
    });
    add("duplicate-stage-member", "component-stages", |p| {
        p["stages"][1] = json!([20, 30]);
    });
    add("nonrepresentative-stage-member", "component-stages", |p| {
        p["stages"][0] = json!([40, 50, 60]);
    });
    add("stage-peer-order", "component-stages", |p| {
        p["stages"][0] = json!([60, 50, 20]);
    });
    add("not-earliest", "component-stages", |p| {
        p["stages"] = json!([[20, 50], [30, 60], [10]]);
    });
    add("reversed-stages", "component-stages", |p| {
        p["stages"].as_array_mut().unwrap().reverse();
    });
    add("empty-stage", "component-stages", |p| {
        p["stages"].as_array_mut().unwrap().insert(0, json!([]));
    });
    add("extra-empty-stage", "component-stages", |p| {
        p["stages"].as_array_mut().unwrap().push(json!([]));
    });
    add("flattened-stages", "component-stages", |p| {
        p["stages"] = json!([[10, 20, 30, 50, 60]]);
    });
    add(
        "merge-reachable-and-unreachable",
        "component-connectivity",
        |p| {
            p["components"][1]["members"] = json!([20, 40, 50]);
            p["components"].as_array_mut().unwrap().remove(3);
            p["stages"] = json!([[20, 60], [30], [10]]);
        },
    );
    for (name, forward, reverse) in [
        ("one-way-forward", true, false),
        ("one-way-reverse", false, true),
        ("disconnected", false, false),
    ] {
        let proposal = json!({"roots":[],"nodes":[
            {"id":0,"successors":if forward {vec![1]} else {vec![]}},
            {"id":1,"successors":if reverse {vec![0]} else {vec![]}}]});
        let plan = json!({"components":[{"members":[0,1],"cyclic":true,"dependencies":[]}],
            "stages":[[0]],"reachable":[],"unreachable":[0,1]});
        output.push((
            name.to_owned(),
            json!({"proposal":proposal,"plan":plan}),
            rejected("component-connectivity"),
        ));
    }
    // Input mutations are bound to this exact proposal, not the origin of a plan.
    let mut wrong_source = proposal.clone();
    wrong_source["roots"] = json!([]);
    output.push((
        "replay-under-other-roots".to_owned(),
        json!({"proposal":wrong_source,"plan":original}),
        rejected("reachability"),
    ));
    let mut invalid = proposal.clone();
    invalid["nodes"][5]["successors"] = json!([999]);
    output.push((
        "invalid-unreachable-source-before-claim".to_owned(),
        json!({"proposal":invalid,"plan":super::cases::empty_plan()}),
        json!({"case":"invalid","value":{"code":"missing-successor","owner":60,"target":999}}),
    ));
    output.push((
        "valid-recovery".to_owned(),
        json!({"proposal":proposal,"plan":original}),
        verified(),
    ));
    output
}

#[test]
fn mutation_oracle_rejects_every_corrupted_claim() {
    let cases = suite();
    assert_eq!(cases.len(), 41);
    for (name, sample, expected) in cases {
        let source = crate::plan::oracle::expected(&sample["proposal"]);
        if source["case"] == "valid" {
            assert_eq!(
                sample["plan"] == source["value"],
                expected["case"] == "verified",
                "{name}"
            );
        } else {
            assert_eq!(source, expected, "{name}");
        }
    }
}
