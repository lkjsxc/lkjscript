use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedWitnessGroupMember {
    pub member: String,
    pub ordinal: u16,
    pub semantic_identity: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedWitnessGroup {
    pub group: String,
    pub recursive: bool,
    pub members: Vec<LockedWitnessGroupMember>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LockedWitnessDependencyRole {
    ListElement,
    ProductField {
        product: String,
        field: String,
        source_order: u64,
    },
    EnumVariantField {
        enumeration: String,
        variant: String,
        field: String,
        variant_source_order: u64,
        field_source_order: u64,
    },
    TypeArgument {
        constructor: String,
        index: u16,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedWitnessDependency {
    pub source_member: String,
    pub role: LockedWitnessDependencyRole,
    pub target_group: String,
    pub target_member: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedTargetMemory {
    pub name: String,
    pub module: String,
    pub memory_plan_id: String,
    pub witness_groups: Vec<LockedWitnessGroup>,
    pub external_witness_dependencies: Vec<LockedWitnessDependency>,
    pub specialization_identity_support: String,
}
