#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MemoryWitnessId([u8; 32]);

impl MemoryWitnessId {
    pub(crate) const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn to_hex(self) -> String {
        lkjscript_contracts::ContractDigest::from_bytes(self.0).to_hex()
    }
}

impl fmt::Display for MemoryWitnessId {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(&self.to_hex())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWitnessRequirement {
    Concrete,
    SpecializationRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryEqualitySupport {
    Unsupported,
    EqualValue,
    EqualList,
    CallerWitnessRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryProcessCodecEligibility {
    Eligible,
    Ineligible,
    CallerWitnessRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryListElementEligibility {
    Copy,
    ImmutableValue,
    UnsupportedAffine,
    UnsupportedBorrow,
    UnsupportedUnresolved,
    CallerWitnessRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryDynamicSize {
    Fixed,
    Dynamic,
    CallerWitnessRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryListStorageKind {
    SegmentedSessionRegion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryListWitness {
    pub element: MemoryWitnessId,
    pub selected: bool,
    pub eligibility: MemoryListElementEligibility,
    pub storage: MemoryListStorageKind,
    pub segment_capacity: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryWitnessDropAction {
    pub path: Vec<MemoryDropPathElement>,
    pub glue: MemoryDropGlueKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryWitnessDropBranch {
    pub active_variant: Option<[u8; 32]>,
    pub actions: Vec<MemoryWitnessDropAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryWitnessFacts {
    pub ty: MemoryType,
    pub semantic_contract: [u8; 32],
    pub semantic: lkjscript_contracts::SemanticDescriptor,
    pub dependencies: Vec<lkjscript_contracts::ExecutableMemoryWitnessDependency>,
    pub requirement: MemoryWitnessRequirement,
    pub mode: MemoryAggregateMode,
    pub capabilities: lkjscript_contracts::MemoryWitnessCapabilities,
    pub closure: MemoryClosureFact,
    pub root_projection: MemoryRootProjection,
    pub domain: MemoryDomain,
    pub copy_share: MemoryCopySharePlan,
    pub drop_glue: Option<MemoryDropGlueKind>,
    pub drop_path: Option<Vec<MemoryWitnessDropBranch>>,
    pub equality: MemoryEqualitySupport,
    pub process_codec: MemoryProcessCodecEligibility,
    pub list_element: MemoryListElementEligibility,
    pub list: Option<MemoryListWitness>,
    pub dynamic_size: MemoryDynamicSize,
    pub contains_borrow: bool,
    pub contains_dynamic_owner: bool,
    pub portability: MemoryPortability,
    pub contention: MemoryContention,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryWitness {
    pub id: MemoryWitnessId,
    pub facts: MemoryWitnessFacts,
}

impl MemoryWitness {
    pub fn recompute_id(&self) -> lkjscript_core::Result<MemoryWitnessId> {
        memory_witness_id(&self.facts)
    }
}

pub(crate) fn memory_witness_id(
    facts: &MemoryWitnessFacts,
) -> lkjscript_core::Result<MemoryWitnessId> {
    let executable = super::executable_facts(facts)?;
    let bytes = lkjscript_contracts::canonical_executable_memory_witness(
        &executable,
        &facts.dependencies,
    );
    Ok(MemoryWitnessId::from_bytes(lkjscript_core::sha256(&bytes)))
}
