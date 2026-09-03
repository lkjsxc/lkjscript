//! Structured ownership for graph-defined state machines over bounded WebSocket sessions.

use super::capability::NormalizedAdapterKind;
use super::prepare::{
    NormalizedProgram, NormalizedRecordLayout, NormalizedTarget, NormalizedVariantLayout,
};
use super::resident::NormalizedResidentDeployment;
use super::resource::NormalizedResourceScope;
use super::value::{NormalizedRecord, NormalizedValue, RecordLayoutIndex, VariantLayoutIndex};
use crate::platform::builtin_standard::BuiltinStandard;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionError, ExecutionFailureClass};
use crate::platform::http::{
    HttpHeader, HttpResponse, encode_live_response, safe_error_response, static_response,
};
use crate::platform::kernel::{
    DeclarationReference, Name, RequirementReference, TypeForm, TypeObject, TypeObjectDigest,
};
use crate::platform::package::RunnerKind;
use crate::platform::session::{
    STRUCTURED_SESSION_CONTRACT_VERSION, SessionLimits, SessionNominalShape, SessionObservation,
    SessionPortRelation, SessionServerReceipt, SessionShapeRead, validate_session_function_type,
};
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket, WebSocketUpgrade};
use axum::http::{Request, Response, StatusCode, Version};
use bytes::Bytes;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot, watch};

const MAXIMUM_SESSION_TARGET_BYTES: usize = 16 * 1024;
const SESSION_PROTOCOL_CLOSE: u16 = 1002;
const SESSION_RESOURCE_CLOSE: u16 = 1009;
const SESSION_INTERNAL_CLOSE: u16 = 1011;
const SESSION_SHUTDOWN_CLOSE: u16 = 1001;

#[derive(Clone)]
pub(crate) struct NormalizedSessionApplication {
    resident: NormalizedResidentDeployment,
    limits: SessionLimits,
    contract: SessionRuntimeContract,
    stream_requirement: RequirementReference,
    admission: Arc<SessionAdmission>,
    shutdown: watch::Sender<bool>,
}

impl NormalizedSessionApplication {
    pub(crate) fn new(
        resident: NormalizedResidentDeployment,
        limits: SessionLimits,
    ) -> Result<Self, Diagnostic> {
        limits.validate()?;
        if resident.target().runner != RunnerKind::Interactive {
            return Err(session_source(
                "normalized_session_runner_kind",
                "structured-session adapter requires an interactive runner target",
            ));
        }
        let contract = SessionRuntimeContract::prepare(resident.program(), resident.target())?;
        let mut stream_requirements = resident
            .deployment()
            .observation()
            .grants
            .iter()
            .filter_map(|(requirement, grant)| {
                (grant.adapter_kind == NormalizedAdapterKind::ByteStream).then_some(*requirement)
            });
        let stream_requirement = stream_requirements.next().ok_or_else(|| {
            session_source(
                "normalized_session_stream_grant",
                "interactive deployment requires one exact byte-stream capability grant",
            )
        })?;
        if stream_requirements.next().is_some() {
            return Err(session_source(
                "normalized_session_stream_grant_ambiguous",
                "interactive deployment has more than one possible message-stream grant",
            ));
        }
        let maximum_stream_bytes = resident
            .deployment()
            .observation()
            .resources
            .streams
            .maximum_total_bytes;
        if maximum_stream_bytes < u64::try_from(limits.maximum_message_bytes).unwrap_or(u64::MAX) {
            return Err(session_source(
                "normalized_session_stream_limit",
                "stream total-byte limit is smaller than the accepted session message limit",
            ));
        }
        let admission = Arc::new(SessionAdmission::new(&limits)?);
        let (shutdown, _) = watch::channel(false);
        Ok(Self {
            resident,
            limits,
            contract,
            stream_requirement,
            admission,
            shutdown,
        })
    }

    pub(crate) fn router(self) -> Router {
        Router::new()
            .fallback(normalized_session_handler)
            .with_state(Arc::new(self))
    }

    pub(crate) async fn serve(
        self,
        listener: TcpListener,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<SessionServerReceipt, Diagnostic> {
        let local_address = listener.local_addr().map_err(|error| {
            session_infrastructure(
                "normalized_session_listener_address",
                format!("structured-session listener address failed: {error}"),
            )
        })?;
        let resident = self.resident.clone();
        let admission = Arc::clone(&self.admission);
        let cancellation_grace = Duration::from_millis(self.limits.cancellation_grace_milliseconds);
        let shutdown_signal = self.shutdown.clone();
        let serving = axum::serve(listener, self.router())
            .with_graceful_shutdown(async move {
                shutdown.await;
                let _ = shutdown_signal.send(true);
            })
            .await
            .map_err(|error| {
                session_infrastructure(
                    "normalized_session_serve",
                    format!("structured-session server failed: {error}"),
                )
            });
        let drained = admission.wait_idle(cancellation_grace).await;
        let resident_shutdown = resident.shutdown().await;
        if !drained {
            return Err(session_infrastructure(
                "normalized_session_shutdown_incomplete",
                "one or more parent session scopes remained after cancellation grace",
            ));
        }
        if let Err(mut error) = serving {
            if resident_shutdown.remaining_tasks != 0 {
                error.notes.push(format!(
                    "{} resident callbacks remained after session server failure cleanup",
                    resident_shutdown.remaining_tasks
                ));
            }
            error
                .notes
                .extend(resident_shutdown.cleanup_failures.iter().map(|failure| {
                    format!("adapter cleanup failed with safe code '{}'", failure.code)
                }));
            return Err(error);
        }
        if resident_shutdown.remaining_tasks != 0 || !resident_shutdown.cleanup_failures.is_empty()
        {
            return Err(session_infrastructure(
                "normalized_session_resident_shutdown_incomplete",
                format!(
                    "{} resident callbacks and {} adapter cleanup failures remained after structured-session shutdown",
                    resident_shutdown.remaining_tasks,
                    resident_shutdown.cleanup_failures.len()
                ),
            ));
        }
        Ok(SessionServerReceipt {
            contract_version: STRUCTURED_SESSION_CONTRACT_VERSION,
            local_address: local_address.to_string(),
            accepted_at_transport: true,
            sessions: admission.observe(),
            shutdown: resident_shutdown,
        })
    }

    pub(crate) fn observe(&self) -> SessionObservation {
        self.admission.observe()
    }

    pub(crate) fn resident(&self) -> &NormalizedResidentDeployment {
        &self.resident
    }

    pub(crate) async fn shutdown(&self) -> crate::platform::runtime::ShutdownReceipt {
        let _ = self.shutdown.send(true);
        let grace = Duration::from_millis(self.limits.cancellation_grace_milliseconds);
        let _ = self.admission.wait_idle(grace).await;
        self.resident.shutdown().await
    }

    async fn invoke_open(
        &self,
        request: SessionOpenRequest,
    ) -> Result<DecodedDecision, ExecutionError> {
        let event = self.contract.event(
            "open",
            Some(structural([
                ("headers", headers_value(request.headers)?),
                ("path", NormalizedValue::text(request.path)),
                ("query", NormalizedValue::text(request.query)),
            ])?),
        )?;
        let receipt = self
            .resident
            .invoke(vec![NormalizedValue::Option(None), event])
            .await?;
        self.contract.decode_decision(
            self.resident.program(),
            receipt.value,
            SessionPhase::Open,
            &self.limits,
        )
    }

    async fn invoke_event(
        &self,
        state: NormalizedValue,
        event: SessionInput,
    ) -> Result<DecodedDecision, ExecutionError> {
        let resources = NormalizedResourceScope::new()?;
        let (phase, event) = match event {
            SessionInput::Message { kind, body } => {
                let kind = self.contract.message_kind(match kind {
                    InboundKind::Text => "text",
                    InboundKind::Binary => "binary",
                })?;
                let body = self.resident.deployment().register_memory_stream(
                    self.stream_requirement,
                    &resources,
                    body.to_vec(),
                )?;
                (
                    SessionPhase::Message,
                    self.contract.event(
                        "message",
                        Some(structural([("body", body), ("kind", kind)])?),
                    )?,
                )
            }
            SessionInput::Tick => (SessionPhase::Tick, self.contract.event("tick", None)?),
            SessionInput::PeerClose { code, reason } => {
                let code = code.map(|value| Box::new(NormalizedValue::I64(i64::from(value))));
                (
                    SessionPhase::PeerClose,
                    self.contract.event(
                        "peer-close",
                        Some(structural([
                            ("code", NormalizedValue::Option(code)),
                            ("reason", NormalizedValue::text(reason)),
                        ])?),
                    )?,
                )
            }
            SessionInput::Shutdown => (
                SessionPhase::Shutdown,
                self.contract.event("shutdown", None)?,
            ),
        };
        let receipt = self
            .resident
            .invoke_scoped(
                resources,
                vec![NormalizedValue::Option(Some(Box::new(state))), event],
            )
            .await?;
        self.contract
            .decode_decision(self.resident.program(), receipt.value, phase, &self.limits)
    }
}

pub(crate) fn validate_program_interactive_targets(
    program: &NormalizedProgram,
) -> Result<(), Diagnostic> {
    for target in program.targets.values() {
        if target.runner == RunnerKind::Interactive {
            SessionRuntimeContract::prepare(program, target)?;
        }
    }
    Ok(())
}

async fn normalized_session_handler(
    State(application): State<Arc<NormalizedSessionApplication>>,
    upgrade: WebSocketUpgrade,
    request: Request<Body>,
) -> Response<Body> {
    let pending = match application.admission.try_pending() {
        Some(permit) => permit,
        None => {
            application
                .admission
                .counters
                .overloaded_handshakes
                .fetch_add(1, Ordering::AcqRel);
            return static_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "session handshake capacity reached",
            );
        }
    };
    let request = match decode_open_request(request, &application.limits) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let session = match application.admission.try_session() {
        Some(permit) => permit,
        None => {
            application
                .admission
                .counters
                .overloaded_handshakes
                .fetch_add(1, Ordering::AcqRel);
            return static_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "session admission capacity reached",
            );
        }
    };
    let decision = match application.invoke_open(request).await {
        Ok(decision) => decision,
        Err(error) => {
            application
                .admission
                .counters
                .rejected_handshakes
                .fetch_add(1, Ordering::AcqRel);
            return safe_error_response(&error, StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    match decision.kind {
        DecisionKind::Reject => {
            application
                .admission
                .counters
                .rejected_handshakes
                .fetch_add(1, Ordering::AcqRel);
            let Some(response) = decision.rejection else {
                return static_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "session rejection was malformed",
                );
            };
            encode_live_response(response).unwrap_or_else(|error| {
                safe_error_response(&error, StatusCode::INTERNAL_SERVER_ERROR)
            })
        }
        DecisionKind::Accept => {
            let Some(state) = decision.state else {
                return static_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "session acceptance was malformed",
                );
            };
            application
                .admission
                .counters
                .admitted_sessions
                .fetch_add(1, Ordering::AcqRel);
            let application = Arc::clone(&application);
            let messages = decision.messages;
            let upgrade = upgrade
                .read_buffer_size(application.limits.maximum_frame_bytes)
                .write_buffer_size(0)
                .max_write_buffer_size(application.limits.maximum_outbound_mailbox_bytes)
                .max_message_size(application.limits.maximum_message_bytes)
                .max_frame_size(application.limits.maximum_frame_bytes)
                .accept_unmasked_frames(false);
            upgrade.on_upgrade(move |socket| async move {
                drop(pending);
                run_parent_session(application, socket, state, messages, session).await;
            })
        }
        _ => static_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "session open returned a phase-invalid decision",
        ),
    }
}

