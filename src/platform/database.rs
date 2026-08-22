//! Generic parameterized PostgreSQL capability with bounded pools and task-scoped transactions.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{
    CallPolicy, CapabilityAdapter, CapabilityTransaction, ExecutionControl, ExecutionError,
    ExecutionFailureClass,
};
use super::language::Visibility;
use super::semantic::OwnerId;
use super::value::Value;
use fallible_iterator::FallibleIterator;
use postgres::error::SqlState;
use postgres::types::ToSql;
use postgres::{Client, Config, NoTls, Row};
use serde::Serialize;
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub const POSTGRES_ADAPTER_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_DATABASE_CONNECTIONS: usize = 1_024;
pub const MAXIMUM_DATABASE_WAIT_MILLISECONDS: u64 = 300_000;
pub const MAXIMUM_DATABASE_ROWS: usize = 1_000_000;
pub const MAXIMUM_DATABASE_COLUMNS: usize = 4_096;

#[derive(Clone)]
pub struct PostgresSecret(Arc<str>);

impl PostgresSecret {
    pub fn new(value: impl Into<Arc<str>>) -> Result<Self, Diagnostic> {
        let value = value.into();
        if value.is_empty() || value.len() > 16 * 1024 {
            return Err(database_diagnostic(
                "database_secret_length",
                "PostgreSQL connection secret must contain 1 through 16384 bytes",
            ));
        }
        Ok(Self(value))
    }

    fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PostgresSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PostgresSecret(<redacted>)")
    }
}

#[derive(Clone, Debug)]
pub struct PostgresPoolConfig {
    pub connection: PostgresSecret,
    pub maximum_connections: usize,
    pub maximum_wait_milliseconds: u64,
    pub statement_timeout_milliseconds: u64,
}

