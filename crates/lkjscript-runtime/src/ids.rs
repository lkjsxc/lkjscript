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

scalar_id!(CoordinatorIdentity);
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
pub struct ApplicationIncarnationId {
    coordinator: CoordinatorIdentity,
    application: ApplicationId,
    incarnation: NonZeroU64,
}

impl ApplicationIncarnationId {
    pub(crate) const fn new(
        coordinator: CoordinatorIdentity,
        application: ApplicationId,
        incarnation: NonZeroU64,
    ) -> Self {
        Self {
            coordinator,
            application,
            incarnation,
        }
    }

    pub const fn coordinator(self) -> CoordinatorIdentity {
        self.coordinator
    }

    pub const fn application(self) -> ApplicationId {
        self.application
    }

    pub const fn incarnation(self) -> u64 {
        self.incarnation.get()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExecutionCellId {
    incarnation: ApplicationIncarnationId,
    serial: NonZeroU64,
}

impl ExecutionCellId {
    pub(crate) const fn new(incarnation: ApplicationIncarnationId, serial: NonZeroU64) -> Self {
        Self {
            incarnation,
            serial,
        }
    }

    pub const fn incarnation(self) -> ApplicationIncarnationId {
        self.incarnation
    }

    pub const fn serial(self) -> u64 {
        self.serial.get()
    }
}
