//! Opt-in observation of immutable per-key reader-image preparation.
//! The small common carrier is an internal timing fixture, not a substitute for
//! complete-source admission, negative controls, or fresh raw C2/C3 stages.
use super::*;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::io::Read;
use std::path::Path;
use std::sync::Barrier;
use std::time::Instant;

const PREFIX: &str = "READER_CACHE_MEASUREMENT";
const SCHEMA: &str = "lkjscript.reader-cache-measurement.v1";
const ENTRIES: [&str; 3] = ["read-syntax", "read-type-scope", "read-nominal"];
const FIXTURE: &str = include_str!("../../../../tests/fixtures/selfhost-source-nominal-flow.lkjc");

struct Observation {
    round: &'static str,
    entry: &'static str,
    origin: Instant,
    events: Vec<Value>,
    lost: bool,
}

thread_local! {
    static ACTIVE: RefCell<Option<Observation>> = const { RefCell::new(None) };
}

struct Reset;

impl Drop for Reset {
    fn drop(&mut self) {
        ACTIVE.with(|slot| *slot.borrow_mut() = None);
    }
}

fn digest(bytes: &[u8]) -> String {
    crate::platform::semantic_id::encode_hex(&Sha256::digest(bytes))
}

fn file_digest(path: &Path) -> String {
    let mut file = std::fs::File::open(path).unwrap();
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let length = file.read(&mut buffer).unwrap();
        if length == 0 {
            return crate::platform::semantic_id::encode_hex(&hash.finalize());
        }
        hash.update(&buffer[..length]);
    }
}

// Raw USER_HZ ticks, deliberately not converted with an assumed frequency.
// Failure is observation unavailability and never changes compiler admission.
fn cpu(path: &str) -> Option<(u64, u64)> {
    let stat = std::fs::read_to_string(path).ok()?;
    let (_, fields) = stat.rsplit_once(')')?;
    let mut fields = fields.split_whitespace();
    let user = fields.nth(11)?.parse().ok()?;
    let system = fields.next()?.parse().ok()?;
    Some((user, system))
}

fn cpu_delta(before: Option<(u64, u64)>, after: Option<(u64, u64)>) -> Value {
    match before.zip(after) {
        Some(((before_user, before_system), (after_user, after_system))) => {
            match (
                after_user.checked_sub(before_user),
                after_system.checked_sub(before_system),
            ) {
                (Some(user), Some(system)) => {
                    json!({ "user": user.to_string(), "system": system.to_string() })
                }
                _ => Value::Null,
            }
        }
        None => Value::Null,
    }
}

pub(in super::super) struct Stamp {
    at: Instant,
    origin: Instant,
    cpu: Option<(u64, u64)>,
}

pub(in super::super) fn start() -> Option<Stamp> {
    ACTIVE.with(|slot| {
        slot.borrow().as_ref().map(|observation| {
            let at = Instant::now();
            Stamp {
                at,
                origin: observation.origin,
                cpu: None,
            }
        })
    })
}

pub(in super::super) fn start_cpu() -> Option<Stamp> {
    start().map(|mut stamp| {
        stamp.cpu = cpu("/proc/thread-self/stat");
        stamp
    })
}

// The lazy payload avoids hashes, clocks, file reads and event allocation in
// ordinary tests. Events are buffered per worker and emitted outside cache locks.
pub(in super::super) fn note(phase: &'static str, payload: impl FnOnce() -> Value) {
    ACTIVE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some(observation) = slot.as_mut() {
            if observation.events.try_reserve(1).is_err() {
                observation.lost = true;
                return;
            }
            observation.events.push(json!({
                "schema": SCHEMA,
                "event": "observation",
                "round": observation.round,
                "entry": observation.entry,
                "phase": phase,
                "thread": format!("{:?}", std::thread::current().id()),
                "detail": payload(),
            }));
        }
    });
}

pub(in super::super) fn finish(phase: &'static str, stamp: Option<Stamp>) {
    if let Some(stamp) = stamp {
        let end = Instant::now();
        let after = stamp.cpu.and_then(|_| cpu("/proc/thread-self/stat"));
        span(phase, &stamp, end, after);
    }
}

