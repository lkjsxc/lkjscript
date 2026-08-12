use crate::{ContractDescriptor, ContractFact, ContractItem, ContractItemKind};

use super::super::{name, METRICS};

pub(crate) fn metrics() -> ContractDescriptor {
    descriptor().item(
        ContractItem::new("record", ContractItemKind::Type)
            .fact(fact("schema", "schema", "lkjscript.metrics"))
            .fact(fact("contract", "contract", "full ContractDigest"))
            .fact(fact(
                "compile",
                "compile",
                "phase timing including package verification and observed source-file count",
            ))
            .fact(fact(
                "execution-path",
                "execution path",
                "baseline-native or vm-fallback with one nullable typed stage code function and detail decline",
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
            ))
            .fact(fact(
                "native-artifact",
                "native artifact",
                "published installed object function code metadata mapping work and relocation counts or unavailable",
            ))
            .fact(fact(
                "native-runtime",
                "native runtime",
                "explicitly saturating entry call stack heap unique cleanup and structural observation counts or unavailable",
            )),
    )
}

fn descriptor() -> ContractDescriptor {
    ContractDescriptor {
        name: name(METRICS),
        items: Vec::new(),
        dependencies: Vec::new(),
    }
}

fn fact(id: &str, name: &str, value: &str) -> ContractFact {
    ContractFact::required(id, name, value)
}
