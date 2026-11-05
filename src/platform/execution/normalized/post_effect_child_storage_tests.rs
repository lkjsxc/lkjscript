//! Real completed effects remain visible when immutable result-owner creation fails.
//!
//! The current-thread hook injects a storage-factory refusal, not a physical OOM.
//! All admission, exact capability bindings, input values and I/O setup precede
//! the hook. Only production result construction can reach the refused factory.

use super::capability::{
    NormalizedAdapterKind, NormalizedCallPolicy, NormalizedCapabilityAdapter,
    NormalizedCapabilityGrantDescriptor, validate_outcome,
};
use super::child_storage::{StorageFailure, refuse_owner_factory_after};
use super::http_client::NormalizedHttpClientAdapter;
use super::object::NormalizedObjectStorageAdapter;
use super::prepare::{NormalizedOperation, NormalizedProgram, NormalizedRequirement};
use super::resource::NormalizedResourceScope;
use super::value::{NormalizedRecord, NormalizedValue};
use crate::platform::builtin_standard::BuiltinStandard;
use crate::platform::compiler::load_artifact;
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::http_client::{
    HttpClient, HttpClientAddressPolicy, HttpClientLimits, HttpClientTrust,
};
use crate::platform::kernel::{
    ExternalVisibility, Idempotency, Name, OperationReference, RequirementReference,
};
use crate::platform::object::{ObjectEngine, ObjectLimits};
use crate::platform::semantic_id::RequirementId;
use crate::platform::stream::{StreamLimits, StreamRegistry};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::io::{self, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

type Check<T = ()> = Result<T, String>;

const LKJOURNAL_ARTIFACT: &[u8] =
    include_bytes!("../../../../applications/lkjournal/generated/lkjournal.lkja");

fn checked<T, E: Debug>(result: Result<T, E>) -> Check<T> {
    result.map_err(|error| format!("{error:?}"))
}

fn failed(result: Result<NormalizedValue, ExecutionError>) -> Check<ExecutionError> {
    match result {
        Err(error) => Ok(error),
        Ok(_) => Err("the selected result owner was not refused".to_owned()),
    }
}

fn runtime() -> Check<tokio::runtime::Runtime> {
    checked(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build(),
    )
}

fn assert_factory_error(error: &ExecutionError, visible: bool) {
    let original = ExecutionError::from(StorageFailure::Allocation);
    assert_eq!(original.class, ExecutionFailureClass::Resource);
    assert_eq!(error.code, original.code);
    assert_eq!(error.message, original.message);
    assert!(matches!(&error.code, Cow::Borrowed(_)));
    assert!(matches!(&error.message, Cow::Borrowed(_)));
    assert_eq!(
        error.class,
        if visible {
            ExecutionFailureClass::PossibleVisibility
        } else {
            ExecutionFailureClass::Resource
        }
    );
    assert_eq!(error.possibly_visible, visible);
    assert!(!error.retryable);
}

fn requirement<'a>(program: &'a NormalizedProgram, name: &str) -> Check<&'a NormalizedRequirement> {
    let target_name = checked(Name::new("serve"))?;
    let target = program
        .root_target(&target_name)
        .ok_or_else(|| "maintained serve target is absent".to_owned())?;
    let component = program
        .components
        .get(target.component.0 as usize)
        .ok_or_else(|| "maintained component index is absent".to_owned())?;
    for index in component.requirements.iter() {
        let requirement = program
            .requirements
            .get(index.0 as usize)
            .ok_or_else(|| "maintained requirement index is absent".to_owned())?;
        if requirement.name.as_str() == name {
            return Ok(requirement);
        }
    }
    Err(format!("maintained requirement {name} is absent"))
}

fn operation<'a>(
    program: &'a NormalizedProgram,
    requirement: &NormalizedRequirement,
    name: &str,
) -> Check<&'a NormalizedOperation> {
    for index in requirement.operations.iter() {
        let operation = program
            .operations
            .get(index.0 as usize)
            .ok_or_else(|| "maintained operation index is absent".to_owned())?;
        if operation.name.as_str() == name {
            return Ok(operation);
        }
    }
    Err(format!("maintained operation {name} is absent"))
}

