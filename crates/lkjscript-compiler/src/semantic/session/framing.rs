use std::io::{Read, Write};

use super::limits::{MAX_SESSION_CUMULATIVE_INPUT_BYTES, MAX_SESSION_CUMULATIVE_OUTPUT_BYTES};
use super::schema::{ProcessCode, SessionProcessError};
use super::SemanticSession;

pub(super) fn read_frame<R: Read>(
    reader: &mut R,
    session: &mut SemanticSession,
) -> Result<Option<Vec<u8>>, SessionProcessError> {
    let mut header = [0_u8; 8];
    let mut read = 0_usize;
    while read < header.len() {
        match reader.read(&mut header[read..]) {
            Ok(0) if read == 0 => return Ok(None),
            Ok(0) => {
                return Err(SessionProcessError::new(
                    ProcessCode::PartialHeader,
                    format!("session header ended after {read} of 8 bytes"),
                ))
            }
            Ok(count) => read += count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => {
                return Err(SessionProcessError::new(
                    ProcessCode::InputFailure,
                    format!("read session header: {error}"),
                ))
            }
        }
    }
    let encoded_length = u64::from_be_bytes(header);
    let length = usize::try_from(encoded_length).map_err(|_| {
        SessionProcessError::new(
            ProcessCode::LengthOverflow,
            format!("session frame length {encoded_length} does not fit usize"),
        )
    })?;
    if encoded_length > session.frame_input_limit() {
        return Err(SessionProcessError::new(
            ProcessCode::FrameTooLarge,
            format!(
                "session frame length {encoded_length} exceeds {}",
                session.frame_input_limit()
            ),
        ));
    }
    let total = encoded_length.checked_add(8).ok_or_else(|| {
        SessionProcessError::new(ProcessCode::LengthOverflow, "session input byte overflow")
    })?;
    let cumulative_limit = session
        .pinned
        .as_ref()
        .map_or(MAX_SESSION_CUMULATIVE_INPUT_BYTES, |pinned| {
            pinned.state.limits.cumulative_input_bytes
        });
    let next = session.input_bytes.checked_add(total).ok_or_else(|| {
        SessionProcessError::new(ProcessCode::LengthOverflow, "session input byte overflow")
    })?;
    if next > cumulative_limit {
        return Err(SessionProcessError::new(
            ProcessCode::FrameTooLarge,
            format!("session cumulative input {next} exceeds {cumulative_limit}"),
        ));
    }
    let mut payload = Vec::new();
    payload.try_reserve_exact(length).map_err(|error| {
        SessionProcessError::new(
            ProcessCode::FrameTooLarge,
            format!("reserve session payload: {error}"),
        )
    })?;
    payload.resize(length, 0);
    read_payload(reader, &mut payload)?;
    session.input_bytes = next;
    Ok(Some(payload))
}

fn read_payload<R: Read>(reader: &mut R, payload: &mut [u8]) -> Result<(), SessionProcessError> {
    let mut read = 0_usize;
    while read < payload.len() {
        match reader.read(&mut payload[read..]) {
            Ok(0) => {
                return Err(SessionProcessError::new(
                    ProcessCode::PartialPayload,
                    format!(
                        "session payload ended after {read} of {} bytes",
                        payload.len()
                    ),
                ))
            }
            Ok(count) => read += count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => {
                return Err(SessionProcessError::new(
                    ProcessCode::InputFailure,
                    format!("read session payload: {error}"),
                ))
            }
        }
    }
    Ok(())
}

pub(super) fn write_frame<W: Write>(
    writer: &mut W,
    payload: &[u8],
    session: &mut SemanticSession,
) -> Result<(), SessionProcessError> {
    let length = u64::try_from(payload.len()).map_err(|_| {
        SessionProcessError::new(
            ProcessCode::LengthOverflow,
            "session output length overflow",
        )
    })?;
    if length > session.frame_output_limit() {
        return Err(SessionProcessError::new(
            ProcessCode::FrameTooLarge,
            "session output frame exceeds selected limit",
        ));
    }
    let total = length.checked_add(8).ok_or_else(|| {
        SessionProcessError::new(ProcessCode::LengthOverflow, "session output byte overflow")
    })?;
    let cumulative_limit = session
        .pinned
        .as_ref()
        .map_or(MAX_SESSION_CUMULATIVE_OUTPUT_BYTES, |pinned| {
            pinned.state.limits.cumulative_output_bytes
        });
    let next = session.output_bytes.checked_add(total).ok_or_else(|| {
        SessionProcessError::new(ProcessCode::LengthOverflow, "session output byte overflow")
    })?;
    if next > cumulative_limit {
        return Err(SessionProcessError::new(
            ProcessCode::FrameTooLarge,
            "session cumulative output limit exceeded",
        ));
    }
    let total = usize::try_from(total).map_err(|_| {
        SessionProcessError::new(
            ProcessCode::LengthOverflow,
            "session frame does not fit usize",
        )
    })?;
    let mut frame = Vec::new();
    frame.try_reserve_exact(total).map_err(|error| {
        SessionProcessError::new(
            ProcessCode::FrameTooLarge,
            format!("reserve output: {error}"),
        )
    })?;
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(payload);
    writer
        .write_all(&frame)
        .map_err(SessionProcessError::output)?;
    session.output_bytes = next;
    session.last_response_bytes = length;
    Ok(())
}
