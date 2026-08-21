//! Requirement-to-grant binding and deterministic adapter contract.

use super::{
    ExecutionControl, ExecutionError, ExecutionFailureClass, PreparedComponent, PreparedRequirement,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::language::{Idempotency, Visibility};
use crate::platform::semantic::OwnerId;
use crate::platform::value::Value;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub const CAPABILITY_GRANT_CONTRACT_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityGrantDescriptor {
    pub contract_version: u16,
    pub interface: OwnerId,
    pub adapter_kind: String,
    pub sharing_domain: String,
    pub authority_revision: String,
    pub descriptor_digest: String,
    pub operations: BTreeSet<String>,
    pub limits: BTreeMap<String, u64>,
}

#[derive(Clone)]
pub struct CapabilityGrant {
    pub requirement: String,
    pub descriptor: CapabilityGrantDescriptor,
    pub adapter: Arc<dyn CapabilityAdapter>,
}

impl std::fmt::Debug for CapabilityGrant {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapabilityGrant")
            .field("requirement", &self.requirement)
            .field("descriptor", &self.descriptor)
            .field("adapter", &"<opaque adapter>")
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct CallPolicy {
    pub requirement: String,
    pub interface: OwnerId,
    pub operation: String,
    pub idempotency: Idempotency,
    pub visibility: Visibility,
    pub limits: BTreeMap<String, u64>,
    pub control: ExecutionControl,
}

pub trait CapabilityAdapter: Send + Sync {
    fn interface(&self) -> &OwnerId;

    fn call(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError>;

    fn begin_transaction(
        &self,
        _policy: &CallPolicy,
    ) -> Result<Box<dyn CapabilityTransaction>, ExecutionError> {
        Err(ExecutionError::new(
            ExecutionFailureClass::Capability,
            "capability_transaction_unsupported",
            "capability adapter does not support transaction scope",
        ))
    }

    /// Releases deployment-owned live resources after admission has stopped and task scopes have
    /// drained. Implementations must make repeated calls idempotent.
    fn shutdown(&self) -> Result<(), ExecutionError> {
        Ok(())
    }
}

/// A transaction is task-scoped and never represented by `Value`. Implementations must make
/// `rollback` idempotent and must also arrange best-effort rollback from their own `Drop` path.
pub trait CapabilityTransaction: Send {
    fn call(&mut self, policy: &CallPolicy, arguments: Vec<Value>)
    -> Result<Value, ExecutionError>;

    fn commit(&mut self) -> Result<(), ExecutionError>;

    fn rollback(&mut self) -> Result<(), ExecutionError>;
}

#[derive(Clone)]
pub struct BoundCapabilities {
    bindings: Arc<BTreeMap<String, BoundCapability>>,
}

impl std::fmt::Debug for BoundCapabilities {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BoundCapabilities")
            .field("aliases", &self.bindings.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[derive(Clone)]
struct BoundCapability {
    requirement: PreparedRequirement,
    descriptor: CapabilityGrantDescriptor,
    adapter: Arc<dyn CapabilityAdapter>,
    calls: Arc<AtomicU64>,
    input_bytes: Arc<AtomicU64>,
    output_bytes: Arc<AtomicU64>,
}

impl BoundCapabilities {
    pub fn bind(
        component: &PreparedComponent,
        grants: Vec<CapabilityGrant>,
    ) -> Result<Self, Diagnostic> {
        let mut supplied = BTreeMap::new();
        for grant in grants {
            if supplied.insert(grant.requirement.clone(), grant).is_some() {
                return Err(binding_error(
                    "grant_requirement_duplicate",
                    "two grants target one component requirement",
                ));
            }
        }
        let mut bindings = BTreeMap::new();
        for (alias, requirement) in &component.requirements {
            let grant = supplied.remove(alias).ok_or_else(|| {
                binding_error(
                    "grant_requirement_missing",
                    format!("requirement '{alias}' has no deployment grant"),
                )
            })?;
            validate_grant(requirement, &grant)?;
            bindings.insert(
                alias.clone(),
                BoundCapability {
                    requirement: requirement.clone(),
                    descriptor: grant.descriptor,
                    adapter: grant.adapter,
                    calls: Arc::new(AtomicU64::new(0)),
                    input_bytes: Arc::new(AtomicU64::new(0)),
                    output_bytes: Arc::new(AtomicU64::new(0)),
                },
            );
        }
        if let Some((alias, _)) = supplied.into_iter().next() {
            return Err(binding_error(
                "grant_requirement_foreign",
                format!("grant targets undeclared requirement '{alias}'"),
            ));
        }
        Ok(Self {
            bindings: Arc::new(bindings),
        })
    }

    /// Creates the counters for one admitted task while retaining the exact adapter bindings.
    /// Deployment-wide concurrency and queue limits are owned by the resident runtime.
    pub fn task_scope(&self) -> Self {
        let bindings = self
            .bindings
            .iter()
            .map(|(alias, binding)| {
                (
                    alias.clone(),
                    BoundCapability {
                        requirement: binding.requirement.clone(),
                        descriptor: binding.descriptor.clone(),
                        adapter: binding.adapter.clone(),
                        calls: Arc::new(AtomicU64::new(0)),
                        input_bytes: Arc::new(AtomicU64::new(0)),
                        output_bytes: Arc::new(AtomicU64::new(0)),
                    },
                )
            })
            .collect();
        Self {
            bindings: Arc::new(bindings),
        }
    }

    pub fn call(
        &self,
        alias: &str,
        operation: &str,
        arguments: Vec<Value>,
    ) -> Result<Value, ExecutionError> {
        self.call_controlled(
            alias,
            operation,
            arguments,
            &ExecutionControl::uncancelled(),
        )
    }

    pub fn call_controlled(
        &self,
        alias: &str,
        operation: &str,
        arguments: Vec<Value>,
        control: &ExecutionControl,
    ) -> Result<Value, ExecutionError> {
        control.check()?;
        let binding = self.binding(alias, operation)?;
        binding.account_call(&arguments)?;
        let policy = binding.policy(operation, control)?;
        let result = binding.adapter.call(&policy, arguments);
        binding.validate_outcome(&policy, result)
    }

    pub fn begin_transaction(&self, alias: &str) -> Result<BoundTransaction, ExecutionError> {
        self.begin_transaction_controlled(alias, &ExecutionControl::uncancelled())
    }

    pub fn begin_transaction_controlled(
        &self,
        alias: &str,
        control: &ExecutionControl,
    ) -> Result<BoundTransaction, ExecutionError> {
        control.check()?;
        let binding = self.binding(alias, "transaction")?.clone();
        binding.account_call(&[])?;
        let policy = binding.policy("transaction", control)?;
        let transaction = binding.adapter.begin_transaction(&policy)?;
        control.check()?;
        Ok(BoundTransaction {
            binding,
            transaction: Some(transaction),
            completed: false,
            control: control.clone(),
        })
    }

    pub fn shutdown(&self) -> Vec<ExecutionError> {
        self.bindings
            .values()
            .filter_map(|binding| binding.adapter.shutdown().err())
            .collect()
    }

    fn binding(&self, alias: &str, operation: &str) -> Result<&BoundCapability, ExecutionError> {
        let binding = self.bindings.get(alias).ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "capability_alias_unbound",
                format!("capability alias '{alias}' is not bound"),
            )
        })?;
        if !binding.requirement.operations.contains_key(operation) {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "capability_operation_unbound",
                format!("operation '{alias}.{operation}' is not granted"),
            ));
        }
        Ok(binding)
    }
}

