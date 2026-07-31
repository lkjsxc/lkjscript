impl OwnedValue {
    pub fn is_unit(&self) -> bool {
        if let Some(value) = self.as_structural() {
            return matches!(
                value.payload,
                SemanticPayload::Inline(InlineStructuralValue::Unit)
            );
        }
        self.unique_byte_vector.is_none() && self.unique_bytes.is_none() && self.root.is_unit()
    }

    pub fn is_empty_list(&self) -> bool {
        self.structural.is_none() && self.root.is_empty_list()
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Some(value) = self.as_structural() {
            return match &value.payload {
                SemanticPayload::Inline(InlineStructuralValue::Bool(value)) => Some(*value),
                _ => None,
            };
        }
        self.root.as_bool()
    }

    pub fn as_i64(&self) -> Option<i64> {
        if let Some(value) = self.as_structural() {
            return match &value.payload {
                SemanticPayload::Inline(InlineStructuralValue::I64(value)) => Some(*value),
                _ => None,
            };
        }
        self.root.as_i64()
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.as_f64_bits().map(f64::from_bits)
    }

    pub fn as_f64_bits(&self) -> Option<u64> {
        if let Some(value) = self.as_structural() {
            return match &value.payload {
                SemanticPayload::Inline(InlineStructuralValue::F64Bits(value)) => Some(*value),
                _ => None,
            };
        }
        self.root.as_f64_bits()
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Some(value) = self.as_structural() {
            return match &value.payload {
                SemanticPayload::String(bytes) => std::str::from_utf8(bytes).ok(),
                SemanticPayload::Static(crate::StaticStructuralLeaf::Symbol(index)) => {
                    self.symbols.get(*index as usize)?.as_deref()
                }
                _ => None,
            };
        }
        if let Some(index) = self.root.as_symbol() {
            return self.symbols.get(index as usize)?.as_deref();
        }
        None
    }

    pub fn as_path_bytes(&self) -> Option<&[u8]> {
        self.as_structural().and_then(SemanticValue::path_bytes)
    }

    pub fn as_byte_vector(&self) -> Option<&[u8]> {
        if let Some(value) = self.as_structural() {
            return match &value.payload {
                SemanticPayload::ByteVector(bytes) => Some(bytes),
                _ => None,
            };
        }
        self.unique_byte_vector.as_deref()
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        if let Some(value) = self.as_structural() {
            return match &value.payload {
                SemanticPayload::Bytes(bytes) => Some(bytes),
                _ => None,
            };
        }
        self.unique_bytes.as_deref()
    }

    pub fn as_resource(&self) -> Option<u32> {
        self.structural
            .is_none()
            .then(|| self.root.as_resource())
            .flatten()
    }

    pub fn as_function(&self) -> Option<u32> {
        if let Some(value) = self.as_structural() {
            return match &value.payload {
                SemanticPayload::Static(crate::StaticStructuralLeaf::Function(function)) => {
                    Some(*function)
                }
                _ => None,
            };
        }
        self.root.as_function()
    }
}
