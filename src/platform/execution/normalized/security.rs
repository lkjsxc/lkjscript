//! Exact Graph 8 bindings for time, randomness, and identifier capabilities.

use super::capability::{NormalizedAdapterKind, NormalizedCallPolicy, NormalizedCapabilityAdapter};
use super::resource::NormalizedResourceScope;
use super::value::NormalizedValue;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{DeclarationReference, OperationReference, ResourceUnit};
#[cfg(test)]
use crate::platform::security::{
    DeterministicClockSource, DeterministicIdentifierSource, DeterministicRandomSource,
};
use crate::platform::security::{
    MAXIMUM_RANDOM_BYTES, secure_identifier, secure_random_bytes, wall_clock_milliseconds,
};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
enum NormalizedSecurityImplementation {
    WallClock,
    #[cfg(test)]
    DeterministicClock(DeterministicClockSource),
    SecureRandom,
    #[cfg(test)]
    DeterministicRandom(DeterministicRandomSource),
    Identifier,
    #[cfg(test)]
    DeterministicIdentifier(DeterministicIdentifierSource),
}

#[derive(Clone, Debug)]
pub(crate) struct NormalizedSecurityAdapter {
    interface: DeclarationReference,
    operation: OperationReference,
    operations: BTreeSet<OperationReference>,
    implementation: NormalizedSecurityImplementation,
}

impl NormalizedSecurityAdapter {
    pub(crate) fn wall_clock(
        interface: DeclarationReference,
        operation: OperationReference,
    ) -> Result<Self, Diagnostic> {
        Self::new(
            interface,
            operation,
            NormalizedSecurityImplementation::WallClock,
        )
    }

    #[cfg(test)]
    pub(crate) fn deterministic_clock(
        interface: DeclarationReference,
        operation: OperationReference,
        observations: Vec<i64>,
    ) -> Result<Self, Diagnostic> {
        Self::new(
            interface,
            operation,
            NormalizedSecurityImplementation::DeterministicClock(DeterministicClockSource::new(
                observations,
            )),
        )
    }

    pub(crate) fn secure_random(
        interface: DeclarationReference,
        operation: OperationReference,
    ) -> Result<Self, Diagnostic> {
        Self::new(
            interface,
            operation,
            NormalizedSecurityImplementation::SecureRandom,
        )
    }

    #[cfg(test)]
    pub(crate) fn deterministic_random(
        interface: DeclarationReference,
        operation: OperationReference,
        values: Vec<Vec<u8>>,
    ) -> Result<Self, Diagnostic> {
        Self::new(
            interface,
            operation,
            NormalizedSecurityImplementation::DeterministicRandom(DeterministicRandomSource::new(
                values,
            )),
        )
    }

    pub(crate) fn identifier(
        interface: DeclarationReference,
        operation: OperationReference,
    ) -> Result<Self, Diagnostic> {
        Self::new(
            interface,
            operation,
            NormalizedSecurityImplementation::Identifier,
        )
    }

    #[cfg(test)]
    pub(crate) fn deterministic_identifier(
        interface: DeclarationReference,
        operation: OperationReference,
        values: Vec<[u8; 16]>,
    ) -> Result<Self, Diagnostic> {
        Self::new(
            interface,
            operation,
            NormalizedSecurityImplementation::DeterministicIdentifier(
                DeterministicIdentifierSource::new(values),
            ),
        )
    }

    fn new(
        interface: DeclarationReference,
        operation: OperationReference,
        implementation: NormalizedSecurityImplementation,
    ) -> Result<Self, Diagnostic> {
        if operation.package != interface.package {
            return Err(security_diagnostic(
                "normalized_security_operation_package",
                "security adapter operation must share the exact interface package",
            ));
        }
        Ok(Self {
            interface,
            operation,
            operations: BTreeSet::from([operation]),
            implementation,
        })
    }

    fn validate_policy(&self, policy: &NormalizedCallPolicy) -> Result<(), ExecutionError> {
        if policy.grant.interface != self.interface || policy.operation != self.operation {
            return Err(security_runtime(
                "normalized_security_binding",
                "security call policy has a foreign exact interface or operation",
            ));
        }
        Ok(())
    }
}

