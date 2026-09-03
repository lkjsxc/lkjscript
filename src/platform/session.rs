//! Graph-owned structured sessions over one bounded resident transport.

use super::change::{CanonicalBaseRead, CanonicalReadWork};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::kernel::{
    DeclarationPayload, DeclarationReference, DependencyRecord, Name, OwnerKey, OwnerRecord,
    PackageInterfaceDeclarationPayload, PackageInterfaceRecord, TypeForm, TypeObject,
    TypeObjectDigest,
};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use super::runtime::ShutdownReceipt;

pub const STRUCTURED_SESSION_CONTRACT_IDENTITY: &str = "lkjscript-structured-session-1";
pub const STRUCTURED_SESSION_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_ACTIVE_SESSIONS: usize = 4_096;
pub const MAXIMUM_PENDING_HANDSHAKES: usize = 4_096;
pub const MAXIMUM_SESSION_MESSAGE_BYTES: usize = 16 * 1_048_576;
pub const MAXIMUM_SESSION_FRAME_BYTES: usize = 16 * 1_048_576;
pub const MAXIMUM_SESSION_HEADER_BYTES: usize = 256 * 1024;
pub const MAXIMUM_SESSION_HEADERS: usize = 1_024;
pub const MAXIMUM_SESSION_MAILBOX_ITEMS: usize = 4_096;
pub const MAXIMUM_SESSION_MAILBOX_BYTES: usize = 64 * 1_048_576;
pub const MAXIMUM_SESSION_STATE_NODES: usize = 1_000_000;
pub const MAXIMUM_SESSION_STATE_BYTES: usize = 64 * 1_048_576;
pub const MAXIMUM_SESSION_TRANSITION_MESSAGES: usize = 4_096;
pub const MAXIMUM_SESSION_TRANSITION_BYTES: usize = 64 * 1_048_576;
pub const MAXIMUM_SESSION_INTERVAL_MILLISECONDS: u64 = 24 * 60 * 60 * 1_000;
pub const MAXIMUM_SESSION_LIFETIME_MILLISECONDS: u64 = 7 * 24 * 60 * 60 * 1_000;
pub const MAXIMUM_SESSION_GRACE_MILLISECONDS: u64 = 60_000;
pub const MAXIMUM_PROCESS_SESSION_BUFFER_BYTES: u64 = 4 * 1_024 * 1_024 * 1_024;

pub(crate) const SESSION_EVENT_NAME: &str = "SessionEvent";
pub(crate) const SESSION_MESSAGE_KIND_NAME: &str = "SessionMessageKind";
pub(crate) const SESSION_DECISION_KIND_NAME: &str = "SessionDecisionKind";
pub(crate) const SESSION_OUTBOUND_NAME: &str = "SessionOutbound";
pub(crate) const SESSION_REJECT_NAME: &str = "SessionReject";
pub(crate) const SESSION_CLOSE_NAME: &str = "SessionClose";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionLimits {
    pub maximum_active_sessions: usize,
    pub maximum_pending_handshakes: usize,
    pub maximum_message_bytes: usize,
    pub maximum_frame_bytes: usize,
    pub maximum_header_bytes: usize,
    pub maximum_headers: usize,
    pub maximum_inbound_mailbox_items: usize,
    pub maximum_inbound_mailbox_bytes: usize,
    pub maximum_outbound_mailbox_items: usize,
    pub maximum_outbound_mailbox_bytes: usize,
    pub maximum_state_nodes: usize,
    pub maximum_state_bytes: usize,
    pub maximum_transition_messages: usize,
    pub maximum_transition_bytes: usize,
    pub tick_interval_milliseconds: u64,
    pub idle_timeout_milliseconds: u64,
    pub maximum_lifetime_milliseconds: u64,
    pub close_grace_milliseconds: u64,
    pub cancellation_grace_milliseconds: u64,
    pub maximum_process_buffer_bytes: u64,
}

impl Default for SessionLimits {
    fn default() -> Self {
        Self {
            maximum_active_sessions: 64,
            maximum_pending_handshakes: 64,
            maximum_message_bytes: 1024 * 1024,
            maximum_frame_bytes: 256 * 1024,
            maximum_header_bytes: 32 * 1024,
            maximum_headers: 128,
            maximum_inbound_mailbox_items: 16,
            maximum_inbound_mailbox_bytes: 4 * 1024 * 1024,
            maximum_outbound_mailbox_items: 64,
            maximum_outbound_mailbox_bytes: 4 * 1024 * 1024,
            maximum_state_nodes: 100_000,
            maximum_state_bytes: 4 * 1024 * 1024,
            maximum_transition_messages: 64,
            maximum_transition_bytes: 1024 * 1024,
            tick_interval_milliseconds: 1_000,
            idle_timeout_milliseconds: 5 * 60 * 1_000,
            maximum_lifetime_milliseconds: 24 * 60 * 60 * 1_000,
            close_grace_milliseconds: 5_000,
            cancellation_grace_milliseconds: 5_000,
            maximum_process_buffer_bytes: 512 * 1024 * 1024,
        }
    }
}