async fn run_parent_session(
    application: Arc<NormalizedSessionApplication>,
    socket: WebSocket,
    initial_state: NormalizedValue,
    initial_messages: Vec<SessionOutbound>,
    _session: ActiveSessionGuard,
) {
    let (sink, stream) = socket.split();
    let (inbound_tx, mut inbound_rx) =
        mpsc::channel(application.limits.maximum_inbound_mailbox_items);
    let (writer_tx, writer_rx) = mpsc::channel(1);
    let inbound_bytes = Arc::new(Semaphore::new(
        application.limits.maximum_inbound_mailbox_bytes,
    ));
    let (cancel, cancel_rx) = watch::channel(false);
    let reader = tokio::spawn(session_reader(
        stream,
        inbound_tx,
        inbound_bytes,
        cancel_rx.clone(),
        Arc::clone(&application.admission),
    ));
    let writer = tokio::spawn(session_writer(sink, writer_rx, cancel_rx));
    let outcome = session_driver(
        Arc::clone(&application),
        &writer_tx,
        &mut inbound_rx,
        initial_state,
        initial_messages,
    )
    .await;
    let _ = cancel.send(true);
    drop(writer_tx);
    let grace = Duration::from_millis(application.limits.cancellation_grace_milliseconds);
    join_child(reader, grace).await;
    join_child(writer, grace).await;
    match outcome {
        SessionOutcome::Completed => {
            application
                .admission
                .counters
                .completed_sessions
                .fetch_add(1, Ordering::AcqRel);
        }
        SessionOutcome::Failed => {
            application
                .admission
                .counters
                .failed_sessions
                .fetch_add(1, Ordering::AcqRel);
        }
    }
}

