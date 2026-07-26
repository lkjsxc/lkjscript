use crate::{ContractDescriptor, ContractDigest, ContractFact, ContractItem, ContractItemKind};

use super::{name, AGENT_PROTOCOL, DIAGNOSTICS, LANGUAGE, SEMANTIC_SOURCE, SOURCE};

pub(super) fn language() -> ContractDescriptor {
    descriptor(LANGUAGE).item(
        ContractItem::new("semantic-forms", ContractItemKind::Rule)
            .fact(fact(
                "generic-enums",
                "generic enums",
                "nominal invariant algebraic data",
            ))
            .fact(fact(
                "match",
                "match",
                "closed exhaustive usefulness-checked patterns",
            ))
            .fact(fact("never", "Never", "uninhabited join-only control type"))
            .fact(fact(
                "control",
                "structured control",
                "return loop while break continue",
            ))
            .fact(fact(
                "numeric",
                "numeric conversions",
                "four explicit checked operations",
            ))
            .fact(fact("option", "Option", "generic enum Some(T)|None"))
            .fact(fact("result", "Result", "generic enum Ok(T)|Err(E)"))
            .fact(fact("errors", "typed errors", "closed nominal error enums"))
            .fact(fact(
                "effects",
                "effects",
                "compiler-derived and not authority",
            ))
            .fact(fact(
                "ownership",
                "ownership",
                "type-derived move borrow and drop facts",
            ))
            .fact(fact(
                "capabilities",
                "capabilities",
                "eight closed explicit unforgeable provider values",
            )),
    )
}

pub(super) fn source(language: ContractDigest) -> ContractDescriptor {
    descriptor(SOURCE)
        .dependency(name(LANGUAGE), language.as_bytes())
        .item(
            ContractItem::new("canonical-grammar", ContractItemKind::Rule)
                .fact(fact("suffix", "source suffix", ".lkjscript"))
                .fact(fact(
                    "projection",
                    "projection",
                    "marker-free line-oriented structural forms",
                ))
                .fact(fact("language-selection", "language selection", "absent"))
                .fact(fact(
                    "module",
                    "module identity",
                    "exact package-root-relative source path",
                ))
                .fact(fact(
                    "imports",
                    "imports",
                    "exact module paths and declaration name lists",
                ))
                .fact(fact(
                    "visibility",
                    "visibility",
                    "private by default explicit public declaration field",
                ))
                .fact(fact("unknown-fields", "unknown forms", "rejected"))
                .fact(fact(
                    "identity",
                    "source identity",
                    "contract logical-path exact-bytes package",
                )),
        )
}

pub(super) fn diagnostics(language: ContractDigest) -> ContractDescriptor {
    descriptor(DIAGNOSTICS)
        .dependency(name(LANGUAGE), language.as_bytes())
        .item(
            ContractItem::new("diagnostic-record", ContractItemKind::Type)
                .fact(fact("code", "code", "stable diagnostic identity"))
                .fact(fact("severity", "severity", "closed classification"))
                .fact(fact("message", "message", "presentation text"))
                .fact(fact("origin", "origin", "package module source span"))
                .fact(fact(
                    "contract",
                    "contract",
                    "optional exact ContractDigest",
                )),
        )
}

pub(super) fn semantic_source(
    source: ContractDigest,
    diagnostics: ContractDigest,
) -> ContractDescriptor {
    descriptor(SEMANTIC_SOURCE)
        .dependency(name(SOURCE), source.as_bytes())
        .dependency(name(DIAGNOSTICS), diagnostics.as_bytes())
        .item(
            ContractItem::new("envelope", ContractItemKind::Type)
                .fact(fact("schema", "schema", "lkjscript.semantic-source"))
                .fact(fact("contract", "contract", "full lowercase SHA-256"))
                .fact(fact("profile", "profile", "closed resource profile name"))
                .fact(fact("root", "root", "contained logical source root"))
                .fact(fact("operation", "operation", "closed operation request")),
        )
        .item(
            ContractItem::new("operations", ContractItemKind::Operation)
                .semantic_order()
                .fact(fact("snapshot", "snapshot", "read exact semantic closure"))
                .fact(fact(
                    "read-entity",
                    "read_entity",
                    "read stable semantic entity",
                ))
                .fact(fact(
                    "query-node",
                    "query_node",
                    "query exact derived facts",
                ))
                .fact(fact(
                    "hole-context",
                    "hole_context",
                    "read typed hole context",
                ))
                .fact(fact(
                    "legal-actions",
                    "legal_actions",
                    "bounded checker-valid actions",
                ))
                .fact(fact("diagnostics", "diagnostics", "structured diagnostics"))
                .fact(fact(
                    "transaction",
                    "apply_transaction",
                    "atomic checked publication",
                )),
        )
}

pub(super) fn agent_protocol(semantic: ContractDigest) -> ContractDescriptor {
    descriptor(AGENT_PROTOCOL)
        .dependency(name(SEMANTIC_SOURCE), semantic.as_bytes())
        .item(
            ContractItem::new("session", ContractItemKind::Operation)
                .fact(fact(
                    "handshake",
                    "handshake",
                    "current descriptors and full digests",
                ))
                .fact(fact("framing", "framing", "eight-byte big-endian length"))
                .fact(fact(
                    "request",
                    "request",
                    "exact current Semantic Source digest",
                ))
                .fact(fact(
                    "stale",
                    "stale session",
                    "fail closed without translation",
                ))
                .fact(fact("unknown", "unknown fields", "rejected")),
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
