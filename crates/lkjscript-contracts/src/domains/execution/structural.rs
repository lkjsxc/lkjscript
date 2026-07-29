use crate::{ContractDescriptor, ContractDigest, ContractFact, ContractItem, ContractItemKind};

use super::super::{name, MEMORY_OBLIGATIONS, STRUCTURAL_OWNERSHIP_DOMAINS};

pub(crate) fn structural_ownership_domains(memory: ContractDigest) -> ContractDescriptor {
    descriptor()
        .dependency(name(MEMORY_OBLIGATIONS), memory.as_bytes())
        .item(
            ContractItem::new("domain-key", ContractItemKind::Type)
                .fact(fact("runtime", "exact structural runtime identity"))
                .fact(fact("class", "closed domain class"))
                .fact(fact("slot", "bounded private slot"))
                .fact(fact("generation", "nonzero nonwrapping generation")),
        )
        .item(
            ContractItem::new("root-key", ContractItemKind::Type)
                .fact(fact("domain", "exact domain key"))
                .fact(fact("class", "closed root class"))
                .fact(fact("location", "private slot and generation"))
                .fact(fact("layout", "exact runtime layout identity"))
                .fact(fact("semantic-type", "exact semantic type identity")),
        )
        .item(
            ContractItem::new("domain-classes", ContractItemKind::Rule)
                .semantic_order()
                .fact(fact("static", "static"))
                .fact(fact("unique", "unique"))
                .fact(fact("region-building", "region-building"))
                .fact(fact("region-owned", "region-owned"))
                .fact(fact("region-sealing", "region-sealing"))
                .fact(fact("region-sealed", "region-sealed"))
                .fact(fact("pool", "pool"))
                .fact(fact("external", "external")),
        )
        .item(
            ContractItem::new("ordinary-region", ContractItemKind::Rule)
                .fact(fact("owner", "one affine region owner"))
                .fact(fact("allocation", "bounded chunk and large-object paths"))
                .fact(fact("cycles", "internal non-owning cycles allowed"))
                .fact(fact("dependencies", "bounded acyclic ownership ledger"))
                .fact(fact("drops", "bounded reverse side-drop ledger"))
                .fact(fact(
                    "release",
                    "dependency work only; no payload traversal",
                )),
        )
        .item(
            ContractItem::new("sealed-region", ContractItemKind::Rule)
                .fact(fact("build", "unique private build"))
                .fact(fact("seal", "atomic verified immutable publication"))
                .fact(fact("owners", "checked non-atomic region-level owners"))
                .fact(fact("weak", "generation-checked non-owning root"))
                .fact(fact("dag", "deterministic strong dependency cycle witness"))
                .fact(fact("release", "ledger release without internal traversal")),
        )
        .item(
            ContractItem::new("typed-pool", ContractItemKind::Rule)
                .fact(fact("owner", "one affine pool owner"))
                .fact(fact("id", "pool slot generation layout and semantic type"))
                .fact(fact("cycles", "non-owning typed IDs"))
                .fact(fact("reuse", "generation advance or permanent retirement"))
                .fact(fact("iteration", "ascending live slot order"))
                .fact(fact("partition", "exact bounded slot range")),
        )
        .item(
            ContractItem::new("placement", ContractItemKind::Rule)
                .fact(fact("authority", "liveness remains in the domain runtime"))
                .fact(fact("home", "resource-plane placement fact"))
                .fact(fact("transfer", "fresh no-live-loan proof"))
                .fact(fact("remote-release", "bounded existing release queue")),
        )
}

fn descriptor() -> ContractDescriptor {
    ContractDescriptor {
        name: name(STRUCTURAL_OWNERSHIP_DOMAINS),
        items: Vec::new(),
        dependencies: Vec::new(),
    }
}

fn fact(id: &str, value: &str) -> ContractFact {
    ContractFact::required(id, id, value)
}
