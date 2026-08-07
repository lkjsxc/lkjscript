use crate::{ContractDescriptor, ContractDigest, ContractFact, ContractItem, ContractItemKind};

use super::super::{name, LANGUAGE, MODULE_INTERFACE, PACKAGE_LOCK, PACKAGE_MANIFEST};

pub(crate) fn package_manifest() -> ContractDescriptor {
    descriptor(PACKAGE_MANIFEST).item(
        ContractItem::new("manifest", ContractItemKind::Type)
            .fact(fact("schema", "schema", "lkjscript.package"))
            .fact(fact("contract", "contract", "full ContractDigest"))
            .fact(fact("name", "name", "human package name"))
            .fact(fact(
                "source-root",
                "source root",
                "contained canonical path",
            ))
            .fact(fact("modules", "module mapping", "exact canonical mapping"))
            .fact(fact("public", "public roots", "explicit root modules"))
            .fact(fact(
                "dependencies",
                "dependencies",
                "local path and expected content hash",
            ))
            .fact(fact(
                "capabilities",
                "capabilities",
                "sorted subset of eight exact provider authorities",
            ))
            .fact(fact("targets", "targets", "closed build target set")),
    )
}

pub(crate) fn module_interface(language: ContractDigest) -> ContractDescriptor {
    descriptor(MODULE_INTERFACE)
        .dependency(name(LANGUAGE), language.as_bytes())
        .item(
            ContractItem::new("interface", ContractItemKind::Type)
                .fact(fact(
                    "identity",
                    "identity",
                    "package hash and canonical module path",
                ))
                .fact(fact(
                    "exports",
                    "exports",
                    "stable public names and semantic types",
                ))
                .fact(fact("effects", "effects", "derived effects"))
                .fact(fact(
                    "capabilities",
                    "capabilities",
                    "explicit provider parameter types and delegation",
                ))
                .fact(fact(
                    "ownership",
                    "ownership",
                    "public parameter modes result mode equality and semantic-snapshot constraints",
                ))
                .fact(fact(
                    "memory-interface",
                    "memory interface",
                    "HIR-derived declaration identity ordered type and trait parameters and minimal hidden operations",
                ))
                .fact(fact("errors", "errors", "closed public error identities")),
        )
}

pub(crate) fn package_lock(manifest: ContractDigest, module: ContractDigest) -> ContractDescriptor {
    descriptor(PACKAGE_LOCK)
        .dependency(name(PACKAGE_MANIFEST), manifest.as_bytes())
        .dependency(name(MODULE_INTERFACE), module.as_bytes())
        .item(
            ContractItem::new("lock-graph", ContractItemKind::Type)
                .fact(fact("root", "root package", "full package content hash"))
                .fact(fact("nodes", "packages", "sorted exact package hashes"))
                .fact(fact(
                    "edges",
                    "edges",
                    "acyclic exact dependency identities",
                ))
                .fact(fact("origins", "origins", "contained local source paths"))
                .fact(fact(
                    "contracts",
                    "contracts",
                    "full current contract digests",
                ))
                .fact(fact(
                    "module-memory",
                    "module memory",
                    "exact HIR-derived public memory-interface digests and source closure",
                ))
                .fact(fact(
                    "target-memory",
                    "target memory",
                    concat!(
                        "MemoryPlanId atomic witness groups members role-bearing external ",
                        "closure and specialization support",
                    ),
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
