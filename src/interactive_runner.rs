//! Generic bounded foreground execution for interactive application artifacts.
//!
//! Headless replay and live terminal execution use the same application-owned initialize,
//! update, and render functions. This owner records deterministic frame/action digests; terminal
//! adaptation is implemented separately and has no application policy.

use crate::application::{
    ApplicationDigest, InteractiveActionOutcome, InteractiveEvent, InteractiveFrame,
    InteractiveStep, prepare_interactive,
};
use crate::error::{ErrorCode, LkError, Result};
use serde::{Deserialize, Serialize};

pub const HEADLESS_REPLAY_CONTRACT_VERSION: u16 = 3;
pub const MAXIMUM_HEADLESS_EVENTS: usize = 10_000;
pub const MAXIMUM_HEADLESS_ACTIONS: usize = 10_000;
pub const MAXIMUM_HEADLESS_INPUT_BYTES: usize = 8 * 1024 * 1024;

const REPLAY_DIGEST_DOMAIN: &str = "lkjscript.interactive-headless-replay.v3";
const FRAME_DIGEST_DOMAIN: &str = "lkjscript.interactive-frame.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HeadlessReplayRequest {
    pub version: u16,
    pub rows: i64,
    pub columns: i64,
    pub events: Vec<InteractiveEvent>,
    pub outcomes: Vec<InteractiveActionOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HeadlessReplayReceipt {
    pub version: u16,
    pub application: ApplicationDigest,
    pub event_count: u64,
    pub action_count: u64,
    pub changed_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_event: Option<u64>,
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
    let mut action_count = 0_u64;
    let mut outcome_offset = 0_usize;
    let mut transition_number = 0_u64;
    let mut exit_event = None;
    let mut execute_nanoseconds = initial.observation.execute_nanoseconds;
    let mut public_value_nanoseconds = initial.observation.public_value_nanoseconds;
    for (offset, event) in request.events.iter().cloned().enumerate() {
        if exit_event.is_some() {
            return Err(LkError::new(
                ErrorCode::InvalidOperand,
                "headless replay contains an event after the application requested exit",
            ));
        }
        let mut step = session.step(event)?;
        let event_number = u64::try_from(offset).unwrap_or(u64::MAX).saturating_add(1);
        let mut requested_exit = false;
        loop {
            transition_number = transition_number.saturating_add(1);
            record_step(&mut replay, transition_number, &step)?;
            if step.changed {
                changed_count = changed_count.saturating_add(1);
            }
            requested_exit |= step.exit;
            execute_nanoseconds =
                execute_nanoseconds.saturating_add(step.observation.execute_nanoseconds);
            public_value_nanoseconds =
                public_value_nanoseconds.saturating_add(step.observation.public_value_nanoseconds);
            final_frame = step.frame;
            let Some(_) = step.action else {
                break;
            };
            if action_count as usize >= MAXIMUM_HEADLESS_ACTIONS {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "headless replay action count exceeds policy",
                ));
            }
            let outcome = request
                .outcomes
                .get(outcome_offset)
                .cloned()
                .ok_or_else(|| {
                    LkError::new(
                        ErrorCode::ProtocolMalformed,
                        "headless replay omits an outcome for an emitted host action",
                    )
                })?;
            outcome_offset = outcome_offset.saturating_add(1);
            action_count = action_count.saturating_add(1);
            step = session.resume(outcome)?;
        }
        if requested_exit {
            exit_event = Some(event_number);
        }
    }
    if outcome_offset != request.outcomes.len() {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "headless replay supplies an outcome that no host action consumed",
        ));
    }
    let final_frame_digest = frame_digest(&final_frame)?;
    Ok(HeadlessReplayReceipt {
        version: HEADLESS_REPLAY_CONTRACT_VERSION,
        application,
        event_count: request.events.len() as u64,
        action_count,
        changed_count,
        exit_event,
        initial_frame_digest,
        final_frame_digest,
        replay_digest: hex(replay.finalize().as_bytes()),
        final_frame,
        execute_nanoseconds,
        public_value_nanoseconds,
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
    if request.events.len() > MAXIMUM_HEADLESS_EVENTS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "headless replay event count exceeds policy",
        ));
    }
    if request.outcomes.len() > MAXIMUM_HEADLESS_ACTIONS {
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
