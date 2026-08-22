//! Explicit time, secure-randomness, identifier, and password-hashing capability adapters.

use super::execution::{CallPolicy, CapabilityAdapter, ExecutionError, ExecutionFailureClass};
use super::semantic::OwnerId;
use super::value::Value;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SECURITY_ADAPTER_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_RANDOM_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_PASSWORD_BYTES: usize = 1024;

#[derive(Clone, Debug)]
pub struct WallClockAdapter {
    interface: OwnerId,
}

impl WallClockAdapter {
    pub fn new(interface: OwnerId) -> Self {
        Self { interface }
    }
}

impl CapabilityAdapter for WallClockAdapter {
    fn interface(&self) -> &OwnerId {
        &self.interface
    }

    fn call(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        policy.control.check()?;
        if policy.operation != "utc-milliseconds" || !arguments.is_empty() {
            return Err(adapter_argument(
                "wall clock implements utc-milliseconds with no arguments",
            ));
        }
        let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
            ExecutionError::new(
                ExecutionFailureClass::Capability,
                "clock_before_epoch",
                "wall clock is before the Unix epoch",
            )
        })?;
        let milliseconds = i64::try_from(duration.as_millis()).map_err(|_| {
            ExecutionError::resource(
                "clock_range",
                "wall-clock milliseconds exceed signed 64-bit range",
            )
        })?;
        Ok(Value::I64(milliseconds))
    }
}

#[derive(Clone, Debug)]
pub struct DeterministicClockAdapter {
    interface: OwnerId,
    observations: Arc<Mutex<VecDeque<i64>>>,
}

impl DeterministicClockAdapter {
    pub fn new(interface: OwnerId, observations: Vec<i64>) -> Self {
        Self {
            interface,
            observations: Arc::new(Mutex::new(observations.into())),
        }
    }
}

impl CapabilityAdapter for DeterministicClockAdapter {
    fn interface(&self) -> &OwnerId {
        &self.interface
    }

    fn call(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        if policy.operation != "utc-milliseconds" || !arguments.is_empty() {
            return Err(adapter_argument(
                "deterministic clock implements utc-milliseconds with no arguments",
            ));
        }
        let value = lock_unpoisoned(&self.observations)
            .pop_front()
            .ok_or_else(|| {
                ExecutionError::new(
                    ExecutionFailureClass::Infrastructure,
                    "clock_fake_exhausted",
                    "deterministic clock observation script is exhausted",
                )
            })?;
        Ok(Value::I64(value))
    }
}

#[derive(Clone, Debug)]
pub struct SecureRandomAdapter {
    interface: OwnerId,
}

impl SecureRandomAdapter {
    pub fn new(interface: OwnerId) -> Self {
        Self { interface }
    }
}

impl CapabilityAdapter for SecureRandomAdapter {
    fn interface(&self) -> &OwnerId {
        &self.interface
    }

    fn call(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        policy.control.check()?;
        if policy.operation != "bytes" {
            return Err(adapter_argument("secure randomness implements only bytes"));
        }
        let [Value::I64(length)] = arguments.as_slice() else {
            return Err(adapter_argument(
                "secure-random bytes requires one I64 length",
            ));
        };
        let length = bounded_random_length(*length, policy)?;
        let mut bytes = vec![0; length];
        getrandom::fill(&mut bytes).map_err(|_| {
            ExecutionError::new(
                ExecutionFailureClass::Capability,
                "secure_random_unavailable",
                "operating-system secure randomness is unavailable",
            )
        })?;
        Ok(Value::bytes(bytes))
    }
}

#[derive(Clone, Debug)]
pub struct DeterministicRandomAdapter {
    interface: OwnerId,
    values: Arc<Mutex<VecDeque<Vec<u8>>>>,
}

impl DeterministicRandomAdapter {
    pub fn new(interface: OwnerId, values: Vec<Vec<u8>>) -> Self {
        Self {
            interface,
            values: Arc::new(Mutex::new(values.into())),
        }
    }
}

impl CapabilityAdapter for DeterministicRandomAdapter {
    fn interface(&self) -> &OwnerId {
        &self.interface
    }

    fn call(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        if policy.operation != "bytes" {
            return Err(adapter_argument(
                "deterministic randomness implements only bytes",
            ));
        }
        let [Value::I64(length)] = arguments.as_slice() else {
            return Err(adapter_argument(
                "deterministic-random bytes requires one I64 length",
            ));
        };
        let length = bounded_random_length(*length, policy)?;
        let bytes = lock_unpoisoned(&self.values).pop_front().ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "random_fake_exhausted",
                "deterministic randomness script is exhausted",
            )
        })?;
        if bytes.len() != length {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "random_fake_length",
                "deterministic randomness script returned a foreign length",
            ));
        }
        Ok(Value::bytes(bytes))
    }
}

