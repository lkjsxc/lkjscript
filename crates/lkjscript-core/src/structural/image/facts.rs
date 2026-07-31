#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct TreeFacts {
    pub nodes: u32,
    pub bytes: u64,
    pub string_bytes: u64,
    pub path_bytes: u64,
}

impl TreeFacts {
    pub(crate) fn checked_add(self, other: Self) -> Option<Self> {
        Some(Self {
            nodes: self.nodes.checked_add(other.nodes)?,
            bytes: self.bytes.checked_add(other.bytes)?,
            string_bytes: self.string_bytes.checked_add(other.string_bytes)?,
            path_bytes: self.path_bytes.checked_add(other.path_bytes)?,
        })
    }
}
