use crate::model::{Capsule, Edge, Node};

use super::{declared_node, edge};

pub fn add(
    capsule: &Capsule,
    capsule_id: &str,
    manifest: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let semantic = match capsule.id.as_str() {
        "vm" => Some(("runtime:reference-vm", "runtime", "reference bytecode VM")),
        "native" => Some((
            "native-abi:linux-x86-64",
            "native-abi",
            "Linux x86-64 native ABI",
        )),
        _ => None,
    };
    if let Some((target, kind, label)) = semantic {
        declared_node(nodes, target, kind, label, manifest);
        edge(edges, capsule_id, target, "owns", manifest, "declared");
    }
}
