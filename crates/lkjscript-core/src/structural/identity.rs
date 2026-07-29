use std::num::{NonZeroU32, NonZeroU64};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StructuralRuntimeId(NonZeroU64);

impl StructuralRuntimeId {
    pub(super) const fn new(value: NonZeroU64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DomainClass {
    Static,
    Unique,
    RegionBuilding,
    RegionOwned,
    RegionSealing,
    RegionSealed,
    Pool,
    External,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DomainKey {
    runtime: StructuralRuntimeId,
    class: DomainClass,
    slot: u32,
    generation: NonZeroU32,
}

impl DomainKey {
    pub const fn runtime(self) -> StructuralRuntimeId {
        self.runtime
    }

    pub const fn class(self) -> DomainClass {
        self.class
    }

    pub const fn slot(self) -> u32 {
        self.slot
    }

    pub const fn generation(self) -> NonZeroU32 {
        self.generation
    }

    pub(super) const fn from_parts(
        runtime: StructuralRuntimeId,
        class: DomainClass,
        slot: u32,
        generation: NonZeroU32,
    ) -> Self {
        Self {
            runtime,
            class,
            slot,
            generation,
        }
    }

    pub(super) const fn with_class(self, class: DomainClass) -> Self {
        Self { class, ..self }
    }
}

macro_rules! identity {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(NonZeroU64);

        impl $name {
            pub const fn new(value: NonZeroU64) -> Self {
                Self(value)
            }

            pub const fn get(self) -> u64 {
                self.0.get()
            }
        }
    };
}

identity!(LayoutIdentity);
identity!(SemanticTypeIdentity);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RootClass {
    RegionInternal,
    RegionPublic,
    SealedPublic,
    PoolElement,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RootKey {
    domain: DomainKey,
    class: RootClass,
    slot: u32,
    generation: NonZeroU32,
    layout: LayoutIdentity,
    semantic_type: SemanticTypeIdentity,
}

impl RootKey {
    pub const fn domain(self) -> DomainKey {
        self.domain
    }

    pub const fn class(self) -> RootClass {
        self.class
    }

    pub const fn slot(self) -> u32 {
        self.slot
    }

    pub const fn generation(self) -> NonZeroU32 {
        self.generation
    }

    pub const fn layout(self) -> LayoutIdentity {
        self.layout
    }

    pub const fn semantic_type(self) -> SemanticTypeIdentity {
        self.semantic_type
    }

    pub(super) const fn from_parts(
        domain: DomainKey,
        class: RootClass,
        slot: u32,
        generation: NonZeroU32,
        layout: LayoutIdentity,
        semantic_type: SemanticTypeIdentity,
    ) -> Self {
        Self {
            domain,
            class,
            slot,
            generation,
            layout,
            semantic_type,
        }
    }
}
