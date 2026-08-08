use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductDefinition {
    pub id: ProductId,
    /// Private nominal identity consumed by memory and runtime metadata.
    pub identity: [u8; 32],
    pub name: String,
    pub origin: Origin,
    pub fields: Vec<ProductField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductField {
    pub identity: [u8; 32],
    pub source_order: u64,
    pub name: String,
    pub ty: Type,
}
