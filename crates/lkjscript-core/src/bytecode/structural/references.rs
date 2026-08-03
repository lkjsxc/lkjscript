#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructuralDestinationFieldRef {
    pub destination: StructuralDestinationId,
    pub field: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructuralAggregateFieldRef {
    pub representation: StructuralRepresentationId,
    pub active_variant: Option<VariantId>,
    pub field: u16,
    pub result: StructuralFieldMetadata,
    pub result_representation: Option<StructuralRepresentationId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructuralPayloadRef {
    pub representation: StructuralRepresentationId,
    pub variant: VariantId,
    pub result: StructuralFieldMetadata,
    pub result_representation: Option<StructuralRepresentationId>,
}
