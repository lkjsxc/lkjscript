//! Runtime-only dense values for normalized Graph 5 execution.

use super::resource::NormalizedResourceHandle;
use crate::platform::kernel::Name;
use std::collections::BTreeMap;
use std::sync::Arc;

macro_rules! dense_index {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub u32);
    };
}

dense_index!(FunctionIndex);
dense_index!(RecordLayoutIndex);
dense_index!(VariantLayoutIndex);
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
    List(Arc<Vec<NormalizedValue>>),
    Map(Arc<BTreeMap<NormalizedMapKey, NormalizedValue>>),
    Function(FunctionIndex),
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
            Self::Function(_) | Self::Resource(_) => false,
            Self::Record(NormalizedRecord::Nominal { fields, .. }) => {
                fields.iter().all(Self::is_durable)
            }
            Self::Record(NormalizedRecord::Structural { fields }) => {
                fields.iter().all(|(_, value)| value.is_durable())
            }
            Self::Variant { payload, .. } => {
                payload.as_ref().is_none_or(|payload| payload.is_durable())
            }
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
            | NormalizedValue::List(_)
            | NormalizedValue::Map(_)
            | NormalizedValue::Function(_)
            | NormalizedValue::Resource(_) => None,
        }
    }
}
