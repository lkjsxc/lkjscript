impl OwnedValue {
    pub fn is_unit(&self) -> bool {
        if let Some(value) = self.as_structural() {
            return matches!(
                value.payload,
                SemanticPayload::Inline(InlineStructuralValue::Unit)
            );
        }
        if let Some(value) = self.as_semantic_dag() {
            return matches!(
                value.root_node().payload,
                crate::SemanticDagPayload::Inline(InlineStructuralValue::Unit)
            );
        }
        self.unique_byte_vector.is_none() && self.unique_bytes.is_none() && self.root.is_unit()
    }

    pub fn is_empty_list(&self) -> bool {
        if let Some(value) = self.as_semantic_dag() {
            return matches!(value.root_node().payload, crate::SemanticDagPayload::EmptyList);
        }
        self.structural.is_none() && self.root.is_empty_list()
    }

    pub fn list_len(&self) -> Option<usize> {
        if let Some(value) = self.as_semantic_dag() {
            return semantic_dag_list_len(value);
        }
        if self.root.is_empty_list() {
            return Some(0);
        }
        let mut value = self.root;
        let mut length = 0_usize;
        while let Some(index) = value.as_owned_list() {
            let index = usize::try_from(index).ok()?;
            let node = self.lists.get(index)?;
            length = length.checked_add(1)?;
            value = node.tail;
        }
        value.is_empty_list().then_some(length)
    }

    pub fn list_i64(&self, requested: usize) -> Option<i64> {
        if let Some(value) = self.as_semantic_dag() {
            return semantic_dag_list_i64(value, requested);
        }
        let mut value = self.root;
        let mut index = 0_usize;
        while let Some(node_index) = value.as_owned_list() {
            let node_index = usize::try_from(node_index).ok()?;
            let node = self.lists.get(node_index)?;
            if index == requested {
                return node.head.as_i64();
            }
            index = index.checked_add(1)?;
            value = node.tail;
        }
        None
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Some(value) = self.as_structural() {
            return match &value.payload {
                SemanticPayload::Inline(InlineStructuralValue::Bool(value)) => Some(*value),
                _ => None,
            };
        }
        if let Some(value) = self.as_semantic_dag() {
            return match &value.root_node().payload {
                crate::SemanticDagPayload::Inline(InlineStructuralValue::Bool(value)) => {
                    Some(*value)
                }
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
        if let Some(value) = self.as_semantic_dag() {
            return match &value.root_node().payload {
                crate::SemanticDagPayload::Inline(InlineStructuralValue::I64(value)) => {
                    Some(*value)
                }
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
        if let Some(value) = self.as_semantic_dag() {
            return match &value.root_node().payload {
                crate::SemanticDagPayload::Inline(InlineStructuralValue::F64Bits(value)) => {
                    Some(*value)
                }
                _ => None,
            };
        }
        self.root.as_f64_bits()
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Some(value) = self.as_structural() {
            return match &value.payload {
                SemanticPayload::String(bytes) => std::str::from_utf8(bytes).ok(),
                SemanticPayload::Static(crate::StaticStructuralLeaf::Symbol(index)) => self
                    .symbols
                    .get(usize::try_from(*index).ok()?)?
                    .as_deref(),
                _ => None,
            };
        }
        if let Some(value) = self.as_semantic_dag() {
            return match &value.root_node().payload {
                crate::SemanticDagPayload::String(bytes) => std::str::from_utf8(bytes).ok(),
                crate::SemanticDagPayload::Static(crate::StaticStructuralLeaf::Symbol(index)) => {
                    self.symbols
                        .get(usize::try_from(*index).ok()?)?
                        .as_deref()
                }
                _ => None,
            };
        }
        if let Some(index) = self.root.as_symbol() {
            return self
                .symbols
                .get(usize::try_from(index).ok()?)?
                .as_deref();
        }
        None
    }

    pub fn as_path_bytes(&self) -> Option<&[u8]> {
        if let Some(value) = self.as_structural() {
            return value.path_bytes();
        }
        match &self.as_semantic_dag()?.root_node().payload {
            crate::SemanticDagPayload::Path(bytes) => Some(bytes),
            _ => None,
        }
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
        if let Some(value) = self.as_semantic_dag() {
            return match &value.root_node().payload {
                crate::SemanticDagPayload::Bytes(bytes) => Some(bytes),
                _ => None,
            };
        }
        self.unique_bytes.as_deref()
    }

    pub fn as_resource(&self) -> Option<u64> {
        self.structural
            .is_none()
            .then(|| self.root.as_resource())
            .flatten()
    }
}
