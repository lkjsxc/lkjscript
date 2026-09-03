//! Exact Artifact 14 object-storage codec over the representation-neutral object engine.

use super::capability::{NormalizedAdapterKind, NormalizedCallPolicy, NormalizedCapabilityAdapter};
use super::prepare::{NormalizedOperation, NormalizedProgram, NormalizedRequirement};
use super::resource::NormalizedResourceScope;
use super::value::{NormalizedRecord, NormalizedValue};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{
    DeclarationReference, ExternalVisibility, Name, OperationReference, RequirementReference,
    ResourceUnit, TypeForm, TypeObjectDigest,
};
use crate::platform::object::{ObjectEngine, ObjectHeadReceipt, ObjectPutReceipt};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const STANDARD_PACKAGE: &str = "pkg_10000000000000000000000000000001";
const OBJECT_INTERFACE: &str = "decl_ac421d578f44958595e92fa9f5fb1d43";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObjectOperation {
    PutNew,
    Get,
    Range,
    Head,
    ReconcilePut,
    Delete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PutField {
    Blake3,
    CleanupPending,
    Key,
    Size,
    Version,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HeadField {
    Etag,
    Key,
    ModifiedMilliseconds,
    Size,
    Version,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StructuralCodec<T> {
    fields: Arc<[(Name, T)]>,
}

#[derive(Clone, Debug)]
pub(crate) struct NormalizedObjectStorageAdapter {
    kind: NormalizedAdapterKind,
    interface: DeclarationReference,
    operations: BTreeMap<OperationReference, ObjectOperation>,
    exact_operations: BTreeSet<OperationReference>,
    put_receipt: Option<StructuralCodec<PutField>>,
    head_receipt: Option<StructuralCodec<HeadField>>,
    stream_requirement: Option<RequirementReference>,
    engine: ObjectEngine,
}

impl NormalizedObjectStorageAdapter {
    pub(crate) fn prepare(
        program: &NormalizedProgram,
        requirement: &NormalizedRequirement,
        kind: NormalizedAdapterKind,
        stream_requirements: &[RequirementReference],
        engine: ObjectEngine,
    ) -> Result<Self, Diagnostic> {
        if !matches!(
            kind,
            NormalizedAdapterKind::ObjectMemory
                | NormalizedAdapterKind::ObjectLocal
                | NormalizedAdapterKind::ObjectS3
        ) {
            return Err(object_diagnostic(
                "normalized_object_adapter_kind",
                "object codec received a foreign adapter kind",
            ));
        }
        require_standard_interface(requirement.interface)?;
        let mut operations = BTreeMap::new();
        let mut put_receipt = None;
        let mut head_receipt = None;
        let mut stream_requirement = None;
        for index in requirement.operations.iter().copied() {
            let operation = program.operations.get(index.0 as usize).ok_or_else(|| {
                object_diagnostic(
                    "normalized_object_operation_index",
                    "object requirement operation escaped the artifact table",
                )
            })?;
            let kind = match operation.name.as_str() {
                "put-new" => {
                    let [exact_stream_requirement] = stream_requirements else {
                        return Err(object_diagnostic(
                            "normalized_object_stream_requirement",
                            "object put-new requires one exact component byte-stream capability slot",
                        ));
                    };
                    stream_requirement = Some(*exact_stream_requirement);
                    validate_signature(
                        program,
                        operation,
                        &[Shape::Text, Shape::ByteStream, Shape::StaticText],
                        Shape::Structural,
                    )?;
                    let codec = StructuralCodec::put_receipt(program, operation.result)?;
                    remember_codec(&mut put_receipt, codec)?;
                    ObjectOperation::PutNew
                }
                "get" => {
                    validate_signature(
                        program,
                        operation,
                        &[Shape::Text, Shape::I64],
                        Shape::Bytes,
                    )?;
                    ObjectOperation::Get
                }
                "range" => {
                    validate_signature(
                        program,
                        operation,
                        &[Shape::Text, Shape::I64, Shape::I64],
                        Shape::Bytes,
                    )?;
                    ObjectOperation::Range
                }
                "head" => {
                    validate_signature(program, operation, &[Shape::Text], Shape::Structural)?;
                    head_receipt = Some(StructuralCodec::head_receipt(program, operation.result)?);
                    ObjectOperation::Head
                }
                "reconcile-put" => {
                    validate_signature(program, operation, &[Shape::Text], Shape::List)?;
                    let item = list_item(program, operation.result)?;
                    let codec = StructuralCodec::put_receipt(program, item)?;
                    remember_codec(&mut put_receipt, codec)?;
                    ObjectOperation::ReconcilePut
                }
                "delete" => {
                    validate_signature(program, operation, &[Shape::Text], Shape::Unit)?;
                    ObjectOperation::Delete
                }
                _ => {
                    return Err(object_diagnostic(
                        "normalized_object_operation",
                        format!(
                            "object adapter does not implement exact operation '{}'",
                            operation.name
                        ),
                    ));
                }
            };
            if operations.insert(operation.reference, kind).is_some() {
                return Err(object_diagnostic(
                    "normalized_object_operation_duplicate",
                    "object requirement repeats an exact operation",
                ));
            }
        }
        let exact_operations = operations.keys().copied().collect();
        Ok(Self {
            kind,
            interface: requirement.interface,
            operations,
            exact_operations,
            put_receipt,
            head_receipt,
            stream_requirement,
            engine,
        })
    }

    fn operation(&self, policy: &NormalizedCallPolicy) -> Result<ObjectOperation, ExecutionError> {
        if policy.grant.interface != self.interface {
            return Err(object_runtime(
                "normalized_object_interface",
                "object call policy has a foreign exact interface",
            ));
        }
        self.operations
            .get(&policy.operation)
            .copied()
            .ok_or_else(|| {
                object_runtime(
                    "normalized_object_operation",
                    "object call policy has a foreign exact operation",
                )
            })
    }
}

impl NormalizedCapabilityAdapter for NormalizedObjectStorageAdapter {
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
            ObjectOperation::PutNew => {
                let [
                    NormalizedValue::Text(key),
                    NormalizedValue::Resource(stream),
                    NormalizedValue::StaticText(content_type),
                ] = arguments.as_slice()
                else {
                    return Err(object_argument(
                        "put-new expects Text key, byte-stream handle, and StaticText content type",
                    ));
                };
                let stream_requirement = self.stream_requirement.ok_or_else(|| {
                    object_runtime(
                        "normalized_object_stream_requirement",
                        "prepared object put-new lost its exact byte-stream capability slot",
                    )
                })?;
                let receipt =
                    self.engine
                        .put_new(key, content_type, control, possible, |control| {
                            resources.read_byte_stream(stream_requirement, *stream, control)
                        })?;
                self.put_receipt
                    .as_ref()
                    .ok_or_else(|| {
                        object_runtime(
                            "normalized_object_put_codec",
                            "put-new has no prepared exact receipt codec",
                        )
                    })
                    .map(|codec| codec.encode_put(receipt))
            }
            ObjectOperation::Get => {
                let [NormalizedValue::Text(key), NormalizedValue::I64(maximum)] =
                    arguments.as_slice()
                else {
                    return Err(object_argument(
                        "get expects Text key and I64 maximum bytes",
                    ));
                };
                let maximum = bounded_read(*maximum, policy)?;
                self.engine
                    .get(key, maximum, possible)
                    .map(NormalizedValue::bytes)
            }
            ObjectOperation::Range => {
                let [
                    NormalizedValue::Text(key),
                    NormalizedValue::I64(start),
                    NormalizedValue::I64(length),
                ] = arguments.as_slice()
                else {
                    return Err(object_argument(
                        "range expects Text key, I64 start, and I64 length",
                    ));
                };
                let start = u64::try_from(*start)
                    .map_err(|_| object_argument("object range start must be non-negative"))?;
                let length = usize::try_from(*length)
                    .map_err(|_| object_argument("object range length must be non-negative"))?;
                self.engine
                    .range(key, start, length, possible)
                    .map(NormalizedValue::bytes)
            }
            ObjectOperation::Head => {
                let [NormalizedValue::Text(key)] = arguments.as_slice() else {
                    return Err(object_argument("head expects one Text key"));
                };
                let receipt = self.engine.head(key, possible)?;
                self.head_receipt
                    .as_ref()
                    .ok_or_else(|| {
                        object_runtime(
                            "normalized_object_head_codec",
                            "head has no prepared exact receipt codec",
                        )
                    })
                    .map(|codec| codec.encode_head(receipt))
            }
            ObjectOperation::ReconcilePut => {
                let [NormalizedValue::Text(key)] = arguments.as_slice() else {
                    return Err(object_argument("reconcile-put expects one Text key"));
                };
                let receipts = self.engine.reconcile_put(key, control, possible)?;
                let codec = self.put_receipt.as_ref().ok_or_else(|| {
                    object_runtime(
                        "normalized_object_put_codec",
                        "reconcile-put has no prepared exact receipt codec",
                    )
                })?;
                Ok(NormalizedValue::List(Arc::new(
                    receipts
                        .into_iter()
                        .map(|receipt| codec.encode_put(receipt))
                        .collect(),
                )))
            }
            ObjectOperation::Delete => {
                let [NormalizedValue::Text(key)] = arguments.as_slice() else {
                    return Err(object_argument("delete expects one Text key"));
                };
                self.engine.delete(key, possible)?;
                Ok(NormalizedValue::Unit)
            }
        }
    }
}

impl StructuralCodec<PutField> {
    fn put_receipt(program: &NormalizedProgram, ty: TypeObjectDigest) -> Result<Self, Diagnostic> {
        let fields = structural_fields(program, ty)?;
        let expected = BTreeMap::from([
            ("blake3", (PutField::Blake3, Shape::Text)),
            ("cleanup_pending", (PutField::CleanupPending, Shape::Bool)),
            ("key", (PutField::Key, Shape::Text)),
            ("size", (PutField::Size, Shape::I64)),
            ("version", (PutField::Version, Shape::Text)),
        ]);
        Self::prepare(program, &fields, &expected, "object put receipt")
    }

    fn encode_put(&self, receipt: ObjectPutReceipt) -> NormalizedValue {
        NormalizedValue::Record(NormalizedRecord::Structural {
            fields: Arc::new(
                self.fields
                    .iter()
                    .map(|(name, field)| {
                        let value = match field {
                            PutField::Blake3 => NormalizedValue::text(receipt.blake3.clone()),
                            PutField::CleanupPending => {
                                NormalizedValue::Bool(receipt.cleanup_pending)
                            }
                            PutField::Key => NormalizedValue::text(receipt.key.clone()),
                            PutField::Size => NormalizedValue::I64(receipt.size),
                            PutField::Version => NormalizedValue::text(receipt.version.clone()),
                        };
                        (name.clone(), value)
                    })
                    .collect(),
            ),
        })
    }
}

impl StructuralCodec<HeadField> {
    fn head_receipt(program: &NormalizedProgram, ty: TypeObjectDigest) -> Result<Self, Diagnostic> {
        let fields = structural_fields(program, ty)?;
        let expected = BTreeMap::from([
            ("etag", (HeadField::Etag, Shape::Text)),
            ("key", (HeadField::Key, Shape::Text)),
            (
                "modified_milliseconds",
                (HeadField::ModifiedMilliseconds, Shape::I64),
            ),
            ("size", (HeadField::Size, Shape::I64)),
            ("version", (HeadField::Version, Shape::Text)),
        ]);
        Self::prepare(program, &fields, &expected, "object head receipt")
    }

    fn encode_head(&self, receipt: ObjectHeadReceipt) -> NormalizedValue {
        NormalizedValue::Record(NormalizedRecord::Structural {
            fields: Arc::new(
                self.fields
                    .iter()
                    .map(|(name, field)| {
                        let value = match field {
                            HeadField::Etag => NormalizedValue::text(receipt.etag.clone()),
                            HeadField::Key => NormalizedValue::text(receipt.key.clone()),
                            HeadField::ModifiedMilliseconds => {
                                NormalizedValue::I64(receipt.modified_milliseconds)
                            }
                            HeadField::Size => NormalizedValue::I64(receipt.size),
                            HeadField::Version => NormalizedValue::text(receipt.version.clone()),
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
        fields: &[(Name, TypeObjectDigest)],
        expected: &BTreeMap<&str, (T, Shape)>,
        label: &str,
    ) -> Result<Self, Diagnostic> {
        if fields.len() != expected.len() {
            return Err(object_diagnostic(
                "normalized_object_receipt_fields",
                format!("{label} has a foreign exact field set"),
            ));
        }
        let fields = fields
            .iter()
            .map(|(name, ty)| {
                let (field, shape) = expected.get(name.as_str()).ok_or_else(|| {
                    object_diagnostic(
                        "normalized_object_receipt_field",
                        format!("{label} contains unknown field '{name}'"),
                    )
                })?;
                if !matches_shape(program, *ty, *shape) {
                    return Err(object_diagnostic(
                        "normalized_object_receipt_type",
                        format!("{label} field '{name}' has a foreign type"),
                    ));
                }
                Ok((name.clone(), *field))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            fields: fields.into(),
        })
    }
}

fn remember_codec<T: Clone + Eq>(
    slot: &mut Option<StructuralCodec<T>>,
    codec: StructuralCodec<T>,
) -> Result<(), Diagnostic> {
    if slot.as_ref().is_some_and(|current| current != &codec) {
        return Err(object_diagnostic(
            "normalized_object_receipt_disagreement",
            "object operations disagree on their exact receipt layout",
        ));
    }
    *slot = Some(codec);
    Ok(())
}

fn require_standard_interface(interface: DeclarationReference) -> Result<(), Diagnostic> {
    if interface.package.to_string() != STANDARD_PACKAGE
        || interface.declaration.to_string() != OBJECT_INTERFACE
    {
        return Err(object_diagnostic(
            "normalized_object_interface",
            "object adapter requires the exact maintained standard ObjectStorage interface",
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
    StaticText,
    ByteStream,
    List,
    Structural,
}

fn validate_signature(
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
        return Err(object_diagnostic(
            "normalized_object_signature",
            format!(
                "exact object operation '{}' has a foreign signature",
                operation.name
            ),
        ));
    }
    Ok(())
}

fn matches_shape(program: &NormalizedProgram, ty: TypeObjectDigest, shape: Shape) -> bool {
    program
        .types
        .get(&ty)
        .is_some_and(|object| match (&object.form, shape) {
            (TypeForm::Unit, Shape::Unit)
            | (TypeForm::Bool, Shape::Bool)
            | (TypeForm::I64, Shape::I64)
            | (TypeForm::Bytes, Shape::Bytes)
            | (TypeForm::Text, Shape::Text)
            | (TypeForm::StaticText, Shape::StaticText)
            | (TypeForm::List { .. }, Shape::List)
            | (TypeForm::StructuralRecord { .. }, Shape::Structural) => true,
            (TypeForm::Stream { item }, Shape::ByteStream) => program
                .types
                .get(item)
                .is_some_and(|item| matches!(item.form, TypeForm::Bytes)),
            _ => false,
        })
}

fn list_item(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
) -> Result<TypeObjectDigest, Diagnostic> {
    let Some(object) = program.types.get(&ty) else {
        return Err(object_diagnostic(
            "normalized_object_type_missing",
            "object operation type is absent from the artifact",
        ));
    };
    let TypeForm::List { item } = object.form else {
        return Err(object_diagnostic(
            "normalized_object_list_type",
            "object operation requires an exact list type",
        ));
    };
    Ok(item)
}

fn structural_fields(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
) -> Result<Vec<(Name, TypeObjectDigest)>, Diagnostic> {
    let Some(object) = program.types.get(&ty) else {
        return Err(object_diagnostic(
            "normalized_object_type_missing",
            "object receipt type is absent from the artifact",
        ));
    };
    let TypeForm::StructuralRecord { fields } = &object.form else {
        return Err(object_diagnostic(
            "normalized_object_receipt_type",
            "object receipt must be an exact structural record",
        ));
    };
    Ok(fields
        .iter()
        .map(|field| (field.name.clone(), field.ty))
        .collect())
}

fn bounded_read(value: i64, policy: &NormalizedCallPolicy) -> Result<usize, ExecutionError> {
    let maximum = usize::try_from(value)
        .map_err(|_| object_argument("object whole-read maximum must be non-negative"))?;
    let grant = policy
        .grant
        .limits
        .iter()
        .find_map(|(name, limit)| (name.as_str() == "maximum_read_bytes").then_some(*limit));
    if grant.is_some_and(|limit| limit.unit != ResourceUnit::Bytes) {
        return Err(object_runtime(
            "normalized_object_read_limit_unit",
            "object maximum_read_bytes grant has a foreign unit",
        ));
    }
    if maximum == 0
        || grant.is_some_and(|limit| {
            u64::try_from(maximum).map_or(true, |maximum| maximum > limit.maximum)
        })
    {
        return Err(ExecutionError::resource(
            "object_read_limit",
            "object whole-read maximum is zero or exceeds its exact grant",
        ));
    }
    Ok(maximum)
}

fn object_argument(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "object_adapter_argument",
        message,
    )
}

fn object_runtime(code: &'static str, message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}

fn object_diagnostic(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Capability, code, message)
}