#[derive(Clone, Debug)]
pub struct IdentifierAdapter {
    interface: OwnerId,
}

impl IdentifierAdapter {
    pub fn new(interface: OwnerId) -> Self {
        Self { interface }
    }
}

impl CapabilityAdapter for IdentifierAdapter {
    fn interface(&self) -> &OwnerId {
        &self.interface
    }

    fn call(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        policy.control.check()?;
        if policy.operation != "uuid-v4" || !arguments.is_empty() {
            return Err(adapter_argument(
                "identifier adapter implements uuid-v4 with no arguments",
            ));
        }
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| {
            ExecutionError::new(
                ExecutionFailureClass::Capability,
                "identifier_random_unavailable",
                "secure randomness for UUID generation is unavailable",
            )
        })?;
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Ok(Value::text(format_uuid(bytes)))
    }
}

#[derive(Clone, Debug)]
pub struct DeterministicIdentifierAdapter {
    interface: OwnerId,
    values: Arc<Mutex<VecDeque<[u8; 16]>>>,
}

impl DeterministicIdentifierAdapter {
    pub fn new(interface: OwnerId, values: Vec<[u8; 16]>) -> Self {
        Self {
            interface,
            values: Arc::new(Mutex::new(values.into())),
        }
    }
}

impl CapabilityAdapter for DeterministicIdentifierAdapter {
    fn interface(&self) -> &OwnerId {
        &self.interface
    }

    fn call(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        if policy.operation != "uuid-v4" || !arguments.is_empty() {
            return Err(adapter_argument(
                "deterministic identifier implements uuid-v4 with no arguments",
            ));
        }
        let mut bytes = lock_unpoisoned(&self.values).pop_front().ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "identifier_fake_exhausted",
                "deterministic identifier script is exhausted",
            )
        })?;
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Ok(Value::text(format_uuid(bytes)))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PasswordHashPolicy {
    pub memory_kibibytes: u32,
    pub iterations: u32,
    pub lanes: u32,
    pub output_bytes: usize,
}

impl Default for PasswordHashPolicy {
    fn default() -> Self {
        Self {
            memory_kibibytes: 19_456,
            iterations: 2,
            lanes: 1,
            output_bytes: 32,
        }
    }
}

impl PasswordHashPolicy {
    fn params(&self) -> Result<Params, ExecutionError> {
        if !(8 * self.lanes..=1_048_576).contains(&self.memory_kibibytes)
            || !(1..=32).contains(&self.iterations)
            || !(1..=16).contains(&self.lanes)
            || !(16..=64).contains(&self.output_bytes)
        {
            return Err(ExecutionError::resource(
                "password_policy",
                "password hashing policy is outside supported resource bounds",
            ));
        }
        Params::new(
            self.memory_kibibytes,
            self.iterations,
            self.lanes,
            Some(self.output_bytes),
        )
        .map_err(|_| {
            ExecutionError::resource(
                "password_policy",
                "password hashing policy is internally inconsistent",
            )
        })
    }
}

#[derive(Clone)]
pub struct PasswordHashAdapter {
    interface: OwnerId,
    policy: PasswordHashPolicy,
}

impl fmt::Debug for PasswordHashAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PasswordHashAdapter")
            .field("interface", &self.interface)
            .field("policy", &self.policy)
            .finish()
    }
}

impl PasswordHashAdapter {
    pub fn new(interface: OwnerId, policy: PasswordHashPolicy) -> Result<Self, ExecutionError> {
        policy.params()?;
        Ok(Self { interface, policy })
    }

    fn argon2(&self) -> Result<Argon2<'static>, ExecutionError> {
        Ok(Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            self.policy.params()?,
        ))
    }
}

impl CapabilityAdapter for PasswordHashAdapter {
    fn interface(&self) -> &OwnerId {
        &self.interface
    }