async fn session_driver(
    application: Arc<NormalizedSessionApplication>,
    writer: &mpsc::Sender<WriterCommand>,
    inbound: &mut mpsc::Receiver<InboundEvent>,
    mut state: NormalizedValue,
    initial_messages: Vec<SessionOutbound>,
) -> SessionOutcome {
    record_outbound(&application.admission, &initial_messages);
    if send_writer(
        writer,
        initial_messages,
        None,
        Duration::from_millis(application.limits.close_grace_milliseconds),
    )
    .await
    .is_err()
    {
        return SessionOutcome::Failed;
    }
    let started = Instant::now();
    let lifetime = Duration::from_millis(application.limits.maximum_lifetime_milliseconds);
    let idle = Duration::from_millis(application.limits.idle_timeout_milliseconds);
    let tick = Duration::from_millis(application.limits.tick_interval_milliseconds);
    let lifetime_sleep = tokio::time::sleep(lifetime);
    let idle_sleep = tokio::time::sleep(idle);
    let tick_sleep = tokio::time::sleep(tick);
    tokio::pin!(lifetime_sleep);
    tokio::pin!(idle_sleep);
    tokio::pin!(tick_sleep);
    let mut shutdown = application.shutdown.subscribe();
    loop {
        let input = tokio::select! {
            biased;
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    DriverInput::Shutdown
                } else {
                    continue;
                }
            }
            inbound = inbound.recv() => match inbound {
                Some(InboundEvent::Message { kind, body, permit }) => {
                    idle_sleep.as_mut().reset(tokio::time::Instant::now() + idle);
                    DriverInput::Message { kind, body, _permit: permit }
                }
                Some(InboundEvent::PeerClose { code, reason }) => DriverInput::PeerClose { code, reason },
                Some(InboundEvent::TransportFailure) | None => {
                    return close_failed(
                        writer,
                        SESSION_PROTOCOL_CLOSE,
                        "transport failure",
                        &application.limits,
                    )
                    .await;
                }
            },
            () = &mut lifetime_sleep => DriverInput::OperationalClose(SESSION_SHUTDOWN_CLOSE, "session lifetime reached"),
            () = &mut idle_sleep => DriverInput::OperationalClose(SESSION_SHUTDOWN_CLOSE, "session idle limit reached"),
            () = &mut tick_sleep => {
                tick_sleep.as_mut().reset(tokio::time::Instant::now() + tick);
                DriverInput::Tick
            }
        };
        let (session_input, terminal, terminal_close, flush_peer_close) = match input {
            DriverInput::Message {
                kind,
                body,
                _permit,
            } => {
                let decision = invoke_reserved(
                    &application,
                    writer,
                    state,
                    SessionInput::Message { kind, body },
                )
                .await;
                drop(_permit);
                match decision {
                    Ok((decision, reservation)) => match decision.kind {
                        DecisionKind::Continue => {
                            let Some(next) = decision.state else {
                                drop(reservation);
                                return close_failed(
                                    writer,
                                    SESSION_PROTOCOL_CLOSE,
                                    "invalid session state",
                                    &application.limits,
                                )
                                .await;
                            };
                            record_outbound(&application.admission, &decision.messages);
                            if send_reserved(
                                reservation,
                                decision.messages,
                                None,
                                Duration::from_millis(application.limits.close_grace_milliseconds),
                            )
                            .await
                            .is_err()
                            {
                                return SessionOutcome::Failed;
                            }
                            state = next;
                            continue;
                        }
                        DecisionKind::Close => {
                            let Some(close) = decision.closing else {
                                drop(reservation);
                                return close_failed(
                                    writer,
                                    SESSION_PROTOCOL_CLOSE,
                                    "invalid close decision",
                                    &application.limits,
                                )
                                .await;
                            };
                            record_outbound(&application.admission, &decision.messages);
                            let _ = send_reserved(
                                reservation,
                                decision.messages,
                                Some(close),
                                Duration::from_millis(application.limits.close_grace_milliseconds),
                            )
                            .await;
                            return SessionOutcome::Completed;
                        }
                        _ => {
                            drop(reservation);
                            return close_failed(
                                writer,
                                SESSION_PROTOCOL_CLOSE,
                                "phase-invalid session decision",
                                &application.limits,
                            )
                            .await;
                        }
                    },
                    Err(error) => {
                        let code = if error.class == ExecutionFailureClass::Resource {
                            SESSION_RESOURCE_CLOSE
                        } else {
                            SESSION_INTERNAL_CLOSE
                        };
                        return close_failed(
                            writer,
                            code,
                            "session callback failed",
                            &application.limits,
                        )
                        .await;
                    }
                }
            }
            DriverInput::Tick => (SessionInput::Tick, false, None, false),
            DriverInput::PeerClose { code, reason } => {
                (SessionInput::PeerClose { code, reason }, true, None, true)
            }
            DriverInput::Shutdown => (
                SessionInput::Shutdown,
                true,
                Some(CloseFrame {
                    code: SESSION_SHUTDOWN_CLOSE,
                    reason: Utf8Bytes::from("service shutdown"),
                }),
                false,
            ),
            DriverInput::OperationalClose(code, reason) => {
                let _ = send_writer(
                    writer,
                    Vec::new(),
                    Some(CloseFrame {
                        code,
                        reason: Utf8Bytes::from(reason),
                    }),
                    Duration::from_millis(application.limits.close_grace_milliseconds),
                )
                .await;
                return SessionOutcome::Completed;
            }
        };
        let decision = invoke_reserved(&application, writer, state, session_input).await;
        match decision {
            Ok((decision, reservation)) if terminal && decision.kind == DecisionKind::Finish => {
                drop(reservation);
                if flush_peer_close {
                    let _ = flush_peer_close_reply(
                        writer,
                        Duration::from_millis(application.limits.close_grace_milliseconds),
                    )
                    .await;
                } else if let Some(close) = terminal_close {
                    let _ = send_writer(
                        writer,
                        Vec::new(),
                        Some(close),
                        Duration::from_millis(application.limits.close_grace_milliseconds),
                    )
                    .await;
                }
                return SessionOutcome::Completed;
            }
            Ok((decision, reservation)) if !terminal && decision.kind == DecisionKind::Continue => {
                let Some(next) = decision.state else {
                    drop(reservation);
                    return close_failed(
                        writer,
                        SESSION_PROTOCOL_CLOSE,
                        "invalid session state",
                        &application.limits,
                    )
                    .await;
                };
                record_outbound(&application.admission, &decision.messages);
                if send_reserved(
                    reservation,
                    decision.messages,
                    None,
                    Duration::from_millis(application.limits.close_grace_milliseconds),
                )
                .await
                .is_err()
                {
                    return SessionOutcome::Failed;
                }
                state = next;
            }
            Ok((decision, reservation)) if !terminal && decision.kind == DecisionKind::Close => {
                let Some(close) = decision.closing else {
                    drop(reservation);
                    return close_failed(
                        writer,
                        SESSION_PROTOCOL_CLOSE,
                        "invalid close decision",
                        &application.limits,
                    )
                    .await;
                };
                record_outbound(&application.admission, &decision.messages);
                let _ = send_reserved(
                    reservation,
                    decision.messages,
                    Some(close),
                    Duration::from_millis(application.limits.close_grace_milliseconds),
                )
                .await;
                return SessionOutcome::Completed;
            }
            Ok((_, reservation)) => {
                drop(reservation);
                return close_failed(
                    writer,
                    SESSION_PROTOCOL_CLOSE,
                    "phase-invalid session decision",
                    &application.limits,
                )
                .await;
            }
            Err(error) => {
                let code = if error.class == ExecutionFailureClass::Resource {
                    SESSION_RESOURCE_CLOSE
                } else {
                    SESSION_INTERNAL_CLOSE
                };
                return close_failed(writer, code, "session callback failed", &application.limits)
                    .await;
            }
        }
        if started.elapsed() >= lifetime {
            return close_failed(
                writer,
                SESSION_SHUTDOWN_CLOSE,
                "session lifetime reached",
                &application.limits,
            )
            .await;
        }
    }
}

async fn invoke_reserved(
    application: &NormalizedSessionApplication,
    writer: &mpsc::Sender<WriterCommand>,
    state: NormalizedValue,
    input: SessionInput,
) -> Result<(DecodedDecision, mpsc::OwnedPermit<WriterCommand>), ExecutionError> {
    // Reserving the only transition-batch slot before graph execution makes queue-capacity
    // failure impossible after an effectful callback. The caller retains this exact permit until
    // it commits the fully validated batch.
    let permit = writer.clone().reserve_owned().await.map_err(|_| {
        session_execution(
            "session_writer_closed",
            "session writer closed before transition admission",
        )
    })?;
    application
        .invoke_event(state, input)
        .await
        .map(|decision| (decision, permit))
}

async fn send_reserved(
    reservation: mpsc::OwnedPermit<WriterCommand>,
    messages: Vec<SessionOutbound>,
    close: Option<CloseFrame>,
    timeout: Duration,
) -> Result<(), ()> {
    let (finished, completion) = oneshot::channel();
    let _ = reservation.send(WriterCommand {
        messages,
        close,
        flush_peer_close: false,
        finished,
    });
    match tokio::time::timeout(timeout, completion).await {
        Ok(Ok(true)) => Ok(()),
        Ok(Ok(false)) | Ok(Err(_)) | Err(_) => Err(()),
    }
}

async fn flush_peer_close_reply(
    writer: &mpsc::Sender<WriterCommand>,
    timeout: Duration,
) -> Result<(), ()> {
    let (finished, completion) = oneshot::channel();
    writer
        .send(WriterCommand {
            messages: Vec::new(),
            close: None,
            flush_peer_close: true,
            finished,
        })
        .await
        .map_err(|_| ())?;
    match tokio::time::timeout(timeout, completion).await {
        Ok(Ok(true)) => Ok(()),
        Ok(Ok(false)) | Ok(Err(_)) | Err(_) => Err(()),
    }
}

