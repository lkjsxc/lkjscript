//! Exact standard `DataStore` codec over the first-party ordered engine.

use super::capability::{
    NormalizedAdapterKind, NormalizedCallPolicy, NormalizedCapabilityAdapter,
    NormalizedCapabilityTransaction, NormalizedTransactionPolicy,
};
use super::prepare::{NormalizedOperation, NormalizedProgram, NormalizedRequirement};
use super::resource::NormalizedResourceScope;
use super::value::{NormalizedRecord, NormalizedValue, RecordLayoutIndex, VariantLayoutIndex};
use crate::platform::data::{
    DataCommitOutcome, DataEntry, DataEntryRevision, DataExpectation, DataKey, DataKeyPart,
    DataScanDirection, DataScanItem, DataScanPage, DataSchema, DataSchemaExpectation, DataStore,
    DataTransaction,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{
    DeclarationReference, Name, OperationReference, TypeForm, TypeObjectDigest,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const STANDARD_PACKAGE: &str = "pkg_10000000000000000000000000000001";
pub(crate) const DATA_INTERFACE: &str = "decl_640e96fa57dee1c09557eb4bc7b53398";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DataOperation {
    SchemaRead,
    SchemaSet,
    Get,
    Scan,
    Put,
    Delete,
    Transaction,
}

#[derive(Clone, Debug)]
struct VariantCodec {
    layout: VariantLayoutIndex,
    cases: BTreeMap<String, u32>,
}

#[derive(Clone, Debug)]
struct RecordCodec {
    layout: RecordLayoutIndex,
    fields: Arc<[Name]>,
}

#[derive(Clone, Debug)]
struct DataCodecs {
    key_part: VariantCodec,
    expectation: VariantCodec,
    schema_expectation: VariantCodec,
    direction: VariantCodec,
    schema: RecordCodec,
    entry: RecordCodec,
    scan_item: RecordCodec,
    scan_page: RecordCodec,
}

#[derive(Clone, Debug)]
pub(crate) struct NormalizedDataAdapter {
    interface: DeclarationReference,
    operations: BTreeMap<OperationReference, DataOperation>,
    exact_operations: BTreeSet<OperationReference>,
    codecs: DataCodecs,
    store: DataStore,
}

impl NormalizedDataAdapter {
    pub(crate) fn prepare(
        program: &NormalizedProgram,
        requirement: &NormalizedRequirement,
        store: DataStore,
    ) -> Result<Self, Diagnostic> {
        require_standard_interface(requirement.interface)?;
        let mut operations = BTreeMap::new();
        let mut key_part = None;
        let mut expectation = None;
        let mut schema_expectation = None;
        let mut direction = None;
        let mut schema = None;
        let mut entry = None;
        let mut scan_item = None;
        let mut scan_page = None;
        for index in requirement.operations.iter().copied() {
            let operation = program.operations.get(index.0 as usize).ok_or_else(|| {
                data_diagnostic(
                    "normalized_data_operation_index",
                    "data requirement operation escaped the artifact table",
                )
            })?;
            let kind = match operation.name.as_str() {
                "schema-read" => {
                    signature(program, operation, &[Shape::StaticText], Shape::List)?;
                    remember_record(
                        &mut schema,
                        RecordCodec::prepare(
                            program,
                            list_item(program, operation.result)?,
                            &["digest", "identity"],
                            "data schema",
                        )?,
                    )?;
                    DataOperation::SchemaRead
                }
                "schema-set" => {
                    signature(
                        program,
                        operation,
                        &[Shape::StaticText, Shape::Variant, Shape::Record],
                        Shape::Bool,
                    )?;
                    remember_variant(
                        &mut schema_expectation,
                        VariantCodec::prepare(
                            program,
                            operation.parameters[1].ty,
                            &["Exact", "Missing"],
                            "data schema expectation",
                        )?,
                    )?;
                    remember_record(
                        &mut schema,
                        RecordCodec::prepare(
                            program,
                            operation.parameters[2].ty,
                            &["digest", "identity"],
                            "data schema",
                        )?,
                    )?;
                    DataOperation::SchemaSet
                }
                "get" => {
                    signature(
                        program,
                        operation,
                        &[Shape::StaticText, Shape::List],
                        Shape::List,
                    )?;
                    remember_variant(
                        &mut key_part,
                        VariantCodec::prepare(
                            program,
                            list_item(program, operation.parameters[1].ty)?,
                            &["Bool", "Bytes", "I64", "Text"],
                            "data key part",
                        )?,
                    )?;
                    remember_record(
                        &mut entry,
                        RecordCodec::prepare(
                            program,
                            list_item(program, operation.result)?,
                            &["revision", "value"],
                            "data entry",
                        )?,
                    )?;
                    DataOperation::Get
                }
                "scan" => {
                    signature(
                        program,
                        operation,
                        &[
                            Shape::StaticText,
                            Shape::List,
                            Shape::Variant,
                            Shape::I64,
                            Shape::I64,
                            Shape::I64,
                            Shape::Bytes,
                        ],
                        Shape::Record,
                    )?;
                    remember_variant(
                        &mut key_part,
                        VariantCodec::prepare(
                            program,
                            list_item(program, operation.parameters[1].ty)?,
                            &["Bool", "Bytes", "I64", "Text"],
                            "data key part",
                        )?,
                    )?;
                    remember_variant(
                        &mut direction,
                        VariantCodec::prepare(
                            program,
                            operation.parameters[2].ty,
                            &["Forward", "Reverse"],
                            "data scan direction",
                        )?,
                    )?;
                    let page = RecordCodec::prepare(
                        program,
                        operation.result,
                        &["bytes", "continuation", "items", "work"],
                        "data scan page",
                    )?;
                    let item_type = record_field_type(program, operation.result, "items")?;
                    remember_record(
                        &mut scan_item,
                        RecordCodec::prepare(
                            program,
                            list_item(program, item_type)?,
                            &["key", "revision", "value"],
                            "data scan item",
                        )?,
                    )?;
                    remember_record(&mut scan_page, page)?;
                    DataOperation::Scan
                }
                "put" => {
                    signature(
                        program,
                        operation,
                        &[Shape::StaticText, Shape::List, Shape::Bytes, Shape::Variant],
                        Shape::Bool,
                    )?;
                    remember_variant(
                        &mut key_part,
                        VariantCodec::prepare(
                            program,
                            list_item(program, operation.parameters[1].ty)?,
                            &["Bool", "Bytes", "I64", "Text"],
                            "data key part",
                        )?,
                    )?;
                    remember_variant(
                        &mut expectation,
                        VariantCodec::prepare(
                            program,
                            operation.parameters[3].ty,
                            &["Exact", "Missing"],
                            "data expectation",
                        )?,
                    )?;
                    DataOperation::Put
                }
                "delete" => {
                    signature(
                        program,
                        operation,
                        &[Shape::StaticText, Shape::List, Shape::Variant],
                        Shape::Bool,
                    )?;
                    remember_variant(
                        &mut key_part,
                        VariantCodec::prepare(
                            program,
                            list_item(program, operation.parameters[1].ty)?,
                            &["Bool", "Bytes", "I64", "Text"],
                            "data key part",
                        )?,
                    )?;
                    remember_variant(
                        &mut expectation,
                        VariantCodec::prepare(
                            program,
                            operation.parameters[2].ty,
                            &["Exact", "Missing"],
                            "data expectation",
                        )?,
                    )?;
                    DataOperation::Delete
                }
                "transaction" => {
                    signature(program, operation, &[], Shape::Unit)?;
                    DataOperation::Transaction
                }
                _ => {
                    return Err(data_diagnostic(
                        "normalized_data_operation",
                        format!(
                            "first-party data adapter does not implement exact operation '{}'",
                            operation.name
                        ),
                    ));
                }
            };
            if operations.insert(operation.reference, kind).is_some() {
                return Err(data_diagnostic(
                    "normalized_data_operation_duplicate",
                    "data requirement repeats an exact operation",
                ));
            }
        }
        let codecs = DataCodecs {
            key_part: require_codec(key_part, "data key part")?,
            expectation: require_codec(expectation, "data expectation")?,
            schema_expectation: require_codec(schema_expectation, "data schema expectation")?,
            direction: require_codec(direction, "data scan direction")?,
            schema: require_codec(schema, "data schema")?,
            entry: require_codec(entry, "data entry")?,
            scan_item: require_codec(scan_item, "data scan item")?,
            scan_page: require_codec(scan_page, "data scan page")?,
        };
        let exact_operations = operations.keys().copied().collect();
        Ok(Self {
            interface: requirement.interface,
            operations,
            exact_operations,
            codecs,
            store,
        })
    }

    pub(crate) fn preflight(&self) -> Result<(), ExecutionError> {
        self.store
            .verify()
            .map(|_| ())
            .map_err(|error| map_data_error(error, false))
    }

    fn operation(&self, policy: &NormalizedCallPolicy) -> Result<DataOperation, ExecutionError> {
        if policy.grant.interface != self.interface {
            return Err(data_runtime(
                "normalized_data_interface",
                "data call policy has a foreign exact interface",
            ));
        }
        self.operations
            .get(&policy.operation)
            .copied()
            .ok_or_else(|| {
                data_runtime(
                    "normalized_data_operation",
                    "data call policy has a foreign exact operation",
                )
            })
    }

    fn call_with(
        &self,
        transaction: &mut DataTransaction,
        operation: DataOperation,
        arguments: &[NormalizedValue],
    ) -> Result<NormalizedValue, ExecutionError> {
        match operation {
            DataOperation::SchemaRead => {
                let [NormalizedValue::StaticText(space)] = arguments else {
                    return Err(data_argument("schema-read expects one StaticText space"));
                };
                let schema = transaction
                    .schema_read(space)
                    .map_err(|error| map_data_error(error, false))?;
                Ok(NormalizedValue::List(Arc::new(
                    schema
                        .into_iter()
                        .map(|schema| self.codecs.schema.encode_schema(schema))
                        .collect(),
                )))
            }
            DataOperation::SchemaSet => {
                let [NormalizedValue::StaticText(space), expected, next] = arguments else {
                    return Err(data_argument(
                        "schema-set expects space, expectation, and schema",
                    ));
                };
                let expected = self
                    .codecs
                    .schema_expectation
                    .decode_schema_expectation(expected, &self.codecs.schema)?;
                let next = self.codecs.schema.decode_schema(next)?;
                transaction
                    .schema_set(space, &expected, next)
                    .map(NormalizedValue::Bool)
                    .map_err(|error| map_data_error(error, false))
            }
            DataOperation::Get => {
                let [NormalizedValue::StaticText(space), key] = arguments else {
                    return Err(data_argument("get expects StaticText space and data key"));
                };
                let key = self.codecs.key_part.decode_key(key, self.store.limits())?;
                let entry = transaction
                    .get(space, &key)
                    .map_err(|error| map_data_error(error, false))?;
                Ok(NormalizedValue::List(Arc::new(
                    entry
                        .into_iter()
                        .map(|entry| self.codecs.entry.encode_entry(entry))
                        .collect(),
                )))
            }
            DataOperation::Scan => {
                let [
                    NormalizedValue::StaticText(space),
                    prefix,
                    direction,
                    NormalizedValue::I64(maximum_items),
                    NormalizedValue::I64(maximum_bytes),
                    NormalizedValue::I64(maximum_work),
                    NormalizedValue::Bytes(continuation),
                ] = arguments
                else {
                    return Err(data_argument(
                        "scan expects space, prefix, direction, three limits, and continuation",
                    ));
                };
                let prefix = self.codecs.key_part.decode_key_parts(prefix)?;
                let direction = self.codecs.direction.decode_direction(direction)?;
                let maximum_items = bounded_usize(*maximum_items, "scan item limit")?;
                let maximum_bytes = bounded_usize(*maximum_bytes, "scan byte limit")?;
                let maximum_work = bounded_usize(*maximum_work, "scan work limit")?;
                let continuation = (!continuation.is_empty()).then_some(continuation.as_ref());
                let page = transaction
                    .scan(
                        space,
                        &prefix,
                        direction,
                        maximum_items,
                        maximum_bytes,
                        maximum_work,
                        continuation,
                    )
                    .map_err(|error| map_data_error(error, false))?;
                self.codecs.scan_page.encode_scan_page(
                    page,
                    &self.codecs.scan_item,
                    &self.codecs.key_part,
                )
            }
            DataOperation::Put => {
                let [
                    NormalizedValue::StaticText(space),
                    key,
                    NormalizedValue::Bytes(value),
                    expected,
                ] = arguments
                else {
                    return Err(data_argument(
                        "put expects space, key, bytes, and expectation",
                    ));
                };
                let key = self.codecs.key_part.decode_key(key, self.store.limits())?;
                let expected = self.codecs.expectation.decode_expectation(expected)?;
                transaction
                    .put(space, &key, value.to_vec(), expected)
                    .map(NormalizedValue::Bool)
                    .map_err(|error| map_data_error(error, false))
            }
            DataOperation::Delete => {
                let [NormalizedValue::StaticText(space), key, expected] = arguments else {
                    return Err(data_argument("delete expects space, key, and expectation"));
                };
                let key = self.codecs.key_part.decode_key(key, self.store.limits())?;
                let expected = self.codecs.expectation.decode_expectation(expected)?;
                transaction
                    .delete(space, &key, expected)
                    .map(NormalizedValue::Bool)
                    .map_err(|error| map_data_error(error, false))
            }
            DataOperation::Transaction => Err(data_argument(
                "transaction entry must open a task-scoped transaction",
            )),
        }
    }
}

impl NormalizedCapabilityAdapter for NormalizedDataAdapter {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::Data
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
        let operation = self.operation(policy)?;
        if matches!(
            operation,
            DataOperation::Put | DataOperation::Delete | DataOperation::SchemaSet
        ) {
            return Err(data_argument(
                "data mutation operations are available only inside transaction scope",
            ));
        }
        let mut transaction = self
            .store
            .begin()
            .map_err(|error| map_data_error(error, false))?;
        self.call_with(&mut transaction, operation, &arguments)
    }

    fn begin_transaction(
        &self,
        policy: &NormalizedTransactionPolicy,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        control.check()?;
        if policy.grant.interface != self.interface
            || !self
                .operations
                .values()
                .any(|operation| *operation == DataOperation::Transaction)
        {
            return Err(data_runtime(
                "normalized_data_transaction_binding",
                "transaction policy has a foreign exact interface or no transaction operation",
            ));
        }
        Ok(Box::new(NormalizedDataTransaction {
            transaction: Some(
                self.store
                    .begin()
                    .map_err(|error| map_data_error(error, false))?,
            ),
            operations: self.operations.clone(),
            adapter: self.clone(),
        }))
    }
}

struct NormalizedDataTransaction {
    transaction: Option<DataTransaction>,
    operations: BTreeMap<OperationReference, DataOperation>,
    adapter: NormalizedDataAdapter,
}

impl NormalizedCapabilityTransaction for NormalizedDataTransaction {
    fn call(
        &mut self,
        policy: &NormalizedCallPolicy,
        arguments: Vec<NormalizedValue>,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        let operation = self
            .operations
            .get(&policy.operation)
            .copied()
            .ok_or_else(|| {
                data_runtime(
                    "normalized_data_transaction_operation",
                    "data transaction received a foreign exact operation",
                )
            })?;
        if operation == DataOperation::Transaction {
            return Err(data_argument("nested data transactions are unavailable"));
        }
        let transaction = self.transaction.as_mut().ok_or_else(|| {
            data_runtime(
                "normalized_data_transaction_closed",
                "data transaction is already closed",
            )
        })?;
        self.adapter.call_with(transaction, operation, &arguments)
    }

    fn commit(&mut self, control: &ExecutionControl) -> Result<(), ExecutionError> {
        control.check()?;
        let transaction = self.transaction.take().ok_or_else(|| {
            data_runtime(
                "normalized_data_transaction_closed",
                "data transaction is already closed",
            )
        })?;
        match transaction
            .commit()
            .map_err(|error| map_data_error(error, true))?
        {
            DataCommitOutcome::Committed { .. } | DataCommitOutcome::Unchanged { .. } => Ok(()),
            DataCommitOutcome::Conflict { .. } => {
                let mut error = ExecutionError::new(
                    ExecutionFailureClass::Capability,
                    "normalized_data_transaction_conflict",
                    "data transaction exact base changed before commit",
                );
                error.retryable = true;
                Err(error)
            }
        }
    }

    fn rollback(&mut self) -> Result<(), ExecutionError> {
        self.transaction.take();
        Ok(())
    }
}

impl VariantCodec {
    fn prepare(
        program: &NormalizedProgram,
        ty: TypeObjectDigest,
        expected: &[&str],
        label: &str,
    ) -> Result<Self, Diagnostic> {
        let Some(object) = program.types.get(&ty) else {
            return Err(data_diagnostic(
                "normalized_data_type_missing",
                format!("{label} type is missing"),
            ));
        };
        let TypeForm::Named { declaration } = object.form else {
            return Err(data_diagnostic(
                "normalized_data_variant_type",
                format!("{label} is not an exact nominal variant"),
            ));
        };
        let (index, layout) = program
            .variants
            .iter()
            .enumerate()
            .find(|(_, layout)| layout.declaration == declaration)
            .ok_or_else(|| {
                data_diagnostic(
                    "normalized_data_variant_layout",
                    format!("{label} has no exact prepared layout"),
                )
            })?;
        let cases = layout
            .cases
            .iter()
            .enumerate()
            .map(|(index, case)| {
                u32::try_from(index)
                    .map(|index| (case.name.to_string(), index))
                    .map_err(|_| {
                        data_diagnostic(
                            "normalized_data_variant_index",
                            "data variant index exceeds the runtime representation",
                        )
                    })
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        if cases.len() != expected.len() || expected.iter().any(|name| !cases.contains_key(*name)) {
            return Err(data_diagnostic(
                "normalized_data_variant_cases",
                format!("{label} has a foreign exact case set"),
            ));
        }
        Ok(Self {
            layout: VariantLayoutIndex(u32::try_from(index).map_err(|_| {
                data_diagnostic(
                    "normalized_data_variant_index",
                    "data variant layout exceeds the runtime representation",
                )
            })?),
            cases,
        })
    }

    fn case<'s, 'v>(
        &'s self,
        value: &'v NormalizedValue,
        label: &str,
    ) -> Result<(&'s str, Option<&'v NormalizedValue>), ExecutionError> {
        let NormalizedValue::Variant {
            layout,
            case,
            payload,
        } = value
        else {
            return Err(data_argument(format!("{label} is not a nominal variant")));
        };
        if *layout != self.layout {
            return Err(data_argument(format!(
                "{label} has a foreign nominal identity"
            )));
        }
        let name = self
            .cases
            .iter()
            .find_map(|(name, index)| (*index == *case).then_some(name.as_str()))
            .ok_or_else(|| data_argument(format!("{label} has a foreign case")))?;
        Ok((name, payload.as_deref()))
    }

    fn decode_key_parts(
        &self,
        value: &NormalizedValue,
    ) -> Result<Vec<DataKeyPart>, ExecutionError> {
        let NormalizedValue::List(parts) = value else {
            return Err(data_argument("data key is not a list"));
        };
        parts
            .iter()
            .map(|part| {
                let (name, payload) = self.case(part, "data key part")?;
                match (name, payload) {
                    ("Bool", Some(NormalizedValue::Bool(value))) => Ok(DataKeyPart::Bool(*value)),
                    ("I64", Some(NormalizedValue::I64(value))) => Ok(DataKeyPart::I64(*value)),
                    ("Text", Some(NormalizedValue::Text(value))) => {
                        Ok(DataKeyPart::Text(value.to_string()))
                    }
                    ("Bytes", Some(NormalizedValue::Bytes(value))) => {
                        Ok(DataKeyPart::Bytes(value.to_vec()))
                    }
                    _ => Err(data_argument(
                        "data key part case and payload have a foreign shape",
                    )),
                }
            })
            .collect()
    }

    fn decode_key(
        &self,
        value: &NormalizedValue,
        limits: &crate::platform::data::DataLimits,
    ) -> Result<DataKey, ExecutionError> {
        DataKey::new(self.decode_key_parts(value)?, limits)
            .map_err(|error| map_data_error(error, false))
    }

    fn encode_key(&self, key: DataKey) -> NormalizedValue {
        NormalizedValue::List(Arc::new(
            key.parts()
                .iter()
                .map(|part| {
                    let (name, payload) = match part {
                        DataKeyPart::Bool(value) => {
                            ("Bool", Some(Box::new(NormalizedValue::Bool(*value))))
                        }
                        DataKeyPart::I64(value) => {
                            ("I64", Some(Box::new(NormalizedValue::I64(*value))))
                        }
                        DataKeyPart::Text(value) => {
                            ("Text", Some(Box::new(NormalizedValue::text(value.clone()))))
                        }
                        DataKeyPart::Bytes(value) => (
                            "Bytes",
                            Some(Box::new(NormalizedValue::bytes(value.clone()))),
                        ),
                    };
                    NormalizedValue::Variant {
                        layout: self.layout,
                        case: self.cases[name],
                        payload,
                    }
                })
                .collect(),
        ))
    }

    fn decode_expectation(
        &self,
        value: &NormalizedValue,
    ) -> Result<DataExpectation, ExecutionError> {
        match self.case(value, "data expectation")? {
            ("Missing", None) => Ok(DataExpectation::Missing),
            ("Exact", Some(NormalizedValue::Bytes(bytes))) => {
                let revision = bytes.as_ref().try_into().map_err(|_| {
                    data_argument("exact data entry revision must contain 32 bytes")
                })?;
                Ok(DataExpectation::Exact(DataEntryRevision::from_bytes(
                    revision,
                )))
            }
            _ => Err(data_argument(
                "data expectation case and payload have a foreign shape",
            )),
        }
    }

    fn decode_schema_expectation(
        &self,
        value: &NormalizedValue,
        schema: &RecordCodec,
    ) -> Result<DataSchemaExpectation, ExecutionError> {
        match self.case(value, "data schema expectation")? {
            ("Missing", None) => Ok(DataSchemaExpectation::Missing),
            ("Exact", Some(value)) => schema
                .decode_schema(value)
                .map(DataSchemaExpectation::Exact),
            _ => Err(data_argument(
                "data schema expectation case and payload have a foreign shape",
            )),
        }
    }

    fn decode_direction(
        &self,
        value: &NormalizedValue,
    ) -> Result<DataScanDirection, ExecutionError> {
        match self.case(value, "data scan direction")? {
            ("Forward", None) => Ok(DataScanDirection::Forward),
            ("Reverse", None) => Ok(DataScanDirection::Reverse),
            _ => Err(data_argument(
                "data scan direction case has a foreign payload",
            )),
        }
    }
}

impl RecordCodec {
    fn prepare(
        program: &NormalizedProgram,
        ty: TypeObjectDigest,
        expected: &[&str],
        label: &str,
    ) -> Result<Self, Diagnostic> {
        let Some(object) = program.types.get(&ty) else {
            return Err(data_diagnostic(
                "normalized_data_type_missing",
                format!("{label} type is missing"),
            ));
        };
        let TypeForm::Named { declaration } = object.form else {
            return Err(data_diagnostic(
                "normalized_data_record_type",
                format!("{label} is not an exact nominal record"),
            ));
        };
        let (index, layout) = program
            .records
            .iter()
            .enumerate()
            .find(|(_, layout)| layout.declaration == declaration)
            .ok_or_else(|| {
                data_diagnostic(
                    "normalized_data_record_layout",
                    format!("{label} has no exact prepared layout"),
                )
            })?;
        if layout.fields.len() != expected.len()
            || expected.iter().any(|name| {
                !layout
                    .fields
                    .iter()
                    .any(|field| field.name.as_str() == *name)
            })
        {
            return Err(data_diagnostic(
                "normalized_data_record_fields",
                format!("{label} has a foreign exact field set"),
            ));
        }
        Ok(Self {
            layout: RecordLayoutIndex(u32::try_from(index).map_err(|_| {
                data_diagnostic(
                    "normalized_data_record_index",
                    "data record layout exceeds the runtime representation",
                )
            })?),
            fields: layout
                .fields
                .iter()
                .map(|field| field.name.clone())
                .collect(),
        })
    }

    fn decode_fields<'s, 'v>(
        &'s self,
        value: &'v NormalizedValue,
        label: &str,
    ) -> Result<BTreeMap<&'s str, &'v NormalizedValue>, ExecutionError> {
        let NormalizedValue::Record(NormalizedRecord::Nominal { layout, fields }) = value else {
            return Err(data_argument(format!("{label} is not a nominal record")));
        };
        if *layout != self.layout || fields.len() != self.fields.len() {
            return Err(data_argument(format!(
                "{label} has a foreign nominal identity or field count"
            )));
        }
        Ok(self
            .fields
            .iter()
            .zip(fields.iter())
            .map(|(name, value)| (name.as_str(), value))
            .collect())
    }

    fn encode_fields(
        &self,
        mut values: BTreeMap<&'static str, NormalizedValue>,
    ) -> Result<NormalizedValue, ExecutionError> {
        let fields = self
            .fields
            .iter()
            .map(|name| {
                values.remove(name.as_str()).ok_or_else(|| {
                    data_runtime(
                        "normalized_data_record_encode",
                        "data record codec omitted an exact field",
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !values.is_empty() {
            return Err(data_runtime(
                "normalized_data_record_encode",
                "data record codec supplied a foreign field",
            ));
        }
        Ok(NormalizedValue::Record(NormalizedRecord::Nominal {
            layout: self.layout,
            fields: Arc::new(fields),
        }))
    }

    fn decode_schema(&self, value: &NormalizedValue) -> Result<DataSchema, ExecutionError> {
        let fields = self.decode_fields(value, "data schema")?;
        let Some(NormalizedValue::Text(identity)) = fields.get("identity") else {
            return Err(data_argument("data schema identity is not Text"));
        };
        let Some(NormalizedValue::Bytes(digest)) = fields.get("digest") else {
            return Err(data_argument("data schema digest is not Bytes"));
        };
        Ok(DataSchema {
            identity: identity.to_string(),
            digest: digest.to_vec(),
        })
    }

    fn encode_schema(&self, schema: DataSchema) -> NormalizedValue {
        self.encode_fields(BTreeMap::from([
            ("digest", NormalizedValue::bytes(schema.digest)),
            ("identity", NormalizedValue::text(schema.identity)),
        ]))
        .unwrap_or(NormalizedValue::Unit)
    }

    fn encode_entry(&self, entry: DataEntry) -> NormalizedValue {
        self.encode_fields(BTreeMap::from([
            (
                "revision",
                NormalizedValue::bytes(entry.revision.bytes().to_vec()),
            ),
            ("value", NormalizedValue::bytes(entry.value)),
        ]))
        .unwrap_or(NormalizedValue::Unit)
    }

    fn encode_scan_item(
        &self,
        item: DataScanItem,
        key: &VariantCodec,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.encode_fields(BTreeMap::from([
            ("key", key.encode_key(item.key)),
            (
                "revision",
                NormalizedValue::bytes(item.revision.bytes().to_vec()),
            ),
            ("value", NormalizedValue::bytes(item.value)),
        ]))
    }

    fn encode_scan_page(
        &self,
        page: DataScanPage,
        item: &RecordCodec,
        key: &VariantCodec,
    ) -> Result<NormalizedValue, ExecutionError> {
        let bytes = i64::try_from(page.bytes).map_err(|_| data_count_error())?;
        let work = i64::try_from(page.work).map_err(|_| data_count_error())?;
        let items = page
            .items
            .into_iter()
            .map(|value| item.encode_scan_item(value, key))
            .collect::<Result<Vec<_>, _>>()?;
        self.encode_fields(BTreeMap::from([
            ("bytes", NormalizedValue::I64(bytes)),
            (
                "continuation",
                NormalizedValue::bytes(page.continuation.unwrap_or_default()),
            ),
            ("items", NormalizedValue::List(Arc::new(items))),
            ("work", NormalizedValue::I64(work)),
        ]))
    }
}

#[derive(Clone, Copy)]
enum Shape {
    Unit,
    Bool,
    I64,
    Bytes,
    StaticText,
    List,
    Record,
    Variant,
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
        return Err(data_diagnostic(
            "normalized_data_signature",
            format!(
                "exact data operation '{}' has a foreign signature",
                operation.name
            ),
        ));
    }
    Ok(())
}

fn matches_shape(program: &NormalizedProgram, ty: TypeObjectDigest, shape: Shape) -> bool {
    let Some(object) = program.types.get(&ty) else {
        return false;
    };
    match (&object.form, shape) {
        (TypeForm::Unit, Shape::Unit)
        | (TypeForm::Bool, Shape::Bool)
        | (TypeForm::I64, Shape::I64)
        | (TypeForm::Bytes, Shape::Bytes)
        | (TypeForm::StaticText, Shape::StaticText)
        | (TypeForm::List { .. }, Shape::List) => true,
        (TypeForm::Named { declaration }, Shape::Record) => program
            .records
            .iter()
            .any(|layout| layout.declaration == *declaration),
        (TypeForm::Named { declaration }, Shape::Variant) => program
            .variants
            .iter()
            .any(|layout| layout.declaration == *declaration),
        _ => false,
    }
}

fn list_item(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
) -> Result<TypeObjectDigest, Diagnostic> {
    let Some(object) = program.types.get(&ty) else {
        return Err(data_diagnostic(
            "normalized_data_type_missing",
            "data list type is missing",
        ));
    };
    let TypeForm::List { item } = object.form else {
        return Err(data_diagnostic(
            "normalized_data_list_type",
            "data codec expected an exact list type",
        ));
    };
    Ok(item)
}

fn record_field_type(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    name: &str,
) -> Result<TypeObjectDigest, Diagnostic> {
    let Some(object) = program.types.get(&ty) else {
        return Err(data_diagnostic(
            "normalized_data_type_missing",
            "data record type is missing",
        ));
    };
    let TypeForm::Named { declaration } = object.form else {
        return Err(data_diagnostic(
            "normalized_data_record_type",
            "data codec expected an exact nominal record",
        ));
    };
    program
        .records
        .iter()
        .find(|layout| layout.declaration == declaration)
        .and_then(|layout| {
            layout
                .fields
                .iter()
                .find(|field| field.name.as_str() == name)
        })
        .map(|field| field.ty)
        .ok_or_else(|| {
            data_diagnostic(
                "normalized_data_record_field",
                format!("data record is missing exact field '{name}'"),
            )
        })
}

fn remember_variant(
    slot: &mut Option<VariantCodec>,
    codec: VariantCodec,
) -> Result<(), Diagnostic> {
    if slot
        .as_ref()
        .is_some_and(|current| current.layout != codec.layout)
    {
        return Err(data_diagnostic(
            "normalized_data_codec_disagreement",
            "data operations disagree on one exact nominal variant",
        ));
    }
    *slot = Some(codec);
    Ok(())
}

fn remember_record(slot: &mut Option<RecordCodec>, codec: RecordCodec) -> Result<(), Diagnostic> {
    if slot
        .as_ref()
        .is_some_and(|current| current.layout != codec.layout)
    {
        return Err(data_diagnostic(
            "normalized_data_codec_disagreement",
            "data operations disagree on one exact nominal record",
        ));
    }
    *slot = Some(codec);
    Ok(())
}

fn require_codec<T>(codec: Option<T>, label: &str) -> Result<T, Diagnostic> {
    codec.ok_or_else(|| {
        data_diagnostic(
            "normalized_data_codec_missing",
            format!("data requirement does not expose the operation needed to bind {label}"),
        )
    })
}

fn require_standard_interface(interface: DeclarationReference) -> Result<(), Diagnostic> {
    if interface.package.to_string() != STANDARD_PACKAGE
        || interface.declaration.to_string() != DATA_INTERFACE
    {
        return Err(data_diagnostic(
            "normalized_data_interface",
            "data adapter requires the exact maintained standard DataStore interface",
        ));
    }
    Ok(())
}

fn bounded_usize(value: i64, label: &str) -> Result<usize, ExecutionError> {
    usize::try_from(value).map_err(|_| {
        ExecutionError::resource(
            "normalized_data_limit",
            format!("{label} must be a positive platform-sized integer"),
        )
    })
}

fn map_data_error(error: Diagnostic, possible_visibility: bool) -> ExecutionError {
    let class = match error.class {
        DiagnosticClass::Resource => ExecutionFailureClass::Resource,
        DiagnosticClass::Cancelled => ExecutionFailureClass::Cancelled,
        DiagnosticClass::Source | DiagnosticClass::Semantic | DiagnosticClass::Capability => {
            ExecutionFailureClass::Capability
        }
        DiagnosticClass::Corrupt | DiagnosticClass::Infrastructure
            if possible_visibility && error.code.contains("unknown") =>
        {
            ExecutionFailureClass::PossibleVisibility
        }
        DiagnosticClass::Corrupt | DiagnosticClass::Infrastructure => {
            ExecutionFailureClass::Infrastructure
        }
    };
    ExecutionError::new(class, error.code, error.message)
}

fn data_count_error() -> ExecutionError {
    ExecutionError::resource(
        "normalized_data_count",
        "data result count exceeds signed runtime representation",
    )
}

fn data_argument(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Capability,
        "normalized_data_argument",
        message,
    )
}

fn data_runtime(code: &'static str, message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}

fn data_diagnostic(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Capability, code, message)
}
