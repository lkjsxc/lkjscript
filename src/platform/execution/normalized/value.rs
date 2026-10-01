//! Runtime-only dense values for normalized Graph 14 execution.

use super::bytes::BytePayload;
use super::resource::NormalizedResourceHandle;
use crate::platform::binary64::Binary64;
use crate::platform::kernel::{Name, TypeObjectDigest};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(test)]
#[path = "map_key_sharing_tests.rs"]
mod map_key_sharing_tests;

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
    /// Successful ordinary local reads, not allocation or byte-copy estimates.
    pub local_value_moves: u64,
    pub local_value_copies: u64,
    pub lists: super::list::Work,
    pub maps: super::map::Work,
    pub bytes: super::bytes::Work,
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
    OwnedProduct(super::owned_product::OwnedProduct),
    Unit,
    Bool(bool),
    I64(i64),
    F64(Binary64),
    ByteBuffer(super::byte_buffer::ByteBuffer),
    OwnedI64Cell(super::owned_i64_cell::OwnedI64Cell),
    Bytes(BytePayload),
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
    Map(super::map::Map),
    Function {
        function: FunctionIndex,
        type_arguments: Arc<[TypeObjectDigest]>,
        effect_arguments: Arc<[crate::platform::kernel::EffectRow]>,
        requirement_arguments: Arc<[crate::platform::kernel::RequirementOperand]>,
        bound_arguments: Option<Arc<Vec<NormalizedValue>>>,
    },
    Resource(NormalizedResourceHandle),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MemoryForm {
    ByteBuffer,
    OwnedI64Cell,
    Product(TypeObjectDigest),
}

impl NormalizedValue {
    pub(super) fn memory_form(&self) -> Option<MemoryForm> {
        match self {
            Self::ByteBuffer(_) => Some(MemoryForm::ByteBuffer),
            Self::OwnedI64Cell(_) => Some(MemoryForm::OwnedI64Cell),
            Self::OwnedProduct(p) => Some(MemoryForm::Product(p.ty())),
            _ => None,
        }
    }

    pub(super) fn memory_validate(
        &self,
        domain: ValueOrigin,
        consume: bool,
    ) -> Result<(), crate::platform::execution::ExecutionError> {
        match self {
            Self::ByteBuffer(token) => token.validate(domain, consume),
            Self::OwnedI64Cell(token) => token.validate(domain, consume),
            Self::OwnedProduct(token) => token.validate(domain, consume),
            _ => Err(crate::platform::execution::ExecutionError::resource(
                "normalized_memory_token",
                "expected a sealed owned-memory token",
            )),
        }
    }

    pub(super) fn memory_borrow(&self) -> Result<Self, crate::platform::execution::ExecutionError> {
        match self {
            Self::ByteBuffer(token) => token.borrow().map(Self::ByteBuffer),
            Self::OwnedI64Cell(token) => token.borrow().map(Self::OwnedI64Cell),
            Self::OwnedProduct(token) => token.borrow().map(Self::OwnedProduct),
            _ => Err(crate::platform::execution::ExecutionError::resource(
                "normalized_memory_token",
                "expected a sealed owned-memory token",
            )),
        }
    }

    pub(super) fn memory_is_borrowed(&self) -> bool {
        match self {
            Self::ByteBuffer(token) => token.is_borrowed(),
            Self::OwnedI64Cell(token) => token.is_borrowed(),
            Self::OwnedProduct(token) => token.is_borrowed(),
            _ => false,
        }
    }

    pub(super) fn memory_owns_live_loans(&self) -> bool {
        match self {
            Self::ByteBuffer(token) => token.owns_live_loans(),
            Self::OwnedI64Cell(token) => token.owns_live_loans(),
            Self::OwnedProduct(token) => token.owns_live_loans(),
            _ => false,
        }
    }

    /// Bounded raw ingress only. The carrier conveys no type or origin certificate;
    /// each evaluator admits every logical child independently before using it.
    pub fn map(
        entries: BTreeMap<NormalizedMapKey, Self>,
    ) -> Result<Self, crate::platform::execution::ExecutionError> {
        Self::map_controlled(
            entries,
            &crate::platform::execution::ExecutionControl::default(),
        )
    }

