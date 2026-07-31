#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProductId(u16);

impl ProductId {
    pub const fn new(raw: u16) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u16 {
        self.0
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductMetadata {
    pub id: ProductId,
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProductFieldRef {
    pub product: ProductId,
    pub field: u8,
}
