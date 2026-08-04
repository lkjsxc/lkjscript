use crate::{ContractDescriptor, ContractDigest, ContractFact, ContractItem, ContractItemKind};

use super::super::{
    name, AGENT_WORK_STATE, CAPSULE_MANIFEST, PUBLIC_FACTS, REPOSITORY_GRAPH, SEMANTIC_SOURCE,
};

pub(crate) fn repository_graph() -> ContractDescriptor {
    descriptor(REPOSITORY_GRAPH).item(
        ContractItem::new("graph", ContractItemKind::Type)
            .fact(fact("schema", "schema", "lkjscript.repository-graph"))
            .fact(fact("contract", "contract", "full ContractDigest"))
            .fact(fact("revision", "revision", "base Git revision"))
            .fact(fact(
                "input-identity",
                "input identity",
                "SHA-256 of canonical emitted graph closure and budget state",
            ))
            .fact(fact(
                "nodes",
                "nodes",
                "bounded files capsules commands public facts and semantic entities",
            ))
            .fact(fact(
                "edges",
                "edges",
                "bounded dependency authority evidence projection and impact edges",
            ))
            .fact(fact(
                "queries",
                "query output",
                "graph identity with exact serialized byte limit",
            )),
    )
}

pub(crate) fn capsule_manifest() -> ContractDescriptor {
    descriptor(CAPSULE_MANIFEST).item(
        ContractItem::new("manifest", ContractItemKind::Type)
            .fact(fact("schema", "schema", "lkjscript.capsule"))
            .fact(fact("contract", "contract", "full ContractDigest"))
            .fact(fact("identity", "id and root", "stable capsule identity"))
            .fact(fact("dependencies", "dependencies", "closed capsule IDs"))
            .fact(fact(
                "authority",
                "authority",
                "facades decisions tests verification",
            ))
            .fact(fact(
                "safety",
                "safety",
                "unsafe capability and provenance facts",
            )),
    )
}

pub fn public_facts() -> ContractDescriptor {
    descriptor(PUBLIC_FACTS).item(
        ContractItem::new("registry", ContractItemKind::Type)
            .fact(fact(
                "schema",
                "schema",
                "lkjscript.public-facts and strict shards",
            ))
            .fact(fact("contract", "contract", "full ContractDigest"))
            .fact(fact(
                "identity",
                "fact IDs",
                "stable lowercase unnumbered names",
            ))
            .fact(fact(
                "status",
                "status",
                "closed nine-status lifecycle vocabulary",
            ))
            .fact(fact(
                "boundary",
                "interface and exclusions",
                "canonical positive interface plus explicit negative scope",
            ))
            .fact(fact(
                "closure",
                "fact closure",
                "authority anchors evidence projections dependencies and digests",
            ))
            .fact(fact(
                "limits",
                "resource limits",
                "strict bounded decode validation graph and reports",
            )),
    )
}

pub(crate) fn agent_work_state(semantic: ContractDigest) -> ContractDescriptor {
    descriptor(AGENT_WORK_STATE)
        .dependency(name(SEMANTIC_SOURCE), semantic.as_bytes())
        .item(
            ContractItem::new("snapshot", ContractItemKind::Type)
                .fact(fact("schema", "schema", "lkjscript.agent-work-state"))
                .fact(fact("contract", "contract", "full ContractDigest"))
                .fact(fact(
                    "revision",
                    "state revision",
                    "monotonic content identity",
                ))
                .fact(fact(
                    "repository",
                    "repository",
                    "base and current Git revisions",
                ))
                .fact(fact(
                    "history",
                    "history",
                    "append-only actions commands and evidence",
                ))
                .fact(fact(
                    "semantic",
                    "semantic context",
                    "exact optional current identities",
                )),
        )
}

fn descriptor(name_value: &str) -> ContractDescriptor {
    ContractDescriptor {
        name: name(name_value),
        items: Vec::new(),
        dependencies: Vec::new(),
    }
}

fn fact(id: &str, name_value: &str, value: &str) -> ContractFact {
    ContractFact::required(id, name_value, value)
}
