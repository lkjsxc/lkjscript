use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralRootTableError {
    AllocationFailed,
    ArithmeticOverflow,
    InvalidKey,
    WrongRuntime,
    StaleRoot,
    MovedRoot,
    DroppedRoot,
    RetiredRoot,
    WrongLayout,
    WrongSemanticType,
    WrongOwnership,
    DuplicateOwner,
    BorrowConflict,
    LiveRoot,
    LiveLoan,
    StaleLoan,
    GenerationExhausted,
    InvariantViolation,
}

impl fmt::Display for StructuralRootTableError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllocationFailed => output.write_str("structural root table allocation failed"),
            Self::ArithmeticOverflow => {
                output.write_str("structural root table arithmetic overflow")
            }
            Self::InvalidKey => output.write_str("invalid structural value key"),
            Self::WrongRuntime => output.write_str("structural root belongs to another runtime"),
            Self::StaleRoot => output.write_str("stale structural value key"),
            Self::MovedRoot => output.write_str("structural value was moved"),
            Self::DroppedRoot => output.write_str("structural value was dropped"),
            Self::RetiredRoot => output.write_str("structural root slot is retired"),
            Self::WrongLayout => output.write_str("wrong structural root layout"),
            Self::WrongSemanticType => output.write_str("wrong structural semantic type"),
            Self::WrongOwnership => output.write_str("wrong structural root ownership state"),
            Self::DuplicateOwner => output.write_str("duplicate structural root owner"),
            Self::BorrowConflict => output.write_str("conflicting structural root borrow"),
            Self::LiveRoot => output.write_str("structural root table has a live root"),
            Self::LiveLoan => output.write_str("structural root has a live loan"),
            Self::StaleLoan => output.write_str("stale structural borrow key"),
            Self::GenerationExhausted => {
                output.write_str("structural root table generation exhausted")
            }
            Self::InvariantViolation => {
                output.write_str("structural root table invariant violation")
            }
        }
    }
}

impl std::error::Error for StructuralRootTableError {}
