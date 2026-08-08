#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectSet(u16);

impl EffectSet {
    /// Effects are not yet knowable because an incomplete expression remains.
    /// The bit propagates through unions and is rejected at the executable boundary.
    pub const UNKNOWN: Self = Self(1 << 15);
    pub const PURE: Self = Self(0);
    pub const ALLOCATES: Self = Self(1 << 0);
    pub const READS_MEMORY: Self = Self(1 << 1);
    pub const WRITES_MEMORY: Self = Self(1 << 2);
    pub const MUTATES_LOCAL: Self = Self(1 << 3);
    pub const HOST_IO: Self = Self(1 << 4);
    pub const MAY_TRAP: Self = Self(1 << 5);
    pub const MAY_EXIT: Self = Self(1 << 6);
    pub const MAY_DIVERGE: Self = Self(1 << 7);
    pub const CONSERVATIVE_CALL: Self = Self::ALLOCATES
        .union(Self::READS_MEMORY)
        .union(Self::WRITES_MEMORY)
        .union(Self::MUTATES_LOCAL)
        .union(Self::HOST_IO)
        .union(Self::MAY_TRAP)
        .union(Self::MAY_EXIT)
        .union(Self::MAY_DIVERGE);

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn is_known(self) -> bool {
        self.0 & Self::UNKNOWN.0 == 0
    }

    pub(crate) const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    pub const fn contains(self, effects: Self) -> bool {
        self.0 & effects.0 == effects.0
    }
}
