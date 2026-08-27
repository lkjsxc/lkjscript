//! Exact artifact-10 PostgreSQL codec over the representation-neutral database engine.

use super::capability::{
    NormalizedAdapterKind, NormalizedCallPolicy, NormalizedCapabilityAdapter,
    NormalizedCapabilityTransaction, NormalizedTransactionPolicy,
};
use super::prepare::{NormalizedOperation, NormalizedProgram, NormalizedRequirement};
use super::resource::NormalizedResourceScope;
use super::value::{NormalizedValue, VariantLayoutIndex};
use crate::platform::database::{
    DatabaseColumnType, DatabaseValue, MAXIMUM_DATABASE_COLUMNS, MAXIMUM_DATABASE_ROWS,
    PostgresEngine, PostgresEngineTransaction, PostgresPool,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{
    DeclarationReference, ExternalVisibility, OperationReference, ResourceUnit, TypeForm,
    TypeObjectDigest,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const STANDARD_PACKAGE: &str = "pkg_10000000000000000000000000000001";
const DATABASE_INTERFACE: &str = "decl_4c1cf20949507973e07ece4ec002c2d7";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DatabaseOperation {
    Execute,
    Query,
    Migration,
    Transaction,
}

#[derive(Clone, Debug)]
struct SqlValueCodec {
    layout: VariantLayoutIndex,
    cases: BTreeMap<String, u32>,
}

#[derive(Clone, Debug)]
struct SqlTypeCodec {
    layout: VariantLayoutIndex,
    cases: BTreeMap<String, u32>,
}

#[derive(Clone, Debug)]
pub(crate) struct NormalizedPostgresAdapter {
    interface: DeclarationReference,
    operations: BTreeMap<OperationReference, DatabaseOperation>,
    exact_operations: BTreeSet<OperationReference>,
    values: SqlValueCodec,
    columns: Option<SqlTypeCodec>,
    engine: PostgresEngine,
}

impl NormalizedPostgresAdapter {
    pub(crate) fn prepare(
        program: &NormalizedProgram,
        requirement: &NormalizedRequirement,
        pool: PostgresPool,
    ) -> Result<Self, Diagnostic> {
        require_standard_interface(requirement.interface)?;
        let mut operations = BTreeMap::new();
        let mut value_type = None;
        let mut column_type = None;
        for index in requirement.operations.iter().copied() {
            let operation = program.operations.get(index.0 as usize).ok_or_else(|| {
                database_diagnostic(
                    "normalized_database_operation_index",
                    "database requirement operation escaped the artifact table",
                )
            })?;
            let kind = match operation.name.as_str() {
                "execute" => {
                    validate_execute(program, operation)?;
                    remember_type(
                        &mut value_type,
                        list_item(program, operation.parameters[1].ty)?,
                    )?;
                    DatabaseOperation::Execute
                }
                "query" => {
                    validate_query(program, operation)?;
                    remember_type(
                        &mut value_type,
                        list_item(program, operation.parameters[1].ty)?,
                    )?;
                    remember_type(
                        &mut column_type,
                        list_item(program, operation.parameters[2].ty)?,
                    )?;
                    DatabaseOperation::Query
                }
                "migration" => {
                    validate_migration(program, operation)?;
                    DatabaseOperation::Migration
                }
                "transaction" => {
                    validate_transaction(program, operation)?;
                    DatabaseOperation::Transaction
                }
                _ => {
                    return Err(database_diagnostic(
                        "normalized_database_operation",
                        format!(
                            "PostgreSQL adapter does not implement exact operation '{}'",
                            operation.name
                        ),
                    ));
                }
            };
            if operations.insert(operation.reference, kind).is_some() {
                return Err(database_diagnostic(
                    "normalized_database_operation_duplicate",
                    "database requirement repeats an exact operation",
                ));
            }
        }
        let value_type = value_type.ok_or_else(|| {
            database_diagnostic(
                "normalized_database_value_type",
                "database requirement must select execute or query to bind SqlValue",
            )
        })?;
        let values = SqlValueCodec::prepare(program, value_type)?;
        let columns = column_type
            .map(|column_type| SqlTypeCodec::prepare(program, column_type))
            .transpose()?;
        let exact_operations = operations.keys().copied().collect();
        Ok(Self {
            interface: requirement.interface,
            operations,
            exact_operations,
            values,
            columns,
            engine: PostgresEngine::new(pool),
        })
    }

    pub(crate) fn preflight(&self) -> Result<(), ExecutionError> {
        self.engine.preflight()
    }

    fn validate_policy(
        &self,
        policy: &NormalizedCallPolicy,
    ) -> Result<DatabaseOperation, ExecutionError> {
        if policy.grant.interface != self.interface {
            return Err(database_runtime(
                "normalized_database_interface",
                "database call policy has a foreign exact interface",
            ));
        }
        self.operations
            .get(&policy.operation)
            .copied()
            .ok_or_else(|| {
                database_runtime(
                    "normalized_database_operation",
                    "database call policy has a foreign exact operation",
                )
            })
    }

    fn execute(
        &self,
        policy: &NormalizedCallPolicy,
        arguments: &[NormalizedValue],
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        let [NormalizedValue::StaticText(statement), parameters] = arguments else {
            return Err(database_argument(
                "execute expects StaticText statement and SqlValue list",
            ));
        };
        let parameters = self.values.decode_list(parameters)?;
        self.engine
            .execute(
                statement,
                &parameters,
                control,
                policy.external_visibility == ExternalVisibility::Possible,
            )
            .map(NormalizedValue::I64)
    }

    fn query(
        &self,
        policy: &NormalizedCallPolicy,
        arguments: &[NormalizedValue],
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        let [
            NormalizedValue::StaticText(statement),
            parameters,
            columns,
            NormalizedValue::I64(maximum_rows),
        ] = arguments
        else {
            return Err(database_argument(
                "query expects StaticText statement, SqlValue list, SqlType list, and I64 maximum rows",
            ));
        };
        let parameters = self.values.decode_list(parameters)?;
        let columns = self
            .columns
            .as_ref()
            .ok_or_else(|| {
                database_runtime(
                    "normalized_database_columns",
                    "query has no prepared SqlType codec",
                )
            })?
            .decode_list(columns)?;
        let maximum_rows = bounded_rows(*maximum_rows, policy)?;
        self.engine
            .query(statement, &parameters, &columns, maximum_rows, control)
            .map(|rows| self.values.encode_rows(rows))
    }

    fn migration(
        &self,
        arguments: &[NormalizedValue],
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        let [
            NormalizedValue::I64(migration_id),
            NormalizedValue::StaticText(checksum),
            NormalizedValue::StaticText(statement),
        ] = arguments
        else {
            return Err(database_argument(
                "migration expects positive I64 id, StaticText checksum, and StaticText statement",
            ));
        };
        self.engine
            .migration(*migration_id, checksum, statement, control)
            .map(NormalizedValue::Bool)
    }
}

impl NormalizedCapabilityAdapter for NormalizedPostgresAdapter {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::Postgres
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
        match self.validate_policy(policy)? {
            DatabaseOperation::Execute => self.execute(policy, &arguments, control),
            DatabaseOperation::Query => self.query(policy, &arguments, control),
            DatabaseOperation::Migration => self.migration(&arguments, control),
            DatabaseOperation::Transaction => Err(database_argument(
                "transaction entry must open a task-scoped transaction",
            )),
        }
    }

    fn begin_transaction(
        &self,
        policy: &NormalizedTransactionPolicy,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        if policy.grant.interface != self.interface
            || !self
                .operations
                .values()
                .any(|operation| *operation == DatabaseOperation::Transaction)
        {
            return Err(database_runtime(
                "normalized_database_transaction_binding",
                "transaction policy has a foreign exact interface or no transaction grant",
            ));
        }
        Ok(Box::new(NormalizedPostgresTransaction {
            transaction: self.engine.begin_transaction(control)?,
            operations: self.operations.clone(),
            values: self.values.clone(),
            columns: self.columns.clone(),
        }))
    }

    fn shutdown(&self) -> Result<(), ExecutionError> {
        self.engine.shutdown()
    }
}

struct NormalizedPostgresTransaction {
    transaction: PostgresEngineTransaction,
    operations: BTreeMap<OperationReference, DatabaseOperation>,
    values: SqlValueCodec,
    columns: Option<SqlTypeCodec>,
}

impl NormalizedCapabilityTransaction for NormalizedPostgresTransaction {
    fn call(
        &mut self,
        policy: &NormalizedCallPolicy,
        arguments: Vec<NormalizedValue>,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        let operation = self
            .operations
            .get(&policy.operation)
            .copied()
            .ok_or_else(|| {
                database_runtime(
                    "normalized_database_transaction_operation",
                    "transaction received a foreign exact operation",
                )
            })?;
        match operation {
            DatabaseOperation::Execute => {
                let [NormalizedValue::StaticText(statement), parameters] = arguments.as_slice()
                else {
                    return Err(database_argument(
                        "execute expects StaticText statement and SqlValue list",
                    ));
                };
                let parameters = self.values.decode_list(parameters)?;
                self.transaction
                    .execute(
                        statement,
                        &parameters,
                        control,
                        policy.external_visibility == ExternalVisibility::Possible,
                    )
                    .map(NormalizedValue::I64)
            }
            DatabaseOperation::Query => {
                let [
                    NormalizedValue::StaticText(statement),
                    parameters,
                    columns,
                    NormalizedValue::I64(maximum_rows),
                ] = arguments.as_slice()
                else {
                    return Err(database_argument(
                        "query expects StaticText statement, SqlValue list, SqlType list, and I64 maximum rows",
                    ));
                };
                let parameters = self.values.decode_list(parameters)?;
                let columns = self
                    .columns
                    .as_ref()
                    .ok_or_else(|| {
                        database_runtime(
                            "normalized_database_columns",
                            "query has no prepared SqlType codec",
                        )
                    })?
                    .decode_list(columns)?;
                let maximum_rows = bounded_rows(*maximum_rows, policy)?;
                let rows = self.transaction.query(
                    statement,
                    &parameters,
                    &columns,
                    maximum_rows,
                    control,
                )?;
                Ok(self.values.encode_rows(rows))
            }
            DatabaseOperation::Migration | DatabaseOperation::Transaction => {
                Err(database_argument(
                    "migration and nested transaction operations are unavailable in a transaction scope",
                ))
            }
        }
    }

    fn commit(&mut self, control: &ExecutionControl) -> Result<(), ExecutionError> {
        self.transaction.commit(control)
    }

    fn rollback(&mut self) -> Result<(), ExecutionError> {
        self.transaction.rollback()
    }
}

impl SqlValueCodec {
    fn prepare(program: &NormalizedProgram, ty: TypeObjectDigest) -> Result<Self, Diagnostic> {
        let (layout, cases) = nominal_variant(program, ty)?;
        for (name, payload) in [
            ("Bool", Some(Primitive::Bool)),
            ("I64", Some(Primitive::I64)),
            ("Text", Some(Primitive::Text)),
            ("Bytes", Some(Primitive::Bytes)),
            ("NullBool", None),
            ("NullI64", None),
            ("NullText", None),
            ("NullBytes", None),
        ] {
            require_variant_case(program, layout, name, payload)?;
        }
        if cases.len() != 8 {
            return Err(database_diagnostic(
                "normalized_database_sql_value_cases",
                "SqlValue must contain exactly the eight standard cases",
            ));
        }
        Ok(Self { layout, cases })
    }

    fn decode_list(&self, value: &NormalizedValue) -> Result<Vec<DatabaseValue>, ExecutionError> {
        let NormalizedValue::List(values) = value else {
            return Err(database_argument("database parameters must be a list"));
        };
        values.iter().map(|value| self.decode(value)).collect()
    }

    fn decode(&self, value: &NormalizedValue) -> Result<DatabaseValue, ExecutionError> {
        let NormalizedValue::Variant {
            layout,
            case,
            payload,
        } = value
        else {
            return Err(database_argument("database parameter is not SqlValue"));
        };
        if *layout != self.layout {
            return Err(database_argument(
                "database parameter has a foreign exact nominal identity",
            ));
        }
        let name = self
            .cases
            .iter()
            .find_map(|(name, index)| (*index == *case).then_some(name.as_str()))
            .ok_or_else(|| database_argument("database parameter case is outside SqlValue"))?;
        match (name, payload.as_deref()) {
            ("Bool", Some(NormalizedValue::Bool(value))) => Ok(DatabaseValue::Bool(Some(*value))),
            ("I64", Some(NormalizedValue::I64(value))) => Ok(DatabaseValue::I64(Some(*value))),
            ("Text", Some(NormalizedValue::Text(value))) => {
                Ok(DatabaseValue::Text(Some(value.to_string())))
            }
            ("Bytes", Some(NormalizedValue::Bytes(value))) => {
                Ok(DatabaseValue::Bytes(Some(value.to_vec())))
            }
            ("NullBool", None) => Ok(DatabaseValue::Bool(None)),
            ("NullI64", None) => Ok(DatabaseValue::I64(None)),
            ("NullText", None) => Ok(DatabaseValue::Text(None)),
            ("NullBytes", None) => Ok(DatabaseValue::Bytes(None)),
            _ => Err(database_argument(
                "database parameter case and payload disagree with SqlValue",
            )),
        }
    }

    fn encode_rows(&self, rows: Vec<Vec<DatabaseValue>>) -> NormalizedValue {
        NormalizedValue::List(Arc::new(
            rows.into_iter()
                .map(|row| {
                    NormalizedValue::List(Arc::new(
                        row.into_iter().map(|value| self.encode(value)).collect(),
                    ))
                })
                .collect(),
        ))
    }

    fn encode(&self, value: DatabaseValue) -> NormalizedValue {
        let (name, payload) = match value {
            DatabaseValue::Bool(Some(value)) => {
                ("Bool", Some(Box::new(NormalizedValue::Bool(value))))
            }
            DatabaseValue::I64(Some(value)) => ("I64", Some(Box::new(NormalizedValue::I64(value)))),
            DatabaseValue::Text(Some(value)) => {
                ("Text", Some(Box::new(NormalizedValue::text(value))))
            }
            DatabaseValue::Bytes(Some(value)) => {
                ("Bytes", Some(Box::new(NormalizedValue::bytes(value))))
            }
            DatabaseValue::Bool(None) => ("NullBool", None),
            DatabaseValue::I64(None) => ("NullI64", None),
            DatabaseValue::Text(None) => ("NullText", None),
            DatabaseValue::Bytes(None) => ("NullBytes", None),
        };
        NormalizedValue::Variant {
            layout: self.layout,
            case: self.cases[name],
            payload,
        }
    }
}

impl SqlTypeCodec {
    fn prepare(program: &NormalizedProgram, ty: TypeObjectDigest) -> Result<Self, Diagnostic> {
        let (layout, cases) = nominal_variant(program, ty)?;
        for name in ["Bool", "I64", "Text", "Bytes"] {
            require_variant_case(program, layout, name, None)?;
        }
        if cases.len() != 4 {
            return Err(database_diagnostic(
                "normalized_database_sql_type_cases",
                "SqlType must contain exactly the four standard unit cases",
            ));
        }
        Ok(Self { layout, cases })
    }

    fn decode_list(
        &self,
        value: &NormalizedValue,
    ) -> Result<Vec<DatabaseColumnType>, ExecutionError> {
        let NormalizedValue::List(values) = value else {
            return Err(database_argument("database column schema must be a list"));
        };
        if values.len() > MAXIMUM_DATABASE_COLUMNS {
            return Err(ExecutionError::resource(
                "database_column_limit",
                "database column schema exceeds its maximum",
            ));
        }
        values
            .iter()
            .map(|value| {
                let NormalizedValue::Variant {
                    layout,
                    case,
                    payload: None,
                } = value
                else {
                    return Err(database_argument("database column type is not SqlType"));
                };
                if *layout != self.layout {
                    return Err(database_argument(
                        "database column type has a foreign exact nominal identity",
                    ));
                }
                let name = self
                    .cases
                    .iter()
                    .find_map(|(name, index)| (*index == *case).then_some(name.as_str()))
                    .ok_or_else(|| database_argument("database column type case is unknown"))?;
                match name {
                    "Bool" => Ok(DatabaseColumnType::Bool),
                    "I64" => Ok(DatabaseColumnType::I64),
                    "Text" => Ok(DatabaseColumnType::Text),
                    "Bytes" => Ok(DatabaseColumnType::Bytes),
                    _ => Err(database_argument("database column type case is unknown")),
                }
            })
            .collect()
    }
}

#[derive(Clone, Copy)]
enum Primitive {
    Bool,
    I64,
    Text,
    Bytes,
}

fn require_standard_interface(interface: DeclarationReference) -> Result<(), Diagnostic> {
    if interface.package.to_string() != STANDARD_PACKAGE
        || interface.declaration.to_string() != DATABASE_INTERFACE
    {
        return Err(database_diagnostic(
            "normalized_database_interface",
            "PostgreSQL adapter requires the exact maintained standard Database interface",
        ));
    }
    Ok(())
}

fn validate_execute(
    program: &NormalizedProgram,
    operation: &NormalizedOperation,
) -> Result<(), Diagnostic> {
    require_signature(
        program,
        operation,
        &[Shape::StaticText, Shape::List],
        Shape::I64,
    )
}

fn validate_query(
    program: &NormalizedProgram,
    operation: &NormalizedOperation,
) -> Result<(), Diagnostic> {
    require_signature(
        program,
        operation,
        &[Shape::StaticText, Shape::List, Shape::List, Shape::I64],
        Shape::List,
    )?;
    let result_item = list_item(program, operation.result)?;
    let _ = list_item(program, result_item)?;
    Ok(())
}

fn validate_migration(
    program: &NormalizedProgram,
    operation: &NormalizedOperation,
) -> Result<(), Diagnostic> {
    require_signature(
        program,
        operation,
        &[Shape::I64, Shape::StaticText, Shape::StaticText],
        Shape::Bool,
    )
}

fn validate_transaction(
    program: &NormalizedProgram,
    operation: &NormalizedOperation,
) -> Result<(), Diagnostic> {
    require_signature(program, operation, &[], Shape::Unit)
}

#[derive(Clone, Copy)]
enum Shape {
    Unit,
    Bool,
    I64,
    StaticText,
    List,
}

fn require_signature(
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
        return Err(database_diagnostic(
            "normalized_database_signature",
            format!(
                "exact database operation '{}' has a foreign signature",
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
                | (TypeForm::StaticText, Shape::StaticText)
                | (TypeForm::List { .. }, Shape::List)
        )
    })
}

fn list_item(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
) -> Result<TypeObjectDigest, Diagnostic> {
    let Some(object) = program.types.get(&ty) else {
        return Err(database_diagnostic(
            "normalized_database_type_missing",
            "database operation type is absent from the prepared artifact",
        ));
    };
    let TypeForm::List { item } = object.form else {
        return Err(database_diagnostic(
            "normalized_database_list_type",
            "database operation requires an exact list type",
        ));
    };
    Ok(item)
}

fn remember_type(
    slot: &mut Option<TypeObjectDigest>,
    value: TypeObjectDigest,
) -> Result<(), Diagnostic> {
    if slot.is_some_and(|current| current != value) {
        return Err(database_diagnostic(
            "normalized_database_type_disagreement",
            "database operations disagree on one exact nominal codec type",
        ));
    }
    *slot = Some(value);
    Ok(())
}

fn nominal_variant(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
) -> Result<(VariantLayoutIndex, BTreeMap<String, u32>), Diagnostic> {
    let Some(object) = program.types.get(&ty) else {
        return Err(database_diagnostic(
            "normalized_database_nominal_type_missing",
            "database nominal codec type is absent from the artifact",
        ));
    };
    let TypeForm::Named { declaration } = object.form else {
        return Err(database_diagnostic(
            "normalized_database_nominal_type",
            "database codec requires an exact nominal variant",
        ));
    };
    let (index, layout) = program
        .variants
        .iter()
        .enumerate()
        .find(|(_, layout)| layout.declaration == declaration)
        .ok_or_else(|| {
            database_diagnostic(
                "normalized_database_variant_layout",
                "database nominal variant has no prepared layout",
            )
        })?;
    let index = u32::try_from(index).map_err(|_| {
        database_diagnostic(
            "normalized_database_variant_index",
            "database variant layout index exceeds its runtime representation",
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
                    database_diagnostic(
                        "normalized_database_case_index",
                        "database variant case index exceeds its runtime representation",
                    )
                })
        })
        .collect::<Result<_, _>>()?;
    Ok((VariantLayoutIndex(index), cases))
}

