use serde_json::{Value, json};

pub fn suite() -> Vec<Value> {
    let mut proposals = crate::components::cases::suite();
    // All loop-free directed graphs on four vertices; three-vertex coverage in
    // the original suite additionally includes every self-loop combination.
    let ids = [-100, i64::MAX, i64::MIN, -1];
    for mask in 0..4096_u32 {
        let mut edge = 0;
        let nodes: Vec<_> = (0..4)
            .map(|source| {
                let mut successors = Vec::new();
                for (target, &id) in ids.iter().enumerate() {
                    if source != target {
                        if mask & (1 << edge) != 0 {
                            successors.push(id);
                        }
                        edge += 1;
                    }
                }
                json!({"id":ids[source],"successors":successors})
            })
            .collect();
        proposals.push(json!({"roots":[ids[mask as usize % 4]],"nodes":nodes}));
    }
    // Several nontrivial cycles with a diamond between components. Duplicates
    // occur both within a member and across distinct members of one component.
    let diamond = json!({"roots":[90,90],"nodes":[
        {"id":90,"successors":[12,12,50]}, {"id":50,"successors":[51,4]},
        {"id":12,"successors":[13,4,4]}, {"id":13,"successors":[12,4]},
        {"id":51,"successors":[50,4,4]}, {"id":4,"successors":[5]},
        {"id":5,"successors":[4]}, {"id":-1,"successors":[-1]},
        {"id":i64::MIN,"successors":[]}]});
    proposals.push(diamond.clone());
    let mut reordered = diamond.clone();
    for node in reordered["nodes"].as_array_mut().unwrap() {
        node["successors"].as_array_mut().unwrap().reverse();
    }
    proposals.push(reordered);
    let mut duplicate = diamond.clone();
    for node in duplicate["nodes"].as_array_mut().unwrap() {
        let original = node["successors"].as_array().unwrap().clone();
        node["successors"].as_array_mut().unwrap().extend(original);
    }
    proposals.push(duplicate);
    let mut rootless = diamond.clone();
    rootless["roots"] = json!([]);
    proposals.push(rootless);
    let mut node_order = diamond;
    node_order["nodes"].as_array_mut().unwrap().reverse();
    proposals.push(node_order);
    // Readiness at a common user must use the longest dependency path, not the
    // most recently visited edge or the order in which ready work was queued.
    for reverse in [false, true] {
        let mut nodes: Vec<_> = (0..96)
            .map(|id| {
                let mut targets = if id < 95 { vec![id + 1] } else { vec![] };
                if id < 94 {
                    targets.push(95);
                }
                if reverse {
                    targets.reverse();
                }
                json!({"id":id,"successors":targets})
            })
            .collect();
        if reverse {
            nodes.reverse();
        }
        proposals.push(json!({"roots":[0],"nodes":nodes}));
    }
    proposals
}

#[test]
fn case_family_and_metamorphic_relations_are_exact() {
    let suite = suite();
    assert_eq!(suite.len(), 4635);
    let start = 532 + 4096;
    let original = super::oracle::expected(&suite[start]);
    let reordered = super::oracle::expected(&suite[start + 1]);
    let duplicated = super::oracle::expected(&suite[start + 2]);
    let rootless = super::oracle::expected(&suite[start + 3]);
    assert_eq!(original, duplicated);
    assert_eq!(original["value"]["stages"], reordered["value"]["stages"]);
    assert_eq!(
        original["value"]["components"],
        rootless["value"]["components"]
    );
    assert_eq!(original["value"]["stages"], rootless["value"]["stages"]);
}
