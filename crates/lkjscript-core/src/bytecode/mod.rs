mod chunk;
mod enum_metadata;
mod structural;

pub use chunk::{
    Chunk, ConstId, Constant, FailureCleanupAction, FailureCleanupPlan, FailureCleanupRange,
    FunctionProto, ProductFieldRef, ProductId, ProductMetadata, ResourceReturnKind,
    UniqueValueKind,
};
pub use enum_metadata::{
    EnumConstructionRef, EnumFieldMetadata, EnumFieldRef, EnumId, EnumMetadata,
    EnumVariantMetadata, EnumVariantRef, RuntimeLayoutId, VariantFieldId, VariantId,
};
pub use structural::*;