async fn close_failed(
    writer: &mpsc::Sender<WriterCommand>,
    code: u16,
    reason: &'static str,
    limits: &SessionLimits,
) -> SessionOutcome {
    let _ = send_writer(
        writer,
        Vec::new(),
        Some(CloseFrame {
            code,
            reason: Utf8Bytes::from(reason),
        }),
        Duration::from_millis(limits.close_grace_milliseconds),
    )
    .await;
    SessionOutcome::Failed
}

async fn send_writer(
    writer: &mpsc::Sender<WriterCommand>,
    messages: Vec<SessionOutbound>,
    close: Option<CloseFrame>,
    timeout: Duration,
) -> Result<(), ()> {
    let (finished, completion) = oneshot::channel();
    writer
        .send(WriterCommand {
            messages,
            close,
            flush_peer_close: false,
            finished,
        })
        .await
        .map_err(|_| ())?;
    match tokio::time::timeout(timeout, completion).await {
        Ok(Ok(true)) => Ok(()),
        Ok(Ok(false)) | Ok(Err(_)) | Err(_) => Err(()),
    }
}

async fn session_reader(
    mut stream: SplitStream<WebSocket>,
    sender: mpsc::Sender<InboundEvent>,
    bytes: Arc<Semaphore>,
    mut cancelled: watch::Receiver<bool>,
    admission: Arc<SessionAdmission>,
) {
    loop {
        let message = tokio::select! {
            biased;
            changed = cancelled.changed() => {
                if changed.is_err() || *cancelled.borrow() {
                    return;
                }
                continue;
            }
            message = stream.next() => message,
        };
        match message {
            Some(Ok(Message::Text(text))) => {
                let body = Bytes::from(text);
                if enqueue_message(&sender, &bytes, InboundKind::Text, body, &admission)
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Some(Ok(Message::Binary(body))) => {
                if enqueue_message(&sender, &bytes, InboundKind::Binary, body, &admission)
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Some(Ok(Message::Close(frame))) => {
                let event = InboundEvent::PeerClose {
                    code: frame.as_ref().map(|frame| frame.code),
                    reason: frame.map_or_else(String::new, |frame| frame.reason.to_string()),
                };
                let _ = sender.send(event).await;
                return;
            }
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
            Some(Err(_)) | None => {
                let _ = sender.send(InboundEvent::TransportFailure).await;
                return;
            }
        }
    }
}

async fn enqueue_message(
    sender: &mpsc::Sender<InboundEvent>,
    bytes: &Arc<Semaphore>,
    kind: InboundKind,
    body: Bytes,
    admission: &SessionAdmission,
) -> Result<(), ()> {
    let length = body.len();
    let permit = if length == 0 {
        None
    } else {
        let count = u32::try_from(length).map_err(|_| ())?;
        Some(
            Arc::clone(bytes)
                .acquire_many_owned(count)
                .await
                .map_err(|_| ())?,
        )
    };
    sender
        .send(InboundEvent::Message { kind, body, permit })
        .await
        .map_err(|_| ())?;
    admission
        .counters
        .inbound_messages
        .fetch_add(1, Ordering::AcqRel);
    admission
        .counters
        .inbound_bytes
        .fetch_add(u64::try_from(length).unwrap_or(u64::MAX), Ordering::AcqRel);
    Ok(())
}

async fn session_writer(
    mut sink: SplitSink<WebSocket, Message>,
    mut receiver: mpsc::Receiver<WriterCommand>,
    mut cancelled: watch::Receiver<bool>,
) {
    loop {
        let command = tokio::select! {
            biased;
            command = receiver.recv() => command,
            changed = cancelled.changed() => {
                if changed.is_err() || *cancelled.borrow() {
                    return;
                }
                continue;
            }
        };
        let Some(command) = command else {
            return;
        };
        let WriterCommand {
            messages,
            close,
            flush_peer_close,
            finished,
        } = command;
        let terminal = close.is_some() || flush_peer_close;
        let mut success = true;
        for message in messages {
            let message = match message {
                SessionOutbound::Text(value) => Message::Text(Utf8Bytes::from(value.to_string())),
                SessionOutbound::Binary(value) => {
                    Message::Binary(Bytes::copy_from_slice(value.as_ref()))
                }
            };
            if sink.send(message).await.is_err() {
                success = false;
                break;
            }
        }
        if success
            && let Some(close) = close
            && sink.send(Message::Close(Some(close))).await.is_err()
        {
            success = false;
        }
        if success && flush_peer_close && sink.flush().await.is_err() {
            success = false;
        }
        let _ = finished.send(success);
        if terminal || !success {
            return;
        }
    }
}

async fn join_child(mut child: tokio::task::JoinHandle<()>, grace: Duration) {
    if tokio::time::timeout(grace, &mut child).await.is_err() {
        child.abort();
        let _ = child.await;
    }
}

#[derive(Clone)]
struct SessionRuntimeContract {
    relation: SessionPortRelation,
    event: VariantContract,
    message_kind: VariantContract,
    decision_kind: VariantContract,
    outbound: VariantContract,
    rejection: RecordContract,
    closing: RecordContract,
}

impl SessionRuntimeContract {
    fn prepare(program: &NormalizedProgram, target: &NormalizedTarget) -> Result<Self, Diagnostic> {
        let port_index = target.port.ok_or_else(|| {
            session_corrupt(
                "normalized_session_target_port",
                "interactive target has no exact port",
            )
        })?;
        let port = program.ports.get(port_index.0 as usize).ok_or_else(|| {
            session_corrupt(
                "normalized_session_port",
                "interactive target port escaped the artifact table",
            )
        })?;
        if port.component != target.component {
            return Err(session_corrupt(
                "normalized_session_component",
                "interactive target and port disagree on their component",
            ));
        }
        let standard = BuiltinStandard::load()?.session_contract()?;
        let relation = validate_session_function_type(
            &ProgramSessionRead { program },
            standard,
            port.function_type,
        )?;
        Ok(Self {
            relation,
            event: VariantContract::prepare(program, standard.event)?,
            message_kind: VariantContract::prepare(program, standard.message_kind)?,
            decision_kind: VariantContract::prepare(program, standard.decision_kind)?,
            outbound: VariantContract::prepare(program, standard.outbound)?,
            rejection: RecordContract::prepare(program, standard.reject)?,
            closing: RecordContract::prepare(program, standard.close)?,
        })
    }

    fn event(
        &self,
        name: &str,
        payload: Option<NormalizedValue>,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.event.value(name, payload)
    }

    fn message_kind(&self, name: &str) -> Result<NormalizedValue, ExecutionError> {
        self.message_kind.value(name, None)
    }

    fn decode_decision(
        &self,
        program: &NormalizedProgram,
        value: NormalizedValue,
        phase: SessionPhase,
        limits: &SessionLimits,
    ) -> Result<DecodedDecision, ExecutionError> {
        let fields = exact_structural(
            &value,
            &["closing", "kind", "messages", "rejection", "state"],
            "session decision",
        )?;
        let kind = match self.decision_kind.case_name(fields["kind"])? {
            "accept" => DecisionKind::Accept,
            "continue" => DecisionKind::Continue,
            "reject" => DecisionKind::Reject,
            "close" => DecisionKind::Close,
            "finish" => DecisionKind::Finish,
            _ => {
                return Err(session_protocol(
                    "session_decision_kind",
                    "session decision selected an unknown canonical case",
                ));
            }
        };
        let state = option_value(fields["state"], "session decision state")?;
        if let Some(state) = &state {
            let mut meter = ValueMeter::new(limits.maximum_state_nodes, limits.maximum_state_bytes);
            meter.validate(program, state, self.relation.state, 0)?;
        }
        let messages = self.decode_messages(fields["messages"], limits)?;
        let rejection = match option_value(fields["rejection"], "session rejection")? {
            Some(value) => Some(self.decode_rejection(&value, limits)?),
            None => None,
        };
        let closing = match option_value(fields["closing"], "session close")? {
            Some(value) => Some(self.decode_close(&value)?),
            None => None,
        };
        let valid = match (phase, kind) {
            (SessionPhase::Open, DecisionKind::Accept) => {
                state.is_some() && rejection.is_none() && closing.is_none()
            }
            (SessionPhase::Open, DecisionKind::Reject) => {
                state.is_none() && messages.is_empty() && rejection.is_some() && closing.is_none()
            }
            (SessionPhase::Message | SessionPhase::Tick, DecisionKind::Continue) => {
                state.is_some() && rejection.is_none() && closing.is_none()
            }
            (SessionPhase::Message | SessionPhase::Tick, DecisionKind::Close) => {
                state.is_none() && rejection.is_none() && closing.is_some()
            }
            (SessionPhase::PeerClose | SessionPhase::Shutdown, DecisionKind::Finish) => {
                state.is_none() && messages.is_empty() && rejection.is_none() && closing.is_none()
            }
            _ => false,
        };
        if !valid {
            return Err(session_protocol(
                "session_phase_decision",
                "session event, state, and decision do not form a valid phase transition",
            ));
        }
        Ok(DecodedDecision {
            kind,
            state,
            messages,
            rejection,
            closing,
        })
    }

    fn decode_messages(
        &self,
        value: &NormalizedValue,
        limits: &SessionLimits,
    ) -> Result<Vec<SessionOutbound>, ExecutionError> {
        let NormalizedValue::List(values) = value else {
            return Err(session_protocol(
                "session_output_messages",
                "session output messages are not a list",
            ));
        };
        if values.len() > limits.maximum_transition_messages
            || values.len() > limits.maximum_outbound_mailbox_items
        {
            return Err(ExecutionError::resource(
                "session_output_message_limit",
                "session transition exceeds its configured output message count",
            ));
        }
        let mut total = 0usize;
        let mut output = Vec::with_capacity(values.len());
        for value in values.iter() {
            let (name, payload) = self.outbound.case(value)?;
            let message = match (name, payload) {
                ("text", Some(NormalizedValue::Text(value))) => {
                    SessionOutbound::Text(Arc::clone(value))
                }
                ("binary", Some(NormalizedValue::Bytes(value))) => {
                    SessionOutbound::Binary(Arc::clone(value))
                }
                _ => {
                    return Err(session_protocol(
                        "session_output_message",
                        "session output message payload disagrees with its canonical case",
                    ));
                }
            };
            let length = message.len();
            if length > limits.maximum_message_bytes {
                return Err(ExecutionError::resource(
                    "session_output_message_bytes",
                    "one session output message exceeds the configured byte limit",
                ));
            }
            total = total.checked_add(length).ok_or_else(|| {
                ExecutionError::resource(
                    "session_output_bytes",
                    "session output byte accounting overflowed",
                )
            })?;
            output.push(message);
        }
        if total > limits.maximum_transition_bytes || total > limits.maximum_outbound_mailbox_bytes
        {
            return Err(ExecutionError::resource(
                "session_output_bytes",
                "session transition exceeds its configured output bytes",
            ));
        }
        Ok(output)
    }

    fn decode_rejection(
        &self,
        value: &NormalizedValue,
        limits: &SessionLimits,
    ) -> Result<HttpResponse, ExecutionError> {
        let fields = self.rejection.fields(value)?;
        let body = match fields.get("body") {
            Some(NormalizedValue::Bytes(value)) if value.len() <= limits.maximum_message_bytes => {
                value.to_vec()
            }
            Some(NormalizedValue::Bytes(_)) => {
                return Err(ExecutionError::resource(
                    "session_reject_body_limit",
                    "session rejection body exceeds the configured bytes",
                ));
            }
            _ => {
                return Err(session_protocol(
                    "session_reject_body",
                    "session rejection body is not Bytes",
                ));
            }
        };
        let headers = decode_headers(
            fields.get("headers").ok_or_else(|| {
                session_protocol("session_reject_headers", "session rejection omits headers")
            })?,
            limits,
        )?;
        let status = match fields.get("status") {
            Some(NormalizedValue::I64(value)) => u16::try_from(*value).map_err(|_| {
                session_protocol(
                    "session_reject_status",
                    "session rejection status is outside unsigned 16-bit range",
                )
            })?,
            _ => {
                return Err(session_protocol(
                    "session_reject_status",
                    "session rejection status is not I64",
                ));
            }
        };
        if !(200..=599).contains(&status) || status == 101 {
            return Err(session_protocol(
                "session_reject_status",
                "session rejection status must be 200 through 599 excluding 101",
            ));
        }
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }

    fn decode_close(&self, value: &NormalizedValue) -> Result<CloseFrame, ExecutionError> {
        let fields = self.closing.fields(value)?;
        let code = match fields.get("code") {
            Some(NormalizedValue::I64(value)) => u16::try_from(*value).map_err(|_| {
                session_protocol(
                    "session_close_code",
                    "session close code is outside unsigned 16-bit range",
                )
            })?,
            _ => {
                return Err(session_protocol(
                    "session_close_code",
                    "session close code is not I64",
                ));
            }
        };
        if !valid_application_close_code(code) {
            return Err(session_protocol(
                "session_close_code",
                "session close code is reserved, invalid, or client-only",
            ));
        }
        let reason = match fields.get("reason") {
            Some(NormalizedValue::Text(value)) if value.len() <= 123 => value.to_string(),
            Some(NormalizedValue::Text(_)) => {
                return Err(ExecutionError::resource(
                    "session_close_reason_limit",
                    "session close reason exceeds the RFC control-frame bound",
                ));
            }
            _ => {
                return Err(session_protocol(
                    "session_close_reason",
                    "session close reason is not Text",
                ));
            }
        };
        Ok(CloseFrame {
            code,
            reason: Utf8Bytes::from(reason),
        })
    }
}

struct ProgramSessionRead<'a> {
    program: &'a NormalizedProgram,
}

impl SessionShapeRead for ProgramSessionRead<'_> {
    fn type_object(&self, digest: TypeObjectDigest) -> Result<TypeObject, Diagnostic> {
        self.program.types.get(&digest).cloned().ok_or_else(|| {
            session_corrupt(
                "normalized_session_type_missing",
                format!("interactive artifact references missing type {digest}"),
            )
        })
    }

    fn nominal_shape(
        &self,
        declaration: DeclarationReference,
    ) -> Result<SessionNominalShape, Diagnostic> {
        if let Some(layout) = self
            .program
            .records
            .iter()
            .find(|layout| layout.declaration == declaration)
        {
            return Ok(SessionNominalShape::Record(
                layout
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), field.ty))
                    .collect(),
            ));
        }
        if let Some(layout) = self
            .program
            .variants
            .iter()
            .find(|layout| layout.declaration == declaration)
        {
            return Ok(SessionNominalShape::Variant(
                layout
                    .cases
                    .iter()
                    .map(|case| (case.name.clone(), case.payload))
                    .collect(),
            ));
        }
        Err(session_corrupt(
            "normalized_session_nominal_missing",
            "interactive artifact omits one exact nominal session layout",
        ))
    }
}