pub struct BoundTransaction {
    binding: BoundCapability,
    transaction: Option<Box<dyn CapabilityTransaction>>,
    completed: bool,
    control: ExecutionControl,
}

impl BoundTransaction {
    pub fn call(
        &mut self,
        operation: &str,
        arguments: Vec<Value>,
    ) -> Result<Value, ExecutionError> {
        self.control.check()?;
        if operation == "transaction" {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "capability_nested_transaction",
                "nested transaction operation is not available through a transaction binding",
            ));
        }
        self.binding.account_call(&arguments)?;
        let policy = self.binding.policy(operation, &self.control)?;
        let result = self
            .transaction
            .as_mut()
            .ok_or_else(transaction_completed)?
            .call(&policy, arguments);
        self.binding.validate_outcome(&policy, result)
    }

    pub fn commit(mut self) -> Result<(), ExecutionError> {
        self.control.check()?;
        let result = self
            .transaction
            .as_mut()
            .ok_or_else(transaction_completed)?
            .commit();
        if result.is_ok() {
            self.completed = true;
            self.transaction = None;
        }
        result
    }

    pub fn rollback(mut self) -> Result<(), ExecutionError> {
        let result = self
            .transaction
            .as_mut()
            .ok_or_else(transaction_completed)?
            .rollback();
        self.completed = true;
        self.transaction = None;
        result
    }
}