fn object_policy(
    requirement: &NormalizedRequirement,
    operation: &NormalizedOperation,
    operations: &BTreeSet<OperationReference>,
) -> NormalizedCallPolicy {
    NormalizedCallPolicy {
        requirement: requirement.reference,
        grant_requirement: requirement.reference,
        requirement_name: requirement.name.clone(),
        operation: operation.reference,
        operation_name: operation.name.clone(),
        idempotency: operation.idempotency,
        external_visibility: operation.external_visibility,
        requirement_limits: Arc::clone(&requirement.limits),
        grant: Arc::new(NormalizedCapabilityGrantDescriptor::for_test(
            requirement.interface,
            NormalizedAdapterKind::ObjectMemory,
            operations.clone(),
            BTreeMap::new(),
        )),
    }
}

fn record_fields(value: &NormalizedValue) -> Check<&[(Name, NormalizedValue)]> {
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = value else {
        return Err("result is not an exact structural record".to_owned());
    };
    Ok(fields.as_slice())
}

fn field<'a>(value: &'a NormalizedValue, name: &str) -> Check<&'a NormalizedValue> {
    record_fields(value)?
        .iter()
        .find_map(|(field, value)| (field.as_str() == name).then_some(value))
        .ok_or_else(|| format!("result field {name} is absent"))
}

fn assert_put(value: &NormalizedValue, key: &str, payload: &[u8]) -> Check {
    assert_eq!(record_fields(value)?.len(), 5);
    assert!(
        matches!(field(value, "key")?, NormalizedValue::Text(actual) if actual.as_ref() == key)
    );
    let size = checked(i64::try_from(payload.len()))?;
    assert!(matches!(field(value, "size")?, NormalizedValue::I64(actual) if *actual == size));
    let digest = blake3::hash(payload).to_hex().to_string();
    assert!(
        matches!(field(value, "blake3")?, NormalizedValue::Text(actual) if actual.as_ref() == digest)
    );
    assert!(matches!(
        field(value, "cleanup_pending")?,
        NormalizedValue::Bool(false)
    ));
    assert!(matches!(field(value, "version")?, NormalizedValue::Text(_)));
    Ok(())
}

