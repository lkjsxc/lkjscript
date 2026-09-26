//! Real signals against copied, source-free HTTP, session and worker processes.
use super::*;
use rustix::process::{Pid, Signal, kill_process};
use serde_json::json;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

const SOURCE: &str = include_str!("../fixtures/resident-termination.lkjc");

struct Resident {
    child: support::SpawnedChild,
    output: PathBuf,
    errors: PathBuf,
    ready: Value,
}

fn events(path: &Path) -> Vec<Value> {
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.len() <= 1024 * 1024, "bounded resident output");
    bytes
        .split(|byte| *byte == b'\n')
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect()
}

impl Resident {
    fn start(public: &Native, command: &str, descriptor: &Value) -> Self {
        let deployment = public.input("selected.json", &descriptor.to_string());
        let output = public.root.path().join("resident.stdout");
        let errors = public.root.path().join("resident.stderr");
        let mut child = support::spawn(
            Command::new(&public.executable)
                .args([command, "--deployment", path(&deployment)])
                .current_dir(public.root.path())
                .env_clear()
                .env("PATH", "")
                .stdin(Stdio::null())
                .stdout(File::create(&output).unwrap())
                .stderr(File::create(&errors).unwrap()),
        )
        .unwrap();
        let until = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(ready) = events(&output)
                .into_iter()
                .find(|value| value["event"] == "ready")
            {
                return Self {
                    child,
                    output,
                    errors,
                    ready,
                };
            }
            assert!(
                child.try_wait().unwrap().is_none(),
                "resident failed before readiness: {} / {}",
                String::from_utf8_lossy(&std::fs::read(&output).unwrap()),
                String::from_utf8_lossy(&std::fs::read(&errors).unwrap())
            );
            assert!(Instant::now() < until, "resident readiness deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn address(&self) -> SocketAddr {
        self.ready["local_address"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap()
    }

    fn stop(mut self, signal: Signal) -> Value {
        // Signal only our still-owned, unreaped direct child; never a process-name match.
        let pid = Pid::from_raw(i32::try_from(self.child.id()).unwrap()).unwrap();
        kill_process(pid, signal).unwrap();
        let until = Instant::now() + Duration::from_secs(15);
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < until, "resident shutdown deadline");
            std::thread::sleep(Duration::from_millis(5));
        };
        let joined = self.child.wait_with_output().unwrap();
        let records = events(&self.output);
        let errors = std::fs::read(&self.errors).unwrap();
        assert!(
            status.success() && joined.status.success(),
            "signal {signal:?} bypassed joined termination: {status}; events={records:?}; stderr={}",
            String::from_utf8_lossy(&errors)
        );
        assert!(errors.is_empty(), "{}", String::from_utf8_lossy(&errors));
        let stopped = records
            .iter()
            .filter(|value| value["event"] == "stopped")
            .collect::<Vec<_>>();
        assert_eq!(stopped.len(), 1, "exactly one stopped event: {records:?}");
        let receipt = stopped[0]["receipt"].clone();
        assert_eq!(receipt["shutdown"]["admission_stopped"], true);
        assert_eq!(receipt["shutdown"]["remaining_tasks"], 0);
        assert_eq!(receipt["shutdown"]["cleanup_failures"], json!([]));
        if self.ready["deployment"]["runner"] == "http" {
            let runtime = &receipt["runtime"];
            assert_eq!(runtime["resident"]["active"], 0);
            assert_eq!(runtime["resident"]["queued"], 0);
            assert_eq!(runtime["admission_permits"], 0);
            assert_eq!(runtime["worker_permits"], 0);
        }
        if self.ready["deployment"]["runner"] == "interactive" {
            assert_eq!(receipt["sessions"]["active_sessions"], 0);
            assert_eq!(receipt["sessions"]["pending_handshakes"], 0);
        }
        eprintln!("joined {signal:?}: {receipt}");
        receipt
    }
}

fn author(runner: &str) -> (Native, Value) {
    author_source(runner, SOURCE)
}

