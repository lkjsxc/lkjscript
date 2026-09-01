//! Exact Graph 6 binding for deployment configuration reads.

use super::capability::{NormalizedAdapterKind, NormalizedCallPolicy, NormalizedCapabilityAdapter};
use super::resource::NormalizedResourceScope;
use super::value::NormalizedValue;
use crate::platform::configuration::{
    ConfigurationOperation, ConfigurationOutput, ConfigurationStore, ConfigurationValue,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{DeclarationReference, OperationReference};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NormalizedConfigurationOperations {
    pub exists: OperationReference,
    pub text: OperationReference,
    pub i64: OperationReference,
    pub bool: OperationReference,
}

#[cfg(test)]
impl NormalizedConfigurationOperations {
    fn exact_set(&self) -> BTreeSet<OperationReference> {
        [self.exists, self.text, self.i64, self.bool]
            .into_iter()
            .collect()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct NormalizedConfigurationAdapter {
    interface: DeclarationReference,
    operations: BTreeMap<OperationReference, ConfigurationOperation>,
    exact_operations: BTreeSet<OperationReference>,
    store: ConfigurationStore,
}

impl NormalizedConfigurationAdapter {
    #[cfg(test)]
    pub(crate) fn new(
        interface: DeclarationReference,
        operations: NormalizedConfigurationOperations,
        values: BTreeMap<String, ConfigurationValue>,
    ) -> Result<Self, Diagnostic> {
        if operations.exact_set().len() != 4 {
            return Err(configuration_diagnostic(
                "normalized_configuration_operation_duplicate",
                "configuration adapter operation identities must be distinct",
            ));
        }
        Self::new_selected(
            interface,
            BTreeMap::from([
                (operations.exists, ConfigurationOperation::Exists),
                (operations.text, ConfigurationOperation::Text),
                (operations.i64, ConfigurationOperation::I64),
                (operations.bool, ConfigurationOperation::Bool),
            ]),
            values,
        )
    }

    pub(crate) fn new_selected(
        interface: DeclarationReference,
        operations: BTreeMap<OperationReference, ConfigurationOperation>,
        values: BTreeMap<String, ConfigurationValue>,
    ) -> Result<Self, Diagnostic> {
        if operations.is_empty() {
            return Err(configuration_diagnostic(
                "normalized_configuration_operation_empty",
                "configuration adapter must bind at least one exact operation",
            ));
        }
        if operations
            .iter()
            .any(|(operation, _)| operation.package != interface.package)
        {
            return Err(configuration_diagnostic(
                "normalized_configuration_operation_package",
                "configuration adapter operations must share the exact interface package",
            ));
        }
        let exact_operations = operations.keys().copied().collect();
        Ok(Self {
            interface,
            operations,
            exact_operations,
            store: ConfigurationStore::new(values)?,
        })
    }
}

impl NormalizedCapabilityAdapter for NormalizedConfigurationAdapter {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::Configuration
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
        if policy.grant.interface != self.interface {
            return Err(configuration_runtime(
                "normalized_configuration_interface",
                "configuration call policy has a foreign exact interface",
            ));
        }
        let operation = self
            .operations
            .get(&policy.operation)
            .copied()
            .ok_or_else(|| {
                configuration_runtime(
                    "normalized_configuration_operation",
                    "configuration call policy has a foreign exact operation",
                )
            })?;
        let [NormalizedValue::StaticText(name)] = arguments.as_slice() else {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Capability,
                "configuration_argument",
                "configuration operation expects one source-origin StaticText field name",
            ));
        };
        match self.store.execute(operation, name)? {
            ConfigurationOutput::Text(value) => Ok(NormalizedValue::text(value)),
            ConfigurationOutput::I64(value) => Ok(NormalizedValue::I64(value)),
            ConfigurationOutput::Bool(value) => Ok(NormalizedValue::Bool(value)),
        }
    }
}

fn configuration_runtime(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}

fn configuration_diagnostic(code: &'static str, message: &'static str) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Capability, code, message)
}

