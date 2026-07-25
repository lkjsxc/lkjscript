use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanError {
    TooManyParameters { count: usize, maximum: usize },
    TooManyItems,
    ForeignId(&'static str),
    UnknownFunction,
    FunctionAlreadyDefined,
    UnknownBlock,
    UnknownValue,
    UnknownLocal,
    BlockAlreadyTerminated,
    EncoderOwnedRuntimeCall,
    InvalidHeapCall,
}

impl fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyParameters { count, maximum } => {
                write!(
                    formatter,
                    "signature has {count} parameters; maximum is {maximum}"
                )
            }
            Self::TooManyItems => formatter.write_str("machine plan exceeds its ID space"),
            Self::ForeignId(kind) => {
                write!(formatter, "{kind} belongs to a different plan or function")
            }
            Self::UnknownFunction => formatter.write_str("unknown machine-plan function"),
            Self::FunctionAlreadyDefined => {
                formatter.write_str("machine-plan function is already defined")
            }
            Self::UnknownBlock => formatter.write_str("unknown machine-plan block"),
            Self::UnknownValue => formatter.write_str("unknown machine-plan value"),
            Self::UnknownLocal => formatter.write_str("unknown machine-plan local"),
            Self::BlockAlreadyTerminated => {
                formatter.write_str("machine-plan block is already terminated")
            }
            Self::EncoderOwnedRuntimeCall => {
                formatter.write_str("runtime call is owned by the native encoder")
            }
            Self::InvalidHeapCall => formatter.write_str("heap runtime call metadata is invalid"),
        }
    }
}

impl std::error::Error for PlanError {}
