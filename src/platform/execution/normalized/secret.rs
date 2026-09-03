//! Exact Graph 9 binding for least-authority secret verification.

use super::capability::{NormalizedAdapterKind, NormalizedCallPolicy, NormalizedCapabilityAdapter};
use super::resource::NormalizedResourceScope;
use super::value::NormalizedValue;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{DeclarationReference, OperationReference};
use crate::platform::secrets::{SecretValue, SecretVerifier};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub(crate) struct NormalizedSecretVerifierAdapter {
    interface: DeclarationReference,
    operation: OperationReference,
    operations: BTreeSet<OperationReference>,
    verifier: SecretVerifier,
}

impl NormalizedSecretVerifierAdapter {
    pub(crate) fn new(
        interface: DeclarationReference,
        operation: OperationReference,
        secret: SecretValue,
        maximum_candidate_bytes: usize,
    ) -> Result<Self, Diagnostic> {
        if operation.package != interface.package {
            return Err(secret_diagnostic(
                "normalized_secret_operation_package",
                "secret verifier operation must share the exact interface package",
            ));
        }
        Ok(Self {
            interface,
            operation,
            operations: BTreeSet::from([operation]),
            verifier: SecretVerifier::new(secret, maximum_candidate_bytes)?,
        })
    }
}

impl NormalizedCapabilityAdapter for NormalizedSecretVerifierAdapter {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::SecretVerifier
    }

    fn interface(&self) -> DeclarationReference {
        self.interface
    }

    fn operations(&self) -> &BTreeSet<OperationReference> {
        &self.operations
    }

    fn call(
        &self,
        policy: &NormalizedCallPolicy,
        arguments: Vec<NormalizedValue>,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        if policy.grant.interface != self.interface || policy.operation != self.operation {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "normalized_secret_binding",
                "secret verifier policy has a foreign exact interface or operation",
            ));
        }
        let [NormalizedValue::Bytes(candidate)] = arguments.as_slice() else {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Capability,
                "secret_argument",
                "secret verifier expects one Bytes candidate",
            ));
        };
        Ok(NormalizedValue::Bool(self.verifier.matches(candidate)?))
    }
}

fn secret_diagnostic(code: &'static str, message: &'static str) -> Diagnostic {
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

    const SEED: &[u8] = b"normalized-secret-adapter";

    fn bindings() -> (
        DeclarationReference,
        RequirementReference,
        OperationReference,
    ) {
        let package = PackageId::migrate(SEED, 0);
        (
            DeclarationReference {
                package,
                declaration: DeclarationId::migrate(SEED, 0),
            },
            RequirementReference {
                package,
                requirement: RequirementId::migrate(SEED, 0),
            },
            OperationReference {
                package,
                operation: OperationId::migrate(SEED, 0),
            },
        )
    }

    fn policy(
        interface: DeclarationReference,
        requirement: RequirementReference,
        operation: OperationReference,
    ) -> NormalizedCallPolicy {
        NormalizedCallPolicy {
            requirement,
            grant_requirement: requirement,
            requirement_name: Name::new("secret").unwrap(),
            operation,
            operation_name: Name::new("matches").unwrap(),
            idempotency: Idempotency::Idempotent,
            external_visibility: ExternalVisibility::None,
            requirement_limits: Arc::from([]),
            grant: Arc::new(NormalizedCapabilityGrantDescriptor::for_test(
                interface,
                NormalizedAdapterKind::SecretVerifier,
                BTreeSet::from([operation]),
                BTreeMap::new(),
            )),
        }
    }

    #[test]
    fn exact_secret_verification_never_exposes_secret_bytes() {
        let (interface, requirement, operation) = bindings();
        let adapter = NormalizedSecretVerifierAdapter::new(
            interface,
            operation,
            SecretValue::for_test(b"private-value"),
            16,
        )
        .expect("exact secret verifier");
        let control = ExecutionControl::uncancelled();
        let resources = NormalizedResourceScope::new().expect("resource scope");
        assert_eq!(
            adapter
                .call(
                    &policy(interface, requirement, operation),
                    vec![NormalizedValue::bytes(b"private-value".to_vec())],
                    &resources,
                    &control,
                )
                .expect("matching secret candidate"),
            NormalizedValue::Bool(true)
        );
        assert_eq!(
            adapter
                .call(
                    &policy(interface, requirement, operation),
                    vec![NormalizedValue::bytes(b"other".to_vec())],
                    &resources,
                    &control,
                )
                .expect("different secret candidate"),
            NormalizedValue::Bool(false)
        );
        assert_eq!(
            adapter
                .call(
                    &policy(interface, requirement, operation),
                    vec![NormalizedValue::bytes(vec![0; 17])],
                    &resources,
                    &control,
                )
                .expect_err("candidate byte bound")
                .code,
            "secret_candidate_limit"
        );

        let foreign = OperationReference {
            package: interface.package,
            operation: OperationId::migrate(SEED, 99),
        };
        assert_eq!(
            adapter
                .call(
                    &policy(interface, requirement, foreign),
                    vec![NormalizedValue::bytes(b"private-value".to_vec())],
                    &resources,
                    &control,
                )
                .expect_err("display name cannot authorize a foreign operation")
                .code,
            "normalized_secret_binding"
        );
    }
}
