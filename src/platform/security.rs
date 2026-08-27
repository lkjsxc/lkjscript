//! Representation-neutral time, randomness, identifier, and password-hashing host mechanisms.

use super::execution::{ExecutionError, ExecutionFailureClass};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::collections::VecDeque;
#[cfg(test)]
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SECURITY_ADAPTER_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_RANDOM_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_PASSWORD_BYTES: usize = 1024;

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

#[derive(Clone, Debug)]
pub(crate) struct PasswordHashEngine {
    policy: PasswordHashPolicy,
}

impl PasswordHashEngine {
    pub(crate) fn new(policy: PasswordHashPolicy) -> Result<Self, ExecutionError> {
        policy.params()?;
        Ok(Self { policy })
    }

    fn argon2(&self) -> Result<Argon2<'static>, ExecutionError> {
        Ok(Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            self.policy.params()?,
        ))
    }

    pub(crate) fn hash(&self, password: &[u8]) -> Result<String, ExecutionError> {
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
        self.argon2()?
            .hash_password(password, &salt)
            .map_err(|_| {
                ExecutionError::resource(
                    "password_hash_resource",
                    "password hashing could not complete within its policy",
                )
            })
            .map(|hash| hash.to_string())
    }

    pub(crate) fn verify(&self, password: &[u8], encoded: &str) -> Result<bool, ExecutionError> {
        validate_password(password)?;
        if encoded.len() > 1024 {
            return Err(adapter_argument("encoded password hash is excessive"));
        }
        let hash = parse_password_hash(encoded)?;
        match self.argon2()?.verify_password(password, &hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(_) => Err(ExecutionError::new(
                ExecutionFailureClass::Capability,
                "password_hash_unsupported",
                "encoded password hash uses unsupported parameters or algorithm",
            )),
        }
    }

    pub(crate) fn needs_upgrade(&self, encoded: &str) -> Result<bool, ExecutionError> {
        let hash = parse_password_hash(encoded)?;
        let expected = self.policy.params()?;
        Ok(hash.algorithm.as_str() != "argon2id"
            || hash.version != Some(19)
            || hash.params.get_decimal("m") != Some(expected.m_cost())
            || hash.params.get_decimal("t") != Some(expected.t_cost())
            || hash.params.get_decimal("p") != Some(expected.p_cost()))
    }
}

fn parse_password_hash(encoded: &str) -> Result<PasswordHash<'_>, ExecutionError> {
    PasswordHash::new(encoded).map_err(|_| {
        ExecutionError::new(
            ExecutionFailureClass::Capability,
            "password_hash_malformed",
            "encoded password hash is malformed",
        )
    })
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub(crate) struct DeterministicClockSource {
    observations: Arc<Mutex<VecDeque<i64>>>,
}

#[cfg(test)]
impl DeterministicClockSource {
    pub(crate) fn new(observations: Vec<i64>) -> Self {
        Self {
            observations: Arc::new(Mutex::new(observations.into())),
        }
    }

    pub(crate) fn next(&self) -> Result<i64, ExecutionError> {
        lock_unpoisoned(&self.observations)
            .pop_front()
            .ok_or_else(|| {
                ExecutionError::new(
                    ExecutionFailureClass::Infrastructure,
                    "clock_fake_exhausted",
                    "deterministic clock observation script is exhausted",
                )
            })
    }
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub(crate) struct DeterministicRandomSource {
    values: Arc<Mutex<VecDeque<Vec<u8>>>>,
}

#[cfg(test)]
impl DeterministicRandomSource {
    pub(crate) fn new(values: Vec<Vec<u8>>) -> Self {
        Self {
            values: Arc::new(Mutex::new(values.into())),
        }
    }

    pub(crate) fn next(&self, length: i64, granted: u64) -> Result<Vec<u8>, ExecutionError> {
        let length = bounded_random_length(length, granted)?;
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
        Ok(bytes)
    }
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub(crate) struct DeterministicIdentifierSource {
    values: Arc<Mutex<VecDeque<[u8; 16]>>>,
}

#[cfg(test)]
impl DeterministicIdentifierSource {
    pub(crate) fn new(values: Vec<[u8; 16]>) -> Self {
        Self {
            values: Arc::new(Mutex::new(values.into())),
        }
    }

