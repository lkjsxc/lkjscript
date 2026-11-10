//! Runtime-only dense values for normalized Graph 14 execution.

use super::bytes::BytePayload;
use super::child_storage::{Captures, Fields, Values};
use super::resource::NormalizedResourceHandle;
use crate::platform::binary64::Binary64;
use crate::platform::kernel::TypeObjectDigest;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "value_view_admission.rs"]
mod view_admission;

pub(super) use view_admission::reject_read_view_type_escape;

#[cfg(test)]
#[path = "map_key_sharing_tests.rs"]
mod map_key_sharing_tests;

// Remaining map, schema, owned-storage, and view admission paths use this bound.
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

#[derive(Clone)]
pub enum NormalizedRecord {
    Nominal {
        layout: RecordLayoutIndex,
        fields: Values,
    },
    Structural {
        fields: Fields,
    },
}

#[derive(Clone)]
pub enum NormalizedValue {
    RegionCap {
        ty: TypeObjectDigest,
        token: crate::platform::execution::region::runtime::Capability<ValueOrigin>,
    },
    RegionPointer {
        ty: TypeObjectDigest,
        pointer: crate::platform::execution::region::runtime::Pointer,
    },
    /// Affine descriptor custody. Cloning this raw carrier yields an inert observation;
    /// independently admitted storage reborrowing is required to obtain another live read.
    ReadView(super::read_view_storage::ReadView),
    OwnedSequence(super::owned_sequence::OwnedSequence),
    OwnedChoice(super::owned_choice::OwnedChoice),
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
        read_scope_arguments: Arc<[crate::platform::kernel::ReadScopeArgument]>,
        bound_arguments: Option<Captures>,
    },
    Resource(NormalizedResourceHandle),
}

// Debug is passive presentation, not a serializer or a semantic admission walk.
// Every composite prints only its own fixed-size metadata and child counts.
impl std::fmt::Debug for NormalizedRecord {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nominal { layout, fields } => formatter
                .debug_struct("Nominal")
                .field("layout", layout)
                .field("fields", fields)
                .finish(),
            Self::Structural { fields } => formatter
                .debug_struct("Structural")
                .field("fields", fields)
                .finish(),
        }
    }
}

