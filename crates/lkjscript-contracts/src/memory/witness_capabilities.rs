#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryWitnessCapabilities {
    pub inline: bool,
    pub static_value: bool,
    pub unique: bool,
    pub ordinary_region: bool,
    pub sealed_region: bool,
    pub borrow: bool,
    pub semantic_snapshot: bool,
    pub list_element: bool,
    pub equality: bool,
}

impl MemoryWitnessCapabilities {
    pub const NONE: Self = Self {
        inline: false,
        static_value: false,
        unique: false,
        ordinary_region: false,
        sealed_region: false,
        borrow: false,
        semantic_snapshot: false,
        list_element: false,
        equality: false,
    };
}