#[test]
fn object_publication_survives_refused_result_owner_and_read_only_errors_remain_resource() -> Check
{
    // This is the maintained public artifact and its exact admitted standard
    // interface, not a fabricated/precomputed object codec.
    let program = checked(NormalizedProgram::prepare(checked(load_artifact(
        LKJOURNAL_ARTIFACT,
    ))?))?;
    let objects = requirement(&program, "objects")?;
    let streams = requirement(&program, "streams")?;
    let runtime = runtime()?;
    let engine = checked(ObjectEngine::in_memory(
        runtime.handle().clone(),
        "result-owner-failure".to_owned(),
        ObjectLimits {
            maximum_object_bytes: 1024,
            maximum_whole_read_bytes: 1024,
        },
    ))?;
    let adapter = checked(NormalizedObjectStorageAdapter::admit(
        &program,
        objects,
        NormalizedAdapterKind::ObjectMemory,
        &[streams.reference],
    ))?(engine.clone());
    let put = object_policy(
        objects,
        operation(&program, objects, "put-new")?,
        adapter.operations(),
    );
    let reconcile = object_policy(
        objects,
        operation(&program, objects, "reconcile-put")?,
        adapter.operations(),
    );
    assert_eq!(put.external_visibility, ExternalVisibility::Possible);
    // This exact maintained operation observes a publication; it grants no
    // external mutation. The adapter reads and hashes the existing object.
    assert_eq!(reconcile.external_visibility, ExternalVisibility::None);
    assert_eq!(reconcile.idempotency, Idempotency::Idempotent);
    let registry = checked(StreamRegistry::new(StreamLimits {
        maximum_chunk_bytes: 4,
        maximum_buffered_chunks: 2,
        maximum_total_bytes: 1024,
        maximum_live_streams: 4,
    }))?;
    let resources = checked(NormalizedResourceScope::new())?;
    let control = ExecutionControl::uncancelled();
    let key = "published-before-refusal";
    let payload = b"published bytes";
    let handle = checked(resources.register_byte_stream(
        streams.reference,
        streams.interface,
        checked(registry.register_memory(payload.to_vec()))?,
    ))?;
    let arguments = vec![
        NormalizedValue::text(key),
        NormalizedValue::Resource(handle),
        NormalizedValue::static_text("application/octet-stream"),
    ];
    let error = {
        let _refusal = refuse_owner_factory_after(0);
        failed(adapter.call(&put, arguments, &resources, &control))?
    };
    assert_factory_error(&error, true);
    // The generic outcome validator still rejects a forbidden visibility
    // contract; it does not silently relabel the completed effect as Resource.
    let mut forbidden = put.clone();
    forbidden.external_visibility = ExternalVisibility::None;
    let forbidden_error = failed(validate_outcome(&forbidden, Err(error)))?;
    assert_eq!(forbidden_error.class, ExecutionFailureClass::Infrastructure);
    assert_eq!(
        forbidden_error.code.as_ref(),
        "normalized_capability_visibility_contract"
    );
    checked(resources.close(streams.reference, handle))?;
    assert_eq!(resources.live_resources(), 0);

    // Observe the host store independently of the failed runtime result codec.
    assert_eq!(checked(engine.get(key, 1024, false))?, payload);
    let published = checked(engine.head(key, false))?;
    assert_eq!(published.key, key);
    assert_eq!(published.size, checked(i64::try_from(payload.len()))?);
    let receipts = checked(engine.reconcile_put(key, &control, false))?;
    let [receipt] = receipts.as_slice() else {
        return Err("completed publication has no unique reconciliation receipt".to_owned());
    };
    assert_eq!(receipt.key, key);
    assert_eq!(receipt.size, published.size);
    assert_eq!(receipt.blake3, blake3::hash(payload).to_hex().to_string());

    // The maintained objects requirement grants reconcile-put, not head.
    // Refuse the receipt owner after its read-only observation has completed.
    let arguments = vec![NormalizedValue::text(key)];
    let error = {
        let _refusal = refuse_owner_factory_after(0);
        failed(adapter.call(&reconcile, arguments, &resources, &control))?
    };
    assert_factory_error(&error, false);
    assert_eq!(checked(engine.head(key, false))?, published);
    assert_eq!(
        checked(engine.reconcile_put(key, &control, false))?,
        receipts
    );
    let reconciled = checked(adapter.call(
        &reconcile,
        vec![NormalizedValue::text(key)],
        &resources,
        &control,
    ))?;
    let NormalizedValue::List(reconciled) = &reconciled else {
        return Err("recovered reconciliation is not a list".to_owned());
    };
    let mut items = reconciled.iter();
    assert_put(
        items
            .next()
            .ok_or_else(|| "recovered receipt is absent".to_owned())?,
        key,
        payload,
    )?;
    assert!(items.next().is_none());

    // The guard restores factories. A different key can be published normally,
    // while the first object and its reconciliation evidence stay unchanged.
    let recovered_key = "published-after-refusal";
    let recovered_payload = b"independent recovery bytes";
    let recovered_handle = checked(resources.register_byte_stream(
        streams.reference,
        streams.interface,
        checked(registry.register_memory(recovered_payload.to_vec()))?,
    ))?;
    let recovered = checked(adapter.call(
        &put,
        vec![
            NormalizedValue::text(recovered_key),
            NormalizedValue::Resource(recovered_handle),
            NormalizedValue::static_text("application/octet-stream"),
        ],
        &resources,
        &control,
    ))?;
    assert_put(&recovered, recovered_key, recovered_payload)?;
    checked(resources.close(streams.reference, recovered_handle))?;
    assert_eq!(resources.live_resources(), 0);
    assert_eq!(
        checked(engine.get(recovered_key, 1024, false))?,
        recovered_payload
    );
    assert_eq!(checked(engine.get(key, 1024, false))?, payload);
    assert_eq!(
        checked(engine.reconcile_put(key, &control, false))?,
        receipts
    );
    Ok(())
}

fn serve_requests(listener: TcpListener, received: &AtomicUsize) -> Check<Vec<String>> {
    let mut requests = Vec::new();
    for body in *b"xyz" {
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err("missing once-only loopback request".to_owned());
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(error) => return Err(format!("accept loopback request: {error}")),
            }
        };
        checked(stream.set_read_timeout(Some(Duration::from_secs(15))))?;
        checked(stream.set_write_timeout(Some(Duration::from_secs(15))))?;
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            checked(stream.read_exact(&mut byte))?;
            request.push(byte[0]);
            if request.len() > 8192 {
                return Err("loopback request escaped the fixture's framing limit".to_owned());
            }
        }
        let request = checked(String::from_utf8(request))?;
        if !request.starts_with("GET /result-owner-failure HTTP/1.1\r\n") {
            return Err(format!("foreign loopback request: {request:?}"));
        }
        requests.push(request);
        received.fetch_add(1, Ordering::Release);
        checked(stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\n"))?;
        checked(stream.write_all(&[body]))?;
        checked(stream.flush())?;
        // Leave the write side open: the client must finish the complete sized
        // response, rather than relying on EOF to release its active request.
        assert_eq!(checked(stream.read(&mut [0]))?, 0);
    }
    Ok(requests)
}