impl PostgresPoolConfig {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.maximum_connections == 0 || self.maximum_connections > MAXIMUM_DATABASE_CONNECTIONS
        {
            return Err(database_diagnostic(
                "database_pool_connections",
                format!("maximum_connections must be 1 through {MAXIMUM_DATABASE_CONNECTIONS}"),
            ));
        }
        for (name, value) in [
            ("maximum_wait_milliseconds", self.maximum_wait_milliseconds),
            (
                "statement_timeout_milliseconds",
                self.statement_timeout_milliseconds,
            ),
        ] {
            if value == 0 || value > MAXIMUM_DATABASE_WAIT_MILLISECONDS {
                return Err(database_diagnostic(
                    "database_pool_time",
                    format!("{name} must be 1 through {MAXIMUM_DATABASE_WAIT_MILLISECONDS}"),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PostgresPoolObservation {
    pub open_connections: usize,
    pub idle_connections: usize,
    pub waiting_callers: usize,
    pub closed: bool,
}

#[derive(Clone)]
pub struct PostgresPool {
    inner: Arc<PoolInner>,
}

impl fmt::Debug for PostgresPool {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PostgresPool")
            .field("observation", &self.observe())
            .finish()
    }
}

struct PoolInner {
    config: PostgresPoolConfig,
    state: Mutex<PoolState>,
    available: Condvar,
}

#[derive(Default)]
struct PoolState {
    idle: Vec<Client>,
    total: usize,
    waiting: usize,
    closed: bool,
}

impl PostgresPool {
    pub fn new(config: PostgresPoolConfig) -> Result<Self, Diagnostic> {
        config.validate()?;
        Ok(Self {
            inner: Arc::new(PoolInner {
                config,
                state: Mutex::new(PoolState::default()),
                available: Condvar::new(),
            }),
        })
    }

    pub fn observe(&self) -> PostgresPoolObservation {
        let state = lock_unpoisoned(&self.inner.state);
        PostgresPoolObservation {
            open_connections: state.total,
            idle_connections: state.idle.len(),
            waiting_callers: state.waiting,
            closed: state.closed,
        }
    }

    pub fn close(&self) -> Result<(), ExecutionError> {
        let mut state = lock_unpoisoned(&self.inner.state);
        state.closed = true;
        let clients = std::mem::take(&mut state.idle);
        let removed = clients.len();
        state.total = state.total.saturating_sub(removed);
        self.inner.available.notify_all();
        drop(state);
        dispose_clients(clients)
    }

    /// Establish one reusable connection before a deployment publishes readiness.
    pub fn preflight(&self) -> Result<(), ExecutionError> {
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(
                self.inner.config.maximum_wait_milliseconds,
            ))
            .ok_or_else(|| {
                ExecutionError::resource(
                    "database_preflight_deadline",
                    "database preflight deadline overflowed",
                )
            })?;
        loop {
            match self.acquire(&ExecutionControl::uncancelled()) {
                Ok(connection) => {
                    drop(connection);
                    return Ok(());
                }
                Err(error)
                    if error.retryable
                        && error.code == "database_connection"
                        && Instant::now() < deadline =>
                {
                    std::thread::sleep(
                        deadline
                            .saturating_duration_since(Instant::now())
                            .min(Duration::from_millis(50)),
                    );
                }
                Err(error) => return Err(error),
            }
        }
    }

    pub(crate) fn acquire(
        &self,
        control: &ExecutionControl,
    ) -> Result<PooledClient, ExecutionError> {
        control.check()?;
        let pool_deadline = Instant::now()
            .checked_add(Duration::from_millis(
                self.inner.config.maximum_wait_milliseconds,
            ))
            .ok_or_else(|| {
                ExecutionError::resource(
                    "database_pool_deadline",
                    "database pool deadline overflowed",
                )
            })?;
        let deadline = control
            .deadline()
            .map_or(pool_deadline, |deadline| deadline.min(pool_deadline));
        loop {
            let mut state = lock_unpoisoned(&self.inner.state);
            if state.closed {
                return Err(ExecutionError::new(
                    ExecutionFailureClass::Capability,
                    "database_pool_closed",
                    "database pool is closed",
                ));
            }
            if let Some(client) = state.idle.pop() {
                return Ok(PooledClient {
                    client: Some(client),
                    pool: self.inner.clone(),
                    reusable: true,
                });
            }
            if state.total < self.inner.config.maximum_connections {
                state.total += 1;
                drop(state);
                let mut config = match self.inner.config.connection.expose().parse::<Config>() {
                    Ok(config) => config,
                    Err(_) => {
                        let mut state = lock_unpoisoned(&self.inner.state);
                        state.total = state.total.saturating_sub(1);
                        self.inner.available.notify_one();
                        return Err(ExecutionError::new(
                            ExecutionFailureClass::Capability,
                            "database_connection_descriptor",
                            "database connection descriptor is malformed",
                        ));
                    }
                };
                let attempt_milliseconds =
                    (self.inner.config.maximum_wait_milliseconds / 4).clamp(250, 5_000);
                config.connect_timeout(Duration::from_millis(attempt_milliseconds));
                let timeout = self.inner.config.statement_timeout_milliseconds;
                let client = match connect_client(config, timeout) {
                    Ok(client) => client,
                    Err(error) => {
                        let mut state = lock_unpoisoned(&self.inner.state);
                        state.total = state.total.saturating_sub(1);
                        self.inner.available.notify_one();
                        return Err(error);
                    }
                };
                return Ok(PooledClient {
                    client: Some(client),
                    pool: self.inner.clone(),
                    reusable: true,
                });
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(ExecutionError::resource(
                    "database_pool_exhausted",
                    "database pool admission wait reached its deadline",
                ));
            }
            control.check()?;
            state.waiting += 1;
            let duration = deadline.saturating_duration_since(now);
            let (mut state, wait) = wait_unpoisoned(&self.inner.available, state, duration);
            state.waiting = state.waiting.saturating_sub(1);
            if wait.timed_out() {
                return Err(ExecutionError::resource(
                    "database_pool_exhausted",
                    "database pool admission wait reached its deadline",
                ));
            }
            drop(state);
            control.check()?;
        }
    }
}

fn connect_client(
    config: Config,
    statement_timeout_milliseconds: u64,
) -> Result<Client, ExecutionError> {
    let connect = move || {
        let mut client = config.connect(NoTls)?;
        client.batch_execute(&format!(
            "SET statement_timeout TO {statement_timeout_milliseconds}; \
             SET lock_timeout TO {statement_timeout_milliseconds};"
        ))?;
        Ok(client)
    };
    if tokio::runtime::Handle::try_current().is_err() {
        return connect().map_err(connection_error);
    }
    let thread = std::thread::Builder::new()
        .name("lkjscript-postgres-connect".to_owned())
        .spawn(connect)
        .map_err(|_| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "database_connect_thread",
                "database connection worker could not be started",
            )
        })?;
    thread
        .join()
        .map_err(|_| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "database_connect_thread",
                "database connection worker terminated unexpectedly",
            )
        })?
        .map_err(connection_error)
}