impl SessionLimits {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        bounded_usize(
            "maximum_active_sessions",
            self.maximum_active_sessions,
            MAXIMUM_ACTIVE_SESSIONS,
        )?;
        bounded_usize(
            "maximum_pending_handshakes",
            self.maximum_pending_handshakes,
            MAXIMUM_PENDING_HANDSHAKES,
        )?;
        bounded_usize(
            "maximum_message_bytes",
            self.maximum_message_bytes,
            MAXIMUM_SESSION_MESSAGE_BYTES,
        )?;
        bounded_usize(
            "maximum_frame_bytes",
            self.maximum_frame_bytes,
            MAXIMUM_SESSION_FRAME_BYTES,
        )?;
        if self.maximum_frame_bytes > self.maximum_message_bytes {
            return Err(session_resource(
                "session_frame_message_limit",
                "maximum_frame_bytes cannot exceed maximum_message_bytes",
            ));
        }
        bounded_usize(
            "maximum_header_bytes",
            self.maximum_header_bytes,
            MAXIMUM_SESSION_HEADER_BYTES,
        )?;
        bounded_usize(
            "maximum_headers",
            self.maximum_headers,
            MAXIMUM_SESSION_HEADERS,
        )?;
        bounded_usize(
            "maximum_inbound_mailbox_items",
            self.maximum_inbound_mailbox_items,
            MAXIMUM_SESSION_MAILBOX_ITEMS,
        )?;
        bounded_usize(
            "maximum_inbound_mailbox_bytes",
            self.maximum_inbound_mailbox_bytes,
            MAXIMUM_SESSION_MAILBOX_BYTES,
        )?;
        bounded_usize(
            "maximum_outbound_mailbox_items",
            self.maximum_outbound_mailbox_items,
            MAXIMUM_SESSION_MAILBOX_ITEMS,
        )?;
        bounded_usize(
            "maximum_outbound_mailbox_bytes",
            self.maximum_outbound_mailbox_bytes,
            MAXIMUM_SESSION_MAILBOX_BYTES,
        )?;
        bounded_usize(
            "maximum_state_nodes",
            self.maximum_state_nodes,
            MAXIMUM_SESSION_STATE_NODES,
        )?;
        bounded_usize(
            "maximum_state_bytes",
            self.maximum_state_bytes,
            MAXIMUM_SESSION_STATE_BYTES,
        )?;
        bounded_usize(
            "maximum_transition_messages",
            self.maximum_transition_messages,
            MAXIMUM_SESSION_TRANSITION_MESSAGES,
        )?;
        bounded_usize(
            "maximum_transition_bytes",
            self.maximum_transition_bytes,
            MAXIMUM_SESSION_TRANSITION_BYTES,
        )?;
        if self.maximum_outbound_mailbox_items < self.maximum_transition_messages
            || self.maximum_outbound_mailbox_bytes < self.maximum_transition_bytes
        {
            return Err(session_resource(
                "session_transition_reservation",
                "outbound mailbox capacity must hold one maximum transition batch",
            ));
        }
        if self.maximum_inbound_mailbox_bytes < self.maximum_message_bytes {
            return Err(session_resource(
                "session_inbound_reservation",
                "inbound mailbox capacity must hold one maximum application message",
            ));
        }
        bounded_u64(
            "tick_interval_milliseconds",
            self.tick_interval_milliseconds,
            MAXIMUM_SESSION_INTERVAL_MILLISECONDS,
        )?;
        bounded_u64(
            "idle_timeout_milliseconds",
            self.idle_timeout_milliseconds,
            MAXIMUM_SESSION_LIFETIME_MILLISECONDS,
        )?;
        bounded_u64(
            "maximum_lifetime_milliseconds",
            self.maximum_lifetime_milliseconds,
            MAXIMUM_SESSION_LIFETIME_MILLISECONDS,
        )?;
        bounded_u64(
            "close_grace_milliseconds",
            self.close_grace_milliseconds,
            MAXIMUM_SESSION_GRACE_MILLISECONDS,
        )?;
        bounded_u64(
            "cancellation_grace_milliseconds",
            self.cancellation_grace_milliseconds,
            MAXIMUM_SESSION_GRACE_MILLISECONDS,
        )?;
        if self.idle_timeout_milliseconds > self.maximum_lifetime_milliseconds {
            return Err(session_resource(
                "session_idle_lifetime_limit",
                "idle timeout cannot exceed maximum session lifetime",
            ));
        }
        if self.maximum_process_buffer_bytes == 0
            || self.maximum_process_buffer_bytes > MAXIMUM_PROCESS_SESSION_BUFFER_BYTES
        {
            return Err(session_resource(
                "session_process_buffer_limit",
                format!(
                    "maximum_process_buffer_bytes must be 1 through {MAXIMUM_PROCESS_SESSION_BUFFER_BYTES}"
                ),
            ));
        }
        let per_session = [
            self.maximum_message_bytes,
            self.maximum_inbound_mailbox_bytes,
            self.maximum_outbound_mailbox_bytes,
            self.maximum_state_bytes,
        ]
        .into_iter()
        .try_fold(0_u64, |total, value| {
            u64::try_from(value)
                .ok()
                .and_then(|value| total.checked_add(value))
        })
        .ok_or_else(|| {
            session_resource(
                "session_buffer_arithmetic",
                "per-session buffer reservation overflowed",
            )
        })?;
        if per_session > self.maximum_process_buffer_bytes {
            return Err(session_resource(
                "session_process_buffer_reservation",
                "process-wide session buffer cannot admit one configured session",
            ));
        }
        if u32::try_from(per_session).is_err() {
            return Err(session_resource(
                "session_buffer_arithmetic",
                "per-session buffer reservation does not fit the runtime semaphore",
            ));
        }
        Ok(())
    }

    pub(crate) fn per_session_buffer_bytes(&self) -> Result<u32, Diagnostic> {
        self.validate()?;
        [
            self.maximum_message_bytes,
            self.maximum_inbound_mailbox_bytes,
            self.maximum_outbound_mailbox_bytes,
            self.maximum_state_bytes,
        ]
        .into_iter()
        .try_fold(0_u64, |total, value| {
            u64::try_from(value)
                .ok()
                .and_then(|value| total.checked_add(value))
        })
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| {
            session_resource(
                "session_buffer_arithmetic",
                "per-session buffer reservation does not fit the runtime semaphore",
            )
        })
    }
}

/// Bounded observations from one resident structured-session listener.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionObservation {
    pub pending_handshakes: usize,
    pub active_sessions: usize,
    pub admitted_sessions: u64,
    pub rejected_handshakes: u64,
    pub completed_sessions: u64,
    pub failed_sessions: u64,
    pub overloaded_handshakes: u64,
    pub inbound_messages: u64,
    pub inbound_bytes: u64,
    pub outbound_messages: u64,
    pub outbound_bytes: u64,
    pub maximum_pending_handshakes: usize,
    pub maximum_active_sessions: usize,
    pub maximum_process_buffer_bytes: u64,
}

