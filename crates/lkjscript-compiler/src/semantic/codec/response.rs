use std::io::Write;

use crate::semantic::codec::error;
use crate::semantic::schema::{ProtocolError, ProtocolErrorCode, Response};

pub(crate) struct PreparedResponse {
    pub response: Response,
    pub bytes: usize,
}

pub(crate) fn prepare_response(
    mut response: Response,
    limit: usize,
) -> Result<PreparedResponse, ProtocolError> {
    for _ in 0..4 {
        let measured = count(&response, limit)?;
        let measured_u64 = u64::try_from(measured).map_err(|_| {
            error(
                ProtocolErrorCode::OutputLimit,
                "response byte count overflow",
            )
        })?;
        if response.charges.output_bytes == measured_u64 {
            return Ok(PreparedResponse {
                response,
                bytes: measured,
            });
        }
        response.charges.output_bytes = measured_u64;
    }
    Err(error(
        ProtocolErrorCode::OutputLimit,
        "response charge did not stabilize",
    ))
}

pub(crate) fn encode_prepared(prepared: &PreparedResponse) -> Result<Vec<u8>, ProtocolError> {
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(prepared.bytes).map_err(|cause| {
        error(
            ProtocolErrorCode::OutputLimit,
            format!("reserve exact response bytes: {cause}"),
        )
    })?;
    serialize(&prepared.response, &mut bytes).map_err(|cause| {
        error(
            ProtocolErrorCode::OutputLimit,
            format!("serialize reserved response: {cause}"),
        )
    })?;
    bytes.write_all(b"\n").map_err(|cause| {
        error(
            ProtocolErrorCode::OutputLimit,
            format!("terminate reserved response: {cause}"),
        )
    })?;
    if bytes.len() != prepared.bytes {
        return Err(error(
            ProtocolErrorCode::OutputLimit,
            "reserved response size changed during encoding",
        ));
    }
    Ok(bytes)
}

fn count(response: &Response, limit: usize) -> Result<usize, ProtocolError> {
    let mut output = CountingOutput { bytes: 0, limit };
    serialize(response, &mut output).map_err(|cause| {
        error(
            ProtocolErrorCode::OutputLimit,
            format!("measure bounded response: {cause}"),
        )
    })?;
    output.write_all(b"\n").map_err(|cause| {
        error(
            ProtocolErrorCode::OutputLimit,
            format!("measure response terminator: {cause}"),
        )
    })?;
    Ok(output.bytes)
}

fn serialize(response: &Response, output: &mut impl Write) -> Result<(), serde_json::Error> {
    super::write_json(output, response)
}

struct CountingOutput {
    bytes: usize,
    limit: usize,
}

impl Write for CountingOutput {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        let attempted = self
            .bytes
            .checked_add(input.len())
            .ok_or_else(limit_error)?;
        if attempted > self.limit {
            return Err(limit_error());
        }
        self.bytes = attempted;
        Ok(input.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn limit_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::FileTooLarge,
        "response exceeds output byte limit",
    )
}