impl Drop for BoundTransaction {
    fn drop(&mut self) {
        if !self.completed {
            if let Some(transaction) = self.transaction.as_mut() {
                let _ = transaction.rollback();
            }
            self.completed = true;
        }
    }
}

impl BoundCapability {
    fn policy(
        &self,
        operation: &str,
        control: &ExecutionControl,
    ) -> Result<CallPolicy, ExecutionError> {
        let contract = self.requirement.operations.get(operation).ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "capability_operation_contract",
                "bound operation lost its interface contract",
            )
        })?;
        Ok(CallPolicy {
            requirement: self.requirement.alias.clone(),
            interface: self.requirement.interface.clone(),
            operation: operation.to_owned(),
            idempotency: contract.idempotency,
            visibility: contract.visibility,
            limits: self.descriptor.limits.clone(),
            control: control.clone(),
        })
    }

    fn account_call(&self, arguments: &[Value]) -> Result<(), ExecutionError> {
        add_bounded(
            &self.calls,
            1,
            self.descriptor.limits.get("maximum_calls").copied(),
            "capability_call_limit",
        )?;
        let bytes = arguments
            .iter()
            .try_fold(0u64, |total, value| {
                total.checked_add(logical_bytes(value)?).ok_or(())
            })
            .map_err(|()| {
                ExecutionError::resource(
                    "capability_input_length",
                    "capability input accounting overflowed",
                )
            })?;
        add_bounded(
            &self.input_bytes,
            bytes,
            self.descriptor.limits.get("maximum_input_bytes").copied(),
            "capability_input_limit",
        )
    }

    fn validate_outcome(
        &self,
        policy: &CallPolicy,
        result: Result<Value, ExecutionError>,
    ) -> Result<Value, ExecutionError> {
        match result {
            Ok(value) => {
                let bytes = logical_bytes(&value).map_err(|()| {
                    ExecutionError::resource(
                        "capability_output_length",
                        "capability output accounting overflowed",
                    )
                })?;
                add_bounded(
                    &self.output_bytes,
                    bytes,
                    self.descriptor.limits.get("maximum_output_bytes").copied(),
                    "capability_output_limit",
                )?;
                Ok(value)
            }
            Err(error)
                if error.class == ExecutionFailureClass::PossibleVisibility
                    && policy.visibility != Visibility::Possible =>
            {
                Err(ExecutionError::new(
                    ExecutionFailureClass::Infrastructure,
                    "capability_visibility_contract",
                    "adapter reported possible visibility for an operation that forbids it",
                ))
            }
            Err(error) => Err(error),
        }
    }
}

