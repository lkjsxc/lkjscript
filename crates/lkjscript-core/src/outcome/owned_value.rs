use std::fmt;

use crate::{Error, Result, SemanticDagSnapshot, Value};

/// A returned value plus key-free structural and list boundary storage.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OwnedListNode {
    pub(crate) head: Value,
    pub(crate) tail: Value,
}

#[derive(Clone, PartialEq)]
pub struct OwnedValue {
    root: Value,
    lists: Vec<OwnedListNode>,
    unique_byte_vector: Option<Vec<u8>>,
    unique_bytes: Option<Vec<u8>>,
    symbols: Vec<Option<String>>,
    structural: Option<Box<OwnedStructuralValue>>,
    semantic_dag: Option<Box<SemanticDagSnapshot>>,
}

impl OwnedValue {
    pub const fn is_invalid(&self) -> bool {
        false
    }

    /// Transfers one collector-free byte-vector result across the execution
    /// boundary without retaining its runtime-local key or store.
    #[doc(hidden)]
    pub fn from_unique_byte_vector(bytes: Vec<u8>) -> Result<Self> {
        Ok(Self {
            root: Value::UNIT,
            lists: Vec::new(),
            unique_byte_vector: Some(bytes),
            unique_bytes: None,
            symbols: Vec::new(),
            structural: None,
            semantic_dag: None,
        })
    }

    #[doc(hidden)]
    pub fn from_unique_bytes(bytes: Vec<u8>) -> Result<Self> {
        Ok(Self {
            root: Value::UNIT,
            lists: Vec::new(),
            unique_byte_vector: None,
            unique_bytes: Some(bytes),
            symbols: Vec::new(),
            structural: None,
            semantic_dag: None,
        })
    }

    pub fn enum_physical_tag(&self) -> Option<u64> {
        if let Some(value) = self.as_structural() {
            return match &value.payload {
                SemanticPayload::Enum { tag, .. } => Some(*tag),
                _ => None,
            };
        }
        match &self.as_semantic_dag()?.root_node().payload {
            crate::SemanticDagPayload::Enum { tag, .. } => Some(*tag),
            _ => None,
        }
    }

    pub fn enum_payload_len(&self) -> Option<usize> {
        if let Some(value) = self.as_structural() {
            let SemanticPayload::Enum { active_payload, .. } = &value.payload else {
                return None;
            };
            return Some(active_payload.len());
        }
        match &self.as_semantic_dag()?.root_node().payload {
            crate::SemanticDagPayload::Enum { fields, .. } => Some(fields.len()),
            _ => None,
        }
    }

    pub fn enum_field_i64(&self, field: usize) -> Option<i64> {
        if let Some(value) = self.as_structural() {
            let SemanticPayload::Enum { active_payload, .. } = &value.payload else {
                return None;
            };
            return match &active_payload.get(field)?.payload {
                SemanticPayload::Inline(InlineStructuralValue::I64(value)) => Some(*value),
                _ => None,
            };
        }
        let dag = self.as_semantic_dag()?;
        let crate::SemanticDagPayload::Enum { fields, .. } = &dag.root_node().payload else {
            return None;
        };
        let node = dag.nodes().get(fields.get(field)?.get() as usize)?;
        match node.payload {
            crate::SemanticDagPayload::Inline(InlineStructuralValue::I64(value)) => Some(value),
            _ => None,
        }
    }

    /// Test/diagnostic inspection of retained reachable snapshot storage.
    #[doc(hidden)]
    pub fn snapshot_object_count(&self) -> usize {
        if let Some(snapshot) = self.as_semantic_dag() {
            return snapshot.nodes().len();
        }
        let Some(root) = self.as_structural() else {
            return self.lists.len();
        };
        let mut work = vec![root];
        let mut count = 0usize;
        while let Some(value) = work.pop() {
            count = count.saturating_add(1);
            match &value.payload {
                SemanticPayload::Product(fields)
                | SemanticPayload::Enum {
                    active_payload: fields,
                    ..
                } => work.extend(fields),
                _ => {}
            }
        }
        count
    }
}

include!("owned_value/snapshot.rs");
include!("owned_value/list_snapshot.rs");
include!("owned_value/structural.rs");
include!("owned_value/semantic_dag/value.rs");
include!("owned_value/structural_validation.rs");
include!("owned_value/views.rs");
include!("owned_value/views_semantic_dag.rs");
include!("owned_value/views_static.rs");
include!("owned_value/symbols.rs");
include!("owned_value/symbol_rewrite.rs");
include!("owned_value/wire.rs");
include!("owned_value/debug.rs");