impl std::fmt::Debug for NormalizedValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => formatter.write_str("Unit"),
            Self::Bool(value) => formatter.debug_tuple("Bool").field(value).finish(),
            Self::I64(value) => formatter.debug_tuple("I64").field(value).finish(),
            Self::F64(value) => formatter.debug_tuple("F64").field(value).finish(),
            Self::Bytes(value) => formatter
                .debug_struct("Bytes")
                .field("len", &value.len())
                .finish(),
            Self::Text(value) => formatter
                .debug_struct("Text")
                .field("bytes", &value.len())
                .finish(),
            Self::StaticText(value) => formatter
                .debug_struct("StaticText")
                .field("bytes", &value.len())
                .finish(),
            Self::Record(value) => formatter.debug_tuple("Record").field(value).finish(),
            Self::Variant {
                layout,
                case,
                payload,
            } => formatter
                .debug_struct("Variant")
                .field("layout", layout)
                .field("case", case)
                .field("has_payload", &payload.is_some())
                .finish(),
            Self::Option(value) => formatter
                .debug_struct("Option")
                .field("is_some", &value.is_some())
                .finish(),
            Self::Result { success, .. } => formatter
                .debug_struct("Result")
                .field("success", success)
                .finish(),
            Self::List(value) => formatter.debug_tuple("List").field(value).finish(),
            Self::Map(value) => formatter.debug_tuple("Map").field(value).finish(),
            Self::Function {
                function,
                type_arguments,
                effect_arguments,
                requirement_arguments,
                read_scope_arguments,
                bound_arguments,
            } => formatter
                .debug_struct("Function")
                .field("function", function)
                .field("type_arguments", &type_arguments.len())
                .field("effect_arguments", &effect_arguments.len())
                .field("requirement_arguments", &requirement_arguments.len())
                .field("read_scope_arguments", &read_scope_arguments.len())
                .field("bound_arguments", bound_arguments)
                .finish(),
            Self::RegionCap { ty, .. } => formatter
                .debug_struct("RegionCap")
                .field("ty", ty)
                .finish_non_exhaustive(),
            Self::RegionPointer { ty, .. } => formatter
                .debug_struct("RegionPointer")
                .field("ty", ty)
                .finish_non_exhaustive(),
            Self::ReadView(_) => formatter.write_str("ReadView(<opaque token>)"),
            Self::OwnedSequence(_) => formatter.write_str("OwnedSequence(<opaque token>)"),
            Self::OwnedChoice(_) => formatter.write_str("OwnedChoice(<opaque token>)"),
            Self::OwnedProduct(_) => formatter.write_str("OwnedProduct(<opaque token>)"),
            Self::ByteBuffer(_) => formatter.write_str("ByteBuffer(<opaque token>)"),
            Self::OwnedI64Cell(_) => formatter.write_str("OwnedI64Cell(<opaque token>)"),
            Self::Resource(_) => formatter.write_str("Resource(<opaque handle>)"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MemoryForm {
    ByteBuffer,
    OwnedI64Cell,
    Product(TypeObjectDigest),
    Choice(TypeObjectDigest),
    Sequence(TypeObjectDigest),
}

impl NormalizedValue {
    /// Negative boundary guard only. Opaque owner tokens and exact types still require
    /// their consuming evaluator's independent admission before any value is used.
    pub fn reject_read_view_escape(
        &self,
        control: &crate::platform::execution::ExecutionControl,
    ) -> Result<(), crate::platform::execution::ExecutionError> {
        view_admission::reject_read_view_value_escape(self, control)
    }

    pub(super) fn memory_form(&self) -> Option<MemoryForm> {
        match self {
            Self::ByteBuffer(_) => Some(MemoryForm::ByteBuffer),
            Self::OwnedI64Cell(_) => Some(MemoryForm::OwnedI64Cell),
            Self::OwnedProduct(p) => Some(MemoryForm::Product(p.ty())),
            Self::OwnedChoice(p) => Some(MemoryForm::Choice(p.ty())),
            Self::OwnedSequence(p) => Some(MemoryForm::Sequence(p.ty())),
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
            Self::OwnedChoice(token) => token.validate(domain, consume),
            Self::OwnedSequence(token) => token.validate(domain, consume),
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
            Self::OwnedChoice(token) => token.borrow().map(Self::OwnedChoice),
            Self::OwnedSequence(token) => token.borrow().map(Self::OwnedSequence),
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
            Self::OwnedChoice(token) => token.is_borrowed(),
            Self::OwnedSequence(token) => token.is_borrowed(),
            _ => false,
        }
    }

    pub(super) fn memory_owns_live_loans(&self) -> bool {
        match self {
            Self::ByteBuffer(token) => token.owns_live_loans(),
            Self::OwnedI64Cell(token) => token.owns_live_loans(),
            Self::OwnedProduct(token) => token.owns_live_loans(),
            Self::OwnedChoice(token) => token.owns_live_loans(),
            Self::OwnedSequence(token) => token.owns_live_loans(),
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
            bytes = bytes.checked_add(charge.bytes).ok_or_else(|| {
                crate::platform::execution::ExecutionError::resource(
                    "normalized_map_storage",
                    "raw map construction storage accounting overflowed",
                )
            })?;
            Ok(())
        })
        .map(Self::Map)
    }

    /// Raw boundary construction. Evaluators separately admit all logical
    /// occurrences and metadata; this constructor confers no semantic eligibility.
    pub fn list(items: Vec<Self>) -> Result<Self, crate::platform::execution::ExecutionError> {
        super::list::List::from_items(items, None, &mut |_| Ok(())).map(Self::List)
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
            Self::ReadView(_)
            | Self::RegionCap { .. }
            | Self::RegionPointer { .. }
            | Self::OwnedProduct(_)
            | Self::OwnedChoice(_)
            | Self::OwnedSequence(_)
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
        bytes = bytes.checked_add(charge.bytes).ok_or_else(|| {
            crate::platform::execution::ExecutionError::resource(
                "normalized_map_storage",
                "raw map operation storage accounting overflowed",
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
    pub(super) fn as_raw(&self) -> &NormalizedValue {
        &self.0
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

/// Preserve authored order while owning unadmitted fields across failure paths.
pub(super) struct RawValues(Vec<NormalizedValue>);

impl RawValues {
    pub(super) fn new(values: Vec<NormalizedValue>) -> Self {
        Self(values)
    }

    pub(super) fn as_slice(&self) -> &[NormalizedValue] {
        &self.0
    }

    pub(super) fn into_vec(mut self) -> Vec<NormalizedValue> {
        std::mem::take(&mut self.0)
    }
}

impl Drop for RawValues {
    fn drop(&mut self) {
        release_raw_values(std::mem::take(&mut self.0));
    }
}

pub(super) fn release_raw_value(value: NormalizedValue) {
    super::child_storage::release_root(value);
}

pub(super) fn release_raw_values(values: Vec<NormalizedValue>) {
    super::child_storage::release_roots(values);
}