/// Terminal receipt for a bounded structured-session listener.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionServerReceipt {
    #[serde(skip_serializing)]
    pub contract_version: u16,
    pub local_address: String,
    pub accepted_at_transport: bool,
    pub sessions: SessionObservation,
    pub shutdown: ShutdownReceipt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SessionStandardDeclarations {
    pub event: DeclarationReference,
    pub message_kind: DeclarationReference,
    pub decision_kind: DeclarationReference,
    pub outbound: DeclarationReference,
    pub reject: DeclarationReference,
    pub close: DeclarationReference,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SessionPortRelation {
    pub function: TypeObjectDigest,
    pub state: TypeObjectDigest,
    pub state_option: TypeObjectDigest,
    pub decision: TypeObjectDigest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SessionNominalShape {
    Record(BTreeMap<Name, TypeObjectDigest>),
    Variant(BTreeMap<Name, Option<TypeObjectDigest>>),
}

pub(crate) trait SessionShapeRead {
    fn type_object(&self, digest: TypeObjectDigest) -> Result<TypeObject, Diagnostic>;
    fn nominal_shape(
        &self,
        declaration: DeclarationReference,
    ) -> Result<SessionNominalShape, Diagnostic>;
}

pub(crate) struct CanonicalSessionRead<'a, B: ?Sized> {
    read: &'a B,
    work: Cell<CanonicalReadWork>,
}

pub(crate) struct ExpressionSessionRead<'a, R> {
    read: &'a R,
}

impl<'a, R> ExpressionSessionRead<'a, R> {
    pub(crate) const fn new(read: &'a R) -> Self {
        Self { read }
    }
}

impl<'a, B: CanonicalBaseRead + ?Sized> CanonicalSessionRead<'a, B> {
    pub(crate) fn new(read: &'a B) -> Self {
        Self {
            read,
            work: Cell::new(CanonicalReadWork::default()),
        }
    }

    pub(crate) fn work(&self) -> CanonicalReadWork {
        self.work.get()
    }

    fn add_work(&self, work: CanonicalReadWork) {
        let mut total = self.work.get();
        total.add(work);
        self.work.set(total);
    }

    fn dependency(
        &self,
        package: super::kernel::PackageId,
    ) -> Result<DependencyRecord, Diagnostic> {
        let read = self.read.read_dependency(package)?;
        self.add_work(read.work);
        read.value
            .filter(|dependency| dependency.package == package)
            .ok_or_else(|| {
                session_semantic(
                    "session_standard_dependency",
                    "session declaration belongs to an unbound package",
                )
            })
    }

    fn record(
        &self,
        declaration: DeclarationReference,
        owner: OwnerKey,
    ) -> Result<CanonicalNominalRecord, Diagnostic> {
        if declaration.package == self.read.package_id() {
            let read = self.read.read_owner(owner)?;
            self.add_work(read.work);
            read.value.map(CanonicalNominalRecord::Local)
        } else {
            let dependency = self.dependency(declaration.package)?;
            let read = self.read.read_package_interface_owner(&dependency, owner)?;
            self.add_work(read.work);
            read.value.map(CanonicalNominalRecord::Foreign)
        }
        .ok_or_else(|| {
            session_semantic(
                "session_standard_member",
                "session nominal declaration or member is missing from its exact package interface",
            )
        })
    }
}

enum CanonicalNominalRecord {
    Local(OwnerRecord),
    Foreign(PackageInterfaceRecord),
}

impl<B: CanonicalBaseRead + ?Sized> SessionShapeRead for CanonicalSessionRead<'_, B> {
    fn type_object(&self, digest: TypeObjectDigest) -> Result<TypeObject, Diagnostic> {
        let read = self.read.read_type_object(digest)?;
        self.add_work(read.work);
        read.value.ok_or_else(|| {
            session_semantic(
                "session_type_missing",
                format!("session relation references missing type {digest}"),
            )
        })
    }

    fn nominal_shape(
        &self,
        declaration: DeclarationReference,
    ) -> Result<SessionNominalShape, Diagnostic> {
        match self.record(declaration, OwnerKey::Declaration(declaration.declaration))? {
            CanonicalNominalRecord::Local(OwnerRecord::Declaration(record)) => {
                match record.payload {
                    DeclarationPayload::Record { fields } => {
                        let mut output = BTreeMap::new();
                        for field in fields {
                            let CanonicalNominalRecord::Local(OwnerRecord::Field(record)) =
                                self.record(declaration, OwnerKey::Field(field))?
                            else {
                                return Err(session_semantic(
                                    "session_standard_member",
                                    "record field has a foreign owner kind",
                                ));
                            };
                            output.insert(record.name, record.ty);
                        }
                        Ok(SessionNominalShape::Record(output))
                    }
                    DeclarationPayload::Variant { cases } => {
                        let mut output = BTreeMap::new();
                        for case in cases {
                            let CanonicalNominalRecord::Local(OwnerRecord::Case(record)) =
                                self.record(declaration, OwnerKey::Case(case))?
                            else {
                                return Err(session_semantic(
                                    "session_standard_member",
                                    "variant case has a foreign owner kind",
                                ));
                            };
                            output.insert(record.name, record.payload);
                        }
                        Ok(SessionNominalShape::Variant(output))
                    }
                    _ => Err(session_semantic(
                        "session_standard_kind",
                        "session declaration is not a record or variant",
                    )),
                }
            }
            CanonicalNominalRecord::Foreign(PackageInterfaceRecord::Declaration(record)) => {
                match record.payload {
                    PackageInterfaceDeclarationPayload::Record { fields } => {
                        let mut output = BTreeMap::new();
                        for field in fields {
                            let CanonicalNominalRecord::Foreign(PackageInterfaceRecord::Field(
                                record,
                            )) = self.record(declaration, OwnerKey::Field(field))?
                            else {
                                return Err(session_semantic(
                                    "session_standard_member",
                                    "record field has a foreign interface kind",
                                ));
                            };
                            output.insert(record.name, record.ty);
                        }
                        Ok(SessionNominalShape::Record(output))
                    }
                    PackageInterfaceDeclarationPayload::Variant { cases } => {
                        let mut output = BTreeMap::new();
                        for case in cases {
                            let CanonicalNominalRecord::Foreign(PackageInterfaceRecord::Case(
                                record,
                            )) = self.record(declaration, OwnerKey::Case(case))?
                            else {
                                return Err(session_semantic(
                                    "session_standard_member",
                                    "variant case has a foreign interface kind",
                                ));
                            };
                            output.insert(record.name, record.payload);
                        }
                        Ok(SessionNominalShape::Variant(output))
                    }
                    _ => Err(session_semantic(
                        "session_standard_kind",
                        "session declaration is not a record or variant",
                    )),
                }
            }
            _ => Err(session_semantic(
                "session_standard_kind",
                "session declaration has a foreign owner kind",
            )),
        }
    }
}

