use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub fn verified() -> Value {
    json!({"case":"verified","value":null})
}
pub fn rejected(code: &str) -> Value {
    json!({"case":"rejected","value":code})
}
pub fn empty_plan() -> Value {
    json!({"components":[],"stages":[],"reachable":[],"unreachable":[]})
}
pub fn normal(proposals: &[Value]) -> Vec<(Value, Value)> {
    proposals
        .iter()
        .map(|proposal| {
            let expected = crate::plan::oracle::expected(proposal);
            let (plan, report) = if expected["case"] == "valid" {
                (expected["value"].clone(), verified())
            } else {
                (empty_plan(), expected)
            };
            (json!({"proposal":proposal,"plan":plan}), report)
        })
        .collect()
}

fn partitions(count: usize) -> Vec<Vec<Vec<usize>>> {
    fn visit(
        next: usize,
        count: usize,
        groups: &mut Vec<Vec<usize>>,
        output: &mut Vec<Vec<Vec<usize>>>,
    ) {
        if next == count {
            output.push(groups.clone());
            return;
        }
        for group in 0..groups.len() {
            groups[group].push(next);
            visit(next + 1, count, groups, output);
            groups[group].pop();
        }
        groups.push(vec![next]);
        visit(next + 1, count, groups, output);
        groups.pop();
    }
    let mut output = Vec::new();
    visit(0, count, &mut Vec::new(), &mut output);
    output
}
fn stage_choices(count: usize) -> Vec<Vec<usize>> {
    (0..count.pow(count as u32))
        .filter_map(|mut encoded| {
            let levels: Vec<_> = (0..count)
                .map(|_| {
                    let level = encoded % count;
                    encoded /= count;
                    level
                })
                .collect();
            let maximum = *levels.iter().max().unwrap();
            (0..=maximum)
                .all(|level| levels.contains(&level))
                .then_some(levels)
        })
        .collect()
}
// Synchronous relaxation is independent of both the producer's readiness queue
// and the checker's direct equality check against a supplied level assignment.
fn relaxed(dependencies: &[Vec<usize>]) -> Option<Vec<usize>> {
    let mut levels = vec![0; dependencies.len()];
    for _ in 0..=dependencies.len() {
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
            return Some(levels);
        }
        levels = next;
    }
    None
}

pub fn exhaustive() -> Vec<(Value, Value)> {
    let partitions = partitions(3);
    assert_eq!(partitions.len(), 5);
    assert_eq!(stage_choices(3).len(), 13);
    let mut output = Vec::new();
    for proposal in crate::components::cases::suite().into_iter().take(512) {
        let canonical = crate::plan::oracle::expected(&proposal)["value"].clone();
        let nodes = proposal["nodes"].as_array().unwrap();
        let ids: Vec<_> = nodes
            .iter()
            .map(|node| node["id"].as_i64().unwrap())
            .collect();
        let positions: BTreeMap<_, _> = ids
            .iter()
            .copied()
            .enumerate()
            .map(|(index, id)| (id, index))
            .collect();
        for groups in &partitions {
            let mut membership = vec![0; nodes.len()];
            for (rank, group) in groups.iter().enumerate() {
                for &position in group {
                    membership[position] = rank;
                }
            }
            let leaders: Vec<_> = groups.iter().map(|group| ids[group[0]]).collect();
            let mut dependencies = vec![Vec::new(); groups.len()];
            let mut seen = vec![BTreeSet::new(); groups.len()];
            for (position, node) in nodes.iter().enumerate() {
                let source = membership[position];
                for target in node["successors"].as_array().unwrap() {
                    let target = membership[positions[&target.as_i64().unwrap()]];
                    if source != target && seen[source].insert(target) {
                        dependencies[source].push(target);
                    }
                }
            }
            let rows: Vec<_> = groups.iter().enumerate().map(|(rank, group)| {
                let cyclic = group.len() > 1 || nodes[group[0]]["successors"].as_array().unwrap().contains(&json!(leaders[rank]));
                json!({"members":group.iter().map(|&position| ids[position]).collect::<Vec<_>>(),
                    "cyclic":cyclic,"dependencies":dependencies[rank].iter().map(|&target| leaders[target]).collect::<Vec<_>>()})
            }).collect();
            let required = relaxed(&dependencies);
            for levels in stage_choices(groups.len()) {
                let mut stages = vec![Vec::new(); levels.iter().max().unwrap() + 1];
                for (rank, &level) in levels.iter().enumerate() {
                    stages[level].push(leaders[rank]);
                }
                let plan = json!({"components":rows,"stages":stages,
                    "reachable":canonical["reachable"],"unreachable":canonical["unreachable"]});
                let report = if plan == canonical {
                    assert_eq!(required.as_ref(), Some(&levels));
                    verified()
                } else if required.as_ref() == Some(&levels) {
                    rejected("component-connectivity")
                } else {
                    rejected("component-stages")
                };
                assert_eq!(report["case"] == "verified", plan == canonical);
                output.push((json!({"proposal":proposal,"plan":plan}), report));
            }
        }
    }
    output
}

#[test]
fn exhaustive_candidate_family_has_one_valid_plan_per_graph() {
    let samples = exhaustive();
    assert_eq!(samples.len(), 11_776);
    assert_eq!(
        samples
            .iter()
            .filter(|(_, output)| output["case"] == "verified")
            .count(),
        512
    );
    assert_eq!(
        samples
            .iter()
            .filter(|(_, output)| output["case"] == "rejected")
            .count(),
        11_264
    );
    let mut unique = BTreeSet::new();
    for (input, _) in samples {
        assert!(unique.insert(input.to_string()));
    }
}