impl NormalizedCapabilityAdapter for NormalizedSecurityAdapter {
    fn kind(&self) -> NormalizedAdapterKind {
        match &self.implementation {
            NormalizedSecurityImplementation::WallClock => NormalizedAdapterKind::WallClock,
            #[cfg(test)]
            NormalizedSecurityImplementation::DeterministicClock(_) => {
                NormalizedAdapterKind::WallClock
            }
            NormalizedSecurityImplementation::SecureRandom => NormalizedAdapterKind::SecureRandom,
            #[cfg(test)]
            NormalizedSecurityImplementation::DeterministicRandom(_) => {
                NormalizedAdapterKind::SecureRandom
            }
            NormalizedSecurityImplementation::Identifier => NormalizedAdapterKind::Identifier,
            #[cfg(test)]
            NormalizedSecurityImplementation::DeterministicIdentifier(_) => {
                NormalizedAdapterKind::Identifier
            }
        }
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
        self.validate_policy(policy)?;
        match &self.implementation {
            NormalizedSecurityImplementation::WallClock => {
                require_no_arguments(&arguments, "wall clock")?;
                Ok(NormalizedValue::I64(wall_clock_milliseconds()?))
            }
            #[cfg(test)]
            NormalizedSecurityImplementation::DeterministicClock(source) => {
                require_no_arguments(&arguments, "deterministic clock")?;
                Ok(NormalizedValue::I64(source.next()?))
            }
            NormalizedSecurityImplementation::SecureRandom => {
                let length = random_length(&arguments, "secure randomness")?;
                Ok(NormalizedValue::bytes(secure_random_bytes(
                    length,
                    grant_limit(
                        policy,
                        "maximum_random_bytes",
                        ResourceUnit::Bytes,
                        MAXIMUM_RANDOM_BYTES as u64,
                    )?,
                )?))
            }
            #[cfg(test)]
            NormalizedSecurityImplementation::DeterministicRandom(source) => {
                let length = random_length(&arguments, "deterministic randomness")?;
                Ok(NormalizedValue::bytes(source.next(
                    length,
                    grant_limit(
                        policy,
                        "maximum_random_bytes",
                        ResourceUnit::Bytes,
                        MAXIMUM_RANDOM_BYTES as u64,
                    )?,
                )?))
            }
            NormalizedSecurityImplementation::Identifier => {
                require_no_arguments(&arguments, "identifier")?;
                Ok(NormalizedValue::text(secure_identifier()?))
            }
            #[cfg(test)]
            NormalizedSecurityImplementation::DeterministicIdentifier(source) => {
                require_no_arguments(&arguments, "deterministic identifier")?;
                Ok(NormalizedValue::text(source.next()?))
            }
        }
    }
}

fn grant_limit(
    policy: &NormalizedCallPolicy,
    name: &str,
    unit: ResourceUnit,
    default: u64,
) -> Result<u64, ExecutionError> {
    let Some(limit) = policy
        .grant
        .limits
        .iter()
        .find_map(|(candidate, limit)| (candidate.as_str() == name).then_some(*limit))
    else {
        return Ok(default);
    };
    if limit.unit != unit {
        return Err(security_runtime(
            "normalized_security_limit_unit",
            format!("security grant limit '{name}' has a foreign resource unit"),
        ));
    }
    Ok(limit.maximum)
}

fn require_no_arguments(
    arguments: &[NormalizedValue],
    adapter: &str,
) -> Result<(), ExecutionError> {
    if arguments.is_empty() {
        Ok(())
    } else {
        Err(security_runtime(
            "security_adapter_argument",
            format!("{adapter} operation expects no arguments"),
        ))
    }
}

fn random_length(arguments: &[NormalizedValue], adapter: &str) -> Result<i64, ExecutionError> {
    let [NormalizedValue::I64(length)] = arguments else {
        return Err(security_runtime(
            "security_adapter_argument",
            format!("{adapter} operation expects one I64 length"),
        ));
    };
    Ok(*length)
}

fn security_runtime(code: &'static str, message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}

fn security_diagnostic(code: &'static str, message: &'static str) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Capability, code, message)
}

#[cfg(test)]
mod tests {
    use super::super::capability::{NormalizedCapabilityGrantDescriptor, NormalizedGrantLimit};
    use super::*;
    use crate::platform::kernel::{
        ExternalVisibility, Idempotency, Name, PackageId, RequirementReference,
    };
    use crate::platform::security::parse_uuid;
    use crate::platform::semantic_id::{DeclarationId, OperationId, RequirementId};
    use std::collections::BTreeMap;
    use std::sync::Arc;

