//! Runtime-only dense values for normalized Graph 14 execution.

use super::resource::NormalizedResourceHandle;
use crate::platform::kernel::{Name, TypeObjectDigest};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

// Current finite value/admission representation, independent of invocation-lifetime quotas.
pub(crate) const MAXIMUM_VALUE_ALLOCATION_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const MAXIMUM_ADMISSION_ITEMS: u64 = 1_000_000;

/// Actual storage arithmetic is checked even when cumulative allocation has no quota.
pub(super) fn collection_storage_bytes(
    items: u64,
    item_bytes: u64,
    code: &'static str,
) -> Result<u64, crate::platform::execution::ExecutionError> {
    items.checked_mul(item_bytes).ok_or_else(|| {
        crate::platform::execution::ExecutionError::resource(
            code,
            "collection allocation size overflowed",
        )
    })
}

/// Neutral process-local origin for raw dense identities, never an affine certificate.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ValueOrigin(u64);

/// Observation units only; these counters do not grant admission or add execution fuel.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize)]
pub(crate) struct ValueWork {
    pub input_admission_nodes: u64,
    pub raw_result_admission_nodes: u64,
    pub capture_admission_nodes: u64,
    pub constructor_child_visits: u64,
    pub internal_guard_descendant_visits: u64,
    pub classification_decisions: u64,
    pub lists: super::list::Work,
}

impl ValueOrigin {
    pub(super) fn fresh() -> Option<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .ok()
        .map(Self)
    }
}

