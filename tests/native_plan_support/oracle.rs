//! Independent scheduler oracle: synchronous longest-path relaxation, not readiness counts.
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub fn expected(proposal: &Value) -> Value {
    // This existing oracle uses pairwise reachability, not the native two-pass SCC traversal.
    let mut output = crate::components::oracle::expected(proposal);
    if output["case"] != "valid" {
        return output;
    }
    let components = output["value"]["components"].as_array_mut().unwrap();
    let count = components.len();
    let mut membership = BTreeMap::new();
    let mut representatives = Vec::new();
    for (rank, component) in components.iter().enumerate() {
        let members = component["members"].as_array().unwrap();
        representatives.push(members[0].as_i64().unwrap());
        for id in members {
            assert!(membership.insert(id.as_i64().unwrap(), rank).is_none());
        }
    }
    let mut dependencies = vec![Vec::new(); count];
    let mut seen = vec![BTreeSet::new(); count];
    for node in proposal["nodes"].as_array().unwrap() {
        let source = membership[&node["id"].as_i64().unwrap()];
        for target in node["successors"].as_array().unwrap() {
            let target = membership[&target.as_i64().unwrap()];
            if source != target && seen[source].insert(target) {
                dependencies[source].push(target);
            }
        }
    }
    let mut levels = vec![0_usize; count];
    // Every iteration reads only the preceding round. No queue, reverse edges or
    // remaining-dependency counts are shared with the product's implementation.
    for round in 0..=count {
        let next: Vec<_> = dependencies
            .iter()
            .map(|targets| {
                targets
                    .iter()
                    .map(|&target| levels[target] + 1)
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        if next == levels {
            break;
        }
        assert!(round < count, "oracle condensation unexpectedly cyclic");
        levels = next;
    }
    let mut stages: Vec<Vec<i64>> = if count == 0 {
        Vec::new()
    } else {
        vec![Vec::new(); levels.iter().copied().max().unwrap() + 1]
    };
    for (rank, component) in components.iter_mut().enumerate() {
        component["dependencies"] = json!(
            dependencies[rank]
                .iter()
                .map(|&target| representatives[target])
                .collect::<Vec<_>>()
        );
        stages[levels[rank]].push(representatives[rank]);
        for &target in &dependencies[rank] {
            assert!(levels[target] < levels[rank]);
        }
    }
    assert!(stages.iter().all(|stage| !stage.is_empty()));
    assert_eq!(stages.iter().map(Vec::len).sum::<usize>(), count);
    output["value"]["stages"] = json!(stages);
    output
}

#[test]
fn independent_stage_oracle_fixes_direction_and_authored_order() {
    let example = expected(&crate::components::cases::example());
    assert_eq!(
        example["value"]["stages"],
        json!([[20, 50, 60], [30], [10]])
    );
    assert_eq!(
        example["value"]["components"][0]["dependencies"],
        json!([20, 30])
    );
    let proposal = json!({"roots":[101],"nodes":[
        {"id":101,"successors":[8,7,8]}, {"id":7,"successors":[]}, {"id":8,"successors":[]}]});
    let plan = expected(&proposal);
    assert_eq!(
        plan["value"]["components"][0]["dependencies"],
        json!([8, 7])
    );
    assert_eq!(plan["value"]["stages"], json!([[7, 8], [101]]));
    assert_eq!(
        expected(&json!({"roots":[],"nodes":[]}))["value"]["stages"],
        json!([])
    );
}