    const SEED: &[u8] = b"normalized-security-adapter";

    fn bindings(
        ordinal: u64,
    ) -> (
        DeclarationReference,
        RequirementReference,
        OperationReference,
    ) {
        let package = PackageId::migrate(SEED, 0);
        (
            DeclarationReference {
                package,
                declaration: DeclarationId::migrate(SEED, ordinal),
            },
            RequirementReference {
                package,
                requirement: RequirementId::migrate(SEED, ordinal),
            },
            OperationReference {
                package,
                operation: OperationId::migrate(SEED, ordinal),
            },
        )
    }

    fn policy(
        interface: DeclarationReference,
        requirement: RequirementReference,
        operation: OperationReference,
        display_name: &str,
        limits: BTreeMap<Name, NormalizedGrantLimit>,
    ) -> NormalizedCallPolicy {
        NormalizedCallPolicy {
            requirement,
            grant_requirement: requirement,
            requirement_name: Name::new("security").unwrap(),
            operation,
            operation_name: Name::new(display_name).unwrap(),
            idempotency: Idempotency::Idempotent,
            external_visibility: ExternalVisibility::None,
            requirement_limits: Arc::from([]),
            grant: Arc::new(NormalizedCapabilityGrantDescriptor::for_test(
                interface,
                match display_name {
                    "utc-milliseconds" => NormalizedAdapterKind::WallClock,
                    "bytes" => NormalizedAdapterKind::SecureRandom,
                    "uuid-v4" => NormalizedAdapterKind::Identifier,
                    _ => NormalizedAdapterKind::SecureRandom,
                },
                BTreeSet::from([operation]),
                limits,
            )),
        }
    }

    #[test]
    fn deterministic_security_sources_use_exact_operations_and_grant_limits() {
        let control = ExecutionControl::uncancelled();
        let resources = NormalizedResourceScope::new().expect("resource scope");
        let (clock_interface, clock_requirement, clock_operation) = bindings(0);
        let clock = NormalizedSecurityAdapter::deterministic_clock(
            clock_interface,
            clock_operation,
            vec![42],
        )
        .expect("exact deterministic clock");
        assert_eq!(
            clock
                .call(
                    &policy(
                        clock_interface,
                        clock_requirement,
                        clock_operation,
                        "utc-milliseconds",
                        BTreeMap::new(),
                    ),
                    Vec::new(),
                    &resources,
                    &control,
                )
                .expect("deterministic clock value"),
            NormalizedValue::I64(42)
        );

        let (random_interface, random_requirement, random_operation) = bindings(1);
        let random = NormalizedSecurityAdapter::deterministic_random(
            random_interface,
            random_operation,
            vec![vec![7; 4]],
        )
        .expect("exact deterministic random source");
        let random_policy = policy(
            random_interface,
            random_requirement,
            random_operation,
            "bytes",
            BTreeMap::from([(
                Name::new("maximum_random_bytes").unwrap(),
                NormalizedGrantLimit {
                    maximum: 4,
                    unit: ResourceUnit::Bytes,
                },
            )]),
        );
        assert_eq!(
            random
                .call(
                    &random_policy,
                    vec![NormalizedValue::I64(4)],
                    &resources,
                    &control,
                )
                .expect("bounded deterministic bytes"),
            NormalizedValue::bytes(vec![7; 4])
        );

        let (identifier_interface, identifier_requirement, identifier_operation) = bindings(2);
        let identifier = NormalizedSecurityAdapter::deterministic_identifier(
            identifier_interface,
            identifier_operation,
            vec![[0; 16]],
        )
        .expect("exact deterministic identifier");
        let NormalizedValue::Text(identifier) = identifier
            .call(
                &policy(
                    identifier_interface,
                    identifier_requirement,
                    identifier_operation,
                    "uuid-v4",
                    BTreeMap::new(),
                ),
                Vec::new(),
                &resources,
                &control,
            )
            .expect("deterministic identifier value")
        else {
            panic!("identifier result type")
        };
        assert_eq!(identifier.as_ref(), "00000000-0000-4000-8000-000000000000");
        assert_eq!(parse_uuid(&identifier).expect("exact UUID")[6] >> 4, 4);
    }