pub(crate) struct PooledClient {
    client: Option<Client>,
    pool: Arc<PoolInner>,
    reusable: bool,
}

impl PooledClient {
    pub(crate) fn client(&mut self) -> Result<&mut Client, ExecutionError> {
        self.client.as_mut().ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "database_connection_missing",
                "pooled database connection disappeared",
            )
        })
    }

    pub(crate) fn discard(&mut self) {
        self.reusable = false;
    }
}

impl Drop for PooledClient {
    fn drop(&mut self) {
        let Some(client) = self.client.take() else {
            return;
        };
        let mut state = lock_unpoisoned(&self.pool.state);
        if self.reusable && !state.closed && !client.is_closed() {
            state.idle.push(client);
        } else {
            state.total = state.total.saturating_sub(1);
            drop(state);
            let _ = dispose_clients(vec![client]);
        }
        self.pool.available.notify_one();
    }
}

impl Drop for PoolInner {
    fn drop(&mut self) {
        let state = match self.state.get_mut() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
        let clients = std::mem::take(&mut state.idle);
        let _ = dispose_clients(clients);
    }
}

fn dispose_clients(clients: Vec<Client>) -> Result<(), ExecutionError> {
    if clients.is_empty() {
        return Ok(());
    }
    if tokio::runtime::Handle::try_current().is_err() {
        drop(clients);
        return Ok(());
    }
    let clients = Arc::new(Mutex::new(Some(clients)));
    let owned = clients.clone();
    let thread = std::thread::Builder::new()
        .name("lkjscript-postgres-close".to_owned())
        .spawn(move || {
            if let Some(clients) = lock_unpoisoned(&owned).take() {
                drop(clients);
            }
        });
    let thread = match thread {
        Ok(thread) => thread,
        Err(_) => {
            if let Some(clients) = lock_unpoisoned(&clients).take() {
                std::mem::forget(clients);
            }
            return Err(ExecutionError::resource(
                "database_cleanup_thread",
                "PostgreSQL cleanup thread could not be created; clients remain process-owned",
            ));
        }
    };
    thread.join().map_err(|_| {
        ExecutionError::new(
            ExecutionFailureClass::Infrastructure,
            "database_cleanup_panic",
            "PostgreSQL cleanup thread terminated unexpectedly",
        )
    })
}

#[derive(Clone)]
pub struct PostgresAdapter {
    interface: OwnerId,
    value_owner: OwnerId,
    type_owner: OwnerId,
    pool: PostgresPool,
}

impl fmt::Debug for PostgresAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PostgresAdapter")
            .field("interface", &self.interface)
            .field("value_owner", &self.value_owner)
            .field("type_owner", &self.type_owner)
            .field("pool", &self.pool)
            .finish()
    }
}

impl PostgresAdapter {
    pub fn new(
        interface: OwnerId,
        value_owner: OwnerId,
        type_owner: OwnerId,
        pool: PostgresPool,
    ) -> Self {
        Self {
            interface,
            value_owner,
            type_owner,
            pool,
        }
    }

    pub fn pool(&self) -> &PostgresPool {
        &self.pool
    }

    fn execute(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        let [statement, parameters] = arguments.as_slice() else {
            return Err(argument_error(
                "execute expects statement and parameter list",
            ));
        };
        let statement = static_statement(statement)?;
        let parameters = decode_parameters(parameters, &self.value_owner)?;
        let mut connection = self.pool.acquire(&policy.control)?;
        execute_on(connection.client()?, statement, &parameters, policy)
            .map(Value::I64)
            .inspect_err(|_| connection.discard())
    }

