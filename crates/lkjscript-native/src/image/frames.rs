use super::*;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FrameHomeKind {
    Local(u32),
    Value(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameHome {
    pub(super) kind: FrameHomeKind,
    pub(super) value_type: ValueType,
    pub(super) rbp_displacement: i32,
}

impl FrameHome {
    #[must_use]
    pub const fn kind(self) -> FrameHomeKind {
        self.kind
    }

    #[must_use]
    pub const fn value_type(self) -> ValueType {
        self.value_type
    }

    #[must_use]
    pub const fn rbp_displacement(self) -> i32 {
        self.rbp_displacement
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameFacts {
    pub(super) function: FunctionId,
    pub(super) frame_bytes: u32,
    pub(super) value_slots: u32,
    pub(super) local_slots: u32,
    pub(super) outgoing_machine_arguments: u8,
    pub(super) uses_red_zone: bool,
    pub(super) call_site_aligned_16: bool,
    pub(super) homes: Vec<FrameHome>,
}

impl FrameFacts {
    #[must_use]
    pub const fn function(&self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn frame_bytes(&self) -> u32 {
        self.frame_bytes
    }

    #[must_use]
    pub const fn value_slots(&self) -> u32 {
        self.value_slots
    }

    #[must_use]
    pub const fn local_slots(&self) -> u32 {
        self.local_slots
    }

    #[must_use]
    pub const fn outgoing_machine_arguments(&self) -> u8 {
        self.outgoing_machine_arguments
    }

    #[must_use]
    pub const fn uses_red_zone(&self) -> bool {
        self.uses_red_zone
    }

    #[must_use]
    pub const fn call_site_aligned_16(&self) -> bool {
        self.call_site_aligned_16
    }

    #[must_use]
    pub fn homes(&self) -> &[FrameHome] {
        &self.homes
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RootLocation {
    pub(super) rbp_displacement: i32,
    pub(super) kind: FrameHomeKind,
    pub(super) reference_type: ReferenceType,
}

impl RootLocation {
    #[must_use]
    pub const fn rbp_displacement(self) -> i32 {
        self.rbp_displacement
    }

    #[must_use]
    pub const fn kind(self) -> FrameHomeKind {
        self.kind
    }

    #[must_use]
    pub const fn reference_type(self) -> ReferenceType {
        self.reference_type
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactStackMap {
    pub(super) roots: Vec<RootLocation>,
}

impl ExactStackMap {
    #[must_use]
    pub fn roots(&self) -> &[RootLocation] {
        &self.roots
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Safepoint {
    pub(super) id: u32,
    pub(super) function: FunctionId,
    pub(super) code_offset: u32,
    pub(super) stack_map: ExactStackMap,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RootMapRequirement {
    pub(super) id: u32,
    pub(super) function: FunctionId,
    pub(super) roots: Vec<RootLocation>,
}

impl Safepoint {
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }

    #[must_use]
    pub const fn function(&self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn code_offset(&self) -> u32 {
        self.code_offset
    }

    #[must_use]
    pub const fn stack_map(&self) -> &ExactStackMap {
        &self.stack_map
    }
}