fn require_variant_case(
    program: &NormalizedProgram,
    layout: VariantLayoutIndex,
    name: &str,
    payload: Option<Primitive>,
) -> Result<(), Diagnostic> {
    let layout = &program.variants[layout.0 as usize];
    let case = layout
        .cases
        .iter()
        .find(|case| case.name.as_str() == name)
        .ok_or_else(|| {
            database_diagnostic(
                "normalized_database_variant_case",
                format!("database codec variant omits exact case '{name}'"),
            )
        })?;
    let valid = match (case.payload, payload) {
        (None, None) => true,
        (Some(actual), Some(expected)) => primitive_matches(program, actual, expected),
        _ => false,
    };
    if !valid {
        return Err(database_diagnostic(
            "normalized_database_variant_payload",
            format!("database codec case '{name}' has a foreign payload"),
        ));
    }
    Ok(())
}

fn primitive_matches(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    primitive: Primitive,
) -> bool {
    program.types.get(&ty).is_some_and(|object| {
        matches!(
            (&object.form, primitive),
            (TypeForm::Bool, Primitive::Bool)
                | (TypeForm::I64, Primitive::I64)
                | (TypeForm::Text, Primitive::Text)
                | (TypeForm::Bytes, Primitive::Bytes)
        )
    })
}

fn bounded_rows(value: i64, policy: &NormalizedCallPolicy) -> Result<usize, ExecutionError> {
    let maximum = usize::try_from(value).map_err(|_| {
        database_argument("database maximum rows must be a non-negative platform-sized integer")
    })?;
    let grant = policy
        .grant
        .limits
        .iter()
        .find_map(|(name, limit)| (name.as_str() == "maximum_rows").then_some(*limit));
    if grant.is_some_and(|limit| limit.unit != ResourceUnit::Items) {
        return Err(database_runtime(
            "normalized_database_row_limit_unit",
            "database maximum_rows grant has a foreign unit",
        ));
    }
    let granted = grant.map_or(MAXIMUM_DATABASE_ROWS as u64, |limit| limit.maximum);
    if maximum == 0
        || maximum > MAXIMUM_DATABASE_ROWS
        || u64::try_from(maximum).map_or(true, |maximum| maximum > granted)
    {
        return Err(ExecutionError::resource(
            "database_row_limit",
            "database maximum rows is zero or exceeds its exact grant",
        ));
    }
    Ok(maximum)
}

fn database_argument(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "database_adapter_argument",
        message,
    )
}

fn database_runtime(code: &'static str, message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}

fn database_diagnostic(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Capability, code, message)
}
