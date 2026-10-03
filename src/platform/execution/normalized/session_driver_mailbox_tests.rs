//! Controlled writer closure/completion around real, successfully completed graph callbacks.
use super::super::byte_stream::{NormalizedByteStreamAdapter, NormalizedByteStreamOperation};
use super::super::capability::{
    NormalizedCallPolicy, NormalizedCapabilityAdapter, NormalizedCapabilityGrant,
    NormalizedCapabilityGrantDescriptor, NormalizedGrantLimit,
};
use super::super::deployment::{NormalizedDeploymentResourcePolicy, NormalizedPreparedDeployment};
use super::super::vm::NormalizedRunPolicy;
use super::*;
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::{OperationReference, ResourceUnit};
use crate::platform::project_creation::{ProjectTemplate, create_project};
use crate::platform::publication::{GraphRepository, PublicationOutcome};
use crate::platform::runtime::ResidentLimits;
use std::collections::BTreeSet;
use std::sync::Mutex;
use std::task::{Context, Waker};

struct ControlledClock {
    interface: DeclarationReference,
    operations: BTreeSet<OperationReference>,
    sender: mailbox::Sender<WriterCommand>,
    receiver: Mutex<Option<mailbox::Receiver<WriterCommand>>>,
    calls: AtomicUsize,
    shutdowns: AtomicUsize,
}

