//! Strict one-shot Edition 1 Semantic Source Schema V1 protocol.

mod charges;
mod codec;
mod dispatch;
mod engine;
mod operations;
mod projection;
mod response_codec;
mod schema;
pub mod session;
mod transaction;
mod tree;

#[cfg(test)]
mod tests;

use std::fmt;

use schema::ProtocolError;

pub const SCHEMA: &str = "lkjscript.semantic-source";
pub const VERSION: u32 = 1;
pub use codec::MAX_REQUEST_BYTES;

#[derive(Debug)]
pub struct SemanticProcessError(ProtocolError);

impl fmt::Display for SemanticProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_json::to_string(&self.0) {
            Ok(rendered) => formatter.write_str(&rendered),
            Err(_) => formatter.write_str("semantic protocol error could not be serialized"),
        }
    }
}

impl std::error::Error for SemanticProcessError {}

/// Decode, validate, execute, and canonically encode one complete request.
///
/// Codec rejection is a process error. Every successfully decoded request
/// yields exactly one protocol response, including semantic operation failure.
pub fn execute(input: &[u8]) -> Result<Vec<u8>, SemanticProcessError> {
    let request = codec::decode_request(input).map_err(SemanticProcessError)?;
    engine::execute_request(request, input.len()).map_err(SemanticProcessError)
}