#[derive(Clone)]
struct VariantContract {
    layout: VariantLayoutIndex,
    cases: BTreeMap<String, u32>,
}

impl VariantContract {
    fn prepare(
        program: &NormalizedProgram,
        declaration: DeclarationReference,
    ) -> Result<Self, Diagnostic> {
        let (index, layout) = variant_layout(program, declaration).ok_or_else(|| {
            session_corrupt(
                "normalized_session_variant_layout",
                "interactive artifact omits one exact canonical variant layout",
            )
        })?;
        let cases = layout
            .cases
            .iter()
            .enumerate()
            .map(|(case, value)| {
                u32::try_from(case)
                    .map(|case| (value.name.as_str().to_owned(), case))
                    .map_err(|_| {
                        session_corrupt(
                            "normalized_session_variant_index",
                            "session variant case exceeds the runtime index domain",
                        )
                    })
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        Ok(Self {
            layout: index,
            cases,
        })
    }

    fn value(
        &self,
        name: &str,
        payload: Option<NormalizedValue>,
    ) -> Result<NormalizedValue, ExecutionError> {
        let case = self.cases.get(name).copied().ok_or_else(|| {
            session_execution(
                "session_canonical_case",
                "canonical session case is absent from its prepared artifact layout",
            )
        })?;
        Ok(NormalizedValue::Variant {
            layout: self.layout,
            case,
            payload: payload.map(Box::new),
        })
    }

    fn case<'a>(
        &self,
        value: &'a NormalizedValue,
    ) -> Result<(&str, Option<&'a NormalizedValue>), ExecutionError> {
        let NormalizedValue::Variant {
            layout,
            case,
            payload,
        } = value
        else {
            return Err(session_protocol(
                "session_nominal_variant",
                "session value is not a canonical nominal variant",
            ));
        };
        if *layout != self.layout {
            return Err(session_protocol(
                "session_nominal_variant",
                "session value uses a foreign nominal variant layout",
            ));
        }
        let name = self
            .cases
            .iter()
            .find_map(|(name, expected)| (*expected == *case).then_some(name.as_str()))
            .ok_or_else(|| {
                session_protocol(
                    "session_variant_case",
                    "session value selects a case outside its canonical layout",
                )
            })?;
        Ok((name, payload.as_deref()))
    }

    fn case_name(&self, value: &NormalizedValue) -> Result<&str, ExecutionError> {
        let (name, payload) = self.case(value)?;
        if payload.is_some() {
            return Err(session_protocol(
                "session_variant_payload",
                "payload-free session case unexpectedly carries a value",
            ));
        }
        Ok(name)
    }
}

#[derive(Clone)]
struct RecordContract {
    layout: RecordLayoutIndex,
    fields: BTreeMap<String, usize>,
}

impl RecordContract {
    fn prepare(
        program: &NormalizedProgram,
        declaration: DeclarationReference,
    ) -> Result<Self, Diagnostic> {
        let (layout, record) = record_layout(program, declaration).ok_or_else(|| {
            session_corrupt(
                "normalized_session_record_layout",
                "interactive artifact omits one exact canonical record layout",
            )
        })?;
        Ok(Self {
            layout,
            fields: record
                .fields
                .iter()
                .enumerate()
                .map(|(index, field)| (field.name.as_str().to_owned(), index))
                .collect(),
        })
    }