    pub(crate) fn map_controlled(
        entries: BTreeMap<NormalizedMapKey, Self>,
        control: &crate::platform::execution::ExecutionControl,
    ) -> Result<Self, crate::platform::execution::ExecutionError> {
        let mut bytes = 0_u64;
        super::map::Map::from_items(entries, MAXIMUM_ADMISSION_ITEMS, &mut |charge| {
            control.check()?;
            bytes = bytes
                .checked_add(charge.bytes)
                .filter(|bytes| *bytes <= MAXIMUM_VALUE_ALLOCATION_BYTES)
                .ok_or_else(|| {
                    crate::platform::execution::ExecutionError::resource(
                        "normalized_map_storage",
                        "raw map construction exceeds its owned storage bound",
                    )
                })?;
            Ok(())
        })
        .map(Self::Map)
    }

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
        Self::Bytes(BytePayload::from(value.into()))
    }

    #[cfg(test)]
    pub fn is_durable(&self) -> bool {
        match self {
            Self::OwnedProduct(_)
            | Self::OwnedI64Cell(_)
            | Self::ByteBuffer(_)
            | Self::Function { .. }
            | Self::Resource(_) => false,
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
            | Self::F64(_)
            | Self::Bytes(_)
            | Self::Text(_)
            | Self::StaticText(_) => true,
        }
    }
}

/// Primitive keys share the same immutable payload as ordinary values. Ordering
/// and equality compare contents, never allocation identity. Sharing is storage
/// only: every logical raw-input occurrence still crosses admission independently.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NormalizedMapKey {
    Bool(bool),
    I64(i64),
    Bytes(BytePayload),
    Text(Arc<str>),
}

impl NormalizedMapKey {
    pub fn from_value(value: NormalizedValue) -> Option<Self> {
        match value {
            NormalizedValue::Bool(value) => Some(Self::Bool(value)),
            NormalizedValue::I64(value) => Some(Self::I64(value)),
            NormalizedValue::Bytes(value) => Some(Self::Bytes(value)),
            NormalizedValue::Text(value) | NormalizedValue::StaticText(value) => {
                Some(Self::Text(value))
            }
            value => {
                release_raw_value(value);
                None
            }
        }
    }

    pub(crate) fn to_value(&self) -> NormalizedValue {
        match self {
            Self::Bool(value) => NormalizedValue::Bool(*value),
            Self::I64(value) => NormalizedValue::I64(*value),
            Self::Bytes(value) => NormalizedValue::Bytes(value.clone()),
            Self::Text(value) => NormalizedValue::Text(Arc::clone(value)),
        }
    }
}

/// Cloning a projected payload shares Arc-backed aggregates. Only consecutive
/// owned option/result/variant boxes allocate; their descendants stop at an Arc.
pub(super) fn projected_value_bytes(
    value: &NormalizedValue,
) -> Result<u64, crate::platform::execution::ExecutionError> {
    let mut value = value;
    let mut bytes = 0_u64;
    let mut depth = 0_u16;
    loop {
        value = match value {
            NormalizedValue::Option(Some(child))
            | NormalizedValue::Result { value: child, .. }
            | NormalizedValue::Variant {
                payload: Some(child),
                ..
            } => child,
            _ => return Ok(bytes),
        };
        depth += 1;
        if depth > 256 {
            return Err(crate::platform::execution::ExecutionError::resource(
                "normalized_value_depth",
                "map projection exceeds finite value depth 256",
            ));
        }
        bytes = bytes
            .checked_add(std::mem::size_of::<NormalizedValue>() as u64)
            .ok_or_else(|| {
                crate::platform::execution::ExecutionError::resource(
                    "normalized_map_storage",
                    "projected value allocation overflows",
                )
            })?;
    }
}

/// Bounded raw-host construction is separate from either evaluator's cumulative
/// ledger. Raw results still cross that evaluator's complete admission boundary.
pub(super) fn raw_map_reservation(
    control: &crate::platform::execution::ExecutionControl,
) -> impl FnMut(super::map::Charge) -> Result<(), crate::platform::execution::ExecutionError> + '_ {
    let mut bytes = 0_u64;
    move |charge| {
        control.check()?;
        bytes = bytes
            .checked_add(charge.bytes)
            .filter(|bytes| *bytes <= MAXIMUM_VALUE_ALLOCATION_BYTES)
            .ok_or_else(|| {
                crate::platform::execution::ExecutionError::resource(
                    "normalized_map_storage",
                    "raw map operation exceeds finite storage",
                )
            })?;
        Ok(())
    }
}

/// Own unadmitted arguments until each value crosses its evaluator's boundary. Rejection
/// destroys even excessively deep raw data iteratively instead of recursing in Rust drop.
pub(super) struct RawValue(NormalizedValue);