impl<R: super::kernel::ExpressionRead> SessionShapeRead for ExpressionSessionRead<'_, R> {
    fn type_object(&self, digest: TypeObjectDigest) -> Result<TypeObject, Diagnostic> {
        self.read.type_object(digest)?.ok_or_else(|| {
            session_semantic(
                "session_type_missing",
                format!("session relation references missing type {digest}"),
            )
        })
    }

    fn nominal_shape(
        &self,
        declaration: DeclarationReference,
    ) -> Result<SessionNominalShape, Diagnostic> {
        let declaration_record = if declaration.package == self.read.package_id() {
            self.read
                .owner(OwnerKey::Declaration(declaration.declaration))?
                .map(ExpressionNominalRecord::Local)
        } else {
            self.read
                .package_interface_owner(
                    declaration.package,
                    OwnerKey::Declaration(declaration.declaration),
                )?
                .map(ExpressionNominalRecord::Foreign)
        }
        .ok_or_else(|| {
            session_semantic(
                "session_standard_declaration",
                "session nominal declaration is missing",
            )
        })?;
        let members = match declaration_record {
            ExpressionNominalRecord::Local(OwnerRecord::Declaration(record)) => {
                match record.payload {
                    DeclarationPayload::Record { fields } => (
                        true,
                        fields.into_iter().map(OwnerKey::Field).collect::<Vec<_>>(),
                    ),
                    DeclarationPayload::Variant { cases } => (
                        false,
                        cases.into_iter().map(OwnerKey::Case).collect::<Vec<_>>(),
                    ),
                    _ => {
                        return Err(session_semantic(
                            "session_standard_kind",
                            "session declaration is not a record or variant",
                        ));
                    }
                }
            }
            ExpressionNominalRecord::Foreign(PackageInterfaceRecord::Declaration(record)) => {
                match record.payload {
                    PackageInterfaceDeclarationPayload::Record { fields } => (
                        true,
                        fields.into_iter().map(OwnerKey::Field).collect::<Vec<_>>(),
                    ),
                    PackageInterfaceDeclarationPayload::Variant { cases } => (
                        false,
                        cases.into_iter().map(OwnerKey::Case).collect::<Vec<_>>(),
                    ),
                    _ => {
                        return Err(session_semantic(
                            "session_standard_kind",
                            "session declaration is not a record or variant",
                        ));
                    }
                }
            }
            _ => {
                return Err(session_semantic(
                    "session_standard_kind",
                    "session declaration has a foreign owner kind",
                ));
            }
        };
        if members.0 {
            let mut output = BTreeMap::new();
            for owner in members.1 {
                let record = self.expression_member(declaration, owner)?;
                let (name, ty) = match record {
                    ExpressionNominalRecord::Local(OwnerRecord::Field(record)) => {
                        (record.name, record.ty)
                    }
                    ExpressionNominalRecord::Foreign(PackageInterfaceRecord::Field(record)) => {
                        (record.name, record.ty)
                    }
                    _ => {
                        return Err(session_semantic(
                            "session_standard_member",
                            "record field has a foreign owner kind",
                        ));
                    }
                };
                output.insert(name, ty);
            }
            Ok(SessionNominalShape::Record(output))
        } else {
            let mut output = BTreeMap::new();
            for owner in members.1 {
                let record = self.expression_member(declaration, owner)?;
                let (name, payload) = match record {
                    ExpressionNominalRecord::Local(OwnerRecord::Case(record)) => {
                        (record.name, record.payload)
                    }
                    ExpressionNominalRecord::Foreign(PackageInterfaceRecord::Case(record)) => {
                        (record.name, record.payload)
                    }
                    _ => {
                        return Err(session_semantic(
                            "session_standard_member",
                            "variant case has a foreign owner kind",
                        ));
                    }
                };
                output.insert(name, payload);
            }
            Ok(SessionNominalShape::Variant(output))
        }
    }
}

enum ExpressionNominalRecord {
    Local(OwnerRecord),
    Foreign(PackageInterfaceRecord),
}

impl<R: super::kernel::ExpressionRead> ExpressionSessionRead<'_, R> {
    fn expression_member(
        &self,
        declaration: DeclarationReference,
        owner: OwnerKey,
    ) -> Result<ExpressionNominalRecord, Diagnostic> {
        let value = if declaration.package == self.read.package_id() {
            self.read.owner(owner)?.map(ExpressionNominalRecord::Local)
        } else {
            self.read
                .package_interface_owner(declaration.package, owner)?
                .map(ExpressionNominalRecord::Foreign)
        };
        value.ok_or_else(|| {
            session_semantic(
                "session_standard_member",
                "session nominal member is missing",
            )
        })
    }
}

pub(crate) fn validate_session_function_type<R: SessionShapeRead>(
    read: &R,
    standard: SessionStandardDeclarations,
    function: TypeObjectDigest,
) -> Result<SessionPortRelation, Diagnostic> {
    validate_standard_session_shapes(read, standard)?;
    let TypeForm::Function { parameters, result } = read.type_object(function)?.form else {
        return Err(session_semantic(
            "session_port_function",
            "interactive port must have a function type",
        ));
    };
    let [state_option, event] = parameters.as_slice() else {
        return Err(session_semantic(
            "session_port_parameters",
            "interactive port must accept exactly state option and session event",
        ));
    };
    let TypeForm::Option { item: state } = read.type_object(*state_option)?.form else {
        return Err(session_semantic(
            "session_port_state_option",
            "interactive port first parameter must be Option<State>",
        ));
    };
    expect_named(read, *event, standard.event, "session_port_event")?;
    validate_ordinary_state(read, state)?;
    let fields = structural_fields(read, result, "session_port_decision")?;
    expect_fields(
        &fields,
        &["closing", "kind", "messages", "rejection", "state"],
        "session_port_decision",
    )?;
    expect_named(
        read,
        fields["kind"],
        standard.decision_kind,
        "session_port_decision_kind",
    )?;
    expect_list_named(
        read,
        fields["messages"],
        standard.outbound,
        "session_port_messages",
    )?;
    expect_option_named(
        read,
        fields["rejection"],
        standard.reject,
        "session_port_rejection",
    )?;
    expect_option_named(
        read,
        fields["closing"],
        standard.close,
        "session_port_closing",
    )?;
    let TypeForm::Option {
        item: repeated_state,
    } = read.type_object(fields["state"])?.form
    else {
        return Err(session_semantic(
            "session_port_decision_state",
            "session decision state must be Option<State>",
        ));
    };
    if repeated_state != state {
        return Err(session_semantic(
            "session_port_state_identity",
            "interactive input and decision must repeat one exact State type",
        ));
    }
    Ok(SessionPortRelation {
        function,
        state,
        state_option: *state_option,
        decision: result,
    })
}