    fn fields<'a>(
        &self,
        value: &'a NormalizedValue,
    ) -> Result<BTreeMap<&str, &'a NormalizedValue>, ExecutionError> {
        let NormalizedValue::Record(NormalizedRecord::Nominal { layout, fields }) = value else {
            return Err(session_protocol(
                "session_nominal_record",
                "session value is not a canonical nominal record",
            ));
        };
        if *layout != self.layout || fields.len() != self.fields.len() {
            return Err(session_protocol(
                "session_nominal_record",
                "session value disagrees with its canonical record layout",
            ));
        }
        self.fields
            .iter()
            .map(|(name, index)| {
                fields
                    .get(*index)
                    .map(|value| (name.as_str(), value))
                    .ok_or_else(|| {
                        session_protocol(
                            "session_record_field",
                            "session record field escaped its canonical layout",
                        )
                    })
            })
            .collect()
    }
}

struct ValueMeter {
    nodes: usize,
    bytes: usize,
    maximum_nodes: usize,
    maximum_bytes: usize,
}

impl ValueMeter {
    const fn new(maximum_nodes: usize, maximum_bytes: usize) -> Self {
        Self {
            nodes: 0,
            bytes: 0,
            maximum_nodes,
            maximum_bytes,
        }
    }

    fn charge(&mut self, bytes: usize) -> Result<(), ExecutionError> {
        self.nodes = self.nodes.checked_add(1).ok_or_else(|| {
            ExecutionError::resource("session_state_nodes", "session state node count overflowed")
        })?;
        self.bytes = self.bytes.checked_add(bytes).ok_or_else(|| {
            ExecutionError::resource("session_state_bytes", "session state byte count overflowed")
        })?;
        if self.nodes > self.maximum_nodes || self.bytes > self.maximum_bytes {
            return Err(ExecutionError::resource(
                "session_state_limit",
                "session state exceeds its configured node or byte bound",
            ));
        }
        Ok(())
    }

    fn validate(
        &mut self,
        program: &NormalizedProgram,
        value: &NormalizedValue,
        ty: TypeObjectDigest,
        depth: usize,
    ) -> Result<(), ExecutionError> {
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(ExecutionError::resource(
                "session_state_depth",
                "session state exceeds the exact type-depth bound",
            ));
        }
        let form = program
            .types
            .get(&ty)
            .map(|value| &value.form)
            .ok_or_else(|| {
                session_execution(
                    "session_state_type_missing",
                    "session state references a type absent from the prepared artifact",
                )
            })?;
        match (value, form) {
            (NormalizedValue::Unit, TypeForm::Unit) => self.charge(0),
            (NormalizedValue::Bool(_), TypeForm::Bool) => self.charge(1),
            (NormalizedValue::I64(_), TypeForm::I64) => self.charge(8),
            (NormalizedValue::Bytes(value), TypeForm::Bytes) => self.charge(value.len()),
            (NormalizedValue::Text(value), TypeForm::Text) => self.charge(value.len()),
            (NormalizedValue::Option(value), TypeForm::Option { item }) => {
                self.charge(0)?;
                if let Some(value) = value {
                    self.validate(program, value, *item, depth + 1)?;
                }
                Ok(())
            }
            (NormalizedValue::List(values), TypeForm::List { item }) => {
                self.charge(0)?;
                for value in values.iter() {
                    self.validate(program, value, *item, depth + 1)?;
                }
                Ok(())
            }
            (NormalizedValue::Map(values), TypeForm::Map { key, value: item }) => {
                self.charge(0)?;
                for (map_key, value) in values.iter() {
                    self.validate(program, &map_key.to_value(), *key, depth + 1)?;
                    self.validate(program, value, *item, depth + 1)?;
                }
                Ok(())
            }
            (
                NormalizedValue::Record(NormalizedRecord::Structural { fields: values }),
                TypeForm::StructuralRecord { fields },
            ) => {
                if values.len() != fields.len()
                    || values
                        .iter()
                        .zip(fields.iter())
                        .any(|((actual, _), expected)| actual != &expected.name)
                {
                    return Err(session_protocol(
                        "session_state_shape",
                        "session state structural record disagrees with its exact type",
                    ));
                }
                self.charge(0)?;
                for ((_, value), field) in values.iter().zip(fields.iter()) {
                    self.validate(program, value, field.ty, depth + 1)?;
                }
                Ok(())
            }
            (value, TypeForm::Named { declaration }) => {
                if let Some((layout, record)) = record_layout(program, *declaration) {
                    let NormalizedValue::Record(NormalizedRecord::Nominal {
                        layout: actual,
                        fields,
                    }) = value
                    else {
                        return Err(session_protocol(
                            "session_state_shape",
                            "session state is not its exact nominal record type",
                        ));
                    };
                    if *actual != layout || fields.len() != record.fields.len() {
                        return Err(session_protocol(
                            "session_state_shape",
                            "session state nominal record disagrees with its exact layout",
                        ));
                    }
                    self.charge(0)?;
                    for (value, field) in fields.iter().zip(record.fields.iter()) {
                        self.validate(program, value, field.ty, depth + 1)?;
                    }
                    return Ok(());
                }
                if let Some((layout, variant)) = variant_layout(program, *declaration) {
                    let NormalizedValue::Variant {
                        layout: actual,
                        case,
                        payload,
                    } = value
                    else {
                        return Err(session_protocol(
                            "session_state_shape",
                            "session state is not its exact nominal variant type",
                        ));
                    };
                    if *actual != layout {
                        return Err(session_protocol(
                            "session_state_shape",
                            "session state nominal variant disagrees with its exact layout",
                        ));
                    }
                    let case = variant.cases.get(*case as usize).ok_or_else(|| {
                        session_protocol(
                            "session_state_shape",
                            "session state variant case escaped its exact layout",
                        )
                    })?;
                    self.charge(0)?;
                    match (payload, case.payload) {
                        (None, None) => Ok(()),
                        (Some(value), Some(ty)) => self.validate(program, value, ty, depth + 1),
                        _ => Err(session_protocol(
                            "session_state_shape",
                            "session state variant payload disagrees with its exact case",
                        )),
                    }
                } else {
                    Err(session_execution(
                        "session_state_layout_missing",
                        "session state nominal type has no exact artifact layout",
                    ))
                }
            }
            _ => Err(session_protocol(
                "session_state_shape",
                "session state runtime value disagrees with its exact ordinary type",
            )),
        }
    }
}

