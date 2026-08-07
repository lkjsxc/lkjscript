#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Architecture {
    X86_64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatingSystem {
    Linux,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Endianness {
    Little,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerWidth {
    Bits64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetSpec {
    pub architecture: Architecture,
    pub operating_system: OperatingSystem,
    pub endianness: Endianness,
    pub pointer_width: PointerWidth,
}

impl TargetSpec {
    pub const LINUX_X86_64: Self = Self {
        architecture: Architecture::X86_64,
        operating_system: OperatingSystem::Linux,
        endianness: Endianness::Little,
        pointer_width: PointerWidth::Bits64,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallingConventionId {
    SystemV64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutablePolicy {
    Forbidden,
    WritableThenExecutable,
}