fn validate_standard_session_shapes<R: SessionShapeRead>(
    read: &R,
    standard: SessionStandardDeclarations,
) -> Result<(), Diagnostic> {
    let message_kind = variant_cases(read, standard.message_kind, "session_standard_message_kind")?;
    expect_cases(
        &message_kind,
        &["binary", "text"],
        "session_standard_message_kind",
    )?;
    require_payloads(
        &message_kind,
        &[None, None],
        "session_standard_message_kind",
    )?;

    let decision_kind = variant_cases(
        read,
        standard.decision_kind,
        "session_standard_decision_kind",
    )?;
    expect_cases(
        &decision_kind,
        &["accept", "close", "continue", "finish", "reject"],
        "session_standard_decision_kind",
    )?;
    require_payloads(
        &decision_kind,
        &[None, None, None, None, None],
        "session_standard_decision_kind",
    )?;

    let outbound = variant_cases(read, standard.outbound, "session_standard_outbound")?;
    expect_cases(&outbound, &["binary", "text"], "session_standard_outbound")?;
    expect_form(
        read,
        outbound["binary"].ok_or_else(|| {
            session_semantic(
                "session_standard_outbound",
                "binary outbound payload is missing",
            )
        })?,
        TypeTag::Bytes,
        "session_standard_outbound",
    )?;
    expect_form(
        read,
        outbound["text"].ok_or_else(|| {
            session_semantic(
                "session_standard_outbound",
                "text outbound payload is missing",
            )
        })?,
        TypeTag::Text,
        "session_standard_outbound",
    )?;

    let reject = nominal_record_fields(read, standard.reject, "session_standard_reject")?;
    expect_fields(
        &reject,
        &["body", "headers", "status"],
        "session_standard_reject",
    )?;
    expect_form(
        read,
        reject["body"],
        TypeTag::Bytes,
        "session_standard_reject",
    )?;
    expect_form(
        read,
        reject["status"],
        TypeTag::I64,
        "session_standard_reject",
    )?;
    expect_header_list(read, reject["headers"], "session_standard_reject")?;

    let close = nominal_record_fields(read, standard.close, "session_standard_close")?;
    expect_fields(&close, &["code", "reason"], "session_standard_close")?;
    expect_form(read, close["code"], TypeTag::I64, "session_standard_close")?;
    expect_form(
        read,
        close["reason"],
        TypeTag::Text,
        "session_standard_close",
    )?;

    let event = variant_cases(read, standard.event, "session_standard_event")?;
    expect_cases(
        &event,
        &["message", "open", "peer-close", "shutdown", "tick"],
        "session_standard_event",
    )?;
    for name in ["shutdown", "tick"] {
        if event[name].is_some() {
            return Err(session_semantic(
                "session_standard_event",
                format!("{name} event must not have a payload"),
            ));
        }
    }
    let open = structural_fields(
        read,
        required_payload(&event, "open")?,
        "session_standard_open",
    )?;
    expect_fields(
        &open,
        &["headers", "path", "query"],
        "session_standard_open",
    )?;
    expect_header_list(read, open["headers"], "session_standard_open")?;
    expect_form(read, open["path"], TypeTag::Text, "session_standard_open")?;
    expect_form(read, open["query"], TypeTag::Text, "session_standard_open")?;
    let message = structural_fields(
        read,
        required_payload(&event, "message")?,
        "session_standard_message",
    )?;
    expect_fields(&message, &["body", "kind"], "session_standard_message")?;
    let TypeForm::Stream { item } = read.type_object(message["body"])?.form else {
        return Err(session_semantic(
            "session_standard_message",
            "message body must be Stream<Bytes>",
        ));
    };
    expect_form(read, item, TypeTag::Bytes, "session_standard_message")?;
    expect_named(
        read,
        message["kind"],
        standard.message_kind,
        "session_standard_message",
    )?;
    let peer = structural_fields(
        read,
        required_payload(&event, "peer-close")?,
        "session_standard_peer_close",
    )?;
    expect_fields(&peer, &["code", "reason"], "session_standard_peer_close")?;
    let TypeForm::Option { item } = read.type_object(peer["code"])?.form else {
        return Err(session_semantic(
            "session_standard_peer_close",
            "peer-close code must be Option<I64>",
        ));
    };
    expect_form(read, item, TypeTag::I64, "session_standard_peer_close")?;
    expect_form(
        read,
        peer["reason"],
        TypeTag::Text,
        "session_standard_peer_close",
    )?;
    Ok(())
}