struct SessionAdmission {
    pending: Arc<Semaphore>,
    active: Arc<Semaphore>,
    buffers: Arc<Semaphore>,
    per_session_buffer_bytes: u32,
    idle: tokio::sync::Notify,
    limits: SessionLimits,
    counters: SessionCounters,
}

impl SessionAdmission {
    fn new(limits: &SessionLimits) -> Result<Self, Diagnostic> {
        let process_bytes = usize::try_from(limits.maximum_process_buffer_bytes).map_err(|_| {
            session_resource(
                "session_process_buffer_platform",
                "process-wide session buffer limit does not fit this platform",
            )
        })?;
        Ok(Self {
            pending: Arc::new(Semaphore::new(limits.maximum_pending_handshakes)),
            active: Arc::new(Semaphore::new(limits.maximum_active_sessions)),
            buffers: Arc::new(Semaphore::new(process_bytes)),
            per_session_buffer_bytes: limits.per_session_buffer_bytes()?,
            idle: tokio::sync::Notify::new(),
            limits: limits.clone(),
            counters: SessionCounters::default(),
        })
    }

    fn try_pending(self: &Arc<Self>) -> Option<PendingHandshakeGuard> {
        let permit = Arc::clone(&self.pending).try_acquire_owned().ok()?;
        let value = self
            .counters
            .pending_handshakes
            .fetch_add(1, Ordering::AcqRel)
            .saturating_add(1);
        update_maximum(&self.counters.maximum_pending_handshakes, value);
        Some(PendingHandshakeGuard {
            admission: Arc::clone(self),
            _permit: permit,
        })
    }

    fn try_session(self: &Arc<Self>) -> Option<ActiveSessionGuard> {
        let active = Arc::clone(&self.active).try_acquire_owned().ok()?;
        let buffers = Arc::clone(&self.buffers)
            .try_acquire_many_owned(self.per_session_buffer_bytes)
            .ok()?;
        let value = self
            .counters
            .active_sessions
            .fetch_add(1, Ordering::AcqRel)
            .saturating_add(1);
        update_maximum(&self.counters.maximum_active_sessions, value);
        Some(ActiveSessionGuard {
            admission: Arc::clone(self),
            _active: active,
            _buffers: buffers,
        })
    }

    fn observe(&self) -> SessionObservation {
        let per_session = u64::from(self.per_session_buffer_bytes);
        let active = self.counters.active_sessions.load(Ordering::Acquire);
        let maximum_active = self
            .counters
            .maximum_active_sessions
            .load(Ordering::Acquire);
        SessionObservation {
            pending_handshakes: self.counters.pending_handshakes.load(Ordering::Acquire),
            active_sessions: active,
            admitted_sessions: self.counters.admitted_sessions.load(Ordering::Acquire),
            rejected_handshakes: self.counters.rejected_handshakes.load(Ordering::Acquire),
            completed_sessions: self.counters.completed_sessions.load(Ordering::Acquire),
            failed_sessions: self.counters.failed_sessions.load(Ordering::Acquire),
            overloaded_handshakes: self.counters.overloaded_handshakes.load(Ordering::Acquire),
            inbound_messages: self.counters.inbound_messages.load(Ordering::Acquire),
            inbound_bytes: self.counters.inbound_bytes.load(Ordering::Acquire),
            outbound_messages: self.counters.outbound_messages.load(Ordering::Acquire),
            outbound_bytes: self.counters.outbound_bytes.load(Ordering::Acquire),
            maximum_pending_handshakes: self
                .counters
                .maximum_pending_handshakes
                .load(Ordering::Acquire),
            maximum_active_sessions: maximum_active,
            maximum_process_buffer_bytes: per_session
                .saturating_mul(u64::try_from(maximum_active).unwrap_or(u64::MAX))
                .min(self.limits.maximum_process_buffer_bytes),
        }
    }

    async fn wait_idle(&self, grace: Duration) -> bool {
        let wait = async {
            while self.counters.active_sessions.load(Ordering::Acquire) != 0 {
                self.idle.notified().await;
            }
        };
        tokio::time::timeout(grace, wait).await.is_ok()
    }
}

#[derive(Default)]
struct SessionCounters {
    pending_handshakes: AtomicUsize,
    active_sessions: AtomicUsize,
    admitted_sessions: AtomicU64,
    rejected_handshakes: AtomicU64,
    completed_sessions: AtomicU64,
    failed_sessions: AtomicU64,
    overloaded_handshakes: AtomicU64,
    inbound_messages: AtomicU64,
    inbound_bytes: AtomicU64,
    outbound_messages: AtomicU64,
    outbound_bytes: AtomicU64,
    maximum_pending_handshakes: AtomicUsize,
    maximum_active_sessions: AtomicUsize,
}

struct PendingHandshakeGuard {
    admission: Arc<SessionAdmission>,
    _permit: OwnedSemaphorePermit,
}

impl Drop for PendingHandshakeGuard {
    fn drop(&mut self) {
        self.admission
            .counters
            .pending_handshakes
            .fetch_sub(1, Ordering::AcqRel);
    }
}

struct ActiveSessionGuard {
    admission: Arc<SessionAdmission>,
    _active: OwnedSemaphorePermit,
    _buffers: OwnedSemaphorePermit,
}

impl Drop for ActiveSessionGuard {
    fn drop(&mut self) {
        self.admission
            .counters
            .active_sessions
            .fetch_sub(1, Ordering::AcqRel);
        self.admission.idle.notify_waiters();
    }
}

enum InboundEvent {
    Message {
        kind: InboundKind,
        body: Bytes,
        permit: Option<OwnedSemaphorePermit>,
    },
    PeerClose {
        code: Option<u16>,
        reason: String,
    },
    TransportFailure,
}

#[derive(Clone, Copy)]
enum InboundKind {
    Text,
    Binary,
}

