use crate::{ContractDescriptor, ContractDigest, ContractFact, ContractItem, ContractItemKind};

use super::super::{name, METRICS, NATIVE_LAYOUT, RUNTIME_CALLS, VERIFIED_SSA};

pub(crate) fn runtime_calls() -> ContractDescriptor {
    descriptor(RUNTIME_CALLS).item(
        ContractItem::new("slots", ContractItemKind::Operation)
            .semantic_order()
            .fact(fact(
                "identity-i64",
                "IdentityI64",
                "(state,i64)->i64 pure test boundary",
            ))
            .fact(fact(
                "poll",
                "Poll",
                "(state)->status safepoint collection-capable",
            ))
            .fact(fact(
                "enter-function",
                "EnterFunction",
                "(state,function)->status",
            ))
            .fact(fact(
                "collect-reference",
                "CollectReference",
                "(state,reference)->status",
            ))
            .fact(fact(
                "heap-dispatch",
                "HeapDispatch",
                "(state,operation,args)->status",
            ))
            .fact(fact(
                "reserve-frame",
                "ReserveFrame",
                "(state,slots)->status",
            ))
            .fact(fact(
                "register-frame",
                "RegisterFrame",
                "(state,frame)->status",
            ))
            .fact(fact(
                "publish-safepoint",
                "PublishSafepoint",
                "(state,map)->status",
            ))
            .fact(fact(
                "unregister-frame",
                "UnregisterFrame",
                "(state,frame)->status",
            )),
    )
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
                .fact(fact("roots", "roots", "exact stack-map and root layout"))
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
                "entries calls runtime calls frames roots",
            ))
            .fact(fact(
                "heap",
                "heap",
                "allocations collections roots and barriers",
            ))
            .fact(fact(
                "native-cache",
                "native cache",
                "hits misses corruption bytes timing publication status",
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
