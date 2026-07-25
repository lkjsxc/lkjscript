use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductDefinition {
    pub id: ProductId,
    pub name: String,
    pub origin: SourceId,
    pub fields: Vec<ProductField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductField {
    pub name: String,
    pub ty: Type,
}
