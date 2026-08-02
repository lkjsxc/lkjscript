use crate::{ContractDescriptor, ContractDigest, ContractFact, ContractItem, ContractItemKind};

use super::super::{name, METRICS, NATIVE_LAYOUT, RUNTIME_CALLS, VERIFIED_SSA};

mod slots;

pub(crate) fn runtime_calls() -> ContractDescriptor {
    descriptor(RUNTIME_CALLS).item(slots::runtime_slots())
}

pub(crate) fn native_layout(ssa: ContractDigest, runtime: ContractDigest) -> ContractDescriptor {
    descriptor(NATIVE_LAYOUT)
        .dependency(name(VERIFIED_SSA), ssa.as_bytes())
        .dependency(name(RUNTIME_CALLS), runtime.as_bytes())
        .item(
            ContractItem::new("image", ContractItemKind::Section)
                .fact(fact(
                    "target",
                    "target",
                    "Linux x86-64 exact target identity",
                ))
                .fact(fact(
                    "code",
                    "code",
                    "installed-image-independent machine bytes",
                ))
                .fact(fact(
                    "relocations",
                    "relocations",
                    "bounded runtime-call relocations",
                ))
                .fact(fact("frames", "frames", "exact native frame layout"))
                .fact(fact(
                    "execution-domain",
                    "execution-domain",
                    "closed combined structural-list island or invocation-region dispatch table",
                ))
                .fact(fact(
                    "unique-values",
                    "unique-values",
                    "exact static/bytes unique/bytes loan/bytes and byte-vector/view words",
                ))
                .fact(fact(
                    "unique-service",
                    "unique-service",
                    "bounded invocation-owned UniqueStore and generation-bearing loan table",
                ))
                .fact(fact(
                    "structural-values",
                    "structural-values",
                    "exact static-string owner view destination type layout projection words",
                ))
                .fact(fact(
                    "structural-service",
                    "structural-service",
                    "verified direct sites and bounded invocation-owned StructuralValueRuntime",
                ))
                .fact(fact(
                    "static-data",
                    "static-data",
                    "immutable accounted image bytes addressed only by verified tokens",
                ))
                .fact(fact(
                    "runtime-value-sites",
                    "runtime-value-sites",
                    "closed list descriptors with exact layout and semantic identities over verified frame homes",
                ))
                .fact(fact(
                    "integrity",
                    "integrity",
                    "complete content and contract digests",
                ))
                .fact(fact(
                    "memory",
                    "memory",
                    "write then execute W^X installation",
                )),
        )
}

pub(crate) fn metrics() -> ContractDescriptor {
    descriptor(METRICS).item(
        ContractItem::new("record", ContractItemKind::Type)
            .fact(fact("schema", "schema", "lkjscript.metrics"))
            .fact(fact("contract", "contract", "full ContractDigest"))
            .fact(fact(
                "compile",
                "compile",
                "phase timing and exact resource facts",
            ))
            .fact(fact(
                "engines",
                "engines",
                "VM baseline proof and fallback facts",
            ))
            .fact(fact(
                "native",
                "native",
                "entries calls runtime calls frames and cleanup obligations",
            ))
            .fact(fact(
                "runtime-values",
                "runtime-values",
                "list segments region products runtime calls and reserved bytes",
            ))
            .fact(fact(
                "segmented-lists",
                "segmented-lists",
                "segments entries physical allocations logical prepends reads and reserved-byte estimates",
            ))
            .fact(fact(
                "unique",
                "unique",
                "operations cleanup live owners live loans release backlog and forged failures",
            ))
            .fact(fact(
                "structural",
                "structural",
                "calls roots views destinations events release work empty completion and teardown failures",
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
