//! Generic bounded foreground execution for interactive application artifacts.
//!
//! Headless replay and live terminal execution use the same application-owned initialize,
//! update, and render functions. This owner records deterministic frame/action digests; terminal
//! adaptation is implemented separately and has no application policy.

use crate::application::{
    ApplicationDigest, InteractiveActionKind, InteractiveActionOutcome, InteractiveActionRequest,
    InteractiveEvent, InteractiveFrame, InteractiveStep, prepare_interactive,
};
use crate::error::{ErrorCode, LkError, Result};
use serde::{Deserialize, Serialize};

pub const HEADLESS_REPLAY_CONTRACT_VERSION: u16 = 4;
pub const MAXIMUM_HEADLESS_TRANSITIONS: usize = 20_000;
pub const MAXIMUM_HEADLESS_ACTIONS: usize = 10_000;
pub const MAXIMUM_HEADLESS_INPUT_BYTES: usize = 8 * 1024 * 1024;

const REPLAY_DIGEST_DOMAIN: &str = "lkjscript.interactive-headless-replay.v4";
const FRAME_DIGEST_DOMAIN: &str = "lkjscript.interactive-frame.v2";
const ACTION_DIGEST_DOMAIN: &str = "lkjscript.interactive-action-trace.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum HeadlessReplayTransition {
    Event(InteractiveEvent),
    Outcome(InteractiveActionOutcome),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HeadlessReplayRequest {
    pub version: u16,
    pub rows: i64,
    pub columns: i64,
    pub transitions: Vec<HeadlessReplayTransition>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HeadlessActionTrace {
    pub transition: u64,
    pub job_id: u64,
    pub kind: InteractiveActionKind,
    pub payload_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HeadlessReplayReceipt {
    pub version: u16,
    pub application: ApplicationDigest,
    pub transition_count: u64,
    pub event_count: u64,
    pub outcome_count: u64,
    pub action_count: u64,
    pub action_trace: Vec<HeadlessActionTrace>,
    pub changed_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_transition: Option<u64>,
    pub initial_frame_digest: String,
    pub final_frame_digest: String,
    pub replay_digest: String,
    pub final_frame: InteractiveFrame,
    pub execute_nanoseconds: u64,
    pub public_value_nanoseconds: u64,
}

pub fn decode_headless_replay(bytes: &[u8]) -> Result<HeadlessReplayRequest> {
    if bytes.len() > MAXIMUM_HEADLESS_INPUT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "headless replay input exceeds byte policy",
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let request = HeadlessReplayRequest::deserialize(&mut deserializer).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("headless replay JSON is malformed: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("headless replay JSON has trailing input: {error}"),
        )
    })?;
    validate_request(&request)?;
    Ok(request)
}