    fn query(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        let [statement, parameters, columns, maximum_rows] = arguments.as_slice() else {
            return Err(argument_error(
                "query expects statement, parameter list, column schema, and maximum rows",
            ));
        };
        let statement = static_statement(statement)?;
        let parameters = decode_parameters(parameters, &self.value_owner)?;
        let columns = decode_column_types(columns, &self.type_owner)?;
        let maximum_rows = decode_maximum_rows(maximum_rows, policy)?;
        let mut connection = self.pool.acquire(&policy.control)?;
        query_on(
            connection.client()?,
            statement,
            &parameters,
            &columns,
            maximum_rows,
            &self.value_owner,
            policy,
        )
        .inspect_err(|_| connection.discard())
    }

    fn migration(
        &self,
        policy: &CallPolicy,
        arguments: Vec<Value>,
    ) -> Result<Value, ExecutionError> {
        let [migration_id, checksum, statement] = arguments.as_slice() else {
            return Err(argument_error(
                "migration expects positive id, checksum, and static statement",
            ));
        };
        let Value::I64(migration_id) = migration_id else {
            return Err(argument_error("migration id must be I64"));
        };
        if *migration_id <= 0 {
            return Err(argument_error("migration id must be positive"));
        }
        let checksum = static_statement(checksum)?;
        let statement = static_statement(statement)?;
        let actual = blake3::hash(statement.as_bytes()).to_hex().to_string();
        if checksum != actual {
            return Err(argument_error(
                "migration checksum does not match its exact statement bytes",
            ));
        }
        if contains_transaction_control(statement) {
            return Err(argument_error(
                "migration statement may not contain transaction control",
            ));
        }
        let mut connection = self.pool.acquire(&policy.control)?;
        let result = migrate_on(
            connection.client()?,
            *migration_id,
            checksum,
            statement,
            policy,
        );
        if result.is_err() {
            connection.discard();
        }
        result.map(Value::Bool)
    }
}

impl CapabilityAdapter for PostgresAdapter {
    fn interface(&self) -> &OwnerId {
        &self.interface
    }

    fn call(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        policy.control.check()?;
        match policy.operation.as_str() {
            "execute" => self.execute(policy, arguments),
            "query" => self.query(policy, arguments),
            "migration" => self.migration(policy, arguments),
            operation => Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "database_operation_unknown",
                format!("PostgreSQL adapter does not implement operation '{operation}'"),
            )),
        }
    }

    fn begin_transaction(
        &self,
        policy: &CallPolicy,
    ) -> Result<Box<dyn CapabilityTransaction>, ExecutionError> {
        policy.control.check()?;
        if policy.operation != "transaction" {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "database_transaction_operation",
                "PostgreSQL transaction entry uses a foreign operation",
            ));
        }
        let mut connection = self.pool.acquire(&policy.control)?;
        if let Err(error) = connection.client()?.batch_execute("BEGIN") {
            connection.discard();
            return Err(map_postgres_error(error, policy, false));
        }
        Ok(Box::new(PostgresTransaction {
            connection: Some(connection),
            value_owner: self.value_owner.clone(),
            type_owner: self.type_owner.clone(),
            transaction_policy: policy.clone(),
            completed: false,
        }))
    }

    fn shutdown(&self) -> Result<(), ExecutionError> {
        self.pool.close()
    }
}

struct PostgresTransaction {
    connection: Option<PooledClient>,
    value_owner: OwnerId,
    type_owner: OwnerId,
    transaction_policy: CallPolicy,
    completed: bool,
}

impl PostgresTransaction {
    fn connection(&mut self) -> Result<&mut PooledClient, ExecutionError> {
        self.connection.as_mut().ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "database_transaction_completed",
                "PostgreSQL transaction has already completed",
            )
        })
    }
}

