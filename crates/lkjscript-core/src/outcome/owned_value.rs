use std::fmt;

use crate::{Error, HeapObj, ProductId, Result, Value};

/// A returned value plus a private snapshot of every reachable VM object.
///
/// No arena index is exposed. The snapshot is independent of the VM arena and
/// remains valid after execution resources are released.
#[derive(Clone, PartialEq)]
pub struct OwnedValue {
    root: Value,
    heap: Vec<Option<HeapObj>>,
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
            let Some(index) = value.as_heap() else {
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
        Ok(Self { root, heap })
    }

    pub fn is_unit(&self) -> bool {
        self.root.is_unit()
    }

    pub fn is_empty_list(&self) -> bool {
        self.root.is_empty_list()
    }

    pub fn is_none(&self) -> bool {
        self.root.is_none()
    }

    pub fn as_bool(&self) -> Option<bool> {
        self.root.as_bool()
    }

    pub fn as_i64(&self) -> Option<i64> {
        if let Some(number) = self.root.as_small_i64() {
            return Some(number);
        }
        match self.object()? {
            HeapObj::Int(number) => Some(*number),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self.object()? {
            HeapObj::Float(number) => Some(*number),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self.object()? {
            HeapObj::Str(text) | HeapObj::Symbol(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_handle(&self) -> Option<u32> {
        self.root.as_handle()
    }

    pub fn product_id(&self) -> Option<ProductId> {
        match self.object()? {
            HeapObj::Product { product, .. } => Some(*product),
            _ => None,
        }
    }

    /// Test/diagnostic inspection of retained reachable snapshot storage.
    #[doc(hidden)]
    pub fn snapshot_object_count(&self) -> usize {
        self.heap.iter().flatten().count()
    }

    fn object(&self) -> Option<&HeapObj> {
        let index = usize::try_from(self.root.as_heap()?).ok()?;
        self.heap.get(index)?.as_ref()
    }
}

impl fmt::Debug for OwnedValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_unit() {
            return formatter.write_str("unit");
        }
        if self.is_empty_list() {
            return formatter.write_str("empty-list");
        }
        if self.is_none() {
            return formatter.write_str("none");
        }
        if let Some(value) = self.as_bool() {
            return value.fmt(formatter);
        }
        if let Some(value) = self.as_i64() {
            return value.fmt(formatter);
        }
        if let Some(value) = self.as_f64() {
            return value.fmt(formatter);
        }
        if let Some(value) = self.as_str() {
            return value.fmt(formatter);
        }
        if let Some(value) = self.as_handle() {
            return write!(formatter, "handle#{value}");
        }
        match self.object() {
            Some(HeapObj::Pair { .. }) => formatter.write_str("#<owned-pair>"),
            Some(HeapObj::Closure { proto, .. }) => write!(formatter, "#<owned-fn:{proto}>"),
            Some(HeapObj::Builtin(id)) => write!(formatter, "#<owned-builtin:{id}>"),
            Some(HeapObj::Buf(bytes)) => write!(formatter, "#<owned-buf:{}>", bytes.len()),
            Some(HeapObj::ResultOk(_)) => formatter.write_str("#<owned-ok>"),
            Some(HeapObj::ResultErr(_)) => formatter.write_str("#<owned-err>"),
            Some(HeapObj::OptionSome(_)) => formatter.write_str("#<owned-some>"),
            Some(HeapObj::Product { product, .. }) => {
                write!(formatter, "#<owned-product:{}>", product.raw())
            }
            Some(HeapObj::Int(_) | HeapObj::Float(_) | HeapObj::Str(_) | HeapObj::Symbol(_)) => {
                formatter.write_str("#<owned-value>")
            }
            None => formatter.write_str("#<invalid-owned-value>"),
        }
    }
}