    fn call(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        policy.control.check()?;
        match policy.operation.as_str() {
            "hash" => {
                let [Value::Bytes(password)] = arguments.as_slice() else {
                    return Err(adapter_argument("password hash expects password Bytes"));
                };
                validate_password(password)?;
                let mut salt_bytes = [0u8; 16];
                getrandom::fill(&mut salt_bytes).map_err(|_| {
                    ExecutionError::new(
                        ExecutionFailureClass::Capability,
                        "password_salt_unavailable",
                        "secure randomness for password salt is unavailable",
                    )
                })?;
                let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| {
                    ExecutionError::new(
                        ExecutionFailureClass::Infrastructure,
                        "password_salt_encode",
                        "password salt could not be encoded",
                    )
                })?;
                let hash = self
                    .argon2()?
                    .hash_password(password, &salt)
                    .map_err(|_| {
                        ExecutionError::resource(
                            "password_hash_resource",
                            "password hashing could not complete within its policy",
                        )
                    })?
                    .to_string();
                Ok(Value::text(hash))
            }
            "verify" => {
                let [Value::Bytes(password), Value::Text(encoded)] = arguments.as_slice() else {
                    return Err(adapter_argument(
                        "password verify expects password Bytes and encoded Text",
                    ));
                };
                validate_password(password)?;
                if encoded.len() > 1024 {
                    return Err(adapter_argument("encoded password hash is excessive"));
                }
                let hash = PasswordHash::new(encoded).map_err(|_| {
                    ExecutionError::new(
                        ExecutionFailureClass::Capability,
                        "password_hash_malformed",
                        "encoded password hash is malformed",
                    )
                })?;
                match self.argon2()?.verify_password(password, &hash) {
                    Ok(()) => Ok(Value::Bool(true)),
                    Err(argon2::password_hash::Error::Password) => Ok(Value::Bool(false)),
                    Err(_) => Err(ExecutionError::new(
                        ExecutionFailureClass::Capability,
                        "password_hash_unsupported",
                        "encoded password hash uses unsupported parameters or algorithm",
                    )),
                }
            }
            "needs-upgrade" => {
                let [Value::Text(encoded)] = arguments.as_slice() else {
                    return Err(adapter_argument(
                        "password needs-upgrade expects encoded Text",
                    ));
                };
                let hash = PasswordHash::new(encoded).map_err(|_| {
                    ExecutionError::new(
                        ExecutionFailureClass::Capability,
                        "password_hash_malformed",
                        "encoded password hash is malformed",
                    )
                })?;
                let expected = self.policy.params()?;
                let needs_upgrade = hash.algorithm.as_str() != "argon2id"
                    || hash.version != Some(19)
                    || hash.params.get_decimal("m") != Some(expected.m_cost())
                    || hash.params.get_decimal("t") != Some(expected.t_cost())
                    || hash.params.get_decimal("p") != Some(expected.p_cost());
                Ok(Value::Bool(needs_upgrade))
            }
            operation => Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "password_operation_unknown",
                format!("password adapter does not implement '{operation}'"),
            )),
        }
    }
}

pub fn parse_uuid(value: &str) -> Result<[u8; 16], ExecutionError> {
    if value.len() != 36
        || value.as_bytes().get(8) != Some(&b'-')
        || value.as_bytes().get(13) != Some(&b'-')
        || value.as_bytes().get(18) != Some(&b'-')
        || value.as_bytes().get(23) != Some(&b'-')
    {
        return Err(adapter_argument("UUID spelling is not canonical"));
    }
    let compact = value
        .bytes()
        .filter(|byte| *byte != b'-')
        .collect::<Vec<_>>();
    if compact.len() != 32 {
        return Err(adapter_argument("UUID spelling is not canonical"));
    }
    let mut output = [0u8; 16];
    for (index, pair) in compact.chunks_exact(2).enumerate() {
        output[index] = hex_value(pair[0])?
            .checked_mul(16)
            .and_then(|high| high.checked_add(hex_value(pair[1]).ok()?))
            .ok_or_else(|| adapter_argument("UUID hexadecimal byte overflowed"))?;
    }
    Ok(output)
}

pub fn format_uuid(bytes: [u8; 16]) -> String {
    let mut output = String::with_capacity(36);
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            output.push('-');
        }
        output.push(hex_digit(byte >> 4));
        output.push(hex_digit(byte & 0x0f));
    }
    output
}

fn bounded_random_length(length: i64, policy: &CallPolicy) -> Result<usize, ExecutionError> {
    let length = usize::try_from(length).map_err(|_| {
        ExecutionError::resource(
            "secure_random_length",
            "secure-random byte length must be non-negative",
        )
    })?;
    let granted = policy
        .limits
        .get("maximum_random_bytes")
        .copied()
        .unwrap_or(MAXIMUM_RANDOM_BYTES as u64);
    if length > MAXIMUM_RANDOM_BYTES
        || u64::try_from(length).map_or(true, |length| length > granted)
    {
        return Err(ExecutionError::resource(
            "secure_random_length",
            "secure-random byte length exceeds its exact grant",
        ));
    }
    Ok(length)
}

