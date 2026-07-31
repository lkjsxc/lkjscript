use super::*;

mod call;
mod descriptors;
mod operation_validation;
mod operations;
mod types;

use operation_validation::*;

pub use call::StructuralCallDescriptor;
pub use descriptors::{
    StructuralAggregateDescriptor, StructuralAggregateKind, StructuralPayloadKind,
    StructuralProjectionDescriptor, StructuralProjectionKind,
};
pub use operations::{StructuralNumericConversion, StructuralOperation};
pub use types::{
    StructuralDestinationType, StructuralKind, StructuralTypeIdentity, StructuralViewType,
};