fn span(phase: &'static str, stamp: &Stamp, end: Instant, after: Option<(u64, u64)>) {
    note(phase, || {
        json!({
            "start_offset_ns": stamp.at.duration_since(stamp.origin).as_nanos().to_string(),
            "elapsed_ns": end.duration_since(stamp.at).as_nanos().to_string(),
            "calling_thread_cpu_ticks": cpu_delta(stamp.cpu, after),
            "native_child_cpu_ticks": null,
        })
    });
}

// One clock boundary closes map wait and starts map hold. The short observer
// recording performed while the guard exists belongs to the hold interval.
pub(in super::super) fn split(phase: &'static str, stamp: Option<Stamp>) -> Option<Stamp> {
    stamp.map(|stamp| {
        let at = Instant::now();
        let after = stamp.cpu.and_then(|_| cpu("/proc/thread-self/stat"));
        span(phase, &stamp, at, after);
        Stamp {
            at,
            origin: stamp.origin,
            cpu: after,
        }
    })
}

fn capture<T>(
    round: &'static str,
    entry: &'static str,
    origin: Instant,
    operation: impl FnOnce() -> T,
) -> (T, Vec<Value>) {
    ACTIVE.with(|slot| {
        assert!(slot.borrow().is_none(), "nested measurement context");
        *slot.borrow_mut() = Some(Observation {
            round,
            entry,
            origin,
            events: Vec::new(),
            lost: false,
        });
    });
    let reset = Reset;
    let result = operation();
    let observation = ACTIVE.with(|slot| slot.borrow_mut().take()).unwrap();
    drop(reset);
    assert!(
        !observation.lost,
        "measurement event allocation failed after execution"
    );
    (result, observation.events)
}

fn emit(value: &Value) {
    eprintln!("{PREFIX} {}", serde_json::to_string(value).unwrap());
}

fn one<'a>(events: &'a [Value], phase: &str) -> &'a Value {
    let mut selected = events
        .iter()
        .filter(|event| event["phase"].as_str() == Some(phase));
    let first = selected
        .next()
        .unwrap_or_else(|| panic!("missing {phase} observation"));
    assert!(selected.next().is_none(), "duplicate {phase} observation");
    &first["detail"]
}

fn trial(round: &'static str, bytes: &[u8], expected: &[u8]) -> Vec<Vec<Value>> {
    let origin = Instant::now();
    let process_cpu = cpu("/proc/self/stat");
    let barrier = Barrier::new(ENTRIES.len() + 1);
    let observations = std::thread::scope(|scope| {
        let handles = ENTRIES.map(|entry| {
            let barrier = &barrier;
            scope.spawn(move || {
                barrier.wait();
                let (_, events) = capture(round, entry, origin, || {
                    emitted_result_bytes(entry, bytes, 0, expected);
                });
                events
            })
        });
        barrier.wait();
        handles.map(|handle| handle.join().unwrap()).to_vec()
    });
    emit(&json!({
        "schema": SCHEMA,
        "event": "trial",
        "round": round,
        "worker_count": ENTRIES.len(),
        "cache_strategy": "per-key-once-lock-native-seed",
        "elapsed_ns": origin.elapsed().as_nanos().to_string(),
        "elapsed_scope": "thread creation, barrier, three existing reader calls and joins",
        "process_cpu_ticks": cpu_delta(process_cpu, cpu("/proc/self/stat")),
        "native_child_cpu_ticks": null,
    }));
    for events in &observations {
        let cache = one(events, "self-produced.cache");
        assert_eq!(cache["hit"], round == "warm");
        for phase in [
            "self-produced.map-wait",
            "self-produced.map-hold",
            "self-produced.image",
        ] {
            one(events, phase);
        }
        let builds = events
            .iter()
            .filter(|event| event["phase"] == "self-produced.build")
            .count();
        assert_eq!(builds, usize::from(round == "cold"));
        if round == "cold" {
            assert_eq!(one(events, "self-produced.projection")["profile"], 9);
            one(events, "self-produced.seed");
            let preparation = events
                .iter()
                .filter(|event| {
                    event["phase"] == "reader.native-result"
                        && event["detail"]["producer"] == "compiler-seed"
                })
                .count();
            assert_eq!(preparation, 1);
        }
        for event in events {
            emit(event);
        }
    }
    observations
}

