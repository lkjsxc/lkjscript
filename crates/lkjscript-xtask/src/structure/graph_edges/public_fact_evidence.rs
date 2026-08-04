use crate::model::{Edge, Node};
use crate::public_facts::{Evidence, EvidenceClass, LocatedFact};
use crate::structure::graph::Budget;

use super::public_facts::{add_file_edge, add_node};

pub fn add(
    fact_id: &str,
    index: usize,
    evidence: &Evidence,
    located: &LocatedFact,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    budget: &mut Budget,
) {
    let id = format!("fact-evidence:{}:{index}", located.fact.id);
    let Ok(label) = serde_json::to_string(evidence) else {
        budget.truncated = true;
        return;
    };
    add_node(nodes, &id, "fact-evidence", &label, &evidence.path, budget);
    add_file_edge(fact_id, &id, "evidenced-by", located, edges, budget);
    add_file_edge(
        &id,
        &format!("file:{}", evidence.path),
        "records",
        located,
        edges,
        budget,
    );
    if matches!(evidence.class, EvidenceClass::ImplementationTest) {
        add_file_edge(
            fact_id,
            &format!("file:{}", evidence.path),
            "tests",
            located,
            edges,
            budget,
        );
    }
}
