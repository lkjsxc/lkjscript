use crate::{ContractDescriptor, ContractDigest, ContractFact, ContractItem, ContractItemKind};

use super::super::{
    name, BYTECODE, NATIVE_LAYOUT, PACKAGE_LOCK, PREPARED_PROGRAM, RUNTIME_CALLS, RUNTIME_CONTROL,
    VERIFIED_SSA,
};

pub(crate) struct PreparedDependencies {
    pub package_lock: ContractDigest,
    pub verified_ssa: ContractDigest,
    pub bytecode: ContractDigest,
    pub runtime_calls: ContractDigest,
    pub native_layout: ContractDigest,
    pub runtime_control: ContractDigest,
}

pub(crate) fn prepared_program(dependencies: PreparedDependencies) -> ContractDescriptor {
    ContractDescriptor {
        name: name(PREPARED_PROGRAM),
        dependencies: vec![
            dependency(PACKAGE_LOCK, dependencies.package_lock),
            dependency(VERIFIED_SSA, dependencies.verified_ssa),
            dependency(BYTECODE, dependencies.bytecode),
            dependency(RUNTIME_CALLS, dependencies.runtime_calls),
            dependency(NATIVE_LAYOUT, dependencies.native_layout),
            dependency(RUNTIME_CONTROL, dependencies.runtime_control),
        ],
        items: vec![
            ContractItem::new("descriptor", ContractItemKind::Type)
                .semantic_order()
                .fact(fact("platform-revision", "sole nonzero platform revision"))
                .fact(fact("package-kind", "locked or explicit development"))
                .fact(fact("package-content", "exact package content identity"))
                .fact(fact("package-root", "exact package graph root"))
                .fact(fact("entry", "exact package entry identity"))
                .fact(fact(
                    "memory-interface",
                    "module and package memory-interface closure",
                ))
                .fact(fact("memory-plan", "independently verified MemoryPlanId"))
                .fact(fact(
                    "witness-closure",
                    "atomic group member and external dependency closure",
                ))
                .fact(fact("semantic-ssa", "canonical verified SSA identity"))
                .fact(fact(
                    "native-ssa",
                    "canonical native-lowerable SSA identity",
                ))
                .fact(fact("bytecode", "canonical validated bytecode identity"))
                .fact(fact("contracts", "exact constituent contract digests")),
            ContractItem::new("process-provenance", ContractItemKind::Type)
                .semantic_order()
                .fact(fact(
                    "bootstrap",
                    "expected prepared entry return semantic and root group/member",
                ))
                .fact(fact(
                    "outcome",
                    "application incarnation cell package prepared and return identities",
                ))
                .fact(fact(
                    "rehydration",
                    "fresh parent runtime canonical equivalence and zero teardown",
                )),
        ],
    }
}

fn dependency(name_value: &str, digest: ContractDigest) -> crate::ContractDependency {
    crate::ContractDependency {
        name: name(name_value),
        digest: digest.as_bytes(),
    }
}

fn fact(id: &str, value: &str) -> ContractFact {
    ContractFact::required(id, id, value)
}