impl CapabilityTransaction for PostgresTransaction {
    fn call(
        &mut self,
        policy: &CallPolicy,
        arguments: Vec<Value>,
    ) -> Result<Value, ExecutionError> {
        policy.control.check()?;
        match policy.operation.as_str() {
            "execute" => {
                let [statement, parameters] = arguments.as_slice() else {
                    return Err(argument_error("execute expects statement and parameters"));
                };
                let statement = static_statement(statement)?;
                let parameters = decode_parameters(parameters, &self.value_owner)?;
                let result =
                    execute_on(self.connection()?.client()?, statement, &parameters, policy);
                result.map(Value::I64)
            }
            "query" => {
                let [statement, parameters, columns, maximum_rows] = arguments.as_slice() else {
                    return Err(argument_error(
                        "query expects statement, parameters, columns, and maximum rows",
                    ));
                };
                let statement = static_statement(statement)?;
                let parameters = decode_parameters(parameters, &self.value_owner)?;
                let columns = decode_column_types(columns, &self.type_owner)?;
                let maximum_rows = decode_maximum_rows(maximum_rows, policy)?;
                let value_owner = self.value_owner.clone();
                query_on(
                    self.connection()?.client()?,
                    statement,
                    &parameters,
                    &columns,
                    maximum_rows,
                    &value_owner,
                    policy,
                )
            }
            "migration" | "transaction" => Err(argument_error(
                "migration and nested transaction operations are unavailable in a transaction scope",
            )),
            _ => Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "database_transaction_operation",
                "transaction received an unknown database operation",
            )),
        }
    }

    fn commit(&mut self) -> Result<(), ExecutionError> {
        let control = ExecutionControl::uncancelled();
        let policy = internal_policy(
            &self.transaction_policy,
            "commit",
            Visibility::Possible,
            &control,
        );
        let result = self
            .connection()?
            .client()?
            .batch_execute("COMMIT")
            .map_err(|error| map_postgres_error(error, &policy, true));
        self.completed = true;
        if result.is_err()
            && let Some(connection) = self.connection.as_mut()
        {
            connection.discard();
        }
        self.connection = None;
        result
    }

    fn rollback(&mut self) -> Result<(), ExecutionError> {
        let control = ExecutionControl::uncancelled();
        let policy = internal_policy(
            &self.transaction_policy,
            "rollback",
            Visibility::None,
            &control,
        );
        let result = self
            .connection()?
            .client()?
            .batch_execute("ROLLBACK")
            .map_err(|error| map_postgres_error(error, &policy, false));
        self.completed = true;
        if result.is_err()
            && let Some(connection) = self.connection.as_mut()
        {
            connection.discard();
        }
        self.connection = None;
        result
    }
}

impl Drop for PostgresTransaction {
    fn drop(&mut self) {
        if !self.completed {
            if let Some(connection) = self.connection.as_mut()
                && connection
                    .client()
                    .and_then(|client| client.batch_execute("ROLLBACK").map_err(connection_error))
                    .is_err()
            {
                connection.discard();
            }
            self.completed = true;
            self.connection = None;
        }
    }
}

enum Parameter {
    Bool(Option<bool>),
    I64(Option<i64>),
    Text(Option<String>),
    Bytes(Option<Vec<u8>>),
}

impl Parameter {
    fn as_sql(&self) -> &(dyn ToSql + Sync) {
        match self {
            Self::Bool(value) => value,
            Self::I64(value) => value,
            Self::Text(value) => value,
            Self::Bytes(value) => value,
        }
    }
}

#[derive(Clone, Copy)]
enum ColumnType {
    Bool,
    I64,
    Text,
    Bytes,
}

fn execute_on(
    client: &mut Client,
    statement: &str,
    parameters: &[Parameter],
    policy: &CallPolicy,
) -> Result<i64, ExecutionError> {
    let parameters = sql_parameters(parameters);
    let count = client
        .execute(statement, &parameters)
        .map_err(|error| map_postgres_error(error, policy, false))?;
    i64::try_from(count).map_err(|_| {
        ExecutionError::resource(
            "database_row_count",
            "affected row count exceeds signed 64-bit range",
        )
    })
}

fn query_on(
    client: &mut Client,
    statement: &str,
    parameters: &[Parameter],
    columns: &[ColumnType],
    maximum_rows: usize,
    value_owner: &OwnerId,
    policy: &CallPolicy,
) -> Result<Value, ExecutionError> {
    let parameters = sql_parameters(parameters);
    let mut rows = client
        .query_raw(statement, parameters)
        .map_err(|error| map_postgres_error(error, policy, false))?;
    let mut output = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|error| map_postgres_error(error, policy, false))?
    {
        if output.len() >= maximum_rows {
            return Err(ExecutionError::resource(
                "database_row_limit",
                "database query returned more rows than its declared maximum",
            ));
        }
        output.push(decode_row(&row, columns, value_owner, policy)?);
    }
    Ok(Value::List(Arc::new(output)))
}

