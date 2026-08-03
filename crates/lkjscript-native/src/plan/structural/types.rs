#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StructuralKind {
    Unit,
    Bool,
    I64,
    F64,
    String,
    Path,
    Bytes,
    ByteVector,
    Product,
    Enum,
    Static,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StructuralTypeIdentity {
    layout: u64,
    semantic_type: u64,
    kind: StructuralKind,
    copyable: bool,
}

impl StructuralTypeIdentity {
    #[must_use]
    pub const fn new(
        layout: u64,
        semantic_type: u64,
        kind: StructuralKind,
        copyable: bool,
    ) -> Self {
        Self {
            layout,
            semantic_type,
            kind,
            copyable,
        }
    }

    #[must_use]
    pub const fn layout(self) -> u64 {
        self.layout
    }

    #[must_use]
    pub const fn semantic_type(self) -> u64 {
        self.semantic_type
    }

    #[must_use]
    pub const fn kind(self) -> StructuralKind {
        self.kind
    }

    #[must_use]
    pub const fn copyable(self) -> bool {
        self.copyable
    }

    pub(crate) const fn is_valid(self) -> bool {
        self.layout != 0 && self.semantic_type != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StructuralViewType {
    projection: u64,
    root: StructuralTypeIdentity,
    projected: StructuralTypeIdentity,
    exclusive: bool,
}

impl StructuralViewType {
    #[must_use]
    pub const fn new(
        projection: u64,
        root: StructuralTypeIdentity,
        projected: StructuralTypeIdentity,
        exclusive: bool,
    ) -> Self {
        Self {
            projection,
            root,
            projected,
            exclusive,
        }
    }

    #[must_use]
    pub const fn projection(self) -> u64 {
        self.projection
    }

    #[must_use]
    pub const fn root(self) -> StructuralTypeIdentity {
        self.root
    }

    #[must_use]
    pub const fn projected(self) -> StructuralTypeIdentity {
        self.projected
    }

    #[must_use]
    pub const fn exclusive(self) -> bool {
        self.exclusive
    }

    pub(crate) const fn is_valid(self) -> bool {
        self.projection != 0 && self.root.is_valid() && self.projected.is_valid()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StructuralStorageRoute {
    Unique,
    Sealed,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StructuralDestinationType {
    aggregate: u64,
    value_type: StructuralTypeIdentity,
    storage: StructuralStorageRoute,
    initialized: u16,
}

impl StructuralDestinationType {
    #[must_use]
    pub const fn new(
        aggregate: u64,
        value_type: StructuralTypeIdentity,
        storage: StructuralStorageRoute,
        initialized: u16,
    ) -> Self {
        Self {
            aggregate,
            value_type,
            storage,
            initialized,
        }
    }

    #[must_use]
    pub const fn aggregate(self) -> u64 {
        self.aggregate
    }

    #[must_use]
    pub const fn value_type(self) -> StructuralTypeIdentity {
        self.value_type
    }

    #[must_use]
    pub const fn storage(self) -> StructuralStorageRoute {
        self.storage
    }

    #[must_use]
    pub const fn initialized(self) -> u16 {
        self.initialized
    }

    pub(crate) const fn is_valid(self) -> bool {
        self.aggregate != 0 && self.value_type.is_valid()
    }
}
