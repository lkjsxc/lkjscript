use super::*;

pub const ENUM_RECURSION_MAX_DEPTH: usize = 32;
pub const ENUM_RECURSION_MAX_WORK: usize = 4_096;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDefinition {
    pub id: EnumId,
    pub name: String,
    pub origin: SourceId,
    pub type_parameters: Vec<String>,
    pub variants: Vec<EnumVariant>,
    pub layout: EnumLayoutFacts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub id: VariantId,
    pub name: String,
    pub source_order: u64,
    pub fields: Vec<EnumVariantField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumLayoutFacts {
    pub identity: RuntimeLayoutId,
    pub recursive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariantField {
    pub id: VariantFieldId,
    pub name: String,
    pub source_order: u64,
    pub ty: Type,
    pub indirect: bool,
}
