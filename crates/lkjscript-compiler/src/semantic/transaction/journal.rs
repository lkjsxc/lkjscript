use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::semantic::codec::error;
use crate::semantic::schema::{ProtocolError, ProtocolErrorCode};
use crate::semantic::transaction::StagedTransaction;

pub(super) fn transaction_id(transaction: &StagedTransaction) -> String {
    let mut bytes = transaction.tree.revision().as_bytes().to_vec();
    for source in &transaction.sources {
        bytes.extend_from_slice(source.logical_path.as_bytes());
        bytes.extend_from_slice(&lkjscript_core::sha256(&source.new_bytes));
    }
    crate::semantic::tree::hex(&lkjscript_core::sha256(&bytes))
}

pub(super) fn write(
    path: &Path,
    transaction: &StagedTransaction,
    state: &str,
) -> Result<(), ProtocolError> {
    let mut file = File::create(path).map_err(|failure| journal_error("create", failure))?;
    writeln!(
        file,
        "schema=lkjscript.publication-journal;version=1;state={state};revision={}",
        transaction.tree.revision()
    )
    .map_err(|failure| journal_error("write header", failure))?;
    for source in &transaction.sources {
        writeln!(
            file,
            "source={};old={};new={}",
            source.logical_path,
            crate::semantic::tree::hex(&lkjscript_core::sha256(&source.old_bytes)),
            crate::semantic::tree::hex(&lkjscript_core::sha256(&source.new_bytes)),
        )
        .map_err(|failure| journal_error("write source record", failure))?;
    }
    file.sync_all()
        .map_err(|failure| journal_error("flush", failure))
}

fn journal_error(action: &str, failure: std::io::Error) -> ProtocolError {
    error(
        ProtocolErrorCode::PublicationFailed,
        format!("{action} publication journal: {failure}"),
    )
}
