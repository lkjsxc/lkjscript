//! Test-only oracle: pairwise reachability, not the product's finish-order algorithm.
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub fn expected(proposal: &Value) -> Value {
    let nodes = proposal["nodes"].as_array().unwrap();
    let roots = proposal["roots"].as_array().unwrap();
    for (dimension, count, limit) in [
        ("nodes", nodes.len(), 4096),
        ("roots", roots.len(), 4096),
        (
            "edges",
            nodes
                .iter()
                .map(|node| node["successors"].as_array().unwrap().len())
                .sum(),
            16384,
        ),
    ] {
        if count > limit {
            return json!({"case":"capacity", "value":dimension});
        }
    }
    let invalid = |code: &str, owner: i64, target: i64| json!({"case":"invalid", "value":{"code":code,"owner":owner,"target":target}});
    let ids: Vec<_> = nodes
        .iter()
        .map(|node| node["id"].as_i64().unwrap())
        .collect();
    let mut positions = BTreeMap::new();
    for (position, &id) in ids.iter().enumerate() {
        if positions.insert(id, position).is_some() {
            return invalid("duplicate-node", id, id);
        }
    }
    for root in roots {
        let id = root.as_i64().unwrap();
        if !positions.contains_key(&id) {
            return invalid("missing-root", -1, id);
        }
    }
    let mut edges = Vec::new();
    for node in nodes {
        let id = node["id"].as_i64().unwrap();
        let mut successors = Vec::new();
        for target in node["successors"].as_array().unwrap() {
            let target = target.as_i64().unwrap();
            let Some(&position) = positions.get(&target) else {
                return invalid("missing-successor", id, target);
            };
            successors.push(position);
        }
        edges.push(successors);
    }
    let reachable: Vec<_> = (0..nodes.len())
        .map(|start| {
            let mut seen = BTreeSet::new();
            let mut pending = vec![start];
            while let Some(position) = pending.pop() {
                if seen.insert(position) {
                    pending.extend(edges[position].iter().copied());
                }
            }
            seen
        })
        .collect();
    let mut assigned = BTreeSet::new();
    let mut components = Vec::new();
    for position in 0..nodes.len() {
        if assigned.contains(&position) {
            continue;
        }
        let members: Vec<_> = (0..nodes.len())
            .filter(|&other| {
                reachable[position].contains(&other) && reachable[other].contains(&position)
            })
            .collect();
        assigned.extend(members.iter().copied());
        let cyclic = members.len() > 1 || edges[position].contains(&position);
        components.push(json!({"members":members.iter().map(|&index| ids[index]).collect::<Vec<_>>(), "cyclic":cyclic}));
    }
    let (mut reached, mut unreached) = (Vec::new(), Vec::new());
    for (position, &id) in ids.iter().enumerate() {
        if roots
            .iter()
            .any(|root| reachable[positions[&root.as_i64().unwrap()]].contains(&position))
        {
            reached.push(id);
        } else {
            unreached.push(id);
        }
    }
    json!({"case":"valid", "value":{"components":components,"reachable":reached,"unreachable":unreached}})
}