fn validate_ordinary_state<R: SessionShapeRead>(
    read: &R,
    root: TypeObjectDigest,
) -> Result<(), Diagnostic> {
    fn visit<R: SessionShapeRead>(
        read: &R,
        ty: TypeObjectDigest,
        visiting: &mut BTreeSet<TypeObjectDigest>,
        complete: &mut BTreeSet<TypeObjectDigest>,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        if complete.contains(&ty) {
            return Ok(());
        }
        if depth > super::kernel::contract::MAXIMUM_TYPE_DEPTH || !visiting.insert(ty) {
            return Err(session_semantic(
                "session_state_cycle",
                "retained session state type is cyclic or too deep",
            ));
        }
        match read.type_object(ty)?.form {
            TypeForm::Unit | TypeForm::Bool | TypeForm::I64 | TypeForm::Bytes | TypeForm::Text => {}
            TypeForm::StructuralRecord { fields } => {
                for field in fields {
                    visit(read, field.ty, visiting, complete, depth + 1)?;
                }
            }
            TypeForm::List { item } | TypeForm::Option { item } => {
                visit(read, item, visiting, complete, depth + 1)?;
            }
            TypeForm::Map { key, value } => {
                if !matches!(
                    read.type_object(key)?.form,
                    TypeForm::Bool | TypeForm::I64 | TypeForm::Bytes | TypeForm::Text
                ) {
                    return Err(session_semantic(
                        "session_state_map_key",
                        "retained session state map keys must be deterministic primitive values",
                    ));
                }
                visit(read, value, visiting, complete, depth + 1)?;
            }
            TypeForm::Named { declaration } => match read.nominal_shape(declaration)? {
                SessionNominalShape::Record(fields) => {
                    for field in fields.into_values() {
                        visit(read, field, visiting, complete, depth + 1)?;
                    }
                }
                SessionNominalShape::Variant(cases) => {
                    for payload in cases.into_values().flatten() {
                        visit(read, payload, visiting, complete, depth + 1)?;
                    }
                }
            },
            TypeForm::StaticText
            | TypeForm::Secret
            | TypeForm::Result { .. }
            | TypeForm::TypeParameter { .. }
            | TypeForm::CapabilityResource { .. }
            | TypeForm::Stream { .. }
            | TypeForm::Function { .. } => {
                return Err(session_semantic(
                    "session_state_live_type",
                    "retained session state contains a live, callable, static, secret, or unresolved type",
                ));
            }
        }
        visiting.remove(&ty);
        complete.insert(ty);
        Ok(())
    }
    visit(read, root, &mut BTreeSet::new(), &mut BTreeSet::new(), 0)
}

fn nominal_record_fields<R: SessionShapeRead>(
    read: &R,
    declaration: DeclarationReference,
    code: &'static str,
) -> Result<BTreeMap<Name, TypeObjectDigest>, Diagnostic> {
    match read.nominal_shape(declaration)? {
        SessionNominalShape::Record(fields) => Ok(fields),
        SessionNominalShape::Variant(_) => Err(session_semantic(
            code,
            "expected a session record declaration",
        )),
    }
}

fn variant_cases<R: SessionShapeRead>(
    read: &R,
    declaration: DeclarationReference,
    code: &'static str,
) -> Result<BTreeMap<Name, Option<TypeObjectDigest>>, Diagnostic> {
    match read.nominal_shape(declaration)? {
        SessionNominalShape::Variant(cases) => Ok(cases),
        SessionNominalShape::Record(_) => Err(session_semantic(
            code,
            "expected a session variant declaration",
        )),
    }
}

fn structural_fields<R: SessionShapeRead>(
    read: &R,
    ty: TypeObjectDigest,
    code: &'static str,
) -> Result<BTreeMap<Name, TypeObjectDigest>, Diagnostic> {
    let TypeForm::StructuralRecord { fields } = read.type_object(ty)?.form else {
        return Err(session_semantic(
            code,
            "expected an exact structural record",
        ));
    };
    Ok(fields
        .into_iter()
        .map(|field| (field.name, field.ty))
        .collect())
}

fn expect_fields<T>(
    fields: &BTreeMap<Name, T>,
    names: &[&str],
    code: &'static str,
) -> Result<(), Diagnostic> {
    if fields.keys().map(Name::as_str).eq(names.iter().copied()) {
        Ok(())
    } else {
        Err(session_semantic(
            code,
            "session record fields do not equal the canonical closed field set",
        ))
    }
}

fn expect_cases(
    cases: &BTreeMap<Name, Option<TypeObjectDigest>>,
    names: &[&str],
    code: &'static str,
) -> Result<(), Diagnostic> {
    if cases.keys().map(Name::as_str).eq(names.iter().copied()) {
        Ok(())
    } else {
        Err(session_semantic(
            code,
            "session variant cases do not equal the canonical closed case set",
        ))
    }
}

fn require_payloads(
    cases: &BTreeMap<Name, Option<TypeObjectDigest>>,
    payloads: &[Option<TypeObjectDigest>],
    code: &'static str,
) -> Result<(), Diagnostic> {
    if cases.values().copied().eq(payloads.iter().copied()) {
        Ok(())
    } else {
        Err(session_semantic(
            code,
            "session variant payload presence is not canonical",
        ))
    }
}

fn required_payload(
    cases: &BTreeMap<Name, Option<TypeObjectDigest>>,
    name: &str,
) -> Result<TypeObjectDigest, Diagnostic> {
    cases
        .iter()
        .find_map(|(candidate, payload)| (candidate.as_str() == name).then_some(*payload))
        .flatten()
        .ok_or_else(|| {
            session_semantic(
                "session_standard_event",
                format!("{name} event payload is missing"),
            )
        })
}

#[derive(Clone, Copy)]
enum TypeTag {
    Bytes,
    I64,
    Text,
}

fn expect_form<R: SessionShapeRead>(
    read: &R,
    ty: TypeObjectDigest,
    tag: TypeTag,
    code: &'static str,
) -> Result<(), Diagnostic> {
    let form = read.type_object(ty)?.form;
    let valid = matches!(
        (tag, form),
        (TypeTag::Bytes, TypeForm::Bytes)
            | (TypeTag::I64, TypeForm::I64)
            | (TypeTag::Text, TypeForm::Text)
    );
    valid
        .then_some(())
        .ok_or_else(|| session_semantic(code, "session field has a foreign type"))
}

fn expect_named<R: SessionShapeRead>(
    read: &R,
    ty: TypeObjectDigest,
    expected: DeclarationReference,
    code: &'static str,
) -> Result<(), Diagnostic> {
    matches!(read.type_object(ty)?.form, TypeForm::Named { declaration } if declaration == expected)
        .then_some(())
        .ok_or_else(|| session_semantic(code, "session field names a foreign nominal type"))
}

fn expect_list_named<R: SessionShapeRead>(
    read: &R,
    ty: TypeObjectDigest,
    expected: DeclarationReference,
    code: &'static str,
) -> Result<(), Diagnostic> {
    let TypeForm::List { item } = read.type_object(ty)?.form else {
        return Err(session_semantic(code, "session field must be a list"));
    };
    expect_named(read, item, expected, code)
}

