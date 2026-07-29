use std::fmt;

use super::{DomainClass, DomainKey, RootKey};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralLimit {
    Domains,
    Objects,
    Bytes,
    Chunks,
    Dependencies,
    DropEntries,
    ReleaseWork,
    RegionOwners,
    PoolSlots,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructuralError {
    ArithmeticOverflow,
    AllocationFailed,
    LimitExceeded(StructuralLimit),
    WrongRuntime,
    WrongDomainClass {
        expected: DomainClass,
        actual: DomainClass,
    },
    StaleDomain(DomainKey),
    StaleRoot(RootKey),
    WrongLayout,
    WrongSemanticType,
    DuplicateDependency,
    DependencyCycle(Vec<DomainKey>),
    LiveLoan,
    LoanUnderflow,
    OwnerOverflow,
    GenerationExhausted,
    RuntimeIdentityExhausted,
    IncompleteSeal,
    UnsupportedDependency,
    WrongPool,
    WrongPartition,
    SlotVacant,
}

impl fmt::Display for StructuralError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArithmeticOverflow => formatter.write_str("structural arithmetic overflow"),
            Self::AllocationFailed => formatter.write_str("structural allocation failed"),
            Self::LimitExceeded(limit) => write!(formatter, "structural limit exceeded: {limit:?}"),
            Self::WrongRuntime => formatter.write_str("wrong structural runtime"),
            Self::WrongDomainClass { expected, actual } => {
                write!(
                    formatter,
                    "wrong domain class: expected {expected:?}, got {actual:?}"
                )
            }
            Self::StaleDomain(key) => write!(formatter, "stale domain key: {key:?}"),
            Self::StaleRoot(key) => write!(formatter, "stale structural root: {key:?}"),
            Self::WrongLayout => formatter.write_str("wrong structural layout"),
            Self::WrongSemanticType => formatter.write_str("wrong structural semantic type"),
            Self::DuplicateDependency => formatter.write_str("duplicate domain dependency"),
            Self::DependencyCycle(witness) => {
                write!(formatter, "domain dependency cycle: {witness:?}")
            }
            Self::LiveLoan => formatter.write_str("structural domain has a live loan"),
            Self::LoanUnderflow => formatter.write_str("structural loan underflow"),
            Self::OwnerOverflow => formatter.write_str("structural owner count overflow"),
            Self::GenerationExhausted => formatter.write_str("structural generation exhausted"),
            Self::RuntimeIdentityExhausted => {
                formatter.write_str("structural runtime identity exhausted")
            }
            Self::IncompleteSeal => {
                formatter.write_str("sealed region is incompletely initialized")
            }
            Self::UnsupportedDependency => formatter.write_str("unsupported domain dependency"),
            Self::WrongPool => formatter.write_str("pool ID belongs to another pool"),
            Self::WrongPartition => formatter.write_str("pool ID is outside the partition"),
            Self::SlotVacant => formatter.write_str("structural slot is vacant"),
        }
    }
}

impl std::error::Error for StructuralError {}
