use std::num::NonZeroU64;

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
    slot: u64,
    generation: NonZeroU64,
}

impl DomainKey {
    pub const fn runtime(self) -> StructuralRuntimeId {
        self.runtime
    }

    pub const fn class(self) -> DomainClass {
        self.class
    }

    pub const fn slot(self) -> u64 {
        self.slot
    }

    pub const fn generation(self) -> NonZeroU64 {
        self.generation
    }

    pub(super) const fn from_parts(
        runtime: StructuralRuntimeId,
        class: DomainClass,
        slot: u64,
        generation: NonZeroU64,
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
    slot: u64,
    generation: NonZeroU64,
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

    pub const fn slot(self) -> u64 {
        self.slot
    }

    pub const fn generation(self) -> NonZeroU64 {
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
        slot: u64,
        generation: NonZeroU64,
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn domain_and_root_slots_and_generations_preserve_high_values() {
        let high = u64::from(u32::MAX) + 17;
        let runtime = StructuralRuntimeId::new(NonZeroU64::MIN);
        let domain = DomainKey::from_parts(
            runtime,
            DomainClass::Unique,
            high,
            NonZeroU64::new(high + 1).expect("high generation"),
        );
        let root = RootKey::from_parts(
            domain,
            RootClass::UniquePublic,
            high + 2,
            NonZeroU64::new(high + 3).expect("high root generation"),
            LayoutIdentity::new(NonZeroU64::MIN),
            SemanticTypeIdentity::new(NonZeroU64::MIN),
        );
        assert_eq!(domain.slot(), high);
        assert_eq!(domain.generation().get(), high + 1);
        assert_eq!(root.slot(), high + 2);
        assert_eq!(root.generation().get(), high + 3);
    }
}
