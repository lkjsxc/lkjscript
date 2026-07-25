#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum I64Comparison {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum F64Comparison {
    OrderedEqual,
    OrderedNotEqual,
    OrderedLessThan,
    OrderedLessThanOrEqual,
    OrderedGreaterThan,
    OrderedGreaterThanOrEqual,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BoolComparison {
    Equal,
    NotEqual,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u32)]
pub enum TrapCode {
    I64Overflow = 1,
    DivisionByZero = 2,
    Explicit = 3,
}

impl TrapCode {
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
}
