use std::num::NonZeroU64;

macro_rules! scalar_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(NonZeroU64);

        impl $name {
            pub const fn new(raw: u64) -> Option<Self> {
                match NonZeroU64::new(raw) {
                    Some(value) => Some(Self(value)),
                    None => None,
                }
            }

            pub const fn get(self) -> u64 {
                self.0.get()
            }

            pub const fn from_nonzero(value: NonZeroU64) -> Self {
                Self(value)
            }
        }
    };
}

scalar_id!(NodeIdentity);
scalar_id!(ApplicationId);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PackageContentId([u8; 32]);

impl PackageContentId {
    pub fn new(digest: [u8; 32]) -> Option<Self> {
        if digest.iter().all(|byte| *byte == 0) {
            None
        } else {
            Some(Self(digest))
        }
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ApplicationGenerationId {
    application: ApplicationId,
    generation: NonZeroU64,
}

impl ApplicationGenerationId {
    pub(crate) const fn new(application: ApplicationId, generation: NonZeroU64) -> Self {
        Self {
            application,
            generation,
        }
    }

    pub const fn application(self) -> ApplicationId {
        self.application
    }

    pub const fn generation(self) -> u64 {
        self.generation.get()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ApplicationInstanceId {
    generation: ApplicationGenerationId,
    serial: NonZeroU64,
}

impl ApplicationInstanceId {
    pub(crate) const fn new(generation: ApplicationGenerationId, serial: NonZeroU64) -> Self {
        Self { generation, serial }
    }

    pub const fn generation(self) -> ApplicationGenerationId {
        self.generation
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExecutionCellId {
    instance: ApplicationInstanceId,
    serial: NonZeroU64,
}

impl ExecutionCellId {
    pub(crate) const fn new(instance: ApplicationInstanceId, serial: NonZeroU64) -> Self {
        Self { instance, serial }
    }

    pub const fn instance(self) -> ApplicationInstanceId {
        self.instance
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }
}
