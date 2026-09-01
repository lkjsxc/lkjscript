//! Exact durable-queue capability adapter over the representation-neutral queue engine.

use super::capability::{NormalizedAdapterKind, NormalizedCallPolicy, NormalizedCapabilityAdapter};
use super::prepare::{NormalizedOperation, NormalizedProgram, NormalizedRequirement};
use super::resource::{NormalizedResourceHandle, NormalizedResourceScope, QueueLeaseInfo};
use super::value::{NormalizedRecord, NormalizedValue, RecordLayoutIndex, VariantLayoutIndex};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{
    DeclarationReference, ExternalVisibility, Name, OperationReference, ParameterUse, TypeForm,
    TypeObjectDigest,
};
use crate::platform::queue::{DurableQueueEngine, JobSnapshot};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const STANDARD_PACKAGE: &str = "pkg_10000000000000000000000000000001";
const QUEUE_INTERFACE: &str = "decl_20a0ef729beda0abf0e743cd7e1126de";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QueueOperation {
    Initialize,
    Enqueue,
    Claim,
    LeaseInfo,
    Heartbeat,
    Complete,
    Fail,
    Cancel,
    Inspect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LeaseInfoField {
    AttemptNumber,
    JobId,
    LeaseUntilMilliseconds,
    Payload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SnapshotField {
    AttemptCount,
    AvailableAtMilliseconds,
    JobId,
    LastErrorClass,
    LeaseUntilMilliseconds,
    Result,
    State,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RecordCodec<T> {
    layout: Option<RecordLayoutIndex>,
    fields: Arc<[(Name, T)]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LeaseStateCodec {
    layout: VariantLayoutIndex,
    absent_case: u32,
    live_case: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct NormalizedDurableQueueAdapter {
    kind: NormalizedAdapterKind,
    interface: DeclarationReference,
    operations: BTreeMap<OperationReference, QueueOperation>,
    exact_operations: BTreeSet<OperationReference>,
    lease_state: Option<LeaseStateCodec>,
    lease_info: Option<RecordCodec<LeaseInfoField>>,
    snapshot: Option<RecordCodec<SnapshotField>>,
    engine: DurableQueueEngine,
}

impl NormalizedDurableQueueAdapter {
    pub(crate) fn prepare(
        program: &NormalizedProgram,
        requirement: &NormalizedRequirement,
        kind: NormalizedAdapterKind,
        engine: DurableQueueEngine,
    ) -> Result<Self, Diagnostic> {
        if kind != NormalizedAdapterKind::DurableQueueData {
            return Err(queue_diagnostic(
                "normalized_queue_adapter_kind",
                "queue codec received a foreign adapter kind",
            ));
        }
        require_standard_interface(requirement.interface)?;
        let resource = Shape::Resource(requirement.interface);
        let mut operations = BTreeMap::new();
        let mut lease_state = None;
        let mut lease_info = None;
        let mut snapshot = None;
        for index in requirement.operations.iter().copied() {
            let operation = program.operations.get(index.0 as usize).ok_or_else(|| {
                queue_diagnostic(
                    "normalized_queue_operation_index",
                    "queue requirement operation escaped the artifact table",
                )
            })?;
            let kind = match operation.name.as_str() {
                "initialize" => {
                    signature(program, operation, &[], Shape::Unit)?;
                    QueueOperation::Initialize
                }
                "enqueue" => {
                    signature(
                        program,
                        operation,
                        &[
                            ordinary(Shape::Text),
                            ordinary(Shape::Text),
                            ordinary(Shape::Bytes),
                            ordinary(Shape::I64),
                            ordinary(Shape::I64),
                        ],
                        Shape::Bool,
                    )?;
                    QueueOperation::Enqueue
                }
                "claim" => {
                    signature(
                        program,
                        operation,
                        &[
                            ordinary(Shape::Text),
                            ordinary(Shape::I64),
                            ordinary(Shape::I64),
                        ],
                        Shape::NamedVariant,
                    )?;
                    merge_lease_state(
                        &mut lease_state,
                        LeaseStateCodec::prepare(program, operation.result, requirement.interface)?,
                    )?;
                    QueueOperation::Claim
                }
                "lease-info" => {
                    signature(
                        program,
                        operation,
                        &[(resource, ParameterUse::Borrow)],
                        Shape::NamedRecord,
                    )?;
                    merge_lease_info(
                        &mut lease_info,
                        RecordCodec::lease_info(program, operation.result)?,
                    )?;
                    QueueOperation::LeaseInfo
                }
                "heartbeat" => {
                    signature(
                        program,
                        operation,
                        &[
                            (resource, ParameterUse::Consume),
                            ordinary(Shape::I64),
                            ordinary(Shape::I64),
                        ],
                        Shape::NamedVariant,
                    )?;
                    merge_lease_state(
                        &mut lease_state,
                        LeaseStateCodec::prepare(program, operation.result, requirement.interface)?,
                    )?;
                    QueueOperation::Heartbeat
                }
                "complete" => {
                    signature(
                        program,
                        operation,
                        &[
                            (resource, ParameterUse::Consume),
                            ordinary(Shape::I64),
                            ordinary(Shape::Bytes),
                        ],
                        Shape::Bool,
                    )?;
                    QueueOperation::Complete
                }
                "fail" => {
                    signature(
                        program,
                        operation,
                        &[
                            (resource, ParameterUse::Consume),
                            ordinary(Shape::I64),
                            ordinary(Shape::Bool),
                            ordinary(Shape::I64),
                            ordinary(Shape::Text),
                        ],
                        Shape::Bool,
                    )?;
                    QueueOperation::Fail
                }
                "cancel" => {
                    signature(
                        program,
                        operation,
                        &[ordinary(Shape::Text), ordinary(Shape::I64)],
                        Shape::Bool,
                    )?;
                    QueueOperation::Cancel
                }
                "inspect" => {
                    signature(program, operation, &[ordinary(Shape::Text)], Shape::List)?;
                    snapshot = Some(RecordCodec::snapshot(
                        program,
                        list_item(program, operation.result)?,
                    )?);
                    QueueOperation::Inspect
                }
                _ => {
                    return Err(queue_diagnostic(
                        "normalized_queue_operation",
                        format!(
                            "durable queue adapter does not implement exact operation '{}'",
                            operation.name
                        ),
                    ));
                }
            };
            if operations.insert(operation.reference, kind).is_some() {
                return Err(queue_diagnostic(
                    "normalized_queue_operation_duplicate",
                    "queue requirement repeats an exact operation",
                ));
            }
        }
        let exact_operations = operations.keys().copied().collect();
        Ok(Self {
            kind,
            interface: requirement.interface,
            operations,
            exact_operations,
            lease_state,
            lease_info,
            snapshot,
            engine,
        })
    }

    pub(crate) fn preflight(&self) -> Result<(), ExecutionError> {
        self.engine.preflight()
    }

    fn operation(&self, policy: &NormalizedCallPolicy) -> Result<QueueOperation, ExecutionError> {
        if policy.grant.interface != self.interface {
            return Err(queue_runtime(
                "normalized_queue_interface",
                "queue call policy has a foreign exact interface",
            ));
        }
        self.operations
            .get(&policy.operation)
            .copied()
            .ok_or_else(|| {
                queue_runtime(
                    "normalized_queue_operation",
                    "queue call policy has a foreign exact operation",
                )
            })
    }

    fn lease_state(&self) -> Result<&LeaseStateCodec, ExecutionError> {
        self.lease_state.as_ref().ok_or_else(|| {
            queue_runtime(
                "normalized_queue_lease_state_codec",
                "queue operation has no prepared exact lease-state codec",
            )
        })
    }
}

impl NormalizedCapabilityAdapter for NormalizedDurableQueueAdapter {
    fn kind(&self) -> NormalizedAdapterKind {
        self.kind
    }

    fn interface(&self) -> DeclarationReference {
        self.interface
    }

    fn operations(&self) -> &BTreeSet<OperationReference> {
        &self.exact_operations
    }

    fn call(
        &self,
        policy: &NormalizedCallPolicy,
        arguments: Vec<NormalizedValue>,
        resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        let possible = policy.external_visibility == ExternalVisibility::Possible;
        match self.operation(policy)? {
            QueueOperation::Initialize => {
                if !arguments.is_empty() {
                    return Err(queue_argument("initialize expects no arguments"));
                }
                self.engine.initialize(control, possible)?;
                Ok(NormalizedValue::Unit)
            }
            QueueOperation::Enqueue => {
                let [
                    NormalizedValue::Text(job_id),
                    NormalizedValue::Text(idempotency_key),
                    NormalizedValue::Bytes(payload),
                    NormalizedValue::I64(available_at),
                    NormalizedValue::I64(created_at),
                ] = arguments.as_slice()
                else {
                    return Err(queue_argument(
                        "enqueue expects job id, idempotency key, payload, available time, and creation time",
                    ));
                };
                self.engine
                    .enqueue(
                        job_id,
                        idempotency_key,
                        payload,
                        *available_at,
                        *created_at,
                        control,
                        possible,
                    )
                    .map(NormalizedValue::Bool)
            }
            QueueOperation::Claim => {
                let [
                    NormalizedValue::Text(worker_id),
                    NormalizedValue::I64(now),
                    NormalizedValue::I64(lease_duration),
                ] = arguments.as_slice()
                else {
                    return Err(queue_argument(
                        "claim expects worker id, current time, and lease duration",
                    ));
                };
                let reservation =
                    resources.reserve_queue_lease(policy.requirement, self.interface)?;
                let lease =
                    self.engine
                        .claim(worker_id, *now, *lease_duration, control, possible)?;
                match lease {
                    Some(lease) => {
                        let handle = reservation.commit(lease)?;
                        Ok(self.lease_state()?.live(handle))
                    }
                    None => Ok(self.lease_state()?.absent()),
                }
            }
            QueueOperation::LeaseInfo => {
                let [NormalizedValue::Resource(handle)] = arguments.as_slice() else {
                    return Err(queue_argument("lease-info expects one queue lease"));
                };
                let info =
                    resources.borrow_queue_lease(policy.requirement, self.interface, *handle)?;
                self.lease_info
                    .as_ref()
                    .ok_or_else(|| {
                        queue_runtime(
                            "normalized_queue_lease_info_codec",
                            "lease-info has no prepared exact result codec",
                        )
                    })?
                    .encode_lease_info(info)
            }
            QueueOperation::Heartbeat => {
                let [
                    NormalizedValue::Resource(handle),
                    NormalizedValue::I64(now),
                    NormalizedValue::I64(lease_duration),
                ] = arguments.as_slice()
                else {
                    return Err(queue_argument(
                        "heartbeat expects a queue lease, current time, and lease duration",
                    ));
                };
                let reservation =
                    resources.reserve_queue_lease(policy.requirement, self.interface)?;
                let lease =
                    resources.consume_queue_lease(policy.requirement, self.interface, *handle)?;
                let renewed =
                    self.engine
                        .heartbeat_lease(lease, *now, *lease_duration, control, possible)?;
                match renewed {
                    Some(renewed) => {
                        let handle = reservation.commit(renewed)?;
                        Ok(self.lease_state()?.live(handle))
                    }
                    None => Ok(self.lease_state()?.absent()),
                }
            }
            QueueOperation::Complete => {
                let [
                    NormalizedValue::Resource(handle),
                    NormalizedValue::I64(now),
                    NormalizedValue::Bytes(result),
                ] = arguments.as_slice()
                else {
                    return Err(queue_argument(
                        "complete expects a queue lease, current time, and result",
                    ));
                };
                let lease =
                    resources.consume_queue_lease(policy.requirement, self.interface, *handle)?;
                self.engine
                    .complete_lease(lease, *now, result, control, possible)
                    .map(NormalizedValue::Bool)
            }
            QueueOperation::Fail => {
                let [
                    NormalizedValue::Resource(handle),
                    NormalizedValue::I64(now),
                    NormalizedValue::Bool(retry),
                    NormalizedValue::I64(retry_at),
                    NormalizedValue::Text(error_class),
                ] = arguments.as_slice()
                else {
                    return Err(queue_argument(
                        "fail expects a queue lease, current time, retry decision, retry time, and error class",
                    ));
                };
                let lease =
                    resources.consume_queue_lease(policy.requirement, self.interface, *handle)?;
                self.engine
                    .fail_lease(
                        lease,
                        *now,
                        *retry,
                        *retry_at,
                        error_class,
                        control,
                        possible,
                    )
                    .map(NormalizedValue::Bool)
            }
            QueueOperation::Cancel => {
                let [NormalizedValue::Text(job_id), NormalizedValue::I64(now)] =
                    arguments.as_slice()
                else {
                    return Err(queue_argument("cancel expects job id and current time"));
                };
                self.engine
                    .cancel(job_id, *now, control, possible)
                    .map(NormalizedValue::Bool)
            }
            QueueOperation::Inspect => {
                let [NormalizedValue::Text(job_id)] = arguments.as_slice() else {
                    return Err(queue_argument("inspect expects one job id"));
                };
                let snapshot = self.engine.inspect(job_id, control)?;
                let codec = self.snapshot.as_ref().ok_or_else(|| {
                    queue_runtime(
                        "normalized_queue_snapshot_codec",
                        "inspect has no prepared exact snapshot codec",
                    )
                })?;
                Ok(NormalizedValue::List(Arc::new(
                    snapshot
                        .into_iter()
                        .map(|snapshot| codec.encode_snapshot(snapshot))
                        .collect(),
                )))
            }
        }
    }

    fn shutdown(&self) -> Result<(), ExecutionError> {
        self.engine.shutdown()
    }
}

impl LeaseStateCodec {
    fn prepare(
        program: &NormalizedProgram,
        ty: TypeObjectDigest,
        interface: DeclarationReference,
    ) -> Result<Self, Diagnostic> {
        let declaration = named_declaration(program, ty, "queue lease state")?;
        let (layout, variant) = program
            .variants
            .iter()
            .enumerate()
            .find(|(_, layout)| layout.declaration == declaration)
            .ok_or_else(|| {
                queue_diagnostic(
                    "normalized_queue_lease_state_layout",
                    "queue lease state has no exact nominal variant layout",
                )
            })?;
        if variant.cases.len() != 2 {
            return Err(queue_diagnostic(
                "normalized_queue_lease_state_cases",
                "queue lease state must contain exactly absent and live cases",
            ));
        }
        let mut absent_case = None;
        let mut live_case = None;
        for (tag, case) in variant.cases.iter().enumerate() {
            let tag = u32::try_from(tag).map_err(|_| {
                queue_diagnostic(
                    "normalized_queue_lease_state_tag",
                    "queue lease state case index exceeds the runtime bound",
                )
            })?;
            match case.name.as_str() {
                "absent" if case.payload.is_none() => absent_case = Some(tag),
                "live"
                    if case
                        .payload
                        .is_some_and(|payload| matches_resource(program, payload, interface)) =>
                {
                    live_case = Some(tag);
                }
                _ => {
                    return Err(queue_diagnostic(
                        "normalized_queue_lease_state_case",
                        "queue lease state has a foreign exact case or payload",
                    ));
                }
            }
        }
        Ok(Self {
            layout: RecordIndex::variant(layout)?,
            absent_case: absent_case.ok_or_else(|| {
                queue_diagnostic(
                    "normalized_queue_lease_state_absent",
                    "queue lease state omits its absent case",
                )
            })?,
            live_case: live_case.ok_or_else(|| {
                queue_diagnostic(
                    "normalized_queue_lease_state_live",
                    "queue lease state omits its live resource case",
                )
            })?,
        })
    }

    fn absent(&self) -> NormalizedValue {
        NormalizedValue::Variant {
            layout: self.layout,
            case: self.absent_case,
            payload: None,
        }
    }

    fn live(&self, handle: NormalizedResourceHandle) -> NormalizedValue {
        NormalizedValue::Variant {
            layout: self.layout,
            case: self.live_case,
            payload: Some(Box::new(NormalizedValue::Resource(handle))),
        }
    }
}

struct RecordIndex;

impl RecordIndex {
    fn record(index: usize) -> Result<RecordLayoutIndex, Diagnostic> {
        u32::try_from(index).map(RecordLayoutIndex).map_err(|_| {
            queue_diagnostic(
                "normalized_queue_record_layout_index",
                "queue record layout exceeds the runtime bound",
            )
        })
    }

    fn variant(index: usize) -> Result<VariantLayoutIndex, Diagnostic> {
        u32::try_from(index).map(VariantLayoutIndex).map_err(|_| {
            queue_diagnostic(
                "normalized_queue_variant_layout_index",
                "queue variant layout exceeds the runtime bound",
            )
        })
    }
}

impl RecordCodec<LeaseInfoField> {
    fn lease_info(program: &NormalizedProgram, ty: TypeObjectDigest) -> Result<Self, Diagnostic> {
        let expected = BTreeMap::from([
            (
                "attempt-number",
                (LeaseInfoField::AttemptNumber, Shape::I64),
            ),
            ("job-id", (LeaseInfoField::JobId, Shape::Text)),
            (
                "lease-until-milliseconds",
                (LeaseInfoField::LeaseUntilMilliseconds, Shape::I64),
            ),
            ("payload", (LeaseInfoField::Payload, Shape::Bytes)),
        ]);
        Self::nominal(program, ty, &expected, "queue lease info")
    }

    fn encode_lease_info(&self, info: QueueLeaseInfo) -> Result<NormalizedValue, ExecutionError> {
        let layout = self.layout.ok_or_else(|| {
            queue_runtime(
                "normalized_queue_lease_info_layout",
                "queue lease info has no nominal runtime layout",
            )
        })?;
        Ok(NormalizedValue::Record(NormalizedRecord::Nominal {
            layout,
            fields: Arc::new(
                self.fields
                    .iter()
                    .map(|(_, field)| match field {
                        LeaseInfoField::AttemptNumber => {
                            NormalizedValue::I64(i64::from(info.attempt_number))
                        }
                        LeaseInfoField::JobId => NormalizedValue::text(info.job_id.clone()),
                        LeaseInfoField::LeaseUntilMilliseconds => {
                            NormalizedValue::I64(info.lease_until_milliseconds)
                        }
                        LeaseInfoField::Payload => NormalizedValue::bytes(info.payload.clone()),
                    })
                    .collect(),
            ),
        }))
    }
}

impl RecordCodec<SnapshotField> {
    fn snapshot(program: &NormalizedProgram, ty: TypeObjectDigest) -> Result<Self, Diagnostic> {
        let expected = BTreeMap::from([
            ("attempt_count", (SnapshotField::AttemptCount, Shape::I64)),
            (
                "available_at_milliseconds",
                (SnapshotField::AvailableAtMilliseconds, Shape::I64),
            ),
            ("job_id", (SnapshotField::JobId, Shape::Text)),
            (
                "last_error_class",
                (SnapshotField::LastErrorClass, Shape::Text),
            ),
            (
                "lease_until_milliseconds",
                (SnapshotField::LeaseUntilMilliseconds, Shape::I64),
            ),
            ("result", (SnapshotField::Result, Shape::Bytes)),
            ("state", (SnapshotField::State, Shape::Text)),
        ]);
        Self::structural(program, ty, &expected, "queue snapshot")
    }

    fn encode_snapshot(&self, snapshot: JobSnapshot) -> NormalizedValue {
        NormalizedValue::Record(NormalizedRecord::Structural {
            fields: Arc::new(
                self.fields
                    .iter()
                    .map(|(name, field)| {
                        let value = match field {
                            SnapshotField::AttemptCount => {
                                NormalizedValue::I64(i64::from(snapshot.attempt_count))
                            }
                            SnapshotField::AvailableAtMilliseconds => {
                                NormalizedValue::I64(snapshot.available_at_milliseconds)
                            }
                            SnapshotField::JobId => NormalizedValue::text(snapshot.job_id.clone()),
                            SnapshotField::LastErrorClass => NormalizedValue::text(
                                snapshot.last_error_class.clone().unwrap_or_default(),
                            ),
                            SnapshotField::LeaseUntilMilliseconds => NormalizedValue::I64(
                                snapshot.lease_until_milliseconds.unwrap_or(-1),
                            ),
                            SnapshotField::Result => {
                                NormalizedValue::bytes(snapshot.result.clone().unwrap_or_default())
                            }
                            SnapshotField::State => NormalizedValue::text(snapshot.state.as_str()),
                        };
                        (name.clone(), value)
                    })
                    .collect(),
            ),
        })
    }
}

impl<T: Copy> RecordCodec<T> {
    fn nominal(
        program: &NormalizedProgram,
        ty: TypeObjectDigest,
        expected: &BTreeMap<&str, (T, Shape)>,
        label: &str,
    ) -> Result<Self, Diagnostic> {
        let declaration = named_declaration(program, ty, label)?;
        let (layout, record) = program
            .records
            .iter()
            .enumerate()
            .find(|(_, layout)| layout.declaration == declaration)
            .ok_or_else(|| {
                queue_diagnostic(
                    "normalized_queue_record_layout",
                    format!("{label} has no exact nominal record layout"),
                )
            })?;
        let fields = record
            .fields
            .iter()
            .map(|field| (field.name.clone(), field.ty))
            .collect();
        Self::prepare(
            program,
            Some(RecordIndex::record(layout)?),
            fields,
            expected,
            label,
        )
    }

    fn structural(
        program: &NormalizedProgram,
        ty: TypeObjectDigest,
        expected: &BTreeMap<&str, (T, Shape)>,
        label: &str,
    ) -> Result<Self, Diagnostic> {
        let fields = structural_fields(program, ty)?;
        Self::prepare(program, None, fields, expected, label)
    }

    fn prepare(
        program: &NormalizedProgram,
        layout: Option<RecordLayoutIndex>,
        fields: Vec<(Name, TypeObjectDigest)>,
        expected: &BTreeMap<&str, (T, Shape)>,
        label: &str,
    ) -> Result<Self, Diagnostic> {
        if fields.len() != expected.len() {
            return Err(queue_diagnostic(
                "normalized_queue_record_fields",
                format!("{label} has a foreign exact field set"),
            ));
        }
        let fields = fields
            .into_iter()
            .map(|(name, ty)| {
                let (field, shape) = expected.get(name.as_str()).ok_or_else(|| {
                    queue_diagnostic(
                        "normalized_queue_record_field",
                        format!("{label} contains unknown field '{name}'"),
                    )
                })?;
                if !matches_shape(program, ty, *shape) {
                    return Err(queue_diagnostic(
                        "normalized_queue_record_type",
                        format!("{label} field '{name}' has a foreign type"),
                    ));
                }
                Ok((name, *field))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            layout,
            fields: fields.into(),
        })
    }
}

fn merge_lease_state(
    current: &mut Option<LeaseStateCodec>,
    candidate: LeaseStateCodec,
) -> Result<(), Diagnostic> {
    if current
        .as_ref()
        .is_some_and(|current| current != &candidate)
    {
        return Err(queue_diagnostic(
            "normalized_queue_lease_state_mismatch",
            "claim and heartbeat disagree on the exact queue lease state",
        ));
    }
    *current = Some(candidate);
    Ok(())
}

fn merge_lease_info(
    current: &mut Option<RecordCodec<LeaseInfoField>>,
    candidate: RecordCodec<LeaseInfoField>,
) -> Result<(), Diagnostic> {
    if current
        .as_ref()
        .is_some_and(|current| current != &candidate)
    {
        return Err(queue_diagnostic(
            "normalized_queue_lease_info_mismatch",
            "queue operations disagree on the exact lease-info record",
        ));
    }
    *current = Some(candidate);
    Ok(())
}

fn require_standard_interface(interface: DeclarationReference) -> Result<(), Diagnostic> {
    if interface.package.to_string() != STANDARD_PACKAGE
        || interface.declaration.to_string() != QUEUE_INTERFACE
    {
        return Err(queue_diagnostic(
            "normalized_queue_interface",
            "queue adapter requires the exact maintained standard DurableQueue interface",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Shape {
    Unit,
    Bool,
    I64,
    Bytes,
    Text,
    List,
    NamedRecord,
    NamedVariant,
    Resource(DeclarationReference),
}

const fn ordinary(shape: Shape) -> (Shape, ParameterUse) {
    (shape, ParameterUse::Unrestricted)
}

fn signature(
    program: &NormalizedProgram,
    operation: &NormalizedOperation,
    parameters: &[(Shape, ParameterUse)],
    result: Shape,
) -> Result<(), Diagnostic> {
    if operation.parameters.len() != parameters.len()
        || operation
            .parameters
            .iter()
            .zip(parameters)
            .any(|(actual, (expected, use_mode))| {
                actual.use_mode != *use_mode || !matches_shape(program, actual.ty, *expected)
            })
        || !matches_shape(program, operation.result, result)
    {
        return Err(queue_diagnostic(
            "normalized_queue_signature",
            format!(
                "exact queue operation '{}' has a foreign signature or parameter-use mode",
                operation.name
            ),
        ));
    }
    Ok(())
}

fn matches_shape(program: &NormalizedProgram, ty: TypeObjectDigest, shape: Shape) -> bool {
    program.types.get(&ty).is_some_and(|object| {
        matches!(
            (&object.form, shape),
            (TypeForm::Unit, Shape::Unit)
                | (TypeForm::Bool, Shape::Bool)
                | (TypeForm::I64, Shape::I64)
                | (TypeForm::Bytes, Shape::Bytes)
                | (TypeForm::Text, Shape::Text)
                | (TypeForm::List { .. }, Shape::List)
                | (TypeForm::Named { .. }, Shape::NamedRecord)
                | (TypeForm::Named { .. }, Shape::NamedVariant)
        ) || match (&object.form, shape) {
            (TypeForm::CapabilityResource { interface }, Shape::Resource(expected)) => {
                *interface == expected
            }
            _ => false,
        }
    })
}

fn matches_resource(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    interface: DeclarationReference,
) -> bool {
    matches_shape(program, ty, Shape::Resource(interface))
}

fn named_declaration(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    label: &str,
) -> Result<DeclarationReference, Diagnostic> {
    let Some(object) = program.types.get(&ty) else {
        return Err(queue_diagnostic(
            "normalized_queue_type_missing",
            format!("{label} type is absent from the artifact"),
        ));
    };
    let TypeForm::Named { declaration } = object.form else {
        return Err(queue_diagnostic(
            "normalized_queue_named_type",
            format!("{label} must use an exact nominal type"),
        ));
    };
    Ok(declaration)
}

fn list_item(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
) -> Result<TypeObjectDigest, Diagnostic> {
    let Some(object) = program.types.get(&ty) else {
        return Err(queue_diagnostic(
            "normalized_queue_type_missing",
            "queue operation type is absent from the artifact",
        ));
    };
    let TypeForm::List { item } = object.form else {
        return Err(queue_diagnostic(
            "normalized_queue_list_type",
            "queue operation requires an exact list type",
        ));
    };
    Ok(item)
}

fn structural_fields(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
) -> Result<Vec<(Name, TypeObjectDigest)>, Diagnostic> {
    let Some(object) = program.types.get(&ty) else {
        return Err(queue_diagnostic(
            "normalized_queue_type_missing",
            "queue record type is absent from the artifact",
        ));
    };
    let TypeForm::StructuralRecord { fields } = &object.form else {
        return Err(queue_diagnostic(
            "normalized_queue_record_type",
            "queue result must use an exact structural record",
        ));
    };
    Ok(fields
        .iter()
        .map(|field| (field.name.clone(), field.ty))
        .collect())
}

fn queue_argument(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "queue_adapter_argument",
        message,
    )
}

fn queue_runtime(code: &'static str, message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}

fn queue_diagnostic(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Capability, code, message)
}