fn decode_row(
    row: &Row,
    columns: &[ColumnType],
    owner: &OwnerId,
    policy: &CallPolicy,
) -> Result<Value, ExecutionError> {
    if row.len() != columns.len() {
        return Err(ExecutionError::new(
            ExecutionFailureClass::Capability,
            "database_column_count",
            format!(
                "database returned {} columns; the application declared {}",
                row.len(),
                columns.len()
            ),
        ));
    }
    let values = columns
        .iter()
        .enumerate()
        .map(|(index, ty)| decode_column(row, index, *ty, owner, policy))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Value::List(Arc::new(values)))
}

fn decode_column(
    row: &Row,
    index: usize,
    ty: ColumnType,
    owner: &OwnerId,
    policy: &CallPolicy,
) -> Result<Value, ExecutionError> {
    let (case, payload) = match ty {
        ColumnType::Bool => match row
            .try_get::<_, Option<bool>>(index)
            .map_err(|error| map_postgres_error(error, policy, false))?
        {
            Some(value) => ("Bool", Some(Value::Bool(value))),
            None => ("NullBool", None),
        },
        ColumnType::I64 => match row
            .try_get::<_, Option<i64>>(index)
            .map_err(|error| map_postgres_error(error, policy, false))?
        {
            Some(value) => ("I64", Some(Value::I64(value))),
            None => ("NullI64", None),
        },
        ColumnType::Text => match row
            .try_get::<_, Option<String>>(index)
            .map_err(|error| map_postgres_error(error, policy, false))?
        {
            Some(value) => ("Text", Some(Value::text(value))),
            None => ("NullText", None),
        },
        ColumnType::Bytes => match row
            .try_get::<_, Option<Vec<u8>>>(index)
            .map_err(|error| map_postgres_error(error, policy, false))?
        {
            Some(value) => ("Bytes", Some(Value::bytes(value))),
            None => ("NullBytes", None),
        },
    };
    Ok(Value::variant(owner.clone(), case, payload))
}

fn migrate_on(
    client: &mut Client,
    migration_id: i64,
    checksum: &str,
    statement: &str,
    policy: &CallPolicy,
) -> Result<bool, ExecutionError> {
    let mut transaction = client
        .transaction()
        .map_err(|error| map_postgres_error(error, policy, false))?;
    transaction
        .batch_execute(
            "CREATE TABLE IF NOT EXISTS lkjscript_schema_migrations (\
             migration_id BIGINT PRIMARY KEY CHECK (migration_id > 0), \
             checksum TEXT NOT NULL CHECK (length(checksum) = 64)); \
             LOCK TABLE lkjscript_schema_migrations IN EXCLUSIVE MODE;",
        )
        .map_err(|error| map_postgres_error(error, policy, false))?;
    let existing = transaction
        .query_opt(
            "SELECT checksum FROM lkjscript_schema_migrations WHERE migration_id = $1",
            &[&migration_id],
        )
        .map_err(|error| map_postgres_error(error, policy, false))?;
    if let Some(row) = existing {
        let existing: String = row
            .try_get(0)
            .map_err(|error| map_postgres_error(error, policy, false))?;
        if existing != checksum {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Capability,
                "database_migration_divergence",
                format!("migration {migration_id} already exists with a different checksum"),
            ));
        }
        transaction
            .rollback()
            .map_err(|error| map_postgres_error(error, policy, false))?;
        return Ok(false);
    }
    transaction
        .batch_execute(statement)
        .map_err(|error| map_postgres_error(error, policy, false))?;
    transaction
        .execute(
            "INSERT INTO lkjscript_schema_migrations (migration_id, checksum) VALUES ($1, $2)",
            &[&migration_id, &checksum],
        )
        .map_err(|error| map_postgres_error(error, policy, false))?;
    transaction
        .commit()
        .map_err(|error| map_postgres_error(error, policy, true))?;
    Ok(true)
}

fn decode_parameters(value: &Value, owner: &OwnerId) -> Result<Vec<Parameter>, ExecutionError> {
    let Value::List(values) = value else {
        return Err(argument_error("database parameters must be a list"));
    };
    values
        .iter()
        .map(|value| decode_parameter(value, owner))
        .collect()
}

