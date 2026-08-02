use crate::{ContractDescriptor, ContractFact, ContractItem, ContractItemKind};

use super::super::{name, RUNTIME_CONTROL};

pub(crate) fn runtime_control() -> ContractDescriptor {
    ContractDescriptor {
        name: name(RUNTIME_CONTROL),
        dependencies: Vec::new(),
        items: vec![
            ContractItem::new("control-store", ContractItemKind::Type)
                .fact(fact("schema", "schema", "lkjscript.control-store"))
                .fact(fact(
                    "platform-revision",
                    "platform revision",
                    "canonical nonzero u64",
                ))
                .fact(fact("contract", "contract digest", "full ContractDigest"))
                .fact(fact("sequence", "sequence", "monotonic u64"))
                .fact(fact("checksum", "checksum", "full SHA-256"))
                .fact(fact(
                    "bounds",
                    "bounds",
                    "closed key value and record maxima",
                )),
            ContractItem::new("application-registry", ContractItemKind::Type)
                .fact(fact(
                    "identity",
                    "registry identity",
                    "monotonic nonzero u64",
                ))
                .fact(fact(
                    "record",
                    "durable record",
                    "package entry grants quotas desired-state",
                ))
                .fact(fact(
                    "operations",
                    "control operations",
                    concat!(
                        "install list start stop restart remove invoke ",
                        "session-register session-heartbeat session-unregister session-list",
                    ),
                ))
                .fact(fact(
                    "database-tenant",
                    "database tenant",
                    "stable registry tenant with incarnation-bound provider",
                )),
            ContractItem::new("application-cell", ContractItemKind::Type)
                .fact(fact(
                    "class",
                    "execution cell class",
                    "trusted-in-process or isolated-process",
                ))
                .fact(fact(
                    "manifest",
                    "manifest",
                    "entry grants quotas and restart policy",
                ))
                .fact(fact(
                    "process-frame",
                    "process frame",
                    "bounded exact little-endian length",
                ))
                .fact(fact(
                    "identity",
                    "identity",
                    "coordinator application incarnation execution-cell",
                ))
                .fact(fact(
                    "outcome",
                    "outcome",
                    concat!(
                        "lossless closed ExecutionOutcome with bounded key-free semantic DAG; ",
                        "exact type/layout identity; backward local edges; final reachable root",
                    ),
                ))
                .fact(fact(
                    "resource-hierarchy",
                    "resource hierarchy",
                    "coordinator application invocation quotas and fair tickets",
                )),
            ContractItem::new("local-control", ContractItemKind::Type)
                .fact(fact("schema", "schema", "lkjscript.local-control"))
                .fact(fact(
                    "platform-revision",
                    "platform revision",
                    "canonical nonzero u64",
                ))
                .fact(fact("contract", "contract digest", "full ContractDigest"))
                .fact(fact("request", "request identity", "nonzero u64"))
                .fact(fact("idempotency", "idempotency identity", "full SHA-256"))
                .fact(fact("principal", "principal", "transport-derived identity"))
                .fact(fact("operation", "operation", "closed typed operation"))
                .fact(fact("frame", "frame", "bounded exact length prefix")),
        ],
    }
}

fn fact(id: &str, name_value: &str, value: &str) -> ContractFact {
    ContractFact::required(id, name_value, value)
}
