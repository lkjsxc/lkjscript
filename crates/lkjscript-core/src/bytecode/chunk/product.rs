pub fn runtime_product_contract_identity(
    plan: crate::MemoryPlanId,
    name: &str,
) -> crate::Result<crate::RuntimeLayoutId> {
    let name_len = u64::try_from(name.len())
        .map_err(|_| crate::Error::msg("product name length exceeds u64"))?;
    let prefix = b"lkjscript.runtime-product\0";
    let capacity = prefix
        .len()
        .checked_add(32 + 8)
        .and_then(|size| size.checked_add(name.len()))
        .ok_or_else(|| crate::Error::msg("product identity size overflow"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| crate::Error::msg("product identity allocation failed"))?;
    bytes.extend_from_slice(prefix);
    bytes.extend_from_slice(&plan.bytes());
    bytes.extend_from_slice(&name_len.to_le_bytes());
    bytes.extend_from_slice(name.as_bytes());
    Ok(crate::RuntimeLayoutId::new(crate::sha256(&bytes)))
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionProductFieldKind {
    Unit,
    Bool,
    I64,
    F64,
    List,
    Product(ProductId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductMetadata {
    pub id: ProductId,
    pub identity: crate::RuntimeLayoutId,
    pub region: bool,
    pub name: String,
    pub fields: Vec<String>,
    pub region_fields: Vec<RegionProductFieldKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProductFieldRef {
    pub product: ProductId,
    pub field: u8,
}