fn decode_parameter(value: &Value, owner: &OwnerId) -> Result<Parameter, ExecutionError> {
    let Value::Variant {
        owner: actual,
        case,
        payload,
    } = value
    else {
        return Err(argument_error(
            "database parameter is not a SqlValue variant",
        ));
    };
    if actual != owner {
        return Err(argument_error(
            "database parameter has a foreign nominal identity",
        ));
    }
    match (case.as_str(), payload.as_deref()) {
        ("Bool", Some(Value::Bool(value))) => Ok(Parameter::Bool(Some(*value))),
        ("I64", Some(Value::I64(value))) => Ok(Parameter::I64(Some(*value))),
        ("Text", Some(Value::Text(value))) => Ok(Parameter::Text(Some(value.to_string()))),
        ("Bytes", Some(Value::Bytes(value))) => Ok(Parameter::Bytes(Some(value.to_vec()))),
        ("NullBool", None) => Ok(Parameter::Bool(None)),
        ("NullI64", None) => Ok(Parameter::I64(None)),
        ("NullText", None) => Ok(Parameter::Text(None)),
        ("NullBytes", None) => Ok(Parameter::Bytes(None)),
        _ => Err(argument_error(
            "database parameter case or payload does not match SqlValue",
        )),
    }
}

fn decode_column_types(value: &Value, owner: &OwnerId) -> Result<Vec<ColumnType>, ExecutionError> {
    let Value::List(values) = value else {
        return Err(argument_error("database column schema must be a list"));
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
            let Value::Variant {
                owner: actual,
                case,
                payload: None,
            } = value
            else {
                return Err(argument_error("database column type is not a unit variant"));
            };
            if actual != owner {
                return Err(argument_error(
                    "database column type has a foreign nominal identity",
                ));
            }
            match case.as_str() {
                "Bool" => Ok(ColumnType::Bool),
                "I64" => Ok(ColumnType::I64),
                "Text" => Ok(ColumnType::Text),
                "Bytes" => Ok(ColumnType::Bytes),
                _ => Err(argument_error("database column type case is unknown")),
            }
        })
        .collect()
}

fn decode_maximum_rows(value: &Value, policy: &CallPolicy) -> Result<usize, ExecutionError> {
    let Value::I64(value) = value else {
        return Err(argument_error("database maximum rows must be I64"));
    };
    let maximum = usize::try_from(*value).map_err(|_| {
        argument_error("database maximum rows must be a non-negative platform-sized integer")
    })?;
    let grant_maximum = policy
        .limits
        .get("maximum_rows")
        .copied()
        .unwrap_or(MAXIMUM_DATABASE_ROWS as u64);
    if maximum == 0
        || maximum > MAXIMUM_DATABASE_ROWS
        || u64::try_from(maximum).map_or(true, |maximum| maximum > grant_maximum)
    {
        return Err(ExecutionError::resource(
            "database_row_limit",
            "database maximum rows is zero or exceeds its exact grant",
        ));
    }
    Ok(maximum)
}

fn sql_parameters(parameters: &[Parameter]) -> Vec<&(dyn ToSql + Sync)> {
    parameters.iter().map(Parameter::as_sql).collect()
}

fn static_statement(value: &Value) -> Result<&str, ExecutionError> {
    let Value::StaticText(value) = value else {
        return Err(argument_error(
            "SQL and migration text must be source-origin StaticText",
        ));
    };
    if value.is_empty() || value.len() > 1024 * 1024 || value.as_bytes().contains(&0) {
        return Err(argument_error(
            "SQL and migration text is empty, excessive, or contains NUL",
        ));
    }
    Ok(value)
}

fn contains_transaction_control(statement: &str) -> bool {
    statement
        .split(|character: char| !character.is_ascii_alphabetic())
        .any(|token| {
            matches!(
                token.to_ascii_lowercase().as_str(),
                "begin" | "commit" | "rollback" | "savepoint"
            )
        })
}

