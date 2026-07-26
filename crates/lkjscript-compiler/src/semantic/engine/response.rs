use lkjscript_core::BudgetLedger;

use crate::semantic::codec::{self, error};
use crate::semantic::schema::{
    Charges, ProtocolError, ProtocolErrorCode, Response, ResponseResult,
};
use crate::semantic::{SCHEMA, VERSION};

use super::EngineOutcome;

pub(super) fn prepare(
    mut response: Response,
    publication: Option<crate::semantic::transaction::StagedTransaction>,
    guard: Option<crate::semantic::transaction::PublicationGuard>,
    ledger: &mut BudgetLedger,
) -> Result<EngineOutcome, ProtocolError> {
    let budget = match &mut response.result {
        ResponseResult::Error { error, .. } => error.budget.take(),
        _ => None,
    };
    let prepared = codec::prepare_response(response, ledger)?;
    Ok(EngineOutcome {
        prepared,
        publication,
        guard,
        budget,
    })
}

pub(super) fn error_response(
    profile: crate::semantic::schema::ResourceProfile,
    revision: Option<String>,
    charges: Charges,
    failure: ProtocolError,
    ledger: &BudgetLedger,
) -> Response {
    Response {
        result: ResponseResult::Error {
            error: Box::new(failure),
            diagnostic: None,
        },
        ..base_response(profile, revision, charges, ledger)
    }
}

pub(super) fn base_response(
    profile: crate::semantic::schema::ResourceProfile,
    revision: Option<String>,
    charges: Charges,
    ledger: &BudgetLedger,
) -> Response {
    Response {
        schema: SCHEMA.to_string(),
        version: VERSION,
        compiler_build: format!("lkjscript-compiler-{}", env!("CARGO_PKG_VERSION")),
        profile,
        profile_identity: crate::semantic::charges::identity_core(ledger.profile()),
        limits: crate::semantic::charges::ProtocolLimits::for_core(ledger.profile()).record(),
        revision,
        charges,
        result: ResponseResult::Error {
            error: Box::new(error(
                ProtocolErrorCode::ValidationFailed,
                "uninitialized response",
            )),
            diagnostic: None,
        },
    }
}