fn validate_grant(
    requirement: &PreparedRequirement,
    grant: &CapabilityGrant,
) -> Result<(), Diagnostic> {
    let descriptor = &grant.descriptor;
    if descriptor.contract_version != CAPABILITY_GRANT_CONTRACT_VERSION {
        return Err(binding_error(
            "grant_contract",
            "capability grant has a predecessor or foreign contract",
        ));
    }
    if descriptor.interface != requirement.interface
        || grant.adapter.interface() != &requirement.interface
    {
        return Err(binding_error(
            "grant_interface",
            "capability grant or adapter has a foreign interface identity",
        ));
    }
    validate_token(&descriptor.adapter_kind, "adapter kind")?;
    validate_token(&descriptor.sharing_domain, "sharing domain")?;
    validate_hex(&descriptor.authority_revision, "authority revision")?;
    validate_hex(&descriptor.descriptor_digest, "descriptor digest")?;
    let required: BTreeSet<_> = requirement.operations.keys().cloned().collect();
    if !required.is_subset(&descriptor.operations) {
        return Err(binding_error(
            "grant_operations",
            "capability grant omits a required operation",
        ));
    }
    for (name, requested_maximum) in &requirement.limits {
        let granted = descriptor.limits.get(name).ok_or_else(|| {
            binding_error(
                "grant_limit_missing",
                format!("capability grant omits required limit '{name}'"),
            )
        })?;
        if granted > requested_maximum {
            return Err(binding_error(
                "grant_limit_excess",
                format!(
                    "capability grant limit '{name}' value {granted} exceeds requested maximum {requested_maximum}"
                ),
            ));
        }
    }
    Ok(())
}

fn add_bounded(
    counter: &AtomicU64,
    amount: u64,
    maximum: Option<u64>,
    code: &'static str,
) -> Result<(), ExecutionError> {
    let mut current = counter.load(Ordering::Relaxed);
    loop {
        let next = current.checked_add(amount).ok_or_else(|| {
            ExecutionError::resource(code, "capability resource counter overflowed")
        })?;
        if maximum.is_some_and(|maximum| next > maximum) {
            return Err(ExecutionError::resource(
                code,
                "capability grant resource limit was exceeded",
            ));
        }
        match counter.compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Relaxed) {
            Ok(_) => return Ok(()),
            Err(actual) => current = actual,
        }
    }
}

fn logical_bytes(value: &Value) -> Result<u64, ()> {
    match value {
        Value::Unit => Ok(0),
        Value::Bool(_) => Ok(1),
        Value::I64(_) => Ok(8),
        Value::Bytes(value) => u64::try_from(value.len()).map_err(|_| ()),
        Value::Text(value) => u64::try_from(value.len()).map_err(|_| ()),
        Value::StaticText(value) => u64::try_from(value.len()).map_err(|_| ()),
        Value::Record { fields, .. } => fields.values().try_fold(0u64, |total, value| {
            total.checked_add(logical_bytes(value)?).ok_or(())
        }),
        Value::Variant { payload, .. } => payload
            .as_ref()
            .map_or(Ok(0), |payload| logical_bytes(payload)),
        Value::List(values) => values.iter().try_fold(0u64, |total, value| {
            total.checked_add(logical_bytes(value)?).ok_or(())
        }),
        Value::Map(values) => values.iter().try_fold(0u64, |total, (key, value)| {
            let key_bytes = match key {
                crate::platform::value::MapKey::Bool(_) => 1,
                crate::platform::value::MapKey::I64(_) => 8,
                crate::platform::value::MapKey::Bytes(value) => {
                    u64::try_from(value.len()).map_err(|_| ())?
                }
                crate::platform::value::MapKey::Text(value) => {
                    u64::try_from(value.len()).map_err(|_| ())?
                }
            };
            total
                .checked_add(key_bytes)
                .and_then(|total| total.checked_add(logical_bytes(value).ok()?))
                .ok_or(())
        }),
        Value::Function(_) | Value::Resource { .. } => Ok(0),
    }
}

fn validate_token(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(binding_error(
            "grant_token",
            format!("{label} is not a canonical bounded token"),
        ));
    }
    Ok(())
}

fn validate_hex(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(binding_error(
            "grant_digest",
            format!("{label} must be 64 lowercase hexadecimal characters"),
        ));
    }
    Ok(())
}

fn transaction_completed() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "capability_transaction_completed",
        "transaction scope has already completed",
    )
}

fn binding_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Capability, code, message)
}

