//! Bounded local stdio session over the current Semantic Source contract.

mod encoding;
mod engine;
mod execute;
mod framing;
mod lifecycle;
mod limits;
mod pending;
mod publication;
mod reservation;
mod schema;
mod source;

use std::io::{Read, Write};

use lkjscript_core::{BudgetAuthority, BudgetCause, BudgetError, BudgetLedger, ResourceCategory};
use schema::{PinnedSession, SessionProcessError};

pub use limits::{
    MAX_SESSION_CUMULATIVE_INPUT_BYTES, MAX_SESSION_CUMULATIVE_OUTPUT_BYTES,
    MAX_SESSION_FRAME_BYTES, MAX_SESSION_LIFETIME_FUEL, MAX_SESSION_REQUESTS,
    MAX_SESSION_RETAINED_METADATA_BYTES, MAX_SESSION_REVISION,
};

pub const SCHEMA: &str = "lkjscript.semantic-session";
pub const CONTRACT: lkjscript_contracts::ContractDigest =
    lkjscript_contracts::AGENT_PROTOCOL_DIGEST;

/// Compiler-owned state for one local semantic session.
pub struct SemanticSession {
    pinned: Option<PinnedSession>,
    revision: u64,
    requests: u64,
    input_bytes: u64,
    output_bytes: u64,
    lifetime_fuel: u64,
    last_response_bytes: u64,
    ledger: Option<BudgetLedger>,
    pending: Option<pending::PendingExecution>,
    closed: bool,
}

impl SemanticSession {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pinned: None,
            revision: 0,
            requests: 0,
            input_bytes: 0,
            output_bytes: 0,
            lifetime_fuel: 0,
            last_response_bytes: 0,
            ledger: None,
            pending: None,
            closed: false,
        }
    }

    #[must_use]
    pub const fn with_ledger(ledger: BudgetLedger) -> Self {
        Self {
            pinned: None,
            revision: 0,
            requests: 0,
            input_bytes: 0,
            output_bytes: 0,
            lifetime_fuel: 0,
            last_response_bytes: 0,
            ledger: Some(ledger),
            pending: None,
            closed: false,
        }
    }

    pub fn serve<R: Read, W: Write>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<(), SessionProcessError> {
        while !self.closed {
            let Some(payload) = framing::read_frame(reader, self)? else {
                break;
            };
            let response = self.handle(&payload)?;
            framing::write_frame(writer, &response, self)?;
        }
        writer.flush().map_err(SessionProcessError::output)
    }

    fn frame_input_limit(&self) -> u64 {
        self.pinned
            .as_ref()
            .map_or(MAX_SESSION_FRAME_BYTES, |pinned| {
                pinned.state.limits.frame_input_bytes
            })
    }

    fn frame_output_limit(&self) -> u64 {
        self.pinned
            .as_ref()
            .map_or(MAX_SESSION_FRAME_BYTES, |pinned| {
                pinned.state.limits.frame_output_bytes
            })
    }
}

#[allow(
    clippy::result_large_err,
    reason = "session preflight preserves the fixed nonallocating budget prefix"
)]
pub(super) fn reserve_session(
    ledger: &mut BudgetLedger,
    category: ResourceCategory,
    amount: u64,
    cause: BudgetCause,
) -> Result<(), BudgetError> {
    super::budget::reserve(
        ledger,
        BudgetAuthority::SessionCaching,
        category,
        amount,
        cause,
    )
}

impl Default for SemanticSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Serve framed requests until clean EOF or a shutdown acknowledgement.
pub fn serve<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> Result<(), SessionProcessError> {
    SemanticSession::new().serve(reader, writer)
}

pub fn serve_with_ledger<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    ledger: BudgetLedger,
) -> Result<(), SessionProcessError> {
    SemanticSession::with_ledger(ledger).serve(reader, writer)
}

#[cfg(test)]
mod tests;