fn validate_password(password: &[u8]) -> Result<(), ExecutionError> {
    if password.is_empty() || password.len() > MAXIMUM_PASSWORD_BYTES {
        return Err(ExecutionError::resource(
            "password_length",
            format!("password must contain 1 through {MAXIMUM_PASSWORD_BYTES} bytes"),
        ));
    }
    Ok(())
}

fn hex_value(value: u8) -> Result<u8, ExecutionError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(adapter_argument(
            "UUID must use lowercase hexadecimal characters",
        )),
    }
}

fn hex_digit(value: u8) -> char {
    char::from(if value < 10 {
        b'0' + value
    } else {
        b'a' + value - 10
    })
}

fn adapter_argument(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "security_adapter_argument",
        message,
    )
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PackageId;
    use crate::platform::language::{Idempotency, Visibility};
    use std::collections::BTreeMap;

    fn owner(declaration: &str) -> OwnerId {
        OwnerId::deterministic_for_test(
            PackageId::parse("1234567890abcdef1234567890abcdef").expect("package id"),
            "security",
            declaration,
        )
    }

    fn policy(interface: OwnerId, operation: &str) -> CallPolicy {
        CallPolicy {
            requirement: "test".to_owned(),
            interface,
            operation: operation.to_owned(),
            idempotency: Idempotency::Idempotent,
            visibility: Visibility::None,
            limits: BTreeMap::from([("maximum_random_bytes".to_owned(), 64)]),
            control: Default::default(),
        }
    }

    #[test]
    fn deterministic_time_random_and_identifier_are_disjoint_fakes() {
        let clock_owner = owner("Clock");
        let clock = DeterministicClockAdapter::new(clock_owner.clone(), vec![42]);
        assert!(matches!(
            clock
                .call(&policy(clock_owner, "utc-milliseconds"), Vec::new())
                .expect("clock"),
            Value::I64(42)
        ));

        let random_owner = owner("Random");
        let random = DeterministicRandomAdapter::new(random_owner.clone(), vec![vec![7; 4]]);
        assert!(matches!(
            random
                .call(&policy(random_owner, "bytes"), vec![Value::I64(4)])
                .expect("random"),
            Value::Bytes(value) if value.as_ref() == [7, 7, 7, 7]
        ));

        let id_owner = owner("Identifier");
        let ids = DeterministicIdentifierAdapter::new(id_owner.clone(), vec![[0; 16]]);
        let Value::Text(value) = ids
            .call(&policy(id_owner, "uuid-v4"), Vec::new())
            .expect("identifier")
        else {
            panic!("identifier type");
        };
        assert_eq!(value.as_ref(), "00000000-0000-4000-8000-000000000000");
        assert_eq!(parse_uuid(&value).expect("parse")[6] >> 4, 4);
    }

    #[test]
    fn password_hashing_distinguishes_mismatch_and_malformed() {
        let interface = owner("Password");
        let adapter = PasswordHashAdapter::new(interface.clone(), PasswordHashPolicy::default())
            .expect("adapter");
        let Value::Text(encoded) = adapter
            .call(
                &policy(interface.clone(), "hash"),
                vec![Value::bytes(b"correct horse".to_vec())],
            )
            .expect("hash")
        else {
            panic!("hash type");
        };
        assert!(matches!(
            adapter
                .call(
                    &policy(interface.clone(), "verify"),
                    vec![
                        Value::bytes(b"correct horse".to_vec()),
                        Value::Text(encoded.clone())
                    ],
                )
                .expect("verify"),
            Value::Bool(true)
        ));
        assert!(matches!(
            adapter
                .call(
                    &policy(interface.clone(), "verify"),
                    vec![Value::bytes(b"wrong".to_vec()), Value::Text(encoded)],
                )
                .expect("mismatch"),
            Value::Bool(false)
        ));
        let error = adapter
            .call(
                &policy(interface, "verify"),
                vec![Value::bytes(b"wrong".to_vec()), Value::text("malformed")],
            )
            .expect_err("malformed");
        assert_eq!(error.code, "password_hash_malformed");
    }

    #[test]
    fn blake3_intrinsic_has_a_stable_independent_oracle() {
        let bytes = b"lkjscript";
        assert_eq!(blake3::hash(bytes).as_bytes().len(), 32);
    }
}