fn author_source(runner: &str, source: &str) -> (Native, Value) {
    let public = Native::template("http");
    let input = public.input(
        "termination.lkjc",
        &source.replacen("base=BASE", &format!("base={}", public.revision()), 1),
    );
    let plan = public.plan(&input, true);
    public.apply(&input, &plan, true);
    public.cli(&["check"], true);
    let artifact = public.root.path().join("termination.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let mut descriptor: Value = serde_json::from_slice(
        &std::fs::read(public.project.join("service.deployment.json")).unwrap(),
    )
    .unwrap();
    descriptor["artifact"] = json!("termination.lkja");
    descriptor["target"] = json!(format!("termination-{runner}"));
    descriptor["listen"] = json!("127.0.0.1:0");
    descriptor["runtime"]["maximum_concurrent_tasks"] = json!(1);
    descriptor["runtime"]["maximum_queued_tasks"] = json!(0);
    // Independent cancellation must beat this deadline without reducing execution quotas.
    descriptor["runtime"]["request_deadline_milliseconds"] = json!(60_000);
    descriptor["runtime"]["shutdown_grace_milliseconds"] = json!(100);
    descriptor["runtime"]["cancellation_grace_milliseconds"] = json!(2_000);
    if runner == "worker" {
        descriptor["listen"] = Value::Null;
        descriptor["http"] = Value::Null;
        descriptor["worker"] = json!({"maximum_workers":1,"idle_wait_milliseconds":60_000});
    } else if runner == "live" {
        descriptor["http"] = Value::Null;
        descriptor["session"] = json!({
            "maximum_active_sessions":2,"maximum_pending_handshakes":2,
            "maximum_message_bytes":4096,"maximum_frame_bytes":4096,
            "maximum_header_bytes":8192,"maximum_headers":32,
            "maximum_inbound_mailbox_items":2,"maximum_inbound_mailbox_bytes":8192,
            "maximum_outbound_mailbox_items":8,"maximum_outbound_mailbox_bytes":8192,
            "maximum_state_nodes":1024,"maximum_state_bytes":4096,
            "maximum_transition_messages":8,"maximum_transition_bytes":8192,
            "tick_interval_milliseconds":1000,"idle_timeout_milliseconds":60_000,
            "maximum_lifetime_milliseconds":86_400_000,"close_grace_milliseconds":100,
            "cancellation_grace_milliseconds":2000,"maximum_process_buffer_bytes":1_048_576
        });
    }
    // These are owned, source-only fixtures; there is no operational data in this graph.
    std::fs::remove_dir_all(&public.project).unwrap();
    std::fs::remove_file(input).unwrap();
    (public, descriptor)
}

fn idle(runner: &str, signal: Signal) {
    let (public, descriptor) = author(runner);
    let command = if runner == "worker" {
        "worker"
    } else {
        "serve"
    };
    // Send immediately after the actual readiness event, without a handler-installation delay.
    Resident::start(&public, command, &descriptor).stop(signal);
}

#[test]
fn resident_http_sigterm_joins_after_ready() {
    idle("http", Signal::TERM);
}
#[test]
fn resident_http_sigint_joins_after_ready() {
    idle("http", Signal::INT);
}
#[test]
fn resident_worker_sigterm_joins_after_ready() {
    idle("worker", Signal::TERM);
}
#[test]
fn resident_worker_sigint_joins_after_ready() {
    idle("worker", Signal::INT);
}
#[test]
fn resident_session_sigterm_joins_after_ready() {
    idle("live", Signal::TERM);
}
#[test]
fn resident_session_sigint_joins_after_ready() {
    idle("live", Signal::INT);
}

fn busy_http(signal: Signal) {
    let (public, descriptor) = author("http");
    let resident = Resident::start(&public, "serve", &descriptor);
    let mut busy = TcpStream::connect_timeout(&resident.address(), Duration::from_secs(5)).unwrap();
    busy.set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    busy.write_all(b"GET /busy HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .unwrap();
    // Do not race the busy request's initial admission with our health probe.
    // A reply here is a failed fixture, not evidence that work is active.
    busy.set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let mut first_bytes = [0_u8; 2048];
    let first = busy.read(&mut first_bytes);
    assert!(
        matches!(&first, Err(error) if matches!(error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut)),
        "busy request completed before cancellation: {first:?}: {}",
        String::from_utf8_lossy(&first_bytes[..first.as_ref().copied().unwrap_or(0)])
    );
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let reply = native_http::send(
            resident.address(),
            "GET",
            "/health",
            &[("Host", "localhost")],
            "",
        );
        if reply.status == 503 {
            assert!(
                reply
                    .headers
                    .contains("x-lkjscript-failure-code: resident_overloaded")
            );
            break;
        }
        assert_eq!(reply.status, 200);
        assert!(
            Instant::now() < until,
            "busy request never occupied the sole worker"
        );
    }
    let receipt = resident.stop(signal);
    assert!(
        receipt["runtime"]["resident"]["cancelled"]
            .as_u64()
            .unwrap()
            >= 1
    );
    drop(busy);
}

#[test]
fn resident_http_sigterm_cancels_active_work() {
    busy_http(Signal::TERM);
}
#[test]
fn resident_http_sigint_cancels_active_work() {
    busy_http(Signal::INT);
}

fn connect_peer(resident: &Resident) -> TcpStream {
    let mut connection =
        TcpStream::connect_timeout(&resident.address(), Duration::from_secs(5)).unwrap();
    connection
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    connection
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    connection.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").unwrap();
    let mut reader = BufReader::new(&mut connection);
    let mut response = String::new();
    loop {
        let mut line = String::new();
        assert_ne!(reader.read_line(&mut line).unwrap(), 0);
        response.push_str(&line);
        assert!(response.len() < 8192);
        if line == "\r\n" {
            break;
        }
    }
    assert!(response.starts_with("HTTP/1.1 101 "), "{response}");
    assert!(
        response
            .to_ascii_lowercase()
            .contains("sec-websocket-accept: s3pplmbitxaq9kygzzhzrbk+xoo=")
    );
    connection
}

