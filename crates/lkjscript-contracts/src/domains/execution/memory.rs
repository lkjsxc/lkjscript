use crate::{ContractDescriptor, ContractDigest, ContractFact, ContractItem, ContractItemKind};

use super::super::{name, LANGUAGE, MEMORY_OBLIGATIONS};

pub(crate) fn memory_obligations(language: ContractDigest) -> ContractDescriptor {
    descriptor()
        .dependency(name(LANGUAGE), language.as_bytes())
        .item(
            ContractItem::new("record", ContractItemKind::Type)
                .fact(fact("schema", "lkjscript.memory-obligations"))
                .fact(fact("contract", "full ContractDigest"))
                .fact(fact("identity", "stable semantic or runtime identity"))
                .fact(fact(
                    "authority",
                    "source, HIR, SSA, runtime, host, artifact, workload",
                ))
                .fact(fact("semantic-type", "value or identity semantics"))
                .fact(fact(
                    "runtime-layout",
                    "stable Current layout identity or not-current",
                ))
                .fact(fact(
                    "mutability-alias-copy",
                    "mutability aliases and copyability",
                ))
                .fact(fact(
                    "ownership",
                    "Current ownership category and escape behavior",
                ))
                .fact(fact(
                    "lifetime",
                    "possible lifetime and strong-cycle participation",
                ))
                .fact(fact("weak", "possible weak links"))
                .fact(fact(
                    "destruction",
                    "destructor and external-resource containment",
                ))
                .fact(fact("portability", "worker portability and contention"))
                .fact(fact("allocation", "frequency and size class"))
                .fact(fact(
                    "deterministic-liveness",
                    "structural child and invocation-key requirements",
                ))
                .fact(fact("object-identity", "observable identity facts"))
                .fact(fact(
                    "placement",
                    "type capability separated from per-value borrow move fusion clone or sealed route",
                ))
                .fact(fact(
                    "reclamation",
                    "candidate deterministic reclamation plan",
                ))
                .fact(fact(
                    "producers",
                    "source, HIR, SSA, bytecode, native and host producers",
                ))
                .fact(fact("tests", "focused tests and workloads"))
                .fact(fact(
                    "status",
                    "Current, Accepted Target, PLACEHOLDER or mixed",
                )),
        )
        .item(
            ContractItem::new("storage-taxonomy", ContractItemKind::Rule)
                .semantic_order()
                .fact(fact("inline", "inline"))
                .fact(fact("static", "static"))
                .fact(fact("stack", "stack"))
                .fact(fact("caller-destination", "caller-destination"))
                .fact(fact("unique", "unique-heap"))
                .fact(fact("region", "region"))
                .fact(fact("sealed-region", "sealed-shared-region"))
                .fact(fact("shared-node", "shared-node"))
                .fact(fact("pool", "pool"))
                .fact(fact("external", "external")),
        )
}

fn descriptor() -> ContractDescriptor {
    ContractDescriptor {
        name: name(MEMORY_OBLIGATIONS),
        items: Vec::new(),
        dependencies: Vec::new(),
    }
}

fn fact(id: &str, value: &str) -> ContractFact {
    ContractFact::required(id, id, value)
}