pub fn run_headless_replay(
    application_bytes: &[u8],
    request: &HeadlessReplayRequest,
) -> Result<HeadlessReplayReceipt> {
    validate_request(request)?;
    let prepared = prepare_interactive(application_bytes)?;
    let application = prepared.digest();
    let (mut session, initial) = prepared.start(request.rows, request.columns)?;
    let initial_frame_digest = frame_digest(&initial.frame)?;
    let mut replay = blake3::Hasher::new_derive_key(REPLAY_DIGEST_DOMAIN);
    replay.update(application.as_bytes().as_slice());
    let request_bytes = serde_json::to_vec(request).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("headless replay request cannot be encoded: {error}"),
        )
    })?;
    replay.update(&(request_bytes.len() as u64).to_le_bytes());
    replay.update(&request_bytes);
    record_step(&mut replay, 0, &initial)?;

    let mut final_frame = initial.frame;
    let mut changed_count = 0_u64;
    let mut action_trace = Vec::new();
    if let Some(action) = &initial.action {
        action_trace.push(trace_action(0, action)?);
    }
    let mut action_count = action_trace.len() as u64;
    let mut event_count = 0_u64;
    let mut outcome_count = 0_u64;
    let mut exit_transition = initial.exit.then_some(0);
    let mut execute_nanoseconds = initial.observation.execute_nanoseconds;
    let mut public_value_nanoseconds = initial.observation.public_value_nanoseconds;
    for (offset, transition) in request.transitions.iter().cloned().enumerate() {
        if exit_transition.is_some() {
            return Err(LkError::new(
                ErrorCode::InvalidOperand,
                "headless replay contains a transition after the application requested exit",
            ));
        }
        let step = match transition {
            HeadlessReplayTransition::Event(event) => {
                event_count = event_count.saturating_add(1);
                session.step(event)?
            }
            HeadlessReplayTransition::Outcome(outcome) => {
                outcome_count = outcome_count.saturating_add(1);
                session.resume(outcome)?
            }
        };
        let transition_number = u64::try_from(offset).unwrap_or(u64::MAX).saturating_add(1);
        record_step(&mut replay, transition_number, &step)?;
        if step.changed {
            changed_count = changed_count.saturating_add(1);
        }
        if let Some(action) = &step.action {
            action_count = action_count.saturating_add(1);
            if action_count as usize > MAXIMUM_HEADLESS_ACTIONS {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "headless replay action count exceeds policy",
                ));
            }
            action_trace.push(trace_action(transition_number, action)?);
        }
        execute_nanoseconds =
            execute_nanoseconds.saturating_add(step.observation.execute_nanoseconds);
        public_value_nanoseconds =
            public_value_nanoseconds.saturating_add(step.observation.public_value_nanoseconds);
        final_frame = step.frame;
        if step.exit {
            exit_transition = Some(transition_number);
        }
    }
    if session.pending_action_id().is_some() {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "headless replay ends before the pending host action receives an outcome",
        ));
    }
    let final_frame_digest = frame_digest(&final_frame)?;
    Ok(HeadlessReplayReceipt {
        version: HEADLESS_REPLAY_CONTRACT_VERSION,
        application,
        transition_count: request.transitions.len() as u64,
        event_count,
        outcome_count,
        action_count,
        action_trace,
        changed_count,
        exit_transition,
        initial_frame_digest,
        final_frame_digest,
        replay_digest: hex(replay.finalize().as_bytes()),
        final_frame,
        execute_nanoseconds,
        public_value_nanoseconds,
    })
}

fn trace_action(transition: u64, action: &InteractiveActionRequest) -> Result<HeadlessActionTrace> {
    let bytes = serde_json::to_vec(action).map_err(|error| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            format!("interactive action trace cannot be encoded: {error}"),
        )
    })?;
    let mut digest = blake3::Hasher::new_derive_key(ACTION_DIGEST_DOMAIN);
    digest.update(&(bytes.len() as u64).to_le_bytes());
    digest.update(&bytes);
    Ok(HeadlessActionTrace {
        transition,
        job_id: action.job_id,
        kind: action.action.kind(),
        payload_digest: hex(digest.finalize().as_bytes()),
    })
}

pub fn frame_digest(frame: &InteractiveFrame) -> Result<String> {
    let bytes = serde_json::to_vec(frame).map_err(|error| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            format!("interactive frame cannot be encoded: {error}"),
        )
    })?;
    let mut digest = blake3::Hasher::new_derive_key(FRAME_DIGEST_DOMAIN);
    digest.update(&(bytes.len() as u64).to_le_bytes());
    digest.update(&bytes);
    Ok(hex(digest.finalize().as_bytes()))
}

fn validate_request(request: &HeadlessReplayRequest) -> Result<()> {
    if request.version != HEADLESS_REPLAY_CONTRACT_VERSION {
        return Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "headless replay contract version is unsupported",
        ));
    }
    if request.transitions.len() > MAXIMUM_HEADLESS_TRANSITIONS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "headless replay transition count exceeds policy",
        ));
    }
    if request
        .transitions
        .iter()
        .filter(|transition| matches!(transition, HeadlessReplayTransition::Outcome(_)))
        .count()
        > MAXIMUM_HEADLESS_ACTIONS
    {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "headless replay outcome count exceeds policy",
        ));
    }
    let encoded = serde_json::to_vec(request).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("headless replay request cannot be encoded: {error}"),
        )
    })?;
    if encoded.len() > MAXIMUM_HEADLESS_INPUT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "headless replay request exceeds byte policy",
        ));
    }
    Ok(())
}

fn record_step(hasher: &mut blake3::Hasher, number: u64, step: &InteractiveStep) -> Result<()> {
    hasher.update(&number.to_le_bytes());
    hasher.update(&[u8::from(step.changed), u8::from(step.exit)]);
    let digest = frame_digest(&step.frame)?;
    hasher.update(&(digest.len() as u64).to_le_bytes());
    hasher.update(digest.as_bytes());
    let action = serde_json::to_vec(&step.action).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("interactive action cannot be encoded: {error}"),
        )
    })?;
    hasher.update(&(action.len() as u64).to_le_bytes());
    hasher.update(&action);
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}
