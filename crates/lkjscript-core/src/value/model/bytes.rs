use super::{Value, ValueKind};

impl Value {
    #[doc(hidden)]
    pub const fn from_static_bytes(index: u16) -> Self {
        Self::new(ValueKind::StaticBytes, index as u64)
    }

    #[doc(hidden)]
    pub const fn from_bytes_key(key: u64) -> Self {
        Self::new(ValueKind::BytesKey, key)
    }

    #[doc(hidden)]
    pub const fn from_byte_vector_key(key: u64) -> Self {
        Self::new(ValueKind::ByteVectorKey, key)
    }

    #[doc(hidden)]
    pub const fn from_bytes_borrow(token: u64) -> Self {
        Self::new(ValueKind::BytesBorrow, token)
    }

    pub const fn from_byte_slice(token: u64, mutable: bool) -> Self {
        Self::new(
            if mutable {
                ValueKind::ByteSliceMut
            } else {
                ValueKind::ByteSlice
            },
            token,
        )
    }

    #[doc(hidden)]
    pub const fn as_static_bytes(self) -> Option<u16> {
        match self.kind {
            ValueKind::StaticBytes => Some(self.payload as u16),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub const fn as_bytes_key(self) -> Option<u64> {
        match self.kind {
            ValueKind::BytesKey => Some(self.payload),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub const fn as_byte_vector_key(self) -> Option<u64> {
        match self.kind {
            ValueKind::ByteVectorKey => Some(self.payload),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub const fn as_bytes_borrow(self) -> Option<u64> {
        match self.kind {
            ValueKind::BytesBorrow => Some(self.payload),
            _ => None,
        }
    }

    pub const fn as_byte_slice(self) -> Option<(u64, bool)> {
        match self.kind {
            ValueKind::ByteSlice => Some((self.payload, false)),
            ValueKind::ByteSliceMut => Some((self.payload, true)),
            _ => None,
        }
    }
}