fn connected_session(signal: Signal) {
    let (public, descriptor) = author("live");
    let resident = Resident::start(&public, "serve", &descriptor);
    let connection = connect_peer(&resident);
    // Keep the peer connected: shutdown, not client EOF, must close and join the session.
    let receipt = resident.stop(signal);
    assert_eq!(receipt["sessions"]["admitted_sessions"], 1);
    assert_eq!(receipt["sessions"]["completed_sessions"], 1);
    assert_eq!(receipt["sessions"]["failed_sessions"], 0);
    drop(connection);
}

#[test]
fn resident_session_sigterm_joins_connected_peer() {
    connected_session(Signal::TERM);
}
#[test]
fn resident_session_sigint_joins_connected_peer() {
    connected_session(Signal::INT);
}

fn pending_open(signal: Signal) {
    // The literal public proposal deliberately never completes its open callback.
    let accept = "(variant std::SessionDecisionKind::accept)";
    assert_eq!(SOURCE.matches(accept).count(), 1);
    let source = SOURCE.replacen(
        accept,
        &format!("(if (call std::i64-equal (call spin) (i64 0)) {accept} {accept})"),
        1,
    );
    let (public, mut descriptor) = author_source("live", &source);
    descriptor["session"]["maximum_pending_handshakes"] = json!(1);
    let resident = Resident::start(&public, "serve", &descriptor);
    let request = b"GET / HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n";
    let mut pending =
        TcpStream::connect_timeout(&resident.address(), Duration::from_secs(5)).unwrap();
    pending
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    pending.write_all(request).unwrap();
    pending
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let mut byte = [0];
    assert!(
        matches!(pending.read(&mut byte), Err(error) if matches!(error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut))
    );
    // A second valid upgrade independently witnesses the occupied handshake slot.
    let mut probe =
        TcpStream::connect_timeout(&resident.address(), Duration::from_secs(5)).unwrap();
    probe
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    probe
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    probe.write_all(request).unwrap();
    let mut line = String::new();
    BufReader::new(&mut probe).read_line(&mut line).unwrap();
    assert!(line.starts_with("HTTP/1.1 503 "), "{line}");
    drop(probe);
    let receipt = resident.stop(signal);
    assert_eq!(receipt["sessions"]["admitted_sessions"], 0);
    assert_eq!(receipt["sessions"]["rejected_handshakes"], 1);
    assert_eq!(receipt["sessions"]["active_sessions"], 0);
    assert_eq!(receipt["sessions"]["pending_handshakes"], 0);
    drop(pending);
}

#[test]
fn resident_session_sigterm_cancels_pending_open() {
    pending_open(Signal::TERM);
}

#[test]
fn resident_session_sigint_cancels_pending_open() {
    pending_open(Signal::INT);
}

fn active_callback(signal: Signal) {
    let message = "(arm std::SessionEvent::message (payload message Message)\n          (call decision (variant std::SessionDecisionKind::continue) (local state)))";
    assert_eq!(SOURCE.matches(message).count(), 1);
    let source = SOURCE.replacen(message,
        "(arm std::SessionEvent::message (payload message Message)\n          (call transition (local state) (local event)))", 1);
    let (public, descriptor) = author_source("live", &source);
    let resident = Resident::start(&public, "serve", &descriptor);
    let mut connection = connect_peer(&resident);
    // One valid masked text frame containing 'a', independently of production codecs.
    connection
        .write_all(&[0x81, 0x81, 1, 2, 3, 4, b'a' ^ 1])
        .unwrap();
    connection
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    assert!(
        matches!(connection.read(&mut [0]), Err(error) if matches!(error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut))
    );
    let mut probe =
        TcpStream::connect_timeout(&resident.address(), Duration::from_secs(5)).unwrap();
    probe
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    probe
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    probe.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").unwrap();
    let mut reader = BufReader::new(&mut probe);
    let mut response = String::new();
    loop {
        let mut line = String::new();
        assert_ne!(reader.read_line(&mut line).unwrap(), 0);
        response.push_str(&line);
        assert!(response.len() < 8192);
        if line == "\r\n" {
            break;
        }
    }
    assert!(response.starts_with("HTTP/1.1 500 "), "{response}");
    assert!(
        response
            .to_ascii_lowercase()
            .contains("x-lkjscript-failure-code: resident_overloaded"),
        "{response}"
    );
    drop(probe);
    // Callback cancellation must be followed by the parent's actual joined teardown.
    let receipt = resident.stop(signal);
    assert_eq!(receipt["shutdown"]["cancellation_requested"], 1);
    assert_eq!(receipt["sessions"]["inbound_messages"], 1);
    assert_eq!(receipt["sessions"]["admitted_sessions"], 1);
    assert_eq!(receipt["sessions"]["completed_sessions"], 0);
    assert_eq!(receipt["sessions"]["failed_sessions"], 1);
    drop(connection);
}

#[test]
fn resident_session_sigterm_joins_cancelled_callback_parent() {
    active_callback(Signal::TERM);
}

#[test]
fn resident_session_sigint_joins_cancelled_callback_parent() {
    active_callback(Signal::INT);
}
