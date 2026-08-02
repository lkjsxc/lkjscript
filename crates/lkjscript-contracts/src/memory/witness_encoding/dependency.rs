#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ExecutableMemoryWitnessRole {
    ListElement,
    ProductField {
        product: [u8; 32],
        field: [u8; 32],
        source_order: u16,
    },
    EnumVariantField {
        enumeration: [u8; 32],
        variant: [u8; 32],
        field: [u8; 32],
        variant_source_order: u16,
        field_source_order: u16,
    },
    TypeArgument {
        constructor: [u8; 32],
        index: u16,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutableMemoryWitnessTarget {
    ExternalWitness([u8; 32]),
    LocalSemantic([u8; 32]),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableMemoryWitnessDependency {
    pub role: ExecutableMemoryWitnessRole,
    pub target: ExecutableMemoryWitnessTarget,
}