fn expect_option_named<R: SessionShapeRead>(
    read: &R,
    ty: TypeObjectDigest,
    expected: DeclarationReference,
    code: &'static str,
) -> Result<(), Diagnostic> {
    let TypeForm::Option { item } = read.type_object(ty)?.form else {
        return Err(session_semantic(code, "session field must be an option"));
    };
    expect_named(read, item, expected, code)
}

fn expect_header_list<R: SessionShapeRead>(
    read: &R,
    ty: TypeObjectDigest,
    code: &'static str,
) -> Result<(), Diagnostic> {
    let TypeForm::List { item } = read.type_object(ty)?.form else {
        return Err(session_semantic(code, "session headers must be a list"));
    };
    let fields = structural_fields(read, item, code)?;
    expect_fields(&fields, &["name", "value"], code)?;
    expect_form(read, fields["name"], TypeTag::Text, code)?;
    expect_form(read, fields["value"], TypeTag::Bytes, code)
}

fn bounded_usize(name: &str, value: usize, maximum: usize) -> Result<(), Diagnostic> {
    if value == 0 || value > maximum {
        Err(session_resource(
            "session_limit",
            format!("{name} must be 1 through {maximum}"),
        ))
    } else {
        Ok(())
    }
}

fn bounded_u64(name: &str, value: u64, maximum: u64) -> Result<(), Diagnostic> {
    if value == 0 || value > maximum {
        Err(session_resource(
            "session_limit",
            format!("{name} must be 1 through {maximum}"),
        ))
    } else {
        Ok(())
    }
}

fn session_semantic(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}