enum DriverInput {
    Message {
        kind: InboundKind,
        body: Bytes,
        _permit: Option<OwnedSemaphorePermit>,
    },
    Tick,
    PeerClose {
        code: Option<u16>,
        reason: String,
    },
    Shutdown,
    OperationalClose(u16, &'static str),
}

enum SessionInput {
    Message { kind: InboundKind, body: Bytes },
    Tick,
    PeerClose { code: Option<u16>, reason: String },
    Shutdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SessionPhase {
    Open,
    Message,
    Tick,
    PeerClose,
    Shutdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DecisionKind {
    Accept,
    Continue,
    Reject,
    Close,
    Finish,
}

struct DecodedDecision {
    kind: DecisionKind,
    state: Option<NormalizedValue>,
    messages: Vec<SessionOutbound>,
    rejection: Option<HttpResponse>,
    closing: Option<CloseFrame>,
}

enum SessionOutbound {
    Text(Arc<str>),
    Binary(Arc<[u8]>),
}

impl SessionOutbound {
    fn len(&self) -> usize {
        match self {
            Self::Text(value) => value.len(),
            Self::Binary(value) => value.len(),
        }
    }
}

struct WriterCommand {
    messages: Vec<SessionOutbound>,
    close: Option<CloseFrame>,
    flush_peer_close: bool,
    finished: oneshot::Sender<bool>,
}

enum SessionOutcome {
    Completed,
    Failed,
}

struct SessionOpenRequest {
    path: String,
    query: String,
    headers: Vec<HttpHeader>,
}

fn decode_open_request(
    request: Request<Body>,
    limits: &SessionLimits,
) -> Result<SessionOpenRequest, Response<Body>> {
    let (parts, _) = request.into_parts();
    if parts.version != Version::HTTP_11 || parts.method != axum::http::Method::GET {
        return Err(static_response(
            StatusCode::BAD_REQUEST,
            "WebSocket sessions require HTTP/1.1 GET",
        ));
    }
    if parts.uri.path().is_empty()
        || parts.uri.path().len() > MAXIMUM_SESSION_TARGET_BYTES
        || parts.uri.query().unwrap_or_default().len() > MAXIMUM_SESSION_TARGET_BYTES
    {
        return Err(static_response(
            StatusCode::URI_TOO_LONG,
            "session request target exceeds its bound",
        ));
    }
    if parts.headers.len() > limits.maximum_headers {
        return Err(static_response(
            StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
            "session request headers exceed their count bound",
        ));
    }
    let mut total = 0usize;
    let mut headers = Vec::with_capacity(parts.headers.len());
    for (name, value) in &parts.headers {
        total = match total
            .checked_add(name.as_str().len())
            .and_then(|length| length.checked_add(value.as_bytes().len()))
        {
            Some(total) if total <= limits.maximum_header_bytes => total,
            _ => {
                return Err(static_response(
                    StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
                    "session request headers exceed their byte bound",
                ));
            }
        };
        headers.push(HttpHeader {
            name: name.as_str().to_owned(),
            value: value.as_bytes().to_vec(),
        });
    }
    headers.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.value.cmp(&right.value))
    });
    Ok(SessionOpenRequest {
        path: parts.uri.path().to_owned(),
        query: parts.uri.query().unwrap_or_default().to_owned(),
        headers,
    })
}

fn headers_value(headers: Vec<HttpHeader>) -> Result<NormalizedValue, ExecutionError> {
    let values = headers
        .into_iter()
        .map(|header| {
            structural([
                ("name", NormalizedValue::text(header.name)),
                ("value", NormalizedValue::bytes(header.value)),
            ])
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(NormalizedValue::List(Arc::new(values)))
}

fn decode_headers(
    value: &NormalizedValue,
    limits: &SessionLimits,
) -> Result<Vec<HttpHeader>, ExecutionError> {
    let NormalizedValue::List(values) = value else {
        return Err(session_protocol(
            "session_reject_headers",
            "session rejection headers are not a list",
        ));
    };
    if values.len() > limits.maximum_headers {
        return Err(ExecutionError::resource(
            "session_reject_header_count",
            "session rejection headers exceed their count bound",
        ));
    }
    let mut total = 0usize;
    values
        .iter()
        .map(|value| {
            let fields = exact_structural(value, &["name", "value"], "session header")?;
            let name = match &fields["name"] {
                NormalizedValue::Text(value) => value.to_string(),
                _ => {
                    return Err(session_protocol(
                        "session_reject_header_name",
                        "session rejection header name is not Text",
                    ));
                }
            };
            let bytes = match &fields["value"] {
                NormalizedValue::Bytes(value) => value.to_vec(),
                _ => {
                    return Err(session_protocol(
                        "session_reject_header_value",
                        "session rejection header value is not Bytes",
                    ));
                }
            };
            total = total
                .checked_add(name.len())
                .and_then(|total| total.checked_add(bytes.len()))
                .ok_or_else(|| {
                    ExecutionError::resource(
                        "session_reject_header_bytes",
                        "session rejection header byte accounting overflowed",
                    )
                })?;
            if total > limits.maximum_header_bytes {
                return Err(ExecutionError::resource(
                    "session_reject_header_bytes",
                    "session rejection headers exceed their byte bound",
                ));
            }
            Ok(HttpHeader { name, value: bytes })
        })
        .collect()
}

fn structural<const N: usize>(
    fields: [(&'static str, NormalizedValue); N],
) -> Result<NormalizedValue, ExecutionError> {
    let mut fields = fields
        .into_iter()
        .map(|(name, value)| {
            Name::new(name).map(|name| (name, value)).map_err(|_| {
                session_execution(
                    "session_static_field",
                    "built-in session field name is invalid",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    fields.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(NormalizedValue::Record(NormalizedRecord::Structural {
        fields: Arc::new(fields),
    }))
}

fn exact_structural<'a>(
    value: &'a NormalizedValue,
    expected: &[&str],
    subject: &'static str,
) -> Result<BTreeMap<&'a str, &'a NormalizedValue>, ExecutionError> {
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = value else {
        return Err(session_protocol(
            "session_structural_value",
            format!("{subject} is not a structural record"),
        ));
    };
    if fields.len() != expected.len()
        || fields
            .iter()
            .zip(expected.iter())
            .any(|((name, _), expected)| name.as_str() != *expected)
    {
        return Err(session_protocol(
            "session_structural_value",
            format!("{subject} fields do not equal the exact canonical shape"),
        ));
    }
    Ok(fields
        .iter()
        .map(|(name, value)| (name.as_str(), value))
        .collect())
}

fn option_value(
    value: &NormalizedValue,
    subject: &'static str,
) -> Result<Option<NormalizedValue>, ExecutionError> {
    let NormalizedValue::Option(value) = value else {
        return Err(session_protocol(
            "session_option_value",
            format!("{subject} is not Option"),
        ));
    };
    Ok(value.as_deref().cloned())
}

fn record_layout(
    program: &NormalizedProgram,
    declaration: DeclarationReference,
) -> Option<(RecordLayoutIndex, &NormalizedRecordLayout)> {
    program
        .records
        .iter()
        .enumerate()
        .find(|(_, layout)| layout.declaration == declaration)
        .and_then(|(index, layout)| {
            u32::try_from(index)
                .ok()
                .map(|index| (RecordLayoutIndex(index), layout))
        })
}

fn variant_layout(
    program: &NormalizedProgram,
    declaration: DeclarationReference,
) -> Option<(VariantLayoutIndex, &NormalizedVariantLayout)> {
    program
        .variants
        .iter()
        .enumerate()
        .find(|(_, layout)| layout.declaration == declaration)
        .and_then(|(index, layout)| {
            u32::try_from(index)
                .ok()
                .map(|index| (VariantLayoutIndex(index), layout))
        })
}

fn valid_application_close_code(code: u16) -> bool {
    matches!(
        code,
        1000 | 1001 | 1002 | 1003 | 1007 | 1008 | 1009 | 1011 | 1012 | 1013 | 1014
    ) || (3000..=4999).contains(&code)
}

fn update_maximum(maximum: &AtomicUsize, value: usize) {
    let _ = maximum.fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
        (value > current).then_some(value)
    });
}

fn record_outbound(admission: &SessionAdmission, messages: &[SessionOutbound]) {
    let bytes = messages.iter().fold(0_u64, |total, message| {
        total.saturating_add(u64::try_from(message.len()).unwrap_or(u64::MAX))
    });
    admission.counters.outbound_messages.fetch_add(
        u64::try_from(messages.len()).unwrap_or(u64::MAX),
        Ordering::AcqRel,
    );
    admission
        .counters
        .outbound_bytes
        .fetch_add(bytes, Ordering::AcqRel);
}

fn session_protocol(code: &'static str, message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}

fn session_execution(code: &'static str, message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Infrastructure, code, message)
}

fn session_source(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

fn session_corrupt(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}

fn session_resource(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Resource, code, message)
}

fn session_infrastructure(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Infrastructure, code, message)
}
