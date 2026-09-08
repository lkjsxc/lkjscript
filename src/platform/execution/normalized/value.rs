//! Runtime-only dense values for normalized Graph 10 execution.

use super::resource::NormalizedResourceHandle;
use crate::platform::kernel::{Name, TypeObjectDigest};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Neutral process-local origin for raw dense identities, never an affine certificate.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ValueOrigin(u64);

/// Observation units only; these counters do not grant admission or add execution fuel.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize)]
pub(crate) struct ValueWork {
    pub input_admission_nodes: u64,
    pub raw_result_admission_nodes: u64,
    pub constructor_child_visits: u64,
    pub internal_guard_descendant_visits: u64,
    pub classification_decisions: u64,
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
    List(Arc<Vec<NormalizedValue>>),
    Map(Arc<BTreeMap<NormalizedMapKey, NormalizedValue>>),
    Function {
        function: FunctionIndex,
        type_arguments: Arc<[TypeObjectDigest]>,
    },
    Resource(NormalizedResourceHandle),
}

impl NormalizedValue {
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
            | NormalizedValue::Variant {
                payload: Some(value),
                ..
            } => values.push(*value),
            NormalizedValue::List(children)
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