impl RawValue {
    pub(super) fn new(value: NormalizedValue) -> Self {
        Self(value)
    }
    pub(super) fn into_raw(mut self) -> NormalizedValue {
        std::mem::replace(&mut self.0, NormalizedValue::Unit)
    }
}

impl Drop for RawValue {
    fn drop(&mut self) {
        if !matches!(self.0, NormalizedValue::Unit) {
            release_raw_value(std::mem::replace(&mut self.0, NormalizedValue::Unit));
        }
    }
}

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

/// Nonfallible raw teardown owns this scratch, separately from execution fuel.
/// The first pending value is inline. Terminal values and single-child wrappers
/// need no heap storage; retained Arc owners are released without visiting their
/// shared contents. Owned argument/capture/record buffers are reused where possible.
/// Additional scratch holds children emitted by uniquely released containers;
/// several shared composite child handles can still occupy that frontier, without
/// traversing or cloning their shared contents. This established cleanup owner is
/// not a cumulative allocation ledger and cannot publish values after refusal.
#[derive(Default)]
pub(super) struct RawValueWork {
    current: Option<NormalizedValue>,
    values: Vec<NormalizedValue>,
}

impl RawValueWork {
    pub(super) fn push(&mut self, mut value: NormalizedValue) {
        loop {
            value = match value {
                NormalizedValue::Option(Some(value))
                | NormalizedValue::Result { value, .. }
                | NormalizedValue::Variant {
                    payload: Some(value),
                    ..
                } => *value,
                NormalizedValue::Unit
                | NormalizedValue::Bool(_)
                | NormalizedValue::I64(_)
                | NormalizedValue::F64(_)
                | NormalizedValue::Bytes(_)
                | NormalizedValue::Text(_)
                | NormalizedValue::StaticText(_)
                | NormalizedValue::Option(None)
                | NormalizedValue::Variant { payload: None, .. }
                | NormalizedValue::Function {
                    bound_arguments: None,
                    ..
                }
                | NormalizedValue::Resource(_)
                | NormalizedValue::ByteBuffer(_)
                | NormalizedValue::OwnedI64Cell(_)
                | NormalizedValue::OwnedProduct(_) => return,
                value => {
                    if let Some(previous) = self.current.replace(value) {
                        self.values.push(previous);
                    }
                    return;
                }
            };
        }
    }

    fn extend_owned(&mut self, mut children: Vec<NormalizedValue>) {
        if self.values.capacity().saturating_sub(self.values.len()) >= children.len() {
            self.values.append(&mut children);
        } else if children.capacity().saturating_sub(children.len()) >= self.values.len() {
            // Reuse the larger already-owned buffer while preserving release order.
            let previous = self.values.len();
            children.append(&mut self.values);
            children.rotate_right(previous);
            self.values = children;
        } else {
            self.values.append(&mut children);
        }
    }

    pub(super) fn release(&mut self) {
        while let Some(value) = self.current.take().or_else(|| self.values.pop()) {
            match value {
                NormalizedValue::Option(Some(value))
                | NormalizedValue::Result { value, .. }
                | NormalizedValue::Variant {
                    payload: Some(value),
                    ..
                } => self.push(*value),
                NormalizedValue::Function {
                    bound_arguments: Some(children),
                    ..
                }
                | NormalizedValue::Record(NormalizedRecord::Nominal {
                    fields: children, ..
                }) => {
                    if let Some(children) = Arc::into_inner(children) {
                        self.extend_owned(children);
                    }
                }
                NormalizedValue::List(mut list) => list.drain_unique(self),
                NormalizedValue::Record(NormalizedRecord::Structural { fields }) => {
                    if let Some(fields) = Arc::into_inner(fields) {
                        for (_, value) in fields {
                            self.push(value);
                        }
                    }
                }
                NormalizedValue::Map(mut entries) => entries.drain_unique(self),
                _ => {}
            }
        }
    }

    #[cfg(test)]
    pub(super) fn scratch_storage(&self) -> (*const NormalizedValue, usize) {
        (self.values.as_ptr(), self.values.capacity())
    }
}

impl Drop for RawValueWork {
    fn drop(&mut self) {
        self.release();
    }
}

pub(super) fn release_raw_value(value: NormalizedValue) {
    let mut work = RawValueWork::default();
    work.push(value);
    work.release();
}

pub(super) fn release_raw_values(values: Vec<NormalizedValue>) {
    let mut work = RawValueWork {
        current: None,
        values,
    };
    work.release();
}
