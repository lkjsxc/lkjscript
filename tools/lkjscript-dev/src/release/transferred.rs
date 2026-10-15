//! Finite transferred acceptance. Behavioral meaning and receipt validation stay with each owner.
mod pair;
use super::{archive, model::*, target, verifier};
use crate::{
    distributed_http, error::DevError, evidence, offline_packages, outbound_http, process,
    pure_tail, stateful_http,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const MAXIMUM_RECEIPT_BYTES: u64 = 128 * 1024 * 1024;
const MAXIMUM_OUTPUT_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Oracle {
    DistributedHttp,
    OutboundHttp,
    OfflinePackages,
    PureTail,
    StatefulHttp,
}

// The only production inventory, also used by target admission and the verifier handoff.
pub(super) const ORACLES: [Oracle; 5] = [
    Oracle::DistributedHttp,
    Oracle::OutboundHttp,
    Oracle::OfflinePackages,
    Oracle::PureTail,
    Oracle::StatefulHttp,
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ChildFacts {
    pub(super) candidate_sha256: String,
    pub(super) verifier_sha256: String,
    pub(super) elapsed_nanoseconds: u64,
    pub(super) commands: u64,
    pub(super) runners: u64,
    pub(super) requests: u64,
    pub(super) cleanup_complete: bool,
}

impl Oracle {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::DistributedHttp => "distributed-http",
            Self::OutboundHttp => "outbound-http",
            Self::OfflinePackages => "offline-packages",
            Self::PureTail => "pure-tail",
            Self::StatefulHttp => "stateful-http",
        }
    }
    pub(super) fn admission_name(self) -> &'static str {
        match self {
            Self::DistributedHttp => "distributed_http",
            Self::OutboundHttp => "outbound_http",
            Self::OfflinePackages => "offline_packages",
            Self::PureTail => "pure_tail",
            Self::StatefulHttp => "stateful_http",
        }
    }
    pub(super) fn prerequisite(self) -> &'static str {
        match self {
            Self::DistributedHttp => "none",
            Self::OutboundHttp => "http_client_adapter_1",
            Self::OfflinePackages => "code_complete_package_container_1",
            Self::PureTail => "pure-tail-execution",
            Self::StatefulHttp => "lkjscript-data-store-1",
        }
    }
    pub(super) fn timeout(self) -> Duration {
        match self {
            Self::PureTail => Duration::from_secs(900),
            Self::OfflinePackages => offline_packages::COMPLETE_PROCESS_TIMEOUT,
            _ => Duration::from_secs(1200),
        }
    }
    pub(super) fn read(
        self,
        receipt: &Path,
        candidate: &Path,
        verifier: &Path,
    ) -> Result<ChildFacts, DevError> {
        regular(receipt)?;
        Ok(match self {
            Self::DistributedHttp => {
                let r = distributed_http::read_transferred_receipt(receipt, candidate, verifier)?;
                ChildFacts {
                    candidate_sha256: r.candidate_sha256,
                    verifier_sha256: r.verifier_sha256,
                    elapsed_nanoseconds: r.elapsed_nanoseconds,
                    commands: r.commands,
                    runners: r.runners,
                    requests: r.responses,
                    cleanup_complete: r.cleanup_complete,
                }
            }
            Self::OutboundHttp => {
                let r = outbound_http::read_transferred_receipt(receipt, candidate, verifier)?;
                ChildFacts {
                    candidate_sha256: r.candidate_sha256,
                    verifier_sha256: r.verifier_sha256,
                    elapsed_nanoseconds: r.elapsed_nanoseconds,
                    commands: r.commands,
                    runners: r.runners,
                    requests: r.requests,
                    cleanup_complete: r.cleanup_complete,
                }
            }
            Self::OfflinePackages => {
                let r = offline_packages::read_transferred_receipt(receipt, candidate, verifier)?;
                ChildFacts {
                    candidate_sha256: r.candidate_sha256,
                    verifier_sha256: r.verifier_sha256,
                    elapsed_nanoseconds: r.elapsed_nanoseconds,
                    commands: u64::try_from(r.commands.len())
                        .map_err(|_| DevError::corrupt("command count overflow"))?,
                    runners: u64::try_from(r.runners.len())
                        .map_err(|_| DevError::corrupt("runner count overflow"))?,
                    requests: 1,
                    cleanup_complete: r.cleanup_complete,
                }
            }
            Self::PureTail => {
                let r = pure_tail::read_transferred_receipt(receipt, candidate, verifier)?;
                let commands = r.command_count();
                ChildFacts {
                    candidate_sha256: r.candidate_sha256,
                    verifier_sha256: r.verifier_sha256,
                    elapsed_nanoseconds: r.elapsed_nanoseconds,
                    commands,
                    runners: 1,
                    requests: 4,
                    cleanup_complete: r.cleanup_complete,
                }
            }
            Self::StatefulHttp => {
                let r = stateful_http::read_transferred_receipt(receipt, candidate, verifier)?;
                ChildFacts {
                    candidate_sha256: r.candidate_sha256,
                    verifier_sha256: r.verifier_sha256,
                    elapsed_nanoseconds: r.elapsed_nanoseconds,
                    commands: r.commands,
                    runners: 3,
                    requests: r.requests,
                    cleanup_complete: r.cleanup_complete,
                }
            }
        })
    }
}

