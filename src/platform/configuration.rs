//! Typed deployment configuration without ambient environment reads in application code.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{ExecutionError, ExecutionFailureClass};
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConfigurationOperation {
    Exists,
    Text,
    I64,
    Bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ConfigurationOutput {
    Text(String),
    I64(i64),
    Bool(bool),
}

#[derive(Clone, Debug)]
pub(crate) struct ConfigurationStore {
    values: Arc<BTreeMap<String, ConfigurationValue>>,
}

impl ConfigurationStore {
    pub(crate) fn new(values: BTreeMap<String, ConfigurationValue>) -> Result<Self, Diagnostic> {
        validate_values(&values)?;
        Ok(Self {
            values: Arc::new(values),
        })
    }

    pub(crate) fn observe_values(
        values: &BTreeMap<String, ConfigurationValue>,
    ) -> Result<ConfigurationObservation, Diagnostic> {
        validate_values(values)?;
        Ok(observation(values))
    }

    pub(crate) fn execute(
        &self,
        operation: ConfigurationOperation,
        name: &str,
    ) -> Result<ConfigurationOutput, ExecutionError> {
        validate_name(name)?;
        if operation == ConfigurationOperation::Exists {
            return Ok(ConfigurationOutput::Bool(self.values.contains_key(name)));
        }
        let value = self.values.get(name).ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Capability,
                "configuration_missing",
                format!("required configuration field '{name}' is absent"),
            )
        })?;
        match (operation, value) {
            (ConfigurationOperation::Text, ConfigurationValue::Text(value)) => {
                Ok(ConfigurationOutput::Text(value.clone()))
            }
            (ConfigurationOperation::I64, ConfigurationValue::I64(value)) => {
                Ok(ConfigurationOutput::I64(*value))
            }
            (ConfigurationOperation::Bool, ConfigurationValue::Bool(value)) => {
                Ok(ConfigurationOutput::Bool(*value))
            }
            (
                ConfigurationOperation::Text
                | ConfigurationOperation::I64
                | ConfigurationOperation::Bool,
                _,
            ) => Err(ExecutionError::new(
                ExecutionFailureClass::Capability,
                "configuration_type",
                format!("configuration field '{name}' has a different declared type"),
            )),
            (ConfigurationOperation::Exists, _) => Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "configuration_operation_state",
                "configuration exists operation reached a value-only branch",
            )),
        }
    }
}

fn validate_values(values: &BTreeMap<String, ConfigurationValue>) -> Result<(), Diagnostic> {
    if values.len() > MAXIMUM_CONFIGURATION_FIELDS {
        return Err(configuration_diagnostic(
            "configuration_field_limit",
            format!("configuration has more than {MAXIMUM_CONFIGURATION_FIELDS} fields"),
        ));
    }
    for (name, value) in values {
        validate_name(name)
            .map_err(|error| configuration_diagnostic("configuration_field_name", error.message))?;
        if value.byte_length() > MAXIMUM_CONFIGURATION_VALUE_BYTES {
            return Err(configuration_diagnostic(
                "configuration_value_limit",
                format!("configuration field '{name}' exceeds its byte limit"),
            ));
        }
    }
    Ok(())
}

fn observation(values: &BTreeMap<String, ConfigurationValue>) -> ConfigurationObservation {
    ConfigurationObservation {
        contract_version: CONFIGURATION_ADAPTER_CONTRACT_VERSION,
        fields: values
            .iter()
            .map(|(name, value)| (name.clone(), value.kind().to_owned()))
            .collect(),
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

    #[test]
    fn exact_types_and_source_origin_names_are_enforced() {
        let store = ConfigurationStore::new(BTreeMap::from([
            (
                "service-title".to_owned(),
                ConfigurationValue::Text("Journal".to_owned()),
            ),
            ("workers".to_owned(), ConfigurationValue::I64(4)),
        ]))
        .expect("store");
        let value = store
            .execute(ConfigurationOperation::Text, "service-title")
            .expect("text value");
        assert_eq!(value, ConfigurationOutput::Text("Journal".to_owned()));
        assert_eq!(
            store
                .execute(ConfigurationOperation::Bool, "service-title")
                .expect_err("wrong typed read must reject")
                .code,
            "configuration_type"
        );
        assert_eq!(
            ConfigurationStore::observe_values(&BTreeMap::from([(
                "service-title".to_owned(),
                ConfigurationValue::Text("Journal".to_owned()),
            )]))
            .expect("observation")
            .fields["service-title"],
            "text"
        );
    }
}
