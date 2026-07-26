//! Strict one-shot Semantic Source protocol for the current exact contract.

mod budget;
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

use lkjscript_core::{BudgetError, BudgetLedger};
use schema::{ProtocolError, ProtocolErrorCode};

pub const SCHEMA: &str = "lkjscript.semantic-source";
pub const CONTRACT: lkjscript_contracts::ContractDigest =
    lkjscript_contracts::SEMANTIC_SOURCE_DIGEST;
pub use codec::MAX_REQUEST_BYTES;

#[derive(Debug)]
pub struct SemanticProcessError {
    protocol: ProtocolError,
    budget: Option<Box<BudgetError>>,
}

impl SemanticProcessError {
    fn from_protocol(mut protocol: ProtocolError) -> Self {
        Self {
            budget: protocol.budget.take(),
            protocol,
        }
    }

    pub fn budget_error(&self) -> Option<&BudgetError> {
        self.budget.as_deref()
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

/// Encoded current-contract response plus an internal typed budget failure, if any.
pub struct SemanticExecution {
    response: Vec<u8>,
    budget: Option<Box<BudgetError>>,
}

impl SemanticExecution {
    pub fn response(&self) -> &[u8] {
        &self.response
    }

    pub fn budget_error(&self) -> Option<&BudgetError> {
        self.budget.as_deref()
    }

    pub fn into_response(self) -> Vec<u8> {
        self.response
    }
}

/// Decode, execute, and encode one request through one caller-owned ledger.
pub fn execute_with_ledger(
    input: &[u8],
    ledger: &mut BudgetLedger,
) -> Result<SemanticExecution, SemanticProcessError> {
    let selected = codec::decode_profile(input).map_err(SemanticProcessError::from_protocol)?;
    if !budget::profile_matches(selected, ledger) {
        return Err(SemanticProcessError::from_protocol(codec::error(
            ProtocolErrorCode::ResourceLimit,
            "request profile does not match outer-owned ledger profile",
        )));
    }
    let request = codec::decode_request_with_ledger(input, ledger)
        .map_err(SemanticProcessError::from_protocol)?;
    let mut outcome = engine::execute_request_with_ledger(request, input.len(), ledger)
        .map_err(SemanticProcessError::from_protocol)?;
    let response =
        codec::encode_prepared(&outcome.prepared).map_err(SemanticProcessError::from_protocol)?;
    publish_outcome(&mut outcome).map_err(SemanticProcessError::from_protocol)?;
    Ok(SemanticExecution {
        response,
        budget: outcome.budget,
    })
}

/// Decode, validate, execute, and canonically encode one complete request.
pub fn execute(input: &[u8]) -> Result<Vec<u8>, SemanticProcessError> {
    let selected = codec::decode_profile(input).map_err(SemanticProcessError::from_protocol)?;
    let mut ledger = BudgetLedger::new(selected.core());
    execute_with_ledger(input, &mut ledger).map(SemanticExecution::into_response)
}

pub(crate) fn publish_outcome(outcome: &mut engine::EngineOutcome) -> Result<(), ProtocolError> {
    if let Some(publication) = outcome.publication.take() {
        let root = publication.tree.root_path().to_path_buf();
        transaction::publish(&publication, &root)?;
    }
    outcome.guard.take();
    Ok(())
}