pub(crate) fn map_postgres_error(
    error: postgres::Error,
    policy: &CallPolicy,
    commit: bool,
) -> ExecutionError {
    if let Some(database) = error.as_db_error() {
        let code = database.code();
        if code == &SqlState::T_R_SERIALIZATION_FAILURE || code == &SqlState::T_R_DEADLOCK_DETECTED
        {
            let mut result = ExecutionError::new(
                ExecutionFailureClass::Capability,
                "database_retryable_transaction",
                "database transaction was rejected by a retryable concurrency class",
            );
            result.retryable = true;
            return result;
        }
        if matches!(
            code,
            &SqlState::UNIQUE_VIOLATION
                | &SqlState::FOREIGN_KEY_VIOLATION
                | &SqlState::CHECK_VIOLATION
                | &SqlState::NOT_NULL_VIOLATION
        ) {
            return ExecutionError::new(
                ExecutionFailureClass::Capability,
                "database_constraint",
                "database rejected a declared constraint",
            );
        }
        return ExecutionError::new(
            ExecutionFailureClass::Capability,
            "database_statement",
            format!("database rejected statement with SQLSTATE {}", code.code()),
        );
    }
    if commit || policy.visibility == Visibility::Possible {
        return ExecutionError::new(
            ExecutionFailureClass::PossibleVisibility,
            "database_visibility_unknown",
            "database connection failed after work may have become visible",
        );
    }
    let mut result = ExecutionError::new(
        ExecutionFailureClass::Capability,
        "database_connection",
        "database connection or protocol operation failed",
    );
    result.retryable = true;
    result
}

fn connection_error(error: postgres::Error) -> ExecutionError {
    let mut result = ExecutionError::new(
        ExecutionFailureClass::Capability,
        "database_connection",
        if error.is_closed() {
            "database connection is closed"
        } else {
            "database connection or protocol operation failed"
        },
    );
    result.retryable = true;
    result
}

fn internal_policy(
    base: &CallPolicy,
    operation: &str,
    visibility: Visibility,
    control: &ExecutionControl,
) -> CallPolicy {
    CallPolicy {
        requirement: base.requirement.clone(),
        interface: base.interface.clone(),
        operation: operation.to_owned(),
        idempotency: super::language::Idempotency::NonIdempotent,
        visibility,
        limits: base.limits.clone(),
        control: control.clone(),
    }
}

fn argument_error(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "database_adapter_argument",
        message,
    )
}

fn database_diagnostic(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn wait_unpoisoned<'a, T>(
    condition: &Condvar,
    guard: MutexGuard<'a, T>,
    duration: Duration,
) -> (MutexGuard<'a, T>, std::sync::WaitTimeoutResult) {
    match condition.wait_timeout(guard, duration) {
        Ok(result) => result,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PackageId;

    fn owner(declaration: &str) -> OwnerId {
        OwnerId::deterministic_for_test(
            PackageId::parse("1234567890abcdef1234567890abcdef").expect("package id"),
            "database",
            declaration,
        )
    }

    #[test]
    fn connection_secrets_are_redacted_and_pool_limits_are_closed() {
        let secret =
            PostgresSecret::new("postgresql://actor:private@localhost/database").expect("secret");
        assert_eq!(format!("{secret:?}"), "PostgresSecret(<redacted>)");
        let config = PostgresPoolConfig {
            connection: secret,
            maximum_connections: 0,
            maximum_wait_milliseconds: 1,
            statement_timeout_milliseconds: 1,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn parameter_values_require_exact_nominal_identity() {
        let expected = owner("SqlValue");
        let value = Value::List(Arc::new(vec![
            Value::variant(expected.clone(), "I64", Some(Value::I64(7))),
            Value::variant(expected.clone(), "NullText", None),
        ]));
        let parameters = decode_parameters(&value, &expected).expect("parameters");
        assert_eq!(parameters.len(), 2);
        assert!(decode_parameters(&value, &owner("ForeignSqlValue")).is_err());
        assert!(static_statement(&Value::text("SELECT 1")).is_err());
        assert_eq!(
            static_statement(&Value::static_text("SELECT $1")).expect("static statement"),
            "SELECT $1"
        );
    }

    #[test]
    fn migration_checksum_and_transaction_control_are_exact() {
        let statement = "CREATE TABLE example (id BIGINT PRIMARY KEY)";
        assert_eq!(
            blake3::hash(statement.as_bytes())
                .to_hex()
                .to_string()
                .len(),
            64
        );
        assert!(contains_transaction_control(
            "BEGIN; CREATE TABLE x (id BIGINT)"
        ));
        assert!(!contains_transaction_control(statement));
    }
}