#[cfg(test)]
mod tests {
    use super::super::capability::NormalizedCapabilityGrantDescriptor;
    use super::*;
    use crate::platform::kernel::{
        ExternalVisibility, Idempotency, Name, PackageId, RequirementReference,
    };
    use crate::platform::semantic_id::{DeclarationId, OperationId, RequirementId};
    use std::sync::Arc;

    const SEED: &[u8] = b"normalized-configuration-adapter";

    fn exact_bindings() -> (
        DeclarationReference,
        RequirementReference,
        NormalizedConfigurationOperations,
    ) {
        let package = PackageId::migrate(SEED, 0);
        let interface = DeclarationReference {
            package,
            declaration: DeclarationId::migrate(SEED, 0),
        };
        let requirement = RequirementReference {
            package,
            requirement: RequirementId::migrate(SEED, 0),
        };
        let operation = |ordinal| OperationReference {
            package,
            operation: OperationId::migrate(SEED, ordinal),
        };
        (
            interface,
            requirement,
            NormalizedConfigurationOperations {
                exists: operation(0),
                text: operation(1),
                i64: operation(2),
                bool: operation(3),
            },
        )
    }

    fn policy(
        interface: DeclarationReference,
        requirement: RequirementReference,
        operation: OperationReference,
        name: &str,
    ) -> NormalizedCallPolicy {
        NormalizedCallPolicy {
            requirement,
            requirement_name: Name::new("config").unwrap(),
            operation,
            operation_name: Name::new(name).unwrap(),
            idempotency: Idempotency::Idempotent,
            external_visibility: ExternalVisibility::None,
            requirement_limits: Arc::from([]),
            grant: Arc::new(NormalizedCapabilityGrantDescriptor::for_test(
                interface,
                NormalizedAdapterKind::Configuration,
                BTreeSet::from([operation]),
                BTreeMap::new(),
            )),
        }
    }

    #[test]
    fn exact_operation_identity_drives_normalized_configuration_reads() {
        let (interface, requirement, operations) = exact_bindings();
        let adapter = NormalizedConfigurationAdapter::new(
            interface,
            operations.clone(),
            BTreeMap::from([
                (
                    "service-title".to_owned(),
                    ConfigurationValue::Text("Journal".to_owned()),
                ),
                ("workers".to_owned(), ConfigurationValue::I64(4)),
            ]),
        )
        .expect("exact normalized configuration adapter");
        let resources = NormalizedResourceScope::new().expect("resource scope");
        assert_eq!(adapter.operations(), &operations.exact_set());

        let result = adapter
            .call(
                &policy(interface, requirement, operations.text, "text"),
                vec![NormalizedValue::static_text("service-title")],
                &resources,
                &ExecutionControl::uncancelled(),
            )
            .expect("exact configuration text read");
        assert_eq!(result, NormalizedValue::text("Journal"));

        let dynamic = adapter
            .call(
                &policy(interface, requirement, operations.text, "text"),
                vec![NormalizedValue::text("service-title")],
                &resources,
                &ExecutionControl::uncancelled(),
            )
            .expect_err("dynamic configuration key must reject");
        assert_eq!(dynamic.code, "configuration_argument");

        let foreign = OperationReference {
            package: interface.package,
            operation: OperationId::migrate(SEED, 99),
        };
        let foreign = adapter
            .call(
                &policy(interface, requirement, foreign, "text"),
                vec![NormalizedValue::static_text("service-title")],
                &resources,
                &ExecutionControl::uncancelled(),
            )
            .expect_err("display name cannot authorize a foreign exact operation");
        assert_eq!(foreign.code, "normalized_configuration_operation");
    }

    #[test]
    fn duplicate_or_cross_package_operation_bindings_reject() {
        let (interface, _, mut operations) = exact_bindings();
        operations.bool = operations.text;
        assert_eq!(
            NormalizedConfigurationAdapter::new(interface, operations, BTreeMap::new())
                .expect_err("duplicate exact operation")
                .code,
            "normalized_configuration_operation_duplicate"
        );

        let (_, _, mut operations) = exact_bindings();
        operations.bool.package = PackageId::migrate(SEED, 1);
        assert_eq!(
            NormalizedConfigurationAdapter::new(interface, operations, BTreeMap::new())
                .expect_err("cross-package exact operation")
                .code,
            "normalized_configuration_operation_package"
        );
    }
}
