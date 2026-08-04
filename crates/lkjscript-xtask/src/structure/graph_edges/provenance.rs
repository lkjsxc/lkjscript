use crate::model::{Audit, Edge, Node};

use super::{edge, node};
use crate::structure::graph::Budget;

pub fn add(audit: &Audit, nodes: &mut Vec<Node>, edges: &mut Vec<Edge>, budget: &mut Budget) {
    for item in &audit.provenance {
        let Some(generator) = &item.generator else {
            continue;
        };
        if !budget.charge(1, 0) {
            return;
        }
        let origin = format!(
            "generated-origin:{}",
            &crate::sha256::digest(generator.as_bytes())[..16]
        );
        node(
            nodes,
            &origin,
            "generated-origin",
            generator,
            "generated",
            "meta/structure/provenance.json",
            None,
            "declared",
        );
        edge(
            edges,
            &origin,
            &format!("file:{}", item.path),
            "generates",
            "meta/structure/provenance.json",
            "declared",
        );
        edge(
            edges,
            &format!("file:{}", item.path),
            &origin,
            "derived-from",
            "meta/structure/provenance.json",
            "declared",
        );
    }
}