#[derive(Clone, Debug)]
pub struct ScriptedCall {
    pub operation: String,
    pub result: Result<Value, ExecutionError>,
}

#[derive(Debug)]
pub struct ScriptedAdapter {
    interface: OwnerId,
    script: Mutex<VecDeque<ScriptedCall>>,
    transactions: Mutex<VecDeque<VecDeque<ScriptedCall>>>,
    observed: Arc<Mutex<Vec<String>>>,
}

impl ScriptedAdapter {
    pub fn new(interface: OwnerId, script: Vec<ScriptedCall>) -> Self {
        Self {
            interface,
            script: Mutex::new(script.into()),
            transactions: Mutex::new(VecDeque::new()),
            observed: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_transactions(
        interface: OwnerId,
        script: Vec<ScriptedCall>,
        transactions: Vec<Vec<ScriptedCall>>,
    ) -> Self {
        Self {
            interface,
            script: Mutex::new(script.into()),
            transactions: Mutex::new(transactions.into_iter().map(VecDeque::from).collect()),
            observed: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn observed(&self) -> Vec<String> {
        self.observed.lock().map_or_else(
            |poisoned| poisoned.into_inner().clone(),
            |value| value.clone(),
        )
    }

    pub fn remaining(&self) -> usize {
        self.script
            .lock()
            .map_or_else(|poisoned| poisoned.into_inner().len(), |value| value.len())
    }
}

impl CapabilityAdapter for ScriptedAdapter {
    fn interface(&self) -> &OwnerId {
        &self.interface
    }

    fn call(&self, policy: &CallPolicy, _arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        self.observed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(policy.operation.clone());
        let call = self
            .script
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop_front()
            .ok_or_else(|| {
                ExecutionError::new(
                    ExecutionFailureClass::Infrastructure,
                    "fake_script_exhausted",
                    "deterministic fake received an unexpected call",
                )
            })?;
        if call.operation != policy.operation {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "fake_script_operation",
                format!(
                    "deterministic fake expected operation '{}' but observed '{}'",
                    call.operation, policy.operation
                ),
            ));
        }
        call.result
    }

    fn begin_transaction(
        &self,
        policy: &CallPolicy,
    ) -> Result<Box<dyn CapabilityTransaction>, ExecutionError> {
        self.observed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(format!("{}.begin", policy.operation));
        let script = self
            .transactions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop_front()
            .ok_or_else(|| {
                ExecutionError::new(
                    ExecutionFailureClass::Infrastructure,
                    "fake_transaction_exhausted",
                    "deterministic fake received an unexpected transaction",
                )
            })?;
        Ok(Box::new(ScriptedTransaction {
            script,
            observed: self.observed.clone(),
            completed: false,
        }))
    }
}

struct ScriptedTransaction {
    script: VecDeque<ScriptedCall>,
    observed: Arc<Mutex<Vec<String>>>,
    completed: bool,
}

impl CapabilityTransaction for ScriptedTransaction {
    fn call(
        &mut self,
        policy: &CallPolicy,
        _arguments: Vec<Value>,
    ) -> Result<Value, ExecutionError> {
        self.observed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(format!("transaction.{}", policy.operation));
        let call = self.script.pop_front().ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "fake_transaction_script_exhausted",
                "deterministic transaction fake received an unexpected call",
            )
        })?;
        if call.operation != policy.operation {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "fake_transaction_operation",
                format!(
                    "deterministic transaction fake expected '{}' but observed '{}'",
                    call.operation, policy.operation
                ),
            ));
        }
        call.result
    }

    fn commit(&mut self) -> Result<(), ExecutionError> {
        if !self.script.is_empty() {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "fake_transaction_incomplete",
                "deterministic transaction committed before consuming its script",
            ));
        }
        self.observed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push("transaction.commit".to_owned());
        self.completed = true;
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), ExecutionError> {
        if !self.completed {
            self.observed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push("transaction.rollback".to_owned());
            self.completed = true;
        }
        Ok(())
    }
}

impl Drop for ScriptedTransaction {
    fn drop(&mut self) {
        let _ = self.rollback();
    }
}
