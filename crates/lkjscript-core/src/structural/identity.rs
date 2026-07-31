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
    UniquePublic,
    StaticPublic,
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

pub fn product_semantic_identity(identity: crate::RuntimeLayoutId) -> SemanticTypeIdentity {
    SemanticTypeIdentity::new(product_nonzero(product_fingerprint(
        0x8f3f_73b5_cf1c_9ade,
        &identity.bytes(),
    )))
}

pub fn product_layout_identity(identity: crate::RuntimeLayoutId) -> LayoutIdentity {
    LayoutIdentity::new(product_nonzero(product_fingerprint(
        0xe55a_7341_0a0f_b861,
        &identity.bytes(),
    )))
}

fn product_fingerprint(mut state: u64, bytes: &[u8]) -> u64 {
    for &byte in bytes {
        state = (state ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    state
}

const fn product_nonzero(value: u64) -> NonZeroU64 {
    match NonZeroU64::new(value) {
        Some(value) => value,
        None => NonZeroU64::MIN,
    }
}
