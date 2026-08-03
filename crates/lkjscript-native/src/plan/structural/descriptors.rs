use super::*;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StructuralPayloadKind {
    String,
    Path,
    Bytes,
    ByteVector,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StructuralProjectionKind {
    Field,
    Utf8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralProjectionDescriptor {
    view_type: StructuralViewType,
    kind: StructuralProjectionKind,
    path: Vec<u16>,
}

impl StructuralProjectionDescriptor {
    #[must_use]
    pub fn new(
        view_type: StructuralViewType,
        kind: StructuralProjectionKind,
        path: Vec<u16>,
    ) -> Self {
        Self {
            view_type,
            kind,
            path,
        }
    }

    #[must_use]
    pub const fn view_type(&self) -> StructuralViewType {
        self.view_type
    }

    #[must_use]
    pub const fn kind(&self) -> StructuralProjectionKind {
        self.kind
    }

    #[must_use]
    pub fn path(&self) -> &[u16] {
        &self.path
    }

    pub(crate) fn canonical(&self) -> bool {
        self.view_type.is_valid() && self.path.len() <= u16::MAX as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StructuralAggregateKind {
    Product,
    Enum(u16),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralAggregateDescriptor {
    identity: u64,
    value_type: StructuralTypeIdentity,
    kind: StructuralAggregateKind,
    fields: Vec<StructuralTypeIdentity>,
}

impl StructuralAggregateDescriptor {
    #[must_use]
    pub fn new(
        identity: u64,
        value_type: StructuralTypeIdentity,
        kind: StructuralAggregateKind,
        fields: Vec<StructuralTypeIdentity>,
    ) -> Self {
        Self {
            identity,
            value_type,
            kind,
            fields,
        }
    }

    #[must_use]
    pub const fn identity(&self) -> u64 {
        self.identity
    }

    #[must_use]
    pub const fn value_type(&self) -> StructuralTypeIdentity {
        self.value_type
    }

    #[must_use]
    pub const fn kind(&self) -> StructuralAggregateKind {
        self.kind
    }

    #[must_use]
    pub fn fields(&self) -> &[StructuralTypeIdentity] {
        &self.fields
    }

    #[doc(hidden)]
    pub fn destination(
        &self,
        storage: StructuralStorageRoute,
        initialized: u16,
    ) -> StructuralDestinationType {
        StructuralDestinationType::new(self.identity, self.value_type, storage, initialized)
    }

    pub(crate) fn canonical(&self) -> bool {
        let right_kind = matches!(
            (self.kind, self.value_type.kind()),
            (StructuralAggregateKind::Product, StructuralKind::Product)
                | (StructuralAggregateKind::Enum(_), StructuralKind::Enum)
        );
        self.identity != 0
            && self.value_type.is_valid()
            && right_kind
            && self.fields.len() <= u16::MAX as usize
            && self.fields.iter().all(|field| field.is_valid())
    }
}