impl NormalizedCapabilityAdapter for ControlledClock {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::WallClock
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
        _: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        assert_eq!(policy.grant.interface, self.interface);
        assert!(self.operations.contains(&policy.operation));
        assert!(arguments.is_empty());
        self.calls.fetch_add(1, Ordering::SeqCst);
        // The initial batch has already been consumed. Only invoke_reserved's
        // capacity permit can account for this full, otherwise empty mailbox.
        assert_eq!(
            self.sender.try_reserve().err(),
            Some(mailbox::ReserveError::Full)
        );
        if let Some(receiver) = self.receiver.lock().unwrap().as_mut() {
            receiver.close();
        }
        // This effect succeeds even when the fixture closes the writer. The VM
        // and session decoder must still finish before a refused commit is observed.
        Ok(NormalizedValue::I64(41))
    }

    fn shutdown(&self) -> Result<(), ExecutionError> {
        self.shutdowns.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn application(
    sender: mailbox::Sender<WriterCommand>,
) -> (
    Arc<NormalizedSessionApplication>,
    Arc<ControlledClock>,
    crate::platform::runtime::structured::StructuredExecutor,
) {
    application_with_limits(
        sender,
        SessionLimits {
            close_grace_milliseconds: 30_000,
            ..SessionLimits::default()
        },
    )
}

fn application_with_limits(
    sender: mailbox::Sender<WriterCommand>,
    limits: SessionLimits,
) -> (
    Arc<NormalizedSessionApplication>,
    Arc<ControlledClock>,
    crate::platform::runtime::structured::StructuredExecutor,
) {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("source");
    let created = create_project(&project, "mailbox-callback", ProjectTemplate::Command).unwrap();
    let repository = GraphRepository::open(&project).unwrap();
    let request = format!(
        "request base={}\n{}",
        created.revision,
        include_str!("../../../../tests/fixtures/session_mailbox_callback.lkchg")
    );
    let decoded =
        crate::platform::control::decode_compact_change("mailbox-callback", request.as_bytes())
            .unwrap();
    let prepared = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    assert!(matches!(
        repository.publish(&prepared.publication).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    let prepared = crate::platform::normalized_lifecycle::prepare_repository(repository).unwrap();
    let program = prepared.program;
    let target = Name::new("mailbox-live").unwrap();
    let component = program.root_target(&target).unwrap().component;
    let requirement = |name: &str| {
        program.components[component.0 as usize]
            .requirements
            .iter()
            .map(|index| &program.requirements[index.0 as usize])
            .find(|requirement| requirement.name.as_str() == name)
            .unwrap()
    };
    let clock = requirement("clock");
    let clock_operations = clock
        .operations
        .iter()
        .map(|index| program.operations[index.0 as usize].reference)
        .collect();
    let adapter = Arc::new(ControlledClock {
        interface: clock.interface,
        operations: clock_operations,
        sender,
        receiver: Mutex::new(None),
        calls: AtomicUsize::new(0),
        shutdowns: AtomicUsize::new(0),
    });
    let streams = requirement("streams");
    assert_eq!(streams.operations.len(), 1);
    let read_all = program.operations[streams.operations[0].0 as usize].reference;
    let stream_adapter = Arc::new(
        NormalizedByteStreamAdapter::new_selected(
            streams.reference,
            streams.interface,
            BTreeMap::from([(read_all, NormalizedByteStreamOperation::ReadAll)]),
        )
        .unwrap(),
    );
    let grant =
        |requirement, adapter: Arc<dyn NormalizedCapabilityAdapter>| NormalizedCapabilityGrant {
            requirement,
            descriptor: NormalizedCapabilityGrantDescriptor::for_test(
                adapter.interface(),
                adapter.kind(),
                adapter.operations().clone(),
                BTreeMap::from([(
                    Name::new("maximum_calls").unwrap(),
                    NormalizedGrantLimit {
                        maximum: 8,
                        unit: ResourceUnit::Calls,
                    },
                )]),
            ),
            adapter,
        };
    let deployment = NormalizedPreparedDeployment::prepare_exact_for_test(
        &program,
        target,
        vec![
            grant(clock.reference, adapter.clone()),
            grant(streams.reference, stream_adapter),
        ],
        NormalizedDeploymentResourcePolicy::default(),
    )
    .unwrap();
    let executor = crate::platform::runtime::structured::StructuredExecutor::for_test(1);
    let resident = NormalizedResidentDeployment::prepare(
        program,
        deployment,
        ResidentLimits::default(),
        NormalizedRunPolicy::default(),
        &executor.handle(),
    )
    .unwrap();
    (
        Arc::new(NormalizedSessionApplication::new(resident, limits).unwrap()),
        adapter,
        executor,
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_mailbox_writer_close_inside_callback_fails_without_replay_or_next_event() {
    let (sender, mut receiver) = mailbox::channel(1).unwrap();
    let (application, adapter, _executor) = application(sender.clone());
    let (inbound, mut events) = mpsc::channel(2);
    let mut driver = Box::pin(session_driver(
        Arc::clone(&application),
        &sender,
        &mut events,
        NormalizedValue::I64(0),
        Vec::new(),
    ));
    // Poll only to initial-batch completion. No graph callback has run yet.
    assert!(
        driver
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let initial = receiver.recv().await.unwrap();
    assert!(initial.messages.is_empty());
    assert!(initial.finished.send(true).is_ok());
    *adapter.receiver.lock().unwrap() = Some(receiver);
    for body in [b"first".as_slice(), b"second".as_slice()] {
        inbound
            .send(InboundEvent::Message {
                kind: InboundKind::Text,
                body: Bytes::copy_from_slice(body),
                permit: None,
            })
            .await
            .unwrap();
    }
    drop(inbound);
    // The inner completion timeout is 30 seconds; refused commit must not wait
    // for it. On failure, drop the driver and join resident cleanup before asserting.
    let outcome = tokio::time::timeout(Duration::from_secs(5), driver).await;
    let cleanup = application.shutdown().await;
    let resident = application.resident().observe();
    let mut receiver = adapter.receiver.lock().unwrap().take().unwrap();
    assert!(matches!(outcome, Ok(SessionOutcome::Failed)));
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        (resident.admitted, resident.completed, resident.failed),
        (1, 1, 0)
    );
    // This count advances only after valid callback output is decoded. Thus a
    // callback/shape error cannot impersonate the desired refused-commit failure.
    assert_eq!(application.observe().outbound_messages, 1);
    assert!(receiver.recv().await.is_none());
    assert_eq!(
        sender.try_reserve().err(),
        Some(mailbox::ReserveError::Closed)
    );
    let Some(InboundEvent::Message { body, .. }) = events.recv().await else {
        panic!("refused transition must leave the next event unconsumed");
    };
    assert_eq!(body.as_ref(), b"second");
    assert!(events.recv().await.is_none());
    assert_eq!((resident.active, resident.queued), (0, 0));
    assert_eq!(application.resident().deployment().live_streams(), 0);
    assert_eq!(cleanup.remaining_tasks, 0);
    assert!(cleanup.cleanup_failures.is_empty());
    assert_eq!(adapter.shutdowns.load(Ordering::SeqCst), 1);
    // This observes failed continuation and no subsequent state consumer; it
    // deliberately does not instrument the driver's private local assignment.
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_lifetime_after_successful_write_stops_queued_callbacks_without_replay() {
    // Message takes the driver's fast `continue`; tick takes its other continuation.
    // Both must observe the same lifetime before admitting a queued next event.
    for first_is_tick in [false, true] {
        let (sender, mut receiver) = mailbox::channel(1).unwrap();
        let limits = SessionLimits {
            maximum_lifetime_milliseconds: 2_000,
            idle_timeout_milliseconds: 2_000,
            tick_interval_milliseconds: if first_is_tick { 1 } else { 10_000 },
            ..SessionLimits::default()
        };
        let lifetime = Duration::from_millis(limits.maximum_lifetime_milliseconds);
        let (application, adapter, _executor) = application_with_limits(sender.clone(), limits);
        let (inbound, mut events) = mpsc::channel(2);
        let message = |body: &'static [u8]| InboundEvent::Message {
            kind: InboundKind::Text,
            body: Bytes::from_static(body),
            permit: None,
        };
        if !first_is_tick {
            inbound.send(message(b"first")).await.unwrap();
            inbound.send(message(b"second")).await.unwrap();
        }
        let driver = session_driver(
            Arc::clone(&application),
            &sender,
            &mut events,
            NormalizedValue::I64(0),
            Vec::new(),
        );
        let controlled_writer = async {
            let mut observed = Vec::new();
            let mut calls_around_delayed_ack = None;
            while let Some(command) = receiver.recv().await {
                if observed.len() == 1 {
                    if first_is_tick {
                        // Let the first tick run before making inbound events ready.
                        inbound.send(message(b"first")).await.unwrap();
                        inbound.send(message(b"second")).await.unwrap();
                    }
                    let before = adapter.calls.load(Ordering::SeqCst);
                    // Start this wait only after a real callback produced its batch.
                    // Its lower bound therefore crosses the driver's lifetime even
                    // if the callback was slow, while remaining within write grace.
                    tokio::time::sleep(lifetime).await;
                    calls_around_delayed_ack = Some((before, adapter.calls.load(Ordering::SeqCst)));
                }
                let terminal = command.close.is_some();
                observed.push((
                    command.messages.len(),
                    command
                        .close
                        .map(|close| (close.code, close.reason.to_string())),
                    command.finished.send(true).is_ok(),
                ));
                if terminal {
                    break;
                }
            }
            (observed, calls_around_delayed_ack)
        };
        // Keep both senders alive: an empty mailbox must not impersonate transport
        // failure. On timeout, drop both futures, then join resident cleanup before
        // asserting. No detached driver/writer task survives a failed regression.
        let result = tokio::time::timeout(Duration::from_secs(10), async {
            tokio::join!(driver, controlled_writer)
        })
        .await;
        let cleanup = application.shutdown().await;
        let resident = application.resident().observe();
        drop(inbound);
        let mut pending = Vec::new();
        while let Some(event) = events.recv().await {
            let InboundEvent::Message { body, .. } = event else {
                panic!("fixture only queues messages");
            };
            pending.push(body);
        }
        let Ok((outcome, (commands, calls_around_delayed_ack))) = result else {
            panic!("driver/writer must finish and join within the finite fixture deadline");
        };
        assert!(matches!(outcome, SessionOutcome::Completed));
        assert_eq!(calls_around_delayed_ack, Some((1, 1)));
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            commands,
            [
                (0, None, true),
                (1, None, true),
                (
                    0,
                    Some((
                        SESSION_SHUTDOWN_CLOSE,
                        "session lifetime reached".to_owned()
                    )),
                    true
                ),
            ],
            "initial batch, one accepted transition, then one bounded lifetime close"
        );
        let expected: &[&[u8]] = if first_is_tick {
            &[b"first", b"second"]
        } else {
            &[b"second"]
        };
        assert_eq!(
            pending.iter().map(Bytes::as_ref).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(application.observe().outbound_messages, 1);
        assert_eq!(
            (resident.admitted, resident.completed, resident.failed),
            (1, 1, 0)
        );
        assert_eq!((resident.active, resident.queued), (0, 0));
        assert_eq!(application.resident().deployment().live_streams(), 0);
        assert_eq!(cleanup.remaining_tasks, 0);
        assert!(cleanup.cleanup_failures.is_empty());
        assert_eq!(adapter.shutdowns.load(Ordering::SeqCst), 1);
        // Capacity one is fully free: no retained reservation or queued command.
        assert!(sender.try_reserve().is_ok());
        drop(receiver);
    }
}
