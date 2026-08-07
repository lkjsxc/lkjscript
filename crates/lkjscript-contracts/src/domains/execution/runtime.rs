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
                    "exact opaque nonzero u64 unique owner and loan tokens",
                ))
                .fact(fact(
                    "unique-service",
                    "unique-service",
                    "policy-accounted invocation-owned UniqueStore with wide identity maps",
                ))
                .fact(fact(
                    "structural-values",
                    "structural-values",
                    "opaque u64 owner view destination tokens and canonical u64 structural indexes",
                ))
                .fact(fact(
                    "structural-service",
                    "structural-service",
                    concat!(
                        "verified direct sites and bounded invocation-owned ",
                        "StructuralValueRuntime with exact unique or sealed routes",
                    ),
                ))
                .fact(fact(
                    "memory-witness-abi",
                    "memory witness ABI",
                    concat!(
                        "ordered canonical u64 hidden validated group/member locators with synchronous ",
                        "native independent-owner and dispose",
                    ),
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
                "phase timing and observed source-file count",
            ))
            .fact(fact(
                "execution-path",
                "execution path",
                "baseline-native or vm-fallback with nullable decline reason",
            ))
            .fact(fact(
                "entry",
                "native entry",
                "native-entered commit-point fact",
            ))
            .fact(fact(
                "timings",
                "execution timings",
                "preflight lower install prepare native VM and total durations",
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
