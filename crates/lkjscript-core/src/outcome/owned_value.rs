use std::fmt;

use crate::{Error, HeapObj, ProductId, Result, RuntimeLayoutId, Value};

/// A returned value plus a private snapshot of every reachable VM object.
///
/// No arena index is exposed. The snapshot is independent of the VM arena and
/// remains valid after execution resources are released.
#[derive(Clone, PartialEq)]
pub struct OwnedValue {
    root: Value,
    heap: Vec<Option<HeapObj>>,
    unique_byte_vector: Option<Vec<u8>>,
    unique_bytes: Option<Vec<u8>>,
    symbols: Vec<Option<String>>,
}

impl OwnedValue {
    pub const fn is_invalid(&self) -> bool {
        false
    }

    /// Builds and verifies an owned snapshot. VM implementations use this when
    /// transferring a returned value across the execution boundary.
    #[doc(hidden)]
    pub fn from_vm_snapshot(root: Value, heap: Vec<Option<HeapObj>>) -> Result<Self> {
        if root.is_invalid() {
            return Err(Error::msg("cannot own an invalid VM value"));
        }
        let mut pending = vec![root];
        let mut visited = vec![false; heap.len()];
        while let Some(value) = pending.pop() {
            let Some(index) = value.as_legacy_traced() else {
                continue;
            };
            let index = usize::try_from(index)
                .map_err(|_| Error::msg("owned value heap index out of range"))?;
            let Some(slot) = heap.get(index) else {
                return Err(Error::msg("owned value heap index out of range"));
            };
            let Some(object) = slot else {
                return Err(Error::msg("owned value references a missing heap object"));
            };
            if visited[index] {
                continue;
            }
            visited[index] = true;
            object.trace(&mut |child| pending.push(child));
        }
        Ok(Self {
            root,
            heap,
            unique_byte_vector: None,
            unique_bytes: None,
            symbols: Vec::new(),
        })
    }

    /// Transfers one collector-free byte-vector result across the execution
    /// boundary without retaining its runtime-local key or store.
    #[doc(hidden)]
    pub fn from_unique_byte_vector(bytes: Vec<u8>) -> Result<Self> {
        Ok(Self {
            root: Value::UNIT,
            heap: Vec::new(),
            unique_byte_vector: Some(bytes),
            unique_bytes: None,
            symbols: Vec::new(),
        })
    }

    #[doc(hidden)]
    pub fn from_unique_bytes(bytes: Vec<u8>) -> Result<Self> {
        Ok(Self {
            root: Value::UNIT,
            heap: Vec::new(),
            unique_byte_vector: None,
            unique_bytes: Some(bytes),
            symbols: Vec::new(),
        })
    }

    pub fn is_unit(&self) -> bool {
        self.unique_byte_vector.is_none() && self.unique_bytes.is_none() && self.root.is_unit()
    }

    pub fn is_empty_list(&self) -> bool {
        self.root.is_empty_list()
    }

    pub fn as_bool(&self) -> Option<bool> {
        self.root.as_bool()
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.root.as_i64()
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.root.as_f64()
    }

    pub fn as_f64_bits(&self) -> Option<u64> {
        self.root.as_f64_bits()
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Some(index) = self.root.as_symbol() {
            return self.symbols.get(index as usize)?.as_deref();
        }
        match self.object()? {
            HeapObj::Str(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_path_bytes(&self) -> Option<&[u8]> {
        match self.object()? {
            HeapObj::Path(bytes) => Some(bytes),
            _ => None,
        }
    }

    pub fn as_byte_vector(&self) -> Option<&[u8]> {
        self.unique_byte_vector.as_deref()
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        self.unique_bytes.as_deref()
    }

    pub fn as_resource(&self) -> Option<u32> {
        self.root.as_resource()
    }

    pub fn as_function(&self) -> Option<u32> {
        self.root.as_function()
    }

    pub fn enum_identity(&self) -> Option<(RuntimeLayoutId, u16)> {
        match self.object()? {
            HeapObj::Enum {
                layout,
                physical_tag,
                ..
            } => Some((*layout, *physical_tag)),
            _ => None,
        }
    }

    pub fn product_id(&self) -> Option<ProductId> {
        match self.object()? {
            HeapObj::Product { product, .. } => Some(*product),
            _ => None,
        }
    }

    pub fn enum_physical_tag(&self) -> Option<u16> {
        match self.object()? {
            HeapObj::Enum { physical_tag, .. } => Some(*physical_tag),
            _ => None,
        }
    }

    pub fn enum_field_i64(&self, field: usize) -> Option<i64> {
        let HeapObj::Enum { active_payload, .. } = self.object()? else {
            return None;
        };
        self.value_i64(*active_payload.get(field)?)
    }

    /// Test/diagnostic inspection of retained reachable snapshot storage.
    #[doc(hidden)]
    pub fn snapshot_object_count(&self) -> usize {
        self.heap.iter().flatten().count()
    }

    fn object(&self) -> Option<&HeapObj> {
        let index = usize::try_from(self.root.as_legacy_traced()?).ok()?;
        self.heap.get(index)?.as_ref()
    }

    fn value_i64(&self, value: Value) -> Option<i64> {
        value.as_i64()
    }
}

include!("owned_value/symbols.rs");
include!("owned_value/debug.rs");
