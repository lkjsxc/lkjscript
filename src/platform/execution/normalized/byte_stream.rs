//! Exact Graph 9 byte-stream capability over task-owned opaque handles.

use super::capability::{NormalizedAdapterKind, NormalizedCallPolicy, NormalizedCapabilityAdapter};
use super::resource::NormalizedResourceScope;
use super::value::{NormalizedRecord, NormalizedValue};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{
    DeclarationReference, Name, OperationReference, RequirementReference,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NormalizedByteStreamOperations {
    pub read: OperationReference,
    pub close: OperationReference,
    pub read_all: OperationReference,
}

#[cfg(test)]
impl NormalizedByteStreamOperations {
    fn exact_set(&self) -> BTreeSet<OperationReference> {
        [self.read, self.close, self.read_all].into_iter().collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NormalizedByteStreamOperation {
    Read,
    Close,
    ReadAll,
}

#[derive(Clone, Debug)]
pub(crate) struct NormalizedByteStreamAdapter {
    requirement: RequirementReference,
    interface: DeclarationReference,
    operations: BTreeMap<OperationReference, NormalizedByteStreamOperation>,
    exact_operations: BTreeSet<OperationReference>,
    chunk_name: Name,
    done_name: Name,
}

impl NormalizedByteStreamAdapter {
    #[cfg(test)]
    pub(crate) fn new(
        requirement: RequirementReference,
        interface: DeclarationReference,
        operations: NormalizedByteStreamOperations,
    ) -> Result<Self, Diagnostic> {
        if operations.exact_set().len() != 3 {
            return Err(stream_diagnostic(
                "normalized_stream_operation_duplicate",
                "byte-stream adapter operation identities must be distinct",
            ));
        }
        Self::new_selected(
            requirement,
            interface,
            BTreeMap::from([
                (operations.read, NormalizedByteStreamOperation::Read),
                (operations.close, NormalizedByteStreamOperation::Close),
                (operations.read_all, NormalizedByteStreamOperation::ReadAll),
            ]),
        )
    }

    pub(crate) fn new_selected(
        requirement: RequirementReference,
        interface: DeclarationReference,
        operations: BTreeMap<OperationReference, NormalizedByteStreamOperation>,
    ) -> Result<Self, Diagnostic> {
        if operations.is_empty() {
            return Err(stream_diagnostic(
                "normalized_stream_operation_empty",
                "byte-stream adapter must bind at least one exact operation",
            ));
        }
        if operations
            .iter()
            .any(|(operation, _)| operation.package != interface.package)
        {
            return Err(stream_diagnostic(
                "normalized_stream_operation_package",
                "byte-stream operations must share the exact interface package",
            ));
        }
        let exact_operations = operations.keys().copied().collect();
        Ok(Self {
            requirement,
            interface,
            operations,
            exact_operations,
            chunk_name: Name::new("chunk")?,
            done_name: Name::new("done")?,
        })
    }

    fn validate_policy(&self, policy: &NormalizedCallPolicy) -> Result<(), ExecutionError> {
        if policy.grant_requirement != self.requirement || policy.grant.interface != self.interface
        {
            return Err(stream_runtime(
                "normalized_stream_binding",
                "byte-stream call policy has a foreign exact requirement or interface",
            ));
        }
        Ok(())
    }

    fn read_result(&self, chunk: Option<Vec<u8>>) -> NormalizedValue {
        let (done, chunk) = match chunk {
            Some(chunk) => (false, chunk),
            None => (true, Vec::new()),
        };
        NormalizedValue::Record(NormalizedRecord::Structural {
            fields: Arc::new(vec![
                (self.chunk_name.clone(), NormalizedValue::bytes(chunk)),
                (self.done_name.clone(), NormalizedValue::Bool(done)),
            ]),
        })
    }
}

impl NormalizedCapabilityAdapter for NormalizedByteStreamAdapter {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::ByteStream
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
        resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        self.validate_policy(policy)?;
        match self
            .operations
            .get(&policy.operation)
            .copied()
            .ok_or_else(|| {
                stream_runtime(
                    "normalized_stream_operation",
                    "byte-stream call policy has a foreign exact operation",
                )
            })? {
            NormalizedByteStreamOperation::Read => {
                let [NormalizedValue::Resource(stream)] = arguments.as_slice() else {
                    return Err(stream_argument(
                        "stream read expects one byte-stream handle",
                    ));
                };
                resources
                    .read_byte_stream(policy.grant_requirement, *stream, control)
                    .map(|chunk| self.read_result(chunk))
            }
            NormalizedByteStreamOperation::Close => {
                let [NormalizedValue::Resource(stream)] = arguments.as_slice() else {
                    return Err(stream_argument(
                        "stream close expects one byte-stream handle",
                    ));
                };
                resources.close(policy.grant_requirement, *stream)?;
                Ok(NormalizedValue::Unit)
            }
            NormalizedByteStreamOperation::ReadAll => {
                let [
                    NormalizedValue::Resource(stream),
                    NormalizedValue::I64(maximum_bytes),
                ] = arguments.as_slice()
                else {
                    return Err(stream_argument(
                        "stream read-all expects a byte-stream handle and positive I64 byte limit",
                    ));
                };
                let maximum_bytes = usize::try_from(*maximum_bytes).map_err(|_| {
                    ExecutionError::resource(
                        "stream_read_all_limit",
                        "whole-stream byte limit must be a positive platform-sized integer",
                    )
                })?;
                resources
                    .read_all_byte_stream(policy.grant_requirement, *stream, maximum_bytes, control)
                    .map(NormalizedValue::bytes)
            }
        }
    }
}

fn stream_argument(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Capability,
        "normalized_stream_argument",
        message,
    )
}

fn stream_runtime(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}

fn stream_diagnostic(code: &'static str, message: &'static str) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::execution::normalized::capability::NormalizedCapabilityGrantDescriptor;
    use crate::platform::kernel::{ExternalVisibility, Idempotency, PackageId};
    use crate::platform::semantic_id::{DeclarationId, OperationId, RequirementId};
    use crate::platform::stream::{StreamLimits, StreamRegistry};
    use std::collections::BTreeMap;

    const SEED: &[u8] = b"normalized-byte-stream-adapter-test";

    fn bindings() -> (
        DeclarationReference,
        RequirementReference,
        NormalizedByteStreamOperations,
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
            NormalizedByteStreamOperations {
                read: OperationReference {
                    package,
                    operation: OperationId::migrate(SEED, 0),
                },
                close: OperationReference {
                    package,
                    operation: OperationId::migrate(SEED, 1),
                },
                read_all: OperationReference {
                    package,
                    operation: OperationId::migrate(SEED, 2),
                },
            },
        )
    }

    fn policy(
        interface: DeclarationReference,
        requirement: RequirementReference,
        operations: &NormalizedByteStreamOperations,
        operation: OperationReference,
        name: &str,
    ) -> NormalizedCallPolicy {
        NormalizedCallPolicy {
            requirement,
            grant_requirement: requirement,
            requirement_name: Name::new("streams").unwrap(),
            operation,
            operation_name: Name::new(name).unwrap(),
            idempotency: Idempotency::Idempotent,
            external_visibility: ExternalVisibility::None,
            requirement_limits: Arc::from([]),
            grant: Arc::new(NormalizedCapabilityGrantDescriptor::for_test(
                interface,
                NormalizedAdapterKind::ByteStream,
                operations.exact_set(),
                BTreeMap::new(),
            )),
        }
    }

    #[test]
    fn exact_stream_operations_read_close_and_consume_handles() {
        let (interface, requirement, operations) = bindings();
        let adapter = NormalizedByteStreamAdapter::new(requirement, interface, operations.clone())
            .expect("exact stream adapter");
        let registry = StreamRegistry::new(StreamLimits {
            maximum_chunk_bytes: 4,
            maximum_buffered_chunks: 2,
            maximum_total_bytes: 64,
            maximum_live_streams: 4,
        })
        .expect("stream registry");
        let scope = NormalizedResourceScope::new().expect("resource scope");
        let handle = scope
            .register_byte_stream(
                requirement,
                interface,
                registry
                    .register_memory(b"stream".to_vec())
                    .expect("memory stream"),
            )
            .expect("stream handle");
        let read = policy(interface, requirement, &operations, operations.read, "read");
        let first = adapter
            .call(
                &read,
                vec![NormalizedValue::Resource(handle)],
                &scope,
                &ExecutionControl::uncancelled(),
            )
            .expect("first bounded read");
        assert_eq!(
            first,
            NormalizedValue::Record(NormalizedRecord::Structural {
                fields: Arc::new(vec![
                    (
                        Name::new("chunk").unwrap(),
                        NormalizedValue::bytes(b"stre".to_vec())
                    ),
                    (Name::new("done").unwrap(), NormalizedValue::Bool(false)),
                ]),
            })
        );
        let second = adapter
            .call(
                &read,
                vec![NormalizedValue::Resource(handle)],
                &scope,
                &ExecutionControl::uncancelled(),
            )
            .expect("second bounded read");
        assert!(matches!(
            second,
            NormalizedValue::Record(NormalizedRecord::Structural { .. })
        ));
        let eof = adapter
            .call(
                &read,
                vec![NormalizedValue::Resource(handle)],
                &scope,
                &ExecutionControl::uncancelled(),
            )
            .expect("stream EOF");
        let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = eof else {
            panic!("stream EOF record")
        };
        assert_eq!(fields[1].1, NormalizedValue::Bool(true));

        let close = policy(
            interface,
            requirement,
            &operations,
            operations.close,
            "close",
        );
        for _ in 0..2 {
            assert_eq!(
                adapter
                    .call(
                        &close,
                        vec![NormalizedValue::Resource(handle)],
                        &scope,
                        &ExecutionControl::uncancelled(),
                    )
                    .expect("idempotent stream close"),
                NormalizedValue::Unit
            );
        }
        assert_eq!(registry.live_streams(), 0);
        assert_eq!(scope.live_resources(), 0);
    }

    #[test]
    fn read_all_enforces_exact_operation_and_consumes_the_handle() {
        let (interface, requirement, operations) = bindings();
        let adapter = NormalizedByteStreamAdapter::new(requirement, interface, operations.clone())
            .expect("exact stream adapter");
        let registry = StreamRegistry::new(StreamLimits::default()).expect("stream registry");
        let scope = NormalizedResourceScope::new().expect("resource scope");
        let handle = scope
            .register_byte_stream(
                requirement,
                interface,
                registry
                    .register_memory(b"whole".to_vec())
                    .expect("memory stream"),
            )
            .expect("stream handle");
        assert_eq!(
            adapter
                .call(
                    &policy(
                        interface,
                        requirement,
                        &operations,
                        operations.read_all,
                        "read-all",
                    ),
                    vec![NormalizedValue::Resource(handle), NormalizedValue::I64(16)],
                    &scope,
                    &ExecutionControl::uncancelled(),
                )
                .expect("whole stream read"),
            NormalizedValue::bytes(b"whole".to_vec())
        );
        assert_eq!(registry.live_streams(), 0);
        assert_eq!(scope.live_resources(), 0);

        let foreign = OperationReference {
            package: interface.package,
            operation: OperationId::migrate(SEED, 99),
        };
        assert_eq!(
            adapter
                .call(
                    &policy(interface, requirement, &operations, foreign, "read-all"),
                    Vec::new(),
                    &scope,
                    &ExecutionControl::uncancelled(),
                )
                .expect_err("display name cannot authorize a foreign operation")
                .code,
            "normalized_stream_operation"
        );
    }
}
