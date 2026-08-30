//! Exact artifact-10 durable-queue codec over the representation-neutral queue engine.

use super::capability::{NormalizedAdapterKind, NormalizedCallPolicy, NormalizedCapabilityAdapter};
use super::prepare::{NormalizedOperation, NormalizedProgram, NormalizedRequirement};
use super::resource::NormalizedResourceScope;
use super::value::{NormalizedRecord, NormalizedValue};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{
    DeclarationReference, ExternalVisibility, Name, OperationReference, TypeForm, TypeObjectDigest,
};
use crate::platform::queue::{DurableQueueEngine, JobLease, JobSnapshot};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const STANDARD_PACKAGE: &str = "pkg_10000000000000000000000000000001";
const QUEUE_INTERFACE: &str = "decl_20a0ef729beda0abf0e743cd7e1126de";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QueueOperation {
    Initialize,
    Enqueue,
    Claim,
    Heartbeat,
    Complete,
    Fail,
    Cancel,
    Inspect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LeaseField {
    AttemptId,
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
struct StructuralCodec<T> {
    fields: Arc<[(Name, T)]>,
}

#[derive(Clone, Debug)]
pub(crate) struct NormalizedDurableQueueAdapter {
    kind: NormalizedAdapterKind,
    interface: DeclarationReference,
    operations: BTreeMap<OperationReference, QueueOperation>,
    exact_operations: BTreeSet<OperationReference>,
    lease: Option<StructuralCodec<LeaseField>>,
    snapshot: Option<StructuralCodec<SnapshotField>>,
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
        let mut operations = BTreeMap::new();
        let mut lease = None;
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
                            Shape::Text,
                            Shape::Text,
                            Shape::Bytes,
                            Shape::I64,
                            Shape::I64,
                        ],
                        Shape::Bool,
                    )?;
                    QueueOperation::Enqueue
                }
                "claim" => {
                    signature(
                        program,
                        operation,
                        &[Shape::Text, Shape::I64, Shape::I64],
                        Shape::List,
                    )?;
                    lease = Some(StructuralCodec::lease(
                        program,
                        list_item(program, operation.result)?,
                    )?);
                    QueueOperation::Claim
                }
                "heartbeat" => {
                    signature(
                        program,
                        operation,
                        &[
                            Shape::Text,
                            Shape::Text,
                            Shape::Text,
                            Shape::I64,
                            Shape::I64,
                        ],
                        Shape::Bool,
                    )?;
                    QueueOperation::Heartbeat
                }
                "complete" => {
                    signature(
                        program,
                        operation,
                        &[
                            Shape::Text,
                            Shape::Text,
                            Shape::Text,
                            Shape::I64,
                            Shape::Bytes,
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
                            Shape::Text,
                            Shape::Text,
                            Shape::Text,
                            Shape::I64,
                            Shape::Bool,
                            Shape::I64,
                            Shape::Text,
                        ],
                        Shape::Bool,
                    )?;
                    QueueOperation::Fail
                }
                "cancel" => {
                    signature(program, operation, &[Shape::Text, Shape::I64], Shape::Bool)?;
                    QueueOperation::Cancel
                }
                "inspect" => {
                    signature(program, operation, &[Shape::Text], Shape::List)?;
                    snapshot = Some(StructuralCodec::snapshot(
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
            lease,
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
        _resources: &NormalizedResourceScope,
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
                    NormalizedValue::I64(lease),
                ] = arguments.as_slice()
                else {
                    return Err(queue_argument(
                        "claim expects worker id, current time, and lease duration",
                    ));
                };
                let lease = self
                    .engine
                    .claim(worker_id, *now, *lease, control, possible)?;
                let codec = self.lease.as_ref().ok_or_else(|| {
                    queue_runtime(
                        "normalized_queue_lease_codec",
                        "claim has no prepared exact lease codec",
                    )
                })?;
                Ok(NormalizedValue::List(Arc::new(
                    lease
                        .into_iter()
                        .map(|lease| codec.encode_lease(lease))
                        .collect(),
                )))
            }
            QueueOperation::Heartbeat => {
                let [
                    NormalizedValue::Text(job_id),
                    NormalizedValue::Text(attempt_id),
                    NormalizedValue::Text(worker_id),
                    NormalizedValue::I64(now),
                    NormalizedValue::I64(lease),
                ] = arguments.as_slice()
                else {
                    return Err(queue_argument(
                        "heartbeat expects job id, attempt id, worker id, current time, and lease duration",
                    ));
                };
                self.engine
                    .heartbeat(
                        job_id, attempt_id, worker_id, *now, *lease, control, possible,
                    )
                    .map(NormalizedValue::Bool)
            }
            QueueOperation::Complete => {
                let [
                    NormalizedValue::Text(job_id),
                    NormalizedValue::Text(attempt_id),
                    NormalizedValue::Text(worker_id),
                    NormalizedValue::I64(now),
                    NormalizedValue::Bytes(result),
                ] = arguments.as_slice()
                else {
                    return Err(queue_argument(
                        "complete expects job id, attempt id, worker id, current time, and result",
                    ));
                };
                self.engine
                    .complete(
                        job_id, attempt_id, worker_id, *now, result, control, possible,
                    )
                    .map(NormalizedValue::Bool)
            }
            QueueOperation::Fail => {
                let [
                    NormalizedValue::Text(job_id),
                    NormalizedValue::Text(attempt_id),
                    NormalizedValue::Text(worker_id),
                    NormalizedValue::I64(now),
                    NormalizedValue::Bool(retry),
                    NormalizedValue::I64(retry_at),
                    NormalizedValue::Text(error_class),
                ] = arguments.as_slice()
                else {
                    return Err(queue_argument(
                        "fail expects job id, attempt id, worker id, current time, retry decision, retry time, and error class",
                    ));
                };
                self.engine
                    .fail(
                        job_id,
                        attempt_id,
                        worker_id,
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

impl StructuralCodec<LeaseField> {
    fn lease(program: &NormalizedProgram, ty: TypeObjectDigest) -> Result<Self, Diagnostic> {
        let expected = BTreeMap::from([
            ("attempt_id", (LeaseField::AttemptId, Shape::Text)),
            ("attempt_number", (LeaseField::AttemptNumber, Shape::I64)),
            ("job_id", (LeaseField::JobId, Shape::Text)),
            (
                "lease_until_milliseconds",
                (LeaseField::LeaseUntilMilliseconds, Shape::I64),
            ),
            ("payload", (LeaseField::Payload, Shape::Bytes)),
        ]);
        Self::prepare(program, ty, &expected, "queue lease")
    }

    fn encode_lease(&self, lease: JobLease) -> NormalizedValue {
        NormalizedValue::Record(NormalizedRecord::Structural {
            fields: Arc::new(
                self.fields
                    .iter()
                    .map(|(name, field)| {
                        let value = match field {
                            LeaseField::AttemptId => {
                                NormalizedValue::text(lease.attempt_id.clone())
                            }
                            LeaseField::AttemptNumber => {
                                NormalizedValue::I64(i64::from(lease.attempt_number))
                            }
                            LeaseField::JobId => NormalizedValue::text(lease.job_id.clone()),
                            LeaseField::LeaseUntilMilliseconds => {
                                NormalizedValue::I64(lease.lease_until_milliseconds)
                            }
                            LeaseField::Payload => NormalizedValue::bytes(lease.payload.clone()),
                        };
                        (name.clone(), value)
                    })
                    .collect(),
            ),
        })
    }
}

impl StructuralCodec<SnapshotField> {
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
        Self::prepare(program, ty, &expected, "queue snapshot")
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

impl<T: Copy> StructuralCodec<T> {
    fn prepare(
        program: &NormalizedProgram,
        ty: TypeObjectDigest,
        expected: &BTreeMap<&str, (T, Shape)>,
        label: &str,
    ) -> Result<Self, Diagnostic> {
        let fields = structural_fields(program, ty)?;
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
            fields: fields.into(),
        })
    }
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
}

fn signature(
    program: &NormalizedProgram,
    operation: &NormalizedOperation,
    parameters: &[Shape],
    result: Shape,
) -> Result<(), Diagnostic> {
    if operation.parameters.len() != parameters.len()
        || operation
            .parameters
            .iter()
            .zip(parameters)
            .any(|(actual, expected)| !matches_shape(program, actual.ty, *expected))
        || !matches_shape(program, operation.result, result)
    {
        return Err(queue_diagnostic(
            "normalized_queue_signature",
            format!(
                "exact queue operation '{}' has a foreign signature",
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
        )
    })
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