fn assert_http_response(value: &NormalizedValue) -> Check {
    assert_eq!(record_fields(value)?.len(), 3);
    assert!(matches!(field(value, "status")?, NormalizedValue::I64(200)));
    assert!(matches!(field(value, "body")?, NormalizedValue::Bytes(body) if body.as_ref() == b"z"));
    let NormalizedValue::List(headers) = field(value, "headers")? else {
        return Err("recovered HTTP headers are not a list".to_owned());
    };
    let mut headers = headers.iter();
    let header = headers
        .next()
        .ok_or_else(|| "recovered Content-Length header is absent".to_owned())?;
    assert_eq!(record_fields(header)?.len(), 2);
    assert!(
        matches!(field(header, "name")?, NormalizedValue::Text(name) if name.as_ref() == "content-length")
    );
    assert!(
        matches!(field(header, "value")?, NormalizedValue::Bytes(value) if value.as_ref() == b"1")
    );
    assert!(headers.next().is_none());
    Ok(())
}

#[test]
fn completed_http_requests_survive_refused_nested_and_outer_result_owners_without_replay() -> Check
{
    let contract = checked(checked(BuiltinStandard::load())?.http_recipe_contract())?;
    let listener = checked(TcpListener::bind(("127.0.0.1", 0)))?;
    checked(listener.set_nonblocking(true))?;
    let endpoint = format!(
        "http://{}/result-owner-failure",
        checked(listener.local_addr())?
    );
    let runtime = runtime()?;
    let client = checked(HttpClient::prepare(
        &endpoint,
        HttpClientAddressPolicy::LoopbackOnly,
        &HttpClientTrust::WebpkiRoots,
        None,
        HttpClientLimits {
            maximum_concurrent_requests: 1,
            maximum_response_body_bytes: 1,
            ..HttpClientLimits::default()
        },
        runtime.handle().clone(),
    ))?;
    let adapter = checked(NormalizedHttpClientAdapter::new(
        contract.http_client_interface,
        contract.http_client_get,
        client,
    ))?;
    let requirement = RequirementReference {
        package: contract.http_client_interface.package,
        requirement: RequirementId::migrate(b"post-effect-child-storage-http", 0),
    };
    let policy = NormalizedCallPolicy {
        requirement,
        grant_requirement: requirement,
        requirement_name: checked(Name::new("outbound"))?,
        operation: contract.http_client_get,
        operation_name: checked(Name::new("get"))?,
        idempotency: Idempotency::Idempotent,
        external_visibility: ExternalVisibility::Possible,
        requirement_limits: Arc::from([]),
        grant: Arc::new(NormalizedCapabilityGrantDescriptor::for_test(
            contract.http_client_interface,
            NormalizedAdapterKind::HttpClient,
            adapter.operations().clone(),
            BTreeMap::new(),
        )),
    };
    let resources = checked(NormalizedResourceScope::new())?;
    let control = ExecutionControl::uncancelled();
    let first = vec![checked(NormalizedValue::list(Vec::new()))?];
    let second = vec![checked(NormalizedValue::list(Vec::new()))?];
    let recovery = vec![checked(NormalizedValue::list(Vec::new()))?];
    let received = AtomicUsize::new(0);

    std::thread::scope(|threads| -> Check {
        let peer = threads.spawn(|| serve_requests(listener, &received));
        let outcome = (|| -> Check {
            // One response header creates one nested Fields owner. Allowance 0
            // refuses it; allowance 1 permits it then refuses the outer record,
            // exercising cleanup of the already completed header/list payload.
            for (allowed, arguments) in [(0, first), (1, second)] {
                let error = {
                    let _refusal = refuse_owner_factory_after(allowed);
                    failed(adapter.call(&policy, arguments, &resources, &control))?
                };
                assert_factory_error(&error, true);
                assert_eq!(received.load(Ordering::Acquire), allowed + 1);
            }
            let recovered = checked(adapter.call(&policy, recovery, &resources, &control))?;
            assert_http_response(&recovered)?;
            assert_eq!(received.load(Ordering::Acquire), 3);
            Ok(())
        })();
        let shutdown = checked(adapter.shutdown());
        let served = peer
            .join()
            .map_err(|_| "loopback fixture thread failed".to_owned())
            .and_then(|result| result);
        outcome?;
        shutdown?;
        let requests = served?;
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0], requests[1]);
        assert_eq!(requests[1], requests[2]);
        assert_eq!(received.load(Ordering::Acquire), 3);
        assert_eq!(resources.live_resources(), 0);
        Ok(())
    })
}