    pub(crate) fn next(&self) -> Result<String, ExecutionError> {
        let bytes = lock_unpoisoned(&self.values).pop_front().ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "identifier_fake_exhausted",
                "deterministic identifier script is exhausted",
            )
        })?;
        Ok(format_uuid(uuid_v4_bytes(bytes)))
    }
}

pub(crate) fn wall_clock_milliseconds() -> Result<i64, ExecutionError> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        ExecutionError::new(
            ExecutionFailureClass::Capability,
            "clock_before_epoch",
            "wall clock is before the Unix epoch",
        )
    })?;
    i64::try_from(duration.as_millis()).map_err(|_| {
        ExecutionError::resource(
            "clock_range",
            "wall-clock milliseconds exceed signed 64-bit range",
        )
    })
}

pub(crate) fn secure_random_bytes(length: i64, granted: u64) -> Result<Vec<u8>, ExecutionError> {
    let length = bounded_random_length(length, granted)?;
    let mut bytes = vec![0; length];
    getrandom::fill(&mut bytes).map_err(|_| {
        ExecutionError::new(
            ExecutionFailureClass::Capability,
            "secure_random_unavailable",
            "operating-system secure randomness is unavailable",
        )
    })?;
    Ok(bytes)
}

pub(crate) fn secure_identifier() -> Result<String, ExecutionError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| {
        ExecutionError::new(
            ExecutionFailureClass::Capability,
            "identifier_random_unavailable",
            "secure randomness for UUID generation is unavailable",
        )
    })?;
    Ok(format_uuid(uuid_v4_bytes(bytes)))
}

fn uuid_v4_bytes(mut bytes: [u8; 16]) -> [u8; 16] {
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes
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
    for (index, pair) in compact.as_chunks::<2>().0.iter().enumerate() {
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

fn bounded_random_length(length: i64, granted: u64) -> Result<usize, ExecutionError> {
    let length = usize::try_from(length).map_err(|_| {
        ExecutionError::resource(
            "secure_random_length",
            "secure-random byte length must be non-negative",
        )
    })?;
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

#[cfg(test)]
fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_time_random_and_identifier_sources_are_disjoint() {
        let clock = DeterministicClockSource::new(vec![42]);
        assert_eq!(clock.next().expect("clock"), 42);
        assert_eq!(
            clock.next().expect_err("clock script exhaustion").code,
            "clock_fake_exhausted"
        );

        let random = DeterministicRandomSource::new(vec![vec![7; 4]]);
        assert_eq!(random.next(4, 64).expect("random"), vec![7; 4]);
        assert_eq!(
            random
                .next(4, 64)
                .expect_err("random script exhaustion")
                .code,
            "random_fake_exhausted"
        );

        let identifiers = DeterministicIdentifierSource::new(vec![[0; 16]]);
        let identifier = identifiers.next().expect("identifier");
        assert_eq!(identifier, "00000000-0000-4000-8000-000000000000");
        assert_eq!(parse_uuid(&identifier).expect("parse")[6] >> 4, 4);
    }

    #[test]
    fn password_engine_distinguishes_mismatch_and_malformed() {
        let engine = PasswordHashEngine::new(PasswordHashPolicy::default()).expect("engine");
        let encoded = engine.hash(b"correct horse").expect("hash");
        assert!(engine.verify(b"correct horse", &encoded).expect("verify"));
        assert!(!engine.verify(b"wrong", &encoded).expect("mismatch"));
        assert_eq!(
            engine
                .verify(b"wrong", "malformed")
                .expect_err("malformed hash")
                .code,
            "password_hash_malformed"
        );
    }

    #[test]
    fn random_and_password_limits_are_bounded() {
        assert_eq!(
            bounded_random_length(-1, 64)
                .expect_err("negative random length")
                .code,
            "secure_random_length"
        );
        assert_eq!(
            validate_password(&[]).expect_err("empty password").code,
            "password_length"
        );
    }
}
