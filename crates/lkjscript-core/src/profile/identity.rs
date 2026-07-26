use lkjscript_contracts::ContractDigest;

use super::{ResourceCeilings, ResourceProfile, ResourceProfileName, RESOURCE_PROFILE_SCHEMA};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceProfileIdentity {
    pub schema: &'static str,
    pub contract: ContractDigest,
    pub name: ResourceProfileName,
    pub resource_categories: ContractDigest,
    pub implementation_maxima_sha256: [u8; 32],
    pub ceilings_sha256: [u8; 32],
    pub host_lowered_ceilings_sha256: Option<[u8; 32]>,
}

pub(super) fn identity(profile: ResourceProfile) -> ResourceProfileIdentity {
    let base = ResourceProfile::new(profile.name).ceilings;
    let actual = profile.ceilings.digest();
    ResourceProfileIdentity {
        schema: RESOURCE_PROFILE_SCHEMA,
        contract: lkjscript_contracts::RESOURCE_PROFILES_DIGEST,
        name: profile.name,
        resource_categories: lkjscript_contracts::RESOURCE_CATEGORIES_DIGEST,
        implementation_maxima_sha256: ResourceCeilings::implementation_maxima().digest(),
        ceilings_sha256: base.digest(),
        host_lowered_ceilings_sha256: (profile.ceilings != base).then_some(actual),
    }
}
