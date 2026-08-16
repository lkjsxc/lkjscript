#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use lkjscript::daemon;
use lkjscript::machine::{self, BoundaryErrorEnvelope};
use lkjscript::transaction::{
    ApplyTransactionRequest, Transaction, TransactionMode, TransactionOp, TransactionResponseSpec,
};
use lkjscript::{Client, DraftSymbol, IdempotencyKey, Request, RequestId, Response, Revision};
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct RunningDaemon {
    child: Child,
    state: PathBuf,
}

impl RunningDaemon {
    fn start(state: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_lkjscriptd"))
            .args([
                "--state",
                state.to_str().expect("UTF-8 state path"),
                "--foreground",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn foreground daemon");
        let endpoint = daemon::endpoint_path(state);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !endpoint.exists() {
            if let Some(status) = child.try_wait().expect("daemon status") {
                panic!("daemon exited before readiness: {status}");
            }
            assert!(Instant::now() < deadline, "daemon readiness timeout");
            thread::sleep(Duration::from_millis(1));
        }
        Self {
            child,
            state: state.to_owned(),
        }
    }

    fn client(&self) -> Client {
        Client::new(daemon::endpoint_path(&self.state))
    }

    fn shutdown(mut self, request_id: u64) {
        assert_eq!(
            self.client()
                .request(RequestId::new(request_id), &Request::Shutdown)
                .expect("shutdown response"),
            Response::Acknowledged
        );
        assert!(self.child.wait().expect("wait daemon").success());
    }
}

impl Drop for RunningDaemon {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn frame(body: &[u8]) -> Vec<u8> {
    let mut bytes = u32::try_from(body.len())
        .expect("frame body length")
        .to_le_bytes()
        .to_vec();
    bytes.extend_from_slice(body);
    bytes
}

fn raw_exchange(state: &Path, bytes: &[u8]) -> Vec<u8> {
    let mut stream = UnixStream::connect(daemon::endpoint_path(state)).expect("connect raw client");
    stream.write_all(bytes).expect("write raw request");
    stream
        .shutdown(Shutdown::Write)
        .expect("half-close raw request");
    let mut response = Vec::new();
    match stream.read_to_end(&mut response) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::ConnectionReset && !response.is_empty() => {}
        Err(error) => panic!("read raw response: {error}"),
    }
    response
}

fn response_body(frame: &[u8]) -> &[u8] {
    assert!(frame.len() >= 4, "response frame header");
    let length = usize::try_from(u32::from_le_bytes(
        frame[..4].try_into().expect("response length"),
    ))
    .expect("response length usize");
    assert_eq!(frame.len(), length + 4, "one exact response frame");
    &frame[4..]
}

fn workspace_directory_count(state: &Path) -> usize {
    fs::read_dir(state.join("workspaces"))
        .expect("workspaces directory")
        .count()
}

#[test]
fn real_daemon_framing_rejects_before_dispatch_and_remains_usable() {
    let temporary = tempfile::tempdir().expect("state directory");
    let daemon = RunningDaemon::start(temporary.path());
    let request_body = machine::encode_request(RequestId::new(11), &Request::CreateWorkspace)
        .expect("create request JSON");
    let valid_frame = frame(&request_body);

    let mut truncated_body = valid_frame.clone();
    truncated_body.pop();
    let mut trailing_byte = valid_frame.clone();
    trailing_byte.push(0);
    let mut second_frame = valid_frame.clone();
    second_frame.extend_from_slice(&valid_frame);
    let cases = [
        ("truncated header", vec![1, 0]),
        ("truncated body", truncated_body),
        (
            "oversized length-only prefix",
            u32::try_from(machine::MAX_JSON_INPUT_BYTES + 1)
                .expect("oversized request length")
                .to_le_bytes()
                .to_vec(),
        ),
        ("trailing byte", trailing_byte),
        ("second frame", second_frame),
    ];

    for (name, bytes) in cases {
        let response = raw_exchange(temporary.path(), &bytes);
        let boundary: BoundaryErrorEnvelope =
            serde_json::from_slice(response_body(&response)).expect("boundary error envelope");
        assert_eq!(boundary.version, machine::JSON_ENVELOPE_VERSION, "{name}");
        assert_eq!(boundary.request_id, None, "{name}");
        assert_eq!(workspace_directory_count(temporary.path()), 0, "{name}");
    }

    let Response::WorkspaceCreated(created) = daemon
        .client()
        .request(RequestId::new(12), &Request::CreateWorkspace)
        .expect("valid request after malformed frames")
    else {
        panic!("workspace response");
    };
    assert_eq!(workspace_directory_count(temporary.path()), 1);
    let head = fs::read(
        temporary
            .path()
            .join("workspaces")
            .join(created.workspace.to_string())
            .join("HEAD"),
    )
    .expect("workspace HEAD");
    assert_eq!(&head[..8], b"LKJHEAD5");
    daemon.shutdown(13);
}

#[test]
fn dropped_keyed_commit_response_replays_exact_receipt_and_daemon_remains_usable() {
    let temporary = tempfile::tempdir().expect("state directory");
    let daemon = RunningDaemon::start(temporary.path());
    let Response::WorkspaceCreated(created) = daemon
        .client()
        .request(RequestId::new(21), &Request::CreateWorkspace)
        .expect("create workspace")
    else {
        panic!("workspace response");
    };
    let workspace = created.workspace;
    let operation = TransactionOp::CreatePackage {
        symbol: DraftSymbol::new("s1"),
        name: "replayed".into(),
    };
    let response = TransactionResponseSpec {
        return_symbols: vec![DraftSymbol::new("s1")],
    };
    let predicted_request = ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::ValidateOnly,
            operations: vec![operation.clone()],
        },
        response: response.clone(),
    };
    let Response::TransactionReceipt(mut expected) = daemon
        .client()
        .request(
            RequestId::new(22),
            &Request::ApplyTransaction(predicted_request),
        )
        .expect("validate-only prediction")
    else {
        panic!("prediction receipt");
    };
    expected.published = true;

    let key = IdempotencyKey::from_bytes([0x5a; 16]);
    let commit = ApplyTransactionRequest {
        transaction: Transaction {
            workspace,
            base_revision: Revision::INITIAL,
            idempotency_key: Some(key),
            mode: TransactionMode::Commit,
            operations: vec![operation],
        },
        response,
    };
    let request = Request::ApplyTransaction(commit.clone());
    let body = machine::encode_request(RequestId::new(23), &request).expect("keyed commit JSON");
    let mut dropped = UnixStream::connect(daemon::endpoint_path(temporary.path()))
        .expect("connect dropped-response client");
    dropped
        .write_all(&frame(&body))
        .expect("write keyed commit");
    dropped
        .shutdown(Shutdown::Write)
        .expect("half-close commit");
    drop(dropped);

    let Response::TransactionReceipt(replayed) = daemon
        .client()
        .request(
            RequestId::new(23),
            &Request::ApplyTransaction(commit.clone()),
        )
        .expect("exact keyed replay")
    else {
        panic!("replay receipt");
    };
    assert_eq!(replayed, expected);
    let Response::TransactionReceipt(repeated) = daemon
        .client()
        .request(RequestId::new(24), &Request::ApplyTransaction(commit))
        .expect("repeated keyed replay")
    else {
        panic!("repeated replay receipt");
    };
    assert_eq!(repeated, replayed);

    let Response::WorkspaceCreated(_) = daemon
        .client()
        .request(RequestId::new(25), &Request::CreateWorkspace)
        .expect("daemon usability after dropped response")
    else {
        panic!("second workspace response");
    };
    daemon.shutdown(26);
}