#[test]
#[ignore = "opt-in cold/warm timing; run alone in a fresh process after nominal graph adoption"]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn selfhost_source_reader_cache_observes_three_distinct_cold_keys() {
    let executable = std::env::current_exe().unwrap();
    let executable_sha256 = file_digest(&executable);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/selfhost/project");
    let head = std::fs::read(root.join("HEAD")).unwrap();
    let repository = GraphRepository::open(&root).unwrap();
    let view = repository.view_current().unwrap();
    let revision = view.revision();
    let (source, _) = compiler();
    let complete_carrier = graph::packet(source);
    let semantic_state = kernel::semantic_state_digest(source).unwrap();
    assert_eq!(&complete_carrier[48..80], semantic_state.bytes());
    let fixture = byte_buffer_tests::author_only(FIXTURE).unwrap();
    let bytes = graph::packet(&fixture);
    let state = kernel::semantic_state_digest(&fixture).unwrap();
    assert_eq!(&bytes[48..80], state.bytes());
    let expected = state.bytes().to_vec();
    emit(&json!({
        "schema": SCHEMA,
        "event": "binding",
        "test_executable": executable,
        "test_executable_sha256": executable_sha256,
        "graph_revision": revision.to_string(),
        "public_head_sha256": digest(&head),
        "semantic_state": crate::platform::semantic_id::encode_hex(&semantic_state.bytes()),
        "source_carrier_sha256": digest(&complete_carrier),
        "source_carrier_bytes": complete_carrier.len(),
        "source_carrier_profile": crate::platform::compiler::selfhost_source_input::SOURCE_PROFILE,
        "owners": source.owners.len(),
        "types": source.types.len(),
        "retirements": source.retirements.len(),
        "fixture_sha256": digest(FIXTURE.as_bytes()),
        "fixture_carrier_sha256": digest(&bytes),
        "fixture_carrier_bytes": bytes.len(),
        "fixture_semantic_state": crate::platform::semantic_id::encode_hex(&state.bytes()),
        "entries": ENTRIES,
        "expected_mode0_profile": 9,
        "scope": "internal reader-image trial; no raw C2/C3 or full-acceptance claim",
    }));
    for entry in ENTRIES {
        // Probe graph initialization is separate from the synchronized misses.
        let origin = Instant::now();
        probes(probe_module(entry));
        emit(&json!({
            "schema": SCHEMA, "event": "probe-setup", "entry": entry,
            "elapsed_ns": origin.elapsed().as_nanos().to_string(),
        }));
        let (actual, events) = capture("prepare", entry, Instant::now(), || result(entry, &bytes));
        assert_eq!(
            actual,
            (0, expected.clone()),
            "all three preexisting reader engines"
        );
        for event in events {
            emit(&event);
        }
    }
    let cold = trial("cold", &bytes, &expected);
    for events in &cold[1..] {
        assert_eq!(
            one(&cold[0], "self-produced.seed"),
            one(events, "self-produced.seed"),
            "all reader projections use one exact native compiler seed identity",
        );
    }
    let warm = trial("warm", &bytes, &expected);
    for (cold, warm) in cold.iter().zip(&warm) {
        assert_eq!(
            one(cold, "self-produced.image"),
            one(warm, "self-produced.image")
        );
    }
    assert_eq!(std::fs::read(root.join("HEAD")).unwrap(), head);
    assert_eq!(repository.view_current().unwrap().revision(), revision);
    assert_eq!(file_digest(&executable), executable_sha256);
    emit(&json!({
        "schema": SCHEMA, "event": "terminal", "status": "passed",
        "cold_keys": ENTRIES.len(), "warm_keys": ENTRIES.len(),
        "test_executable_sha256": executable_sha256,
        "public_head_sha256": digest(&head),
        "graph_revision": revision.to_string(),
    }));
}