    #[test]
    fn display_names_cannot_authorize_foreign_security_operations() {
        let resources = NormalizedResourceScope::new().expect("resource scope");
        let (interface, requirement, operation) = bindings(3);
        let adapter =
            NormalizedSecurityAdapter::deterministic_clock(interface, operation, vec![42])
                .expect("exact deterministic clock");
        let foreign = OperationReference {
            package: interface.package,
            operation: OperationId::migrate(SEED, 99),
        };
        assert_eq!(
            adapter
                .call(
                    &policy(
                        interface,
                        requirement,
                        foreign,
                        "utc-milliseconds",
                        BTreeMap::new(),
                    ),
                    Vec::new(),
                    &resources,
                    &ExecutionControl::uncancelled(),
                )
                .expect_err("foreign exact operation")
                .code,
            "normalized_security_binding"
        );

        assert_eq!(
            NormalizedSecurityAdapter::wall_clock(
                interface,
                OperationReference {
                    package: PackageId::migrate(SEED, 1),
                    operation: operation.operation,
                },
            )
            .expect_err("foreign operation package")
            .code,
            "normalized_security_operation_package"
        );
    }

    #[test]
    fn production_security_sources_preserve_exact_shapes_and_units() {
        let control = ExecutionControl::uncancelled();
        let resources = NormalizedResourceScope::new().expect("resource scope");
        let (clock_interface, clock_requirement, clock_operation) = bindings(10);
        let clock = NormalizedSecurityAdapter::wall_clock(clock_interface, clock_operation)
            .expect("exact wall clock");
        let NormalizedValue::I64(milliseconds) = clock
            .call(
                &policy(
                    clock_interface,
                    clock_requirement,
                    clock_operation,
                    "utc-milliseconds",
                    BTreeMap::new(),
                ),
                Vec::new(),
                &resources,
                &control,
            )
            .expect("wall clock value")
        else {
            panic!("wall clock result type")
        };
        assert!(milliseconds > 0);

        let (random_interface, random_requirement, random_operation) = bindings(11);
        let random = NormalizedSecurityAdapter::secure_random(random_interface, random_operation)
            .expect("exact secure randomness");
        let random_policy = policy(
            random_interface,
            random_requirement,
            random_operation,
            "bytes",
            BTreeMap::from([(
                Name::new("maximum_random_bytes").unwrap(),
                NormalizedGrantLimit {
                    maximum: 8,
                    unit: ResourceUnit::Bytes,
                },
            )]),
        );
        let NormalizedValue::Bytes(bytes) = random
            .call(
                &random_policy,
                vec![NormalizedValue::I64(8)],
                &resources,
                &control,
            )
            .expect("secure random bytes")
        else {
            panic!("secure random result type")
        };
        assert_eq!(bytes.len(), 8);

        let wrong_unit = policy(
            random_interface,
            random_requirement,
            random_operation,
            "bytes",
            BTreeMap::from([(
                Name::new("maximum_random_bytes").unwrap(),
                NormalizedGrantLimit {
                    maximum: 8,
                    unit: ResourceUnit::Calls,
                },
            )]),
        );
        assert_eq!(
            random
                .call(
                    &wrong_unit,
                    vec![NormalizedValue::I64(8)],
                    &resources,
                    &control,
                )
                .expect_err("foreign random limit unit")
                .code,
            "normalized_security_limit_unit"
        );

        let (identifier_interface, identifier_requirement, identifier_operation) = bindings(12);
        let identifier =
            NormalizedSecurityAdapter::identifier(identifier_interface, identifier_operation)
                .expect("exact secure identifier");
        let NormalizedValue::Text(identifier) = identifier
            .call(
                &policy(
                    identifier_interface,
                    identifier_requirement,
                    identifier_operation,
                    "uuid-v4",
                    BTreeMap::new(),
                ),
                Vec::new(),
                &resources,
                &control,
            )
            .expect("secure identifier value")
        else {
            panic!("secure identifier result type")
        };
        let bytes = parse_uuid(&identifier).expect("secure UUID");
        assert_eq!(bytes[6] >> 4, 4);
        assert_eq!(bytes[8] >> 6, 2);
    }
}
