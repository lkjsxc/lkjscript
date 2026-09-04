//! Exact Graph 10 binding for password hashing and verification.

use super::capability::{NormalizedAdapterKind, NormalizedCallPolicy, NormalizedCapabilityAdapter};
use super::resource::NormalizedResourceScope;
use super::value::NormalizedValue;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{DeclarationReference, OperationReference};
use crate::platform::security::{PasswordHashEngine, PasswordHashPolicy};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NormalizedPasswordHashOperations {
    pub hash: OperationReference,
    pub verify: OperationReference,
    pub needs_upgrade: OperationReference,
}

#[cfg(test)]
impl NormalizedPasswordHashOperations {
    fn exact_set(&self) -> BTreeSet<OperationReference> {
        [self.hash, self.verify, self.needs_upgrade]
            .into_iter()
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NormalizedPasswordHashOperation {
    Hash,
    Verify,
    NeedsUpgrade,
}

#[derive(Clone, Debug)]
pub(crate) struct NormalizedPasswordHashAdapter {
    interface: DeclarationReference,
    operations: BTreeMap<OperationReference, NormalizedPasswordHashOperation>,
    exact_operations: BTreeSet<OperationReference>,
    engine: PasswordHashEngine,
}

impl NormalizedPasswordHashAdapter {
    #[cfg(test)]
    pub(crate) fn new(
        interface: DeclarationReference,
        operations: NormalizedPasswordHashOperations,
        policy: PasswordHashPolicy,
    ) -> Result<Self, Diagnostic> {
        if operations.exact_set().len() != 3 {
            return Err(password_diagnostic(
                "normalized_password_operation_duplicate",
                "password adapter operation identities must be distinct",
            ));
        }
        Self::new_selected(
            interface,
            BTreeMap::from([
                (operations.hash, NormalizedPasswordHashOperation::Hash),
                (operations.verify, NormalizedPasswordHashOperation::Verify),
                (
                    operations.needs_upgrade,
                    NormalizedPasswordHashOperation::NeedsUpgrade,
                ),
            ]),
            policy,
        )
    }

    pub(crate) fn new_selected(
        interface: DeclarationReference,
        operations: BTreeMap<OperationReference, NormalizedPasswordHashOperation>,
        policy: PasswordHashPolicy,
    ) -> Result<Self, Diagnostic> {
        if operations.is_empty() {
            return Err(password_diagnostic(
                "normalized_password_operation_empty",
                "password adapter must bind at least one exact operation",
            ));
        }
        if operations
            .iter()
            .any(|(operation, _)| operation.package != interface.package)
        {
            return Err(password_diagnostic(
                "normalized_password_operation_package",
                "password adapter operations must share the exact interface package",
            ));
        }
        let engine = PasswordHashEngine::new(policy).map_err(|error| {
            password_diagnostic(
                "normalized_password_policy",
                format!("password adapter policy is invalid: {}", error.message),
            )
        })?;
        let exact_operations = operations.keys().copied().collect();
        Ok(Self {
            interface,
            operations,
            exact_operations,
            engine,
        })
    }

    fn validate_policy(&self, policy: &NormalizedCallPolicy) -> Result<(), ExecutionError> {
        if policy.grant.interface != self.interface
            || !self.exact_operations.contains(&policy.operation)
        {
            return Err(password_runtime(
                "normalized_password_binding",
                "password call policy has a foreign exact interface or operation",
            ));
        }
        Ok(())
    }
}

impl NormalizedCapabilityAdapter for NormalizedPasswordHashAdapter {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::PasswordHash
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
        self.validate_policy(policy)?;
        let operation = self
            .operations
            .get(&policy.operation)
            .copied()
            .ok_or_else(|| {
                password_runtime(
                    "normalized_password_binding",
                    "password operation escaped its exact adapter set",
                )
            })?;
        if operation == NormalizedPasswordHashOperation::Hash {
            let [NormalizedValue::Bytes(password)] = arguments.as_slice() else {
                return Err(password_argument("password hash expects password Bytes"));
            };
            return Ok(NormalizedValue::text(self.engine.hash(password)?));
        }
        if operation == NormalizedPasswordHashOperation::Verify {
            let [
                NormalizedValue::Bytes(password),
                NormalizedValue::Text(encoded),
            ] = arguments.as_slice()
            else {
                return Err(password_argument(
                    "password verify expects password Bytes and encoded Text",
                ));
            };
            return Ok(NormalizedValue::Bool(
                self.engine.verify(password, encoded)?,
            ));
        }
        if operation == NormalizedPasswordHashOperation::NeedsUpgrade {
            let [NormalizedValue::Text(encoded)] = arguments.as_slice() else {
                return Err(password_argument(
                    "password needs-upgrade expects encoded Text",
                ));
            };
            return Ok(NormalizedValue::Bool(self.engine.needs_upgrade(encoded)?));
        }
        Err(password_runtime(
            "normalized_password_binding",
            "password operation escaped its exact adapter set",
        ))
    }
}

fn password_argument(message: &'static str) -> ExecutionError {
    password_runtime("security_adapter_argument", message)
}

fn password_runtime(code: &'static str, message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}

fn password_diagnostic(code: &'static str, message: impl Into<String>) -> Diagnostic {
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
    use std::collections::BTreeMap;
    use std::sync::Arc;

    const SEED: &[u8] = b"normalized-password-adapter";

    fn exact_bindings() -> (
        DeclarationReference,
        RequirementReference,
        NormalizedPasswordHashOperations,
    ) {
        let package = PackageId::migrate(SEED, 0);
        let operation = |ordinal| OperationReference {
            package,
            operation: OperationId::migrate(SEED, ordinal),
        };
        (
            DeclarationReference {
                package,
                declaration: DeclarationId::migrate(SEED, 0),
            },
            RequirementReference {
                package,
                requirement: RequirementId::migrate(SEED, 0),
            },
            NormalizedPasswordHashOperations {
                hash: operation(0),
                verify: operation(1),
                needs_upgrade: operation(2),
            },
        )
    }

    fn policy(
        interface: DeclarationReference,
        requirement: RequirementReference,
        operation: OperationReference,
        display_name: &str,
    ) -> NormalizedCallPolicy {
        NormalizedCallPolicy {
            requirement,
            grant_requirement: requirement,
            requirement_name: Name::new("password").unwrap(),
            operation,
            operation_name: Name::new(display_name).unwrap(),
            idempotency: Idempotency::Idempotent,
            external_visibility: ExternalVisibility::None,
            requirement_limits: Arc::from([]),
            grant: Arc::new(NormalizedCapabilityGrantDescriptor::for_test(
                interface,
                NormalizedAdapterKind::PasswordHash,
                BTreeSet::from([operation]),
                BTreeMap::new(),
            )),
        }
    }

    fn fast_policy() -> PasswordHashPolicy {
        PasswordHashPolicy {
            memory_kibibytes: 8,
            iterations: 1,
            lanes: 1,
            output_bytes: 16,
        }
    }

    #[test]
    fn exact_password_operations_share_one_hashing_engine() {
        let (interface, requirement, operations) = exact_bindings();
        let adapter =
            NormalizedPasswordHashAdapter::new(interface, operations.clone(), fast_policy())
                .expect("exact normalized password adapter");
        let control = ExecutionControl::uncancelled();
        let resources = NormalizedResourceScope::new().expect("resource scope");
        let NormalizedValue::Text(encoded) = adapter
            .call(
                &policy(interface, requirement, operations.hash, "hash"),
                vec![NormalizedValue::bytes(b"correct horse".to_vec())],
                &resources,
                &control,
            )
            .expect("password hash")
        else {
            panic!("password hash result type")
        };
        assert_eq!(
            adapter
                .call(
                    &policy(interface, requirement, operations.verify, "verify"),
                    vec![
                        NormalizedValue::bytes(b"correct horse".to_vec()),
                        NormalizedValue::Text(Arc::clone(&encoded)),
                    ],
                    &resources,
                    &control,
                )
                .expect("password verification"),
            NormalizedValue::Bool(true)
        );
        assert_eq!(
            adapter
                .call(
                    &policy(interface, requirement, operations.verify, "verify"),
                    vec![
                        NormalizedValue::bytes(b"wrong".to_vec()),
                        NormalizedValue::Text(Arc::clone(&encoded)),
                    ],
                    &resources,
                    &control,
                )
                .expect("password mismatch"),
            NormalizedValue::Bool(false)
        );
        assert_eq!(
            adapter
                .call(
                    &policy(
                        interface,
                        requirement,
                        operations.needs_upgrade,
                        "needs-upgrade",
                    ),
                    vec![NormalizedValue::Text(encoded)],
                    &resources,
                    &control,
                )
                .expect("current password parameters"),
            NormalizedValue::Bool(false)
        );
    }

    #[test]
    fn password_display_names_cannot_authorize_foreign_operations() {
        let (interface, requirement, operations) = exact_bindings();
        let adapter =
            NormalizedPasswordHashAdapter::new(interface, operations.clone(), fast_policy())
                .expect("exact normalized password adapter");
        let resources = NormalizedResourceScope::new().expect("resource scope");
        let foreign = OperationReference {
            package: interface.package,
            operation: OperationId::migrate(SEED, 99),
        };
        assert_eq!(
            adapter
                .call(
                    &policy(interface, requirement, foreign, "hash"),
                    vec![NormalizedValue::bytes(b"password".to_vec())],
                    &resources,
                    &ExecutionControl::uncancelled(),
                )
                .expect_err("foreign operation with matching name")
                .code,
            "normalized_password_binding"
        );

        let mut duplicate = operations;
        duplicate.needs_upgrade = duplicate.verify;
        assert_eq!(
            NormalizedPasswordHashAdapter::new(interface, duplicate, fast_policy())
                .expect_err("duplicate password operation")
                .code,
            "normalized_password_operation_duplicate"
        );
    }
}
