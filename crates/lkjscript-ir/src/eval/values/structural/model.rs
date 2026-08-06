use lkjscript_core::{
    StructuralDestinationKey, StructuralType, StructuralValueKey, StructuralViewKey,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvalStructuralOwner {
    pub key: StructuralValueKey,
    pub value_type: StructuralType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvalStructuralView {
    pub owner: StructuralValueKey,
    pub key: StructuralViewKey,
    pub root_type: StructuralType,
    pub value_type: StructuralType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvalStructuralDestination {
    pub key: StructuralDestinationKey,
    pub value_type: StructuralType,
    pub type_id: crate::StructuralTypeId,
    pub storage: crate::StructuralStorage,
    pub route: [u8; 32],
    pub active_variant: Option<crate::VariantId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StaticStringArtifact {
    pub(crate) identity: u64,
    pub(crate) text: Box<str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AggregateMode {
    Structural,
    Region,
    Legacy,
    ResourceAdapter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ClosureClass {
    Inline,
    Static,
    Dynamic,
    Legacy { dynamic_reachable: bool },
    Resource,
}

impl ClosureClass {
    pub(crate) const fn dynamic_reachable(self) -> bool {
        matches!(
            self,
            Self::Dynamic
                | Self::Legacy {
                    dynamic_reachable: true
                }
        )
    }
}
