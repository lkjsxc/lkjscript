//! Strict one-shot Semantic Source protocol for the current exact contract.

mod charges;
mod codec;
mod dispatch;
mod engine;
mod operations;
mod projection;
pub(crate) mod schema;
pub mod session;
pub(crate) mod transaction;
pub(crate) mod tree;

#[cfg(test)]
mod tests;

use std::fmt;

use schema::ProtocolError;

pub const SCHEMA: &str = "lkjscript.semantic-source";
pub const CONTRACT: lkjscript_contracts::ContractDigest =
    lkjscript_contracts::SEMANTIC_SOURCE_DIGEST;
pub use codec::MAX_REQUEST_BYTES;

#[derive(Debug)]
pub struct SemanticProcessError {
    protocol: ProtocolError,
}

impl SemanticProcessError {
    fn from_protocol(protocol: ProtocolError) -> Self {
        Self { protocol }
    }
}

impl fmt::Display for SemanticProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_json::to_string(&self.protocol) {
            Ok(rendered) => formatter.write_str(&rendered),
            Err(_) => formatter.write_str("semantic protocol error could not be serialized"),
        }
    }
}

impl std::error::Error for SemanticProcessError {}

/// Decode, validate, execute, and canonically encode one complete request.
pub fn execute(input: &[u8]) -> Result<Vec<u8>, SemanticProcessError> {
    let request = codec::decode_request(input).map_err(SemanticProcessError::from_protocol)?;
    let mut outcome = engine::execute_request(request, input.len())
        .map_err(SemanticProcessError::from_protocol)?;
    let response =
        codec::encode_prepared(&outcome.prepared).map_err(SemanticProcessError::from_protocol)?;
    publish_outcome(&mut outcome).map_err(SemanticProcessError::from_protocol)?;
    Ok(response)
}

pub(crate) fn publish_outcome(outcome: &mut engine::EngineOutcome) -> Result<(), ProtocolError> {
    if let Some(publication) = outcome.publication.take() {
        let root = publication.tree.root_path().to_path_buf();
        transaction::publish(&publication, &root)?;
    }
    outcome.guard.take();
    Ok(())
}
