mod chunk;
mod enum_metadata;

pub use chunk::{
    Chunk, ConstId, Constant, FunctionProto, ProductFieldRef, ProductId, ProductMetadata,
    ResourceReturnKind,
};
pub use enum_metadata::{
    EnumConstructionRef, EnumFieldMetadata, EnumFieldRef, EnumId, EnumMetadata,
    EnumVariantMetadata, EnumVariantRef, RuntimeLayoutId, VariantFieldId, VariantId,
};
