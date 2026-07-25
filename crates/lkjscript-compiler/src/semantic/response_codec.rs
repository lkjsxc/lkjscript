use std::io::Write;

use crate::semantic::codec::error;
use crate::semantic::schema::{ProtocolError, ProtocolErrorCode, Response};

pub(crate) fn encode_response(mut response: Response) -> Result<Vec<u8>, ProtocolError> {
    let limit = super::charges::ProtocolLimits::for_profile(response.profile).response_bytes;
    for _ in 0..4 {
        let mut output = BoundedOutput {
            bytes: Vec::new(),
            limit,
        };
        serde_json::to_writer(&mut output, &response).map_err(|cause| {
            error(
                ProtocolErrorCode::OutputLimit,
                format!("serialize bounded response: {cause}"),
            )
        })?;
        output.write_all(b"\n").map_err(|cause| {
            error(
                ProtocolErrorCode::OutputLimit,
                format!("terminate bounded response: {cause}"),
            )
        })?;
        let measured = u64::try_from(output.bytes.len()).map_err(|_| {
            error(
                ProtocolErrorCode::OutputLimit,
                "response byte count overflow",
            )
        })?;
        if response.charges.output_bytes == measured {
            return Ok(output.bytes);
        }
        response.charges.output_bytes = measured;
    }
    Err(error(
        ProtocolErrorCode::OutputLimit,
        "response charge did not stabilize",
    ))
}

struct BoundedOutput {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for BoundedOutput {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        let attempted = self
            .bytes
            .len()
            .checked_add(input.len())
            .ok_or_else(limit_error)?;
        if attempted > self.limit {
            return Err(limit_error());
        }
        self.bytes.extend_from_slice(input);
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
