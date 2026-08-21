//! Deployment-only secret resolution with redacted diagnostics and no semantic serialization.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{CallPolicy, CapabilityAdapter, ExecutionError, ExecutionFailureClass};
use super::semantic::OwnerId;
use super::value::Value;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

pub const SECRET_CATALOG_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_SECRET_BYTES: usize = 64 * 1024;
pub const SECRET_VERIFIER_CONTRACT_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentSecretBinding {
    pub name: String,
    pub variable: String,
}

#[derive(Clone)]
pub struct SecretValue(Arc<[u8]>);

impl SecretValue {
    pub(crate) fn text(&self) -> Result<&str, Diagnostic> {
        std::str::from_utf8(&self.0)
            .map_err(|_| secret_error("secret_utf8", "secret required as text is not valid UTF-8"))
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretValue(<redacted>)")
    }
}

#[derive(Clone)]
pub struct SecretCatalog {
    values: Arc<BTreeMap<String, SecretValue>>,
}

impl fmt::Debug for SecretCatalog {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecretCatalog")
            .field("names", &self.values.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl SecretCatalog {
    pub fn from_environment(bindings: &[EnvironmentSecretBinding]) -> Result<Self, Diagnostic> {
        let mut names = BTreeSet::new();
        let mut variables = BTreeSet::new();
        let mut values = BTreeMap::new();
        for binding in bindings {
            validate_token(&binding.name, "secret name")?;
            validate_environment_name(&binding.variable)?;
            if !names.insert(binding.name.as_str()) {
                return Err(secret_error(
                    "secret_name_duplicate",
                    format!("secret name '{}' is declared twice", binding.name),
                ));
            }
            if !variables.insert(binding.variable.as_str()) {
                return Err(secret_error(
                    "secret_variable_duplicate",
                    "one environment variable may not implicitly share two secret authorities",
                ));
            }
            let value = std::env::var_os(&binding.variable).ok_or_else(|| {
                secret_error(
                    "secret_missing",
                    format!("required secret '{}' is unavailable", binding.name),
                )
            })?;
            let value = value.into_encoded_bytes();
            if value.is_empty() || value.len() > MAXIMUM_SECRET_BYTES {
                return Err(secret_error(
                    "secret_size",
                    format!(
                        "required secret '{}' is empty or exceeds {MAXIMUM_SECRET_BYTES} bytes",
                        binding.name
                    ),
                ));
            }
            values.insert(binding.name.clone(), SecretValue(Arc::from(value)));
        }
        Ok(Self {
            values: Arc::new(values),
        })
    }

    pub fn names(&self) -> Vec<String> {
        self.values.keys().cloned().collect()
    }

    pub(crate) fn require(&self, name: &str) -> Result<&SecretValue, Diagnostic> {
        self.values.get(name).ok_or_else(|| {
            secret_error(
                "secret_binding_missing",
                format!("adapter references undeclared secret '{name}'"),
            )
        })
    }
}

/// A least-authority adapter that answers equality without exposing the deployment secret as a
/// semantic value. Candidate bytes are request-owned and never retained or included in errors.
#[derive(Clone)]
pub struct SecretVerifierAdapter {
    interface: OwnerId,
    secret: SecretValue,
    maximum_candidate_bytes: usize,
}

impl fmt::Debug for SecretVerifierAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecretVerifierAdapter")
            .field("interface", &self.interface)
            .field("maximum_candidate_bytes", &self.maximum_candidate_bytes)
            .field("secret", &"<redacted>")
            .finish()
    }
}

impl SecretVerifierAdapter {
    pub fn new(
        interface: OwnerId,
        secret: SecretValue,
        maximum_candidate_bytes: usize,
    ) -> Result<Self, Diagnostic> {
        if maximum_candidate_bytes == 0 || maximum_candidate_bytes > MAXIMUM_SECRET_BYTES {
            return Err(secret_error(
                "secret_candidate_limit",
                format!("secret candidate limit must be in 1..={MAXIMUM_SECRET_BYTES} bytes"),
            ));
        }
        Ok(Self {
            interface,
            secret,
            maximum_candidate_bytes,
        })
    }
}

impl CapabilityAdapter for SecretVerifierAdapter {
    fn interface(&self) -> &OwnerId {
        &self.interface
    }

    fn call(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        if policy.operation != "matches" {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Capability,
                "secret_operation",
                "secret verifier operation is not supported",
            ));
        }
        let [Value::Bytes(candidate)] = arguments.as_slice() else {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Capability,
                "secret_argument",
                "secret verifier expects one Bytes candidate",
            ));
        };
        if candidate.len() > self.maximum_candidate_bytes {
            return Err(ExecutionError::resource(
                "secret_candidate_limit",
                "secret candidate exceeds its exact byte limit",
            ));
        }
        Ok(Value::Bool(constant_time_equal(&self.secret.0, candidate)))
    }
}

fn constant_time_equal(secret: &[u8], candidate: &[u8]) -> bool {
    let maximum = secret.len().max(candidate.len());
    let mut difference = secret.len() ^ candidate.len();
    for index in 0..maximum {
        difference |= usize::from(
            secret.get(index).copied().unwrap_or(0) ^ candidate.get(index).copied().unwrap_or(0),
        );
    }
    difference == 0
}

fn validate_token(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.is_empty()
        || value.len() > 256
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return Err(secret_error(
            "secret_token",
            format!("{label} is not a canonical token"),
        ));
    }
    Ok(())
}

fn validate_environment_name(value: &str) -> Result<(), Diagnostic> {
    if value.is_empty()
        || value.len() > 256
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        || value.as_bytes().first().is_some_and(u8::is_ascii_digit)
    {
        return Err(secret_error(
            "secret_environment_name",
            "secret environment variable name is not canonical uppercase ASCII",
        ));
    }
    Ok(())
}

fn secret_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Capability, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_debug_and_errors_do_not_disclose_values() {
        let value = SecretValue(Arc::from(b"private-value".as_slice()));
        assert_eq!(format!("{value:?}"), "SecretValue(<redacted>)");
        assert!(validate_environment_name("DATABASE_SECRET").is_ok());
        assert!(validate_environment_name("database-secret").is_err());
        assert!(constant_time_equal(b"same", b"same"));
        assert!(!constant_time_equal(b"same", b"other"));
        assert!(!constant_time_equal(b"same", b"same-longer"));
    }
}