fn session_resource(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Resource, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::kernel::{StructuralTypeField, TypeObjectInterner};
    use crate::platform::semantic_id::DeclarationId;

    #[derive(Clone, Copy)]
    enum StateKind {
        Ordinary,
        Mismatched,
        Stream,
    }

    struct ShapeOracle {
        objects: BTreeMap<TypeObjectDigest, TypeObject>,
        shapes: BTreeMap<DeclarationReference, SessionNominalShape>,
    }

    impl SessionShapeRead for ShapeOracle {
        fn type_object(&self, digest: TypeObjectDigest) -> Result<TypeObject, Diagnostic> {
            self.objects.get(&digest).cloned().ok_or_else(|| {
                session_semantic(
                    "test_shape_missing",
                    "independent shape oracle missed a type",
                )
            })
        }

        fn nominal_shape(
            &self,
            declaration: DeclarationReference,
        ) -> Result<SessionNominalShape, Diagnostic> {
            self.shapes.get(&declaration).cloned().ok_or_else(|| {
                session_semantic(
                    "test_shape_missing",
                    "independent shape oracle missed a declaration",
                )
            })
        }
    }

    fn name(value: &str) -> Name {
        Name::new(value).expect("test name")
    }

    fn fields(
        values: impl IntoIterator<Item = (&'static str, TypeObjectDigest)>,
    ) -> BTreeMap<Name, TypeObjectDigest> {
        values
            .into_iter()
            .map(|(field, ty)| (name(field), ty))
            .collect()
    }

    fn cases(
        values: impl IntoIterator<Item = (&'static str, Option<TypeObjectDigest>)>,
    ) -> BTreeMap<Name, Option<TypeObjectDigest>> {
        values
            .into_iter()
            .map(|(case, payload)| (name(case), payload))
            .collect()
    }

    fn structural(
        types: &mut TypeObjectInterner,
        values: impl IntoIterator<Item = (&'static str, TypeObjectDigest)>,
    ) -> TypeObjectDigest {
        types
            .intern(TypeForm::StructuralRecord {
                fields: values
                    .into_iter()
                    .map(|(field, ty)| StructuralTypeField {
                        name: name(field),
                        ty,
                    })
                    .collect(),
            })
            .expect("test structural type")
    }

    fn session_oracle(
        state_kind: StateKind,
    ) -> (ShapeOracle, SessionStandardDeclarations, TypeObjectDigest) {
        const SEED: &[u8] = b"structured-session-shape-oracle";
        let package = super::super::kernel::PackageId::migrate(SEED, 0);
        let declaration = |ordinal| DeclarationReference {
            package,
            declaration: DeclarationId::migrate(SEED, ordinal),
        };
        let standard = SessionStandardDeclarations {
            event: declaration(0),
            message_kind: declaration(1),
            decision_kind: declaration(2),
            outbound: declaration(3),
            reject: declaration(4),
            close: declaration(5),
        };
        let mut types = TypeObjectInterner::default();
        let unit = types.intern(TypeForm::Unit).expect("unit type");
        let i64 = types.intern(TypeForm::I64).expect("i64 type");
        let bytes = types.intern(TypeForm::Bytes).expect("bytes type");
        let text = types.intern(TypeForm::Text).expect("text type");
        let stream_bytes = types
            .intern(TypeForm::Stream { item: bytes })
            .expect("stream type");
        let header = structural(&mut types, [("name", text), ("value", bytes)]);
        let headers = types
            .intern(TypeForm::List { item: header })
            .expect("header list");
        let optional_code = types
            .intern(TypeForm::Option { item: i64 })
            .expect("optional close code");
        let message_kind = types
            .intern(TypeForm::Named {
                declaration: standard.message_kind,
            })
            .expect("message kind");
        let decision_kind = types
            .intern(TypeForm::Named {
                declaration: standard.decision_kind,
            })
            .expect("decision kind");
        let outbound = types
            .intern(TypeForm::Named {
                declaration: standard.outbound,
            })
            .expect("outbound");
        let reject = types
            .intern(TypeForm::Named {
                declaration: standard.reject,
            })
            .expect("reject");
        let close = types
            .intern(TypeForm::Named {
                declaration: standard.close,
            })
            .expect("close");
        let event = types
            .intern(TypeForm::Named {
                declaration: standard.event,
            })
            .expect("event");
        let open = structural(
            &mut types,
            [("headers", headers), ("path", text), ("query", text)],
        );
        let message = structural(&mut types, [("body", stream_bytes), ("kind", message_kind)]);
        let peer_close = structural(&mut types, [("code", optional_code), ("reason", text)]);
        let state = match state_kind {
            StateKind::Ordinary | StateKind::Mismatched => unit,
            StateKind::Stream => stream_bytes,
        };
        let repeated_state = match state_kind {
            StateKind::Mismatched => text,
            StateKind::Ordinary | StateKind::Stream => state,
        };
        let state_option = types
            .intern(TypeForm::Option { item: state })
            .expect("state option");
        let repeated_state_option = types
            .intern(TypeForm::Option {
                item: repeated_state,
            })
            .expect("repeated state option");
        let outbound_list = types
            .intern(TypeForm::List { item: outbound })
            .expect("outbound list");
        let rejection = types
            .intern(TypeForm::Option { item: reject })
            .expect("rejection option");
        let closing = types
            .intern(TypeForm::Option { item: close })
            .expect("closing option");
        let decision = structural(
            &mut types,
            [
                ("closing", closing),
                ("kind", decision_kind),
                ("messages", outbound_list),
                ("rejection", rejection),
                ("state", repeated_state_option),
            ],
        );
        let function = types
            .intern(TypeForm::Function {
                parameters: vec![state_option, event],
                result: decision,
            })
            .expect("session function");
        let shapes = BTreeMap::from([
            (
                standard.message_kind,
                SessionNominalShape::Variant(cases([("binary", None), ("text", None)])),
            ),
            (
                standard.decision_kind,
                SessionNominalShape::Variant(cases([
                    ("accept", None),
                    ("close", None),
                    ("continue", None),
                    ("finish", None),
                    ("reject", None),
                ])),
            ),
            (
                standard.outbound,
                SessionNominalShape::Variant(cases([
                    ("binary", Some(bytes)),
                    ("text", Some(text)),
                ])),
            ),
            (
                standard.reject,
                SessionNominalShape::Record(fields([
                    ("body", bytes),
                    ("headers", headers),
                    ("status", i64),
                ])),
            ),
            (
                standard.close,
                SessionNominalShape::Record(fields([("code", i64), ("reason", text)])),
            ),
            (
                standard.event,
                SessionNominalShape::Variant(cases([
                    ("message", Some(message)),
                    ("open", Some(open)),
                    ("peer-close", Some(peer_close)),
                    ("shutdown", None),
                    ("tick", None),
                ])),
            ),
        ]);
        (
            ShapeOracle {
                objects: types.into_objects(),
                shapes,
            },
            standard,
            function,
        )
    }

    #[test]
    fn default_limits_are_valid_and_support_a_twenty_four_hour_session() {
        let limits = SessionLimits::default();
        limits.validate().expect("default session limits");
        assert!(limits.maximum_lifetime_milliseconds >= 24 * 60 * 60 * 1_000);
        assert_eq!(
            limits
                .per_session_buffer_bytes()
                .expect("default reservation"),
            13 * 1024 * 1024
        );
    }

    #[test]
    fn every_session_limit_dimension_rejects_zero_independently() {
        let zeroers: [fn(&mut SessionLimits); 20] = [
            |value| value.maximum_active_sessions = 0,
            |value| value.maximum_pending_handshakes = 0,
            |value| value.maximum_message_bytes = 0,
            |value| value.maximum_frame_bytes = 0,
            |value| value.maximum_header_bytes = 0,
            |value| value.maximum_headers = 0,
            |value| value.maximum_inbound_mailbox_items = 0,
            |value| value.maximum_inbound_mailbox_bytes = 0,
            |value| value.maximum_outbound_mailbox_items = 0,
            |value| value.maximum_outbound_mailbox_bytes = 0,
            |value| value.maximum_state_nodes = 0,
            |value| value.maximum_state_bytes = 0,
            |value| value.maximum_transition_messages = 0,
            |value| value.maximum_transition_bytes = 0,
            |value| value.tick_interval_milliseconds = 0,
            |value| value.idle_timeout_milliseconds = 0,
            |value| value.maximum_lifetime_milliseconds = 0,
            |value| value.close_grace_milliseconds = 0,
            |value| value.cancellation_grace_milliseconds = 0,
            |value| value.maximum_process_buffer_bytes = 0,
        ];
        for zero in zeroers {
            let mut limits = SessionLimits::default();
            zero(&mut limits);
            assert!(limits.validate().is_err());
        }
    }

    #[test]
    fn session_limits_enforce_reservation_and_lifetime_relations() {
        let mut limits = SessionLimits::default();
        limits.maximum_frame_bytes = limits.maximum_message_bytes + 1;
        assert_eq!(
            limits.validate().expect_err("frame/message relation").code,
            "session_frame_message_limit"
        );

        let mut limits = SessionLimits::default();
        limits.maximum_transition_messages = limits.maximum_outbound_mailbox_items + 1;
        assert_eq!(
            limits.validate().expect_err("item reservation").code,
            "session_transition_reservation"
        );

        let mut limits = SessionLimits::default();
        limits.maximum_inbound_mailbox_bytes = limits.maximum_message_bytes - 1;
        assert_eq!(
            limits.validate().expect_err("inbound reservation").code,
            "session_inbound_reservation"
        );

        let mut limits = SessionLimits::default();
        limits.idle_timeout_milliseconds = limits.maximum_lifetime_milliseconds + 1;
        assert_eq!(
            limits.validate().expect_err("idle/lifetime relation").code,
            "session_idle_lifetime_limit"
        );

        let limits = SessionLimits {
            maximum_process_buffer_bytes: 1,
            ..SessionLimits::default()
        };
        assert_eq!(
            limits.validate().expect_err("process reservation").code,
            "session_process_buffer_reservation"
        );
    }

    #[test]
    fn independent_shape_oracle_accepts_only_one_closed_repeated_state() {
        let (read, standard, function) = session_oracle(StateKind::Ordinary);
        let relation = validate_session_function_type(&read, standard, function)
            .expect("exact relational session type");
        assert_eq!(relation.function, function);

        let (read, standard, function) = session_oracle(StateKind::Mismatched);
        assert_eq!(
            validate_session_function_type(&read, standard, function)
                .expect_err("mismatched repeated state")
                .code,
            "session_port_state_identity"
        );

        let (read, standard, function) = session_oracle(StateKind::Stream);
        assert_eq!(
            validate_session_function_type(&read, standard, function)
                .expect_err("live retained stream")
                .code,
            "session_state_live_type"
        );
    }
}