macro_rules! dense_index {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub u32);
    };
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FunctionIndex(pub u32, pub ValueOrigin);
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RecordLayoutIndex(pub u32, pub ValueOrigin);
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VariantLayoutIndex(pub u32, pub ValueOrigin);
dense_index!(RequirementIndex);
dense_index!(OperationIndex);
dense_index!(ComponentIndex);
dense_index!(PortIndex);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizedRecord {
    Nominal {
        layout: RecordLayoutIndex,
        fields: Arc<Vec<NormalizedValue>>,
    },
    Structural {
        fields: Arc<Vec<(Name, NormalizedValue)>>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizedValue {
    Unit,
    Bool(bool),
    I64(i64),
    Bytes(Arc<[u8]>),
    Text(Arc<str>),
    StaticText(Arc<str>),
    Record(NormalizedRecord),
    Variant {
        layout: VariantLayoutIndex,
        case: u32,
        payload: Option<Box<NormalizedValue>>,
    },
    Option(Option<Box<NormalizedValue>>),
    // Runtime ingress can carry this existing type; no external result codec is added.
    #[allow(dead_code)]
    Result {
        success: bool,
        value: Box<NormalizedValue>,
    },
    List(super::list::List),
    Map(Arc<BTreeMap<NormalizedMapKey, NormalizedValue>>),
    Function {
        function: FunctionIndex,
        type_arguments: Arc<[TypeObjectDigest]>,
        effect_arguments: Arc<[crate::platform::kernel::EffectRow]>,
        bound_arguments: Option<Arc<Vec<NormalizedValue>>>,
    },
    Resource(NormalizedResourceHandle),
}

impl NormalizedValue {
    /// Bounded raw boundary construction. Evaluators separately admit all logical
    /// occurrences and metadata; this constructor confers no semantic eligibility.
    pub fn list(items: Vec<Self>) -> Result<Self, crate::platform::execution::ExecutionError> {
        let mut bytes = 0_u64;
        super::list::List::from_items(items, super::list::MAXIMUM_LENGTH as u64, &mut |charge| {
            bytes = bytes
                .checked_add(charge.bytes)
                .filter(|n| *n <= 268_435_456)
                .ok_or_else(|| {
                    crate::platform::execution::ExecutionError::new(
                        crate::platform::execution::ExecutionFailureClass::Resource,
                        "normalized_list_storage",
                        "raw list construction exceeds its owned storage bound",
                    )
                })?;
            Ok(())
        })
        .map(Self::List)
    }

    pub fn text(value: impl Into<Arc<str>>) -> Self {
        Self::Text(value.into())
    }

    #[cfg(test)]
    pub fn static_text(value: impl Into<Arc<str>>) -> Self {
        Self::StaticText(value.into())
    }

    pub fn bytes(value: impl Into<Arc<[u8]>>) -> Self {
        Self::Bytes(value.into())
    }

    #[cfg(test)]
    pub fn is_durable(&self) -> bool {
        match self {
            Self::Function { .. } | Self::Resource(_) => false,
            Self::Record(NormalizedRecord::Nominal { fields, .. }) => {
                fields.iter().all(Self::is_durable)
            }
            Self::Record(NormalizedRecord::Structural { fields }) => {
                fields.iter().all(|(_, value)| value.is_durable())
            }
            Self::Variant { payload, .. } => {
                payload.as_ref().is_none_or(|payload| payload.is_durable())
            }
            Self::Option(value) => value.as_ref().is_none_or(|value| value.is_durable()),
            Self::Result { value, .. } => value.is_durable(),
            Self::List(items) => items.iter().all(Self::is_durable),
            Self::Map(entries) => entries.values().all(Self::is_durable),
            Self::Unit
            | Self::Bool(_)
            | Self::I64(_)
            | Self::Bytes(_)
            | Self::Text(_)
            | Self::StaticText(_) => true,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NormalizedMapKey {
    Bool(bool),
    I64(i64),
    Bytes(Vec<u8>),
    Text(String),
}

impl NormalizedMapKey {
    pub fn from_value(value: NormalizedValue) -> Option<Self> {
        match value {
            NormalizedValue::Bool(value) => Some(Self::Bool(value)),
            NormalizedValue::I64(value) => Some(Self::I64(value)),
            NormalizedValue::Bytes(value) => Some(Self::Bytes(value.to_vec())),
            NormalizedValue::Text(value) | NormalizedValue::StaticText(value) => {
                Some(Self::Text(value.to_string()))
            }
            NormalizedValue::Unit
            | NormalizedValue::Record(_)
            | NormalizedValue::Variant { .. }
            | NormalizedValue::Option(_)
            | NormalizedValue::Result { .. }
            | NormalizedValue::List(_)
            | NormalizedValue::Map(_)
            | NormalizedValue::Function { .. }
            | NormalizedValue::Resource(_) => None,
        }
    }

    pub(crate) fn to_value(&self) -> NormalizedValue {
        match self {
            Self::Bool(value) => NormalizedValue::Bool(*value),
            Self::I64(value) => NormalizedValue::I64(*value),
            Self::Bytes(value) => NormalizedValue::bytes(value.clone()),
            Self::Text(value) => NormalizedValue::text(value.clone()),
        }
    }
}

/// Own unadmitted arguments until each value crosses its evaluator's boundary. Rejection
/// destroys even excessively deep raw data iteratively instead of recursing in Rust drop.
pub(super) struct RawArguments(Vec<NormalizedValue>);

impl RawArguments {
    pub(super) fn new(mut values: Vec<NormalizedValue>) -> Self {
        values.reverse();
        Self(values)
    }
    pub(super) fn into_vec(mut self) -> Vec<NormalizedValue> {
        let mut values = std::mem::take(&mut self.0);
        values.reverse();
        values
    }
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub(super) fn len(&self) -> usize {
        self.0.len()
    }
}
impl Iterator for RawArguments {
    type Item = NormalizedValue;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}
impl Drop for RawArguments {
    fn drop(&mut self) {
        release_raw_values(std::mem::take(&mut self.0));
    }
}

pub(super) fn release_raw_values(mut values: Vec<NormalizedValue>) {
    while let Some(value) = values.pop() {
        match value {
            NormalizedValue::Option(Some(value))
            | NormalizedValue::Result { value, .. }
            | NormalizedValue::Variant {
                payload: Some(value),
                ..
            } => values.push(*value),
            NormalizedValue::Function {
                bound_arguments: Some(children),
                ..
            }
            | NormalizedValue::Record(NormalizedRecord::Nominal {
                fields: children, ..
            }) => {
                if let Some(mut children) = Arc::into_inner(children) {
                    if values.is_empty() {
                        values = children;
                    } else {
                        values.append(&mut children);
                    }
                }
            }
            NormalizedValue::List(mut list) => list.drain_unique(&mut values),
            NormalizedValue::Record(NormalizedRecord::Structural { fields }) => {
                if let Some(fields) = Arc::into_inner(fields) {
                    values.extend(fields.into_iter().map(|(_, value)| value));
                }
            }
            NormalizedValue::Map(entries) => {
                if let Some(entries) = Arc::into_inner(entries) {
                    values.extend(entries.into_values());
                }
            }
            _ => {}
        }
    }
}
