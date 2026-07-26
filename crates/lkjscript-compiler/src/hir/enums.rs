use super::*;

pub const ENUM_RECURSION_MAX_DEPTH: usize = 32;
pub const ENUM_RECURSION_MAX_WORK: usize = 4_096;
pub const MAX_ENUM_VARIANTS: usize = 15;
pub const MAX_VARIANT_FIELDS: usize = 15;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDefinition {
    pub id: EnumId,
    pub name: String,
    pub origin: SourceId,
    pub type_parameters: Vec<String>,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub id: VariantId,
    pub name: String,
    pub source_order: u16,
    pub fields: Vec<EnumVariantField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariantField {
    pub id: VariantFieldId,
    pub name: String,
    pub source_order: u16,
    pub ty: Type,
}
