use crate::{ContractDescriptor, ContractDigest, ContractFact};

use super::super::{name, RESOURCE_PROFILES, SEMANTIC_RESOURCE_PLANE, VERIFIED_SSA};

mod graph;
mod placement;

pub(crate) fn semantic_resource_plane(
    profiles: ContractDigest,
    ssa: ContractDigest,
) -> ContractDescriptor {
    descriptor()
        .dependency(name(RESOURCE_PROFILES), profiles.as_bytes())
        .dependency(name(VERIFIED_SSA), ssa.as_bytes())
        .item(graph::identities())
        .item(graph::authority())
        .item(placement::topology())
        .item(graph::accesses())
        .item(graph::tasks())
        .item(placement::plan())
        .item(placement::policies())
        .item(placement::runtime())
        .item(placement::memory_homes())
        .item(placement::metrics())
}

fn descriptor() -> ContractDescriptor {
    ContractDescriptor {
        name: name(SEMANTIC_RESOURCE_PLANE),
        items: Vec::new(),
        dependencies: Vec::new(),
    }
}

fn fact(id: &str, name_value: &str, value: &str) -> ContractFact {
    ContractFact::required(id, name_value, value)
}
