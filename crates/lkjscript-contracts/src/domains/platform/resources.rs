use crate::{ContractDescriptor, ContractDigest, ContractFact, ContractItem, ContractItemKind};

use super::super::{name, RESOURCE_CATEGORIES, RESOURCE_PROFILES};

pub(crate) fn resource_categories() -> ContractDescriptor {
    let mut categories =
        ContractItem::new("ordered-categories", ContractItemKind::Resource).semantic_order();
    for (index, category) in RESOURCE_CATEGORY_NAMES.into_iter().enumerate() {
        categories.facts.push(ContractFact::required(
            format!("category-{index:02}"),
            category,
            "checked monotonic count",
        ));
    }
    descriptor(RESOURCE_CATEGORIES).item(categories)
}

pub(crate) fn resource_profiles(categories: ContractDigest) -> ContractDescriptor {
    descriptor(RESOURCE_PROFILES)
        .dependency(name(RESOURCE_CATEGORIES), categories.as_bytes())
        .item(
            ContractItem::new("profile-identity", ContractItemKind::Type)
                .fact(fact("name", "name", "stable profile name"))
                .fact(fact(
                    "categories",
                    "categories",
                    "resource-category contract digest",
                ))
                .fact(fact("maxima", "maxima", "implementation-maxima digest"))
                .fact(fact("ceilings", "ceilings", "selected ceiling-set digest"))
                .fact(fact("host", "host ceilings", "optional lower-only digest")),
        )
        .item(
            ContractItem::new("profiles", ContractItemKind::Variant)
                .fact(fact("build", "build", "closed ceiling set"))
                .fact(fact("default", "default", "closed ceiling set"))
                .fact(fact("deterministic", "deterministic", "closed ceiling set"))
                .fact(fact("sandbox", "sandbox", "closed ceiling set"))
                .fact(fact("trusted-local", "trusted-local", "closed ceiling set")),
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

const RESOURCE_CATEGORY_NAMES: [&str; 72] = [
    "source-bytes",
    "source-units",
    "import-edges",
    "tokens",
    "schema-nodes",
    "top-level-declarations",
    "product-fields",
    "parser-work",
    "validation-work",
    "path-work",
    "type-nesting",
    "type-work",
    "trait-work",
    "ownership-expressions",
    "ownership-retained-state",
    "hir-functions",
    "hir-expressions",
    "ssa-functions",
    "ssa-blocks",
    "ssa-values",
    "ssa-edges",
    "ssa-frame-states",
    "diagnostics",
    "protocol-request-bytes",
    "protocol-response-bytes",
    "enum-declarations",
    "enum-variants",
    "variant-fields",
    "enum-recursion-work",
    "patterns",
    "match-arms",
    "usefulness-rows",
    "usefulness-columns",
    "usefulness-specialization-work",
    "match-plans",
    "exhaustiveness-witness-bytes",
    "hole-count",
    "hole-candidates",
    "hole-search-work",
    "legal-actions",
    "semantic-session-lifetime-fuel",
    "semantic-session-input-bytes",
    "semantic-session-output-bytes",
    "semantic-session-nodes",
    "semantic-session-snapshots",
    "semantic-session-retained-bytes",
    "semantic-session-cache-entries",
    "semantic-session-cached-revisions",
    "transactions",
    "transaction-operations",
    "transaction-impact-nodes",
    "staged-publication-bytes",
    "staged-publication-nodes",
    "logical-aggregate-constructions",
    "worker-threads",
    "worker-groups",
    "task-classes",
    "task-instances",
    "task-dependencies",
    "task-access-records",
    "task-scope-children",
    "ready-queue-entries",
    "task-descriptor-bytes",
    "task-result-bytes",
    "worker-scratch-bytes",
    "scheduler-work",
    "scheduler-decisions",
    "scheduler-steals",
    "scheduler-migrations",
    "scheduler-wakeups",
    "remote-release-records",
    "decision-trace-records",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_resource_descriptor_names_and_order_are_exact() {
        let expected = [
            "worker-threads",
            "worker-groups",
            "task-classes",
            "task-instances",
            "task-dependencies",
            "task-access-records",
            "task-scope-children",
            "ready-queue-entries",
            "task-descriptor-bytes",
            "task-result-bytes",
            "worker-scratch-bytes",
            "scheduler-work",
            "scheduler-decisions",
            "scheduler-steals",
            "scheduler-migrations",
            "scheduler-wakeups",
            "remote-release-records",
            "decision-trace-records",
        ];
        assert_eq!(&RESOURCE_CATEGORY_NAMES[54..], expected);
        let descriptor = resource_categories();
        let facts = &descriptor.items[0].facts[54..];
        assert_eq!(facts.len(), expected.len());
        for (index, (fact, name)) in facts.iter().zip(expected).enumerate() {
            assert_eq!(fact.stable_id, format!("category-{:02}", index + 54));
            assert_eq!(fact.name, name);
            assert_eq!(fact.value, "checked monotonic count");
        }
    }
}