pub(super) fn roles() -> Vec<&'static str> {
    std::iter::once("release-verify")
        .chain(ORACLES.map(Oracle::name))
        .collect()
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ReceiptIdentity {
    file: ExternalEvidence,
    digest: evidence::VerificationDigest,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    FreshPassed,
    Failed,
    NotRun,
}

impl Status {
    fn exit_code(self) -> u8 {
        u8::from(self != Self::FreshPassed)
    }
}

pub(super) fn command(mut arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let operation =
        crate::next_utf8(&mut arguments, "transferred operation")?.ok_or_else(|| {
            DevError::usage("release transferred requires pair-run, pair-verify, exact-run, exact-verify, installation-run or installation-verify")
        })?;
    if matches!(
        operation.as_str(),
        "pair-run"
            | "pair-verify"
            | "exact-run"
            | "exact-verify"
            | "installation-run"
            | "installation-verify"
    ) {
        pair::command(&operation, arguments)
    } else {
        Err(DevError::usage(format!(
            "unknown transferred operation '{operation}'"
        )))
    }
}

// The command owns this signal lifetime and joins it before returning. No signal handler runs
// application work; it only asks the bounded child supervisor to terminate its owned tree.
pub(super) struct Cancellation {
    pub(super) control: process::ProcessControl,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Cancellation {
    pub(super) fn new() -> Result<Self, DevError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()?;
        let (mut interrupt, mut terminate) = {
            let _entered = runtime.enter();
            (
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?,
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?,
            )
        };
        let control = process::ProcessControl::default();
        let signal_control = control.clone();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let thread = std::thread::Builder::new()
            .name("transfer-cancellation".to_owned())
            .spawn(move || {
                runtime.block_on(async move {
                    tokio::select! {
                        _ = interrupt.recv() => signal_control.kill(),
                        _ = terminate.recv() => signal_control.kill(),
                        _ = stopped => (),
                    }
                });
            })?;
        Ok(Self {
            control,
            stop: Some(stop),
            thread: Some(thread),
        })
    }

    pub(super) fn finish(mut self) -> Result<(), DevError> {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            thread.join().map_err(|_| {
                DevError::infrastructure(
                    "release cancellation listener failed during joined cleanup",
                )
            })?;
        }
        Ok(())
    }
}
impl Drop for Cancellation {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn external(path: &Path) -> Result<ExternalEvidence, DevError> {
    regular(path)?;
    require(
        fs::metadata(path)?.len() <= 384 * 1024 * 1024,
        "transferred input exceeds byte bound",
    )?;
    let (sha256, byte_length) = archive::sha256_file(path)?;
    Ok(ExternalEvidence {
        path: path.display().to_string(),
        byte_length,
        sha256,
    })
}
fn receipt_identity(path: &Path) -> Result<ReceiptIdentity, DevError> {
    let file = external(path)?;
    let bytes = process::read_bounded(path, MAXIMUM_RECEIPT_BYTES)?;
    Ok(ReceiptIdentity {
        file,
        digest: evidence::VerificationDigest::of(&bytes),
    })
}
fn regular(path: &Path) -> Result<(), DevError> {
    super::require_absolute_regular(path, "transferred input")?;
    require(
        path.canonicalize()? == path,
        "transferred path contains a symlink or noncanonical component",
    )
}
fn require(condition: bool, message: &str) -> Result<(), DevError> {
    if condition {
        Ok(())
    } else {
        Err(DevError::corrupt(message))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pair_interface_requires_independent_asset_inputs() {
        for operation in ["pair-run", "pair-verify"] {
            let error =
                command([OsString::from(operation)].into_iter()).expect_err("missing inputs");
            assert!(error.to_string().contains("--exact-assets"), "{error}");
        }
    }
    #[test]
    fn transferred_inventory_is_independently_specified() {
        assert_eq!(
            roles(),
            [
                "release-verify",
                "distributed-http",
                "outbound-http",
                "offline-packages",
                "pure-tail",
                "stateful-http"
            ]
        );
        assert_eq!(
            ORACLES.map(Oracle::admission_name),
            [
                "distributed_http",
                "outbound_http",
                "offline_packages",
                "pure_tail",
                "stateful_http"
            ]
        );
    }
    #[test]
    fn options_and_paths_reject_foreign_or_ambiguous_input() {
        assert!(command(["unknown"].into_iter().map(OsString::from)).is_err());
        assert!(regular(Path::new("relative")).is_err());
        let root = tempfile::tempdir().expect("root");
        let real = root.path().join("real");
        fs::write(&real, b"file").expect("file");
        std::os::unix::fs::symlink(&real, root.path().join("link")).expect("link");
        assert!(regular(&root.path().join("link")).is_err());
    }
}

#[cfg(test)]
#[path = "transferred/tests.rs"]
mod fault_tests;
