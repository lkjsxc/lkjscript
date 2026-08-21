//! Typed deployment configuration without ambient environment reads in application code.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{CallPolicy, CapabilityAdapter, ExecutionError, ExecutionFailureClass};
use super::semantic::OwnerId;
use super::value::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

pub const CONFIGURATION_ADAPTER_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_CONFIGURATION_FIELDS: usize = 4_096;
pub const MAXIMUM_CONFIGURATION_VALUE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ConfigurationValue {
    Text(String),
    I64(i64),
    Bool(bool),
}

impl ConfigurationValue {
    fn kind(&self) -> &'static str {
        match self {
            Self::Text(_) => "text",
            Self::I64(_) => "i64",
            Self::Bool(_) => "bool",
        }
    }

    fn byte_length(&self) -> usize {
        match self {
            Self::Text(value) => value.len(),
            Self::I64(value) => value.to_string().len(),
            Self::Bool(value) => value.to_string().len(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationObservation {
    pub contract_version: u16,
    pub fields: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct ConfigurationAdapter {
    interface: OwnerId,
    values: Arc<BTreeMap<String, ConfigurationValue>>,
}

impl ConfigurationAdapter {
    pub fn new(
        interface: OwnerId,
        values: BTreeMap<String, ConfigurationValue>,
    ) -> Result<Self, Diagnostic> {
        if values.len() > MAXIMUM_CONFIGURATION_FIELDS {
            return Err(configuration_diagnostic(
                "configuration_field_limit",
                format!("configuration has more than {MAXIMUM_CONFIGURATION_FIELDS} fields"),
            ));
        }
        for (name, value) in &values {
            validate_name(name).map_err(|error| {
                configuration_diagnostic("configuration_field_name", error.message)
            })?;
            if value.byte_length() > MAXIMUM_CONFIGURATION_VALUE_BYTES {
                return Err(configuration_diagnostic(
                    "configuration_value_limit",
                    format!("configuration field '{name}' exceeds its byte limit"),
                ));
            }
        }
        Ok(Self {
            interface,
            values: Arc::new(values),
        })
    }

    pub fn observe_redacted(&self) -> ConfigurationObservation {
        ConfigurationObservation {
            contract_version: CONFIGURATION_ADAPTER_CONTRACT_VERSION,
            fields: self
                .values
                .iter()
                .map(|(name, value)| (name.clone(), value.kind().to_owned()))
                .collect(),
        }
    }
}

impl CapabilityAdapter for ConfigurationAdapter {
    fn interface(&self) -> &OwnerId {
        &self.interface
    }

    fn call(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        policy.control.check()?;
        let [Value::StaticText(name)] = arguments.as_slice() else {
            return Err(configuration_argument(
                "configuration operation expects one source-origin StaticText field name",
            ));
        };
        validate_name(name)?;
        if policy.operation == "exists" {
            return Ok(Value::Bool(self.values.contains_key(name.as_ref())));
        }
        let value = self.values.get(name.as_ref()).ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Capability,
                "configuration_missing",
                format!("required configuration field '{name}' is absent"),
            )
        })?;
        match (policy.operation.as_str(), value) {
            ("text", ConfigurationValue::Text(value)) => Ok(Value::text(value.clone())),
            ("i64", ConfigurationValue::I64(value)) => Ok(Value::I64(*value)),
            ("bool", ConfigurationValue::Bool(value)) => Ok(Value::Bool(*value)),
            ("text" | "i64" | "bool", _) => Err(ExecutionError::new(
                ExecutionFailureClass::Capability,
                "configuration_type",
                format!("configuration field '{name}' has a different declared type"),
            )),
            (operation, _) => Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "configuration_operation_unknown",
                format!("configuration adapter does not implement '{operation}'"),
            )),
        }
    }
}

fn validate_name(name: &str) -> Result<(), ExecutionError> {
    if name.is_empty()
        || name.len() > 256
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return Err(configuration_argument(
            "configuration field name is not a canonical token",
        ));
    }
    Ok(())
}

fn configuration_argument(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Capability,
        "configuration_argument",
        message,
    )
}

fn configuration_diagnostic(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PackageId;
    use crate::platform::execution::ExecutionControl;
    use crate::platform::language::{Idempotency, Visibility};

    fn owner() -> OwnerId {
        OwnerId {
            package: PackageId::parse("1234567890abcdef1234567890abcdef").expect("package id"),
            module: "configuration".to_owned(),
            declaration: "Configuration".to_owned(),
        }
    }

    fn policy(operation: &str) -> CallPolicy {
        CallPolicy {
            requirement: "config".to_owned(),
            interface: owner(),
            operation: operation.to_owned(),
            idempotency: Idempotency::Idempotent,
            visibility: Visibility::None,
            limits: BTreeMap::new(),
            control: ExecutionControl::uncancelled(),
        }
    }

    #[test]
    fn exact_types_and_source_origin_names_are_enforced() {
        let adapter = ConfigurationAdapter::new(
            owner(),
            BTreeMap::from([
                (
                    "service-title".to_owned(),
                    ConfigurationValue::Text("Journal".to_owned()),
                ),
                ("workers".to_owned(), ConfigurationValue::I64(4)),
            ]),
        )
        .expect("adapter");
        let value = adapter
            .call(&policy("text"), vec![Value::static_text("service-title")])
            .expect("text value");
        assert!(matches!(value, Value::Text(value) if value.as_ref() == "Journal"));
        let dynamic = adapter
            .call(&policy("text"), vec![Value::text("service-title")])
            .expect_err("dynamic key rejects");
        assert_eq!(dynamic.code, "configuration_argument");
        assert_eq!(adapter.observe_redacted().fields["service-title"], "text");
    }
}
