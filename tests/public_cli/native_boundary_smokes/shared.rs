//! One copied host serves a CPU application and an independent HTTP application.
use super::*;
use base64::Engine;
use rustix::process::{Pid, Signal, kill_process};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc;

const CONTRACT: &str = include_str!("../../../examples/parallel-work/contract.lkjc");
const CARRIERS: &str = include_str!("../../../examples/parallel-work/carriers.lkjc");
const APPLICATION: &str = include_str!("../../../examples/parallel-work/application.lkjc");
const HTTP: &str = include_str!("../../../examples/parallel-work/http.lkjc");
const SERVICE: &str = include_str!("../../../examples/parallel-work/service.deployment.json");

struct Family {
    contract: Fixture,
    carriers: Fixture,
    application: Fixture,
    contract_export: Export,
    carriers_export: Export,
}

impl Family {
    fn author() -> Self {
        let contract = Fixture::new("minimal");
        contract.author(CONTRACT);
        let contract_export = export(&contract);
        let carriers = Fixture::new("command");
        stage(&carriers, &contract_export);
        carriers.author(&substitute(CARRIERS, "CONTRACT", &contract_export));
        let carriers_export = export(&carriers);
        let application = Fixture::new("command");
        stage(&application, &contract_export);
        stage(&application, &carriers_export);
        application.author(&substitute(
            &substitute(APPLICATION, "CONTRACT", &contract_export),
            "CARRIERS",
            &carriers_export,
        ));
        Self {
            contract,
            carriers,
            application,
            contract_export,
            carriers_export,
        }
    }

    fn detach(&self) {
        for fixture in [&self.contract, &self.carriers, &self.application] {
            fixture.detach();
        }
        assert!(!self.contract_export.file.exists() && !self.carriers_export.file.exists());
    }

    fn fixtures(self) -> Vec<Fixture> {
        vec![self.contract, self.carriers, self.application]
    }
}

struct Export {
    file: PathBuf,
    package: String,
    semantic: String,
    revision: String,
    transport: String,
}

fn export(fixture: &Fixture) -> Export {
    let file = fixture.root.path().join("package.lkjp");
    let records = fixture.host(
        &[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            path(&file),
        ],
        true,
    );
    Export {
        file,
        package: field(&records, "package", "id").to_owned(),
        semantic: field(&records, "package", "revision").to_owned(),
        revision: field(&records, "package", "package-revision").to_owned(),
        transport: field(&records, "package", "transport").to_owned(),
    }
}

fn stage(fixture: &Fixture, package: &Export) {
    fixture.host(
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            &package.transport,
            "--input-file",
            path(&package.file),
        ],
        true,
    );
}

fn substitute(source: &str, prefix: &str, package: &Export) -> String {
    source
        .replace(&format!("{prefix}_PACKAGE_REVISION"), &package.revision)
        .replace(&format!("{prefix}_PACKAGE"), &package.package)
        .replace(&format!("{prefix}_REVISION"), &package.semantic)
}

/// Independent integer loops and remainder derive every byte, not only checksums.
fn expected(n: usize, rounds: usize) -> Value {
    let original = (0..n)
        .map(|index| u8::try_from(index % 256).unwrap())
        .collect::<Vec<_>>();
    let transformed = original
        .iter()
        .map(|octet| {
            let mut value = u32::from(*octet);
            for _ in 0..rounds {
                value = (37 * value + 11) % 256;
            }
            u8::try_from(value).unwrap()
        })
        .collect::<Vec<_>>();
    let checksum = transformed
        .iter()
        .enumerate()
        .map(|(index, octet)| i64::try_from(index + 1).unwrap() * i64::from(*octet))
        .sum::<i64>();
    let mut bytes = original;
    bytes.extend(transformed);
    json!({"payload":{"$bytes":base64::engine::general_purpose::STANDARD.encode(bytes)},"buffer_sum":checksum,"scalar_sum":checksum})
}

fn get(address: SocketAddr, route: &str) -> (u16, String) {
    let timeout = Duration::from_secs(120);
    let deadline = Instant::now() + timeout;
    let mut stream = TcpStream::connect_timeout(&address, timeout).unwrap();
    let request = format!(
        "GET {route} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
    );
    let mut remaining = request.as_bytes();
    while !remaining.is_empty() {
        stream
            .set_write_timeout(Some(
                deadline
                    .checked_duration_since(Instant::now())
                    .expect("absolute HTTP request deadline"),
            ))
            .unwrap();
        let count = stream.write(remaining).unwrap();
        assert!(count > 0);
        remaining = &remaining[count..];
    }
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        stream
            .set_read_timeout(Some(
                deadline
                    .checked_duration_since(Instant::now())
                    .expect("absolute HTTP request deadline"),
            ))
            .unwrap();
        let count = stream.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        assert!(
            bytes.len() + count <= 262144,
            "bounded native HTTP response"
        );
        bytes.extend_from_slice(&buffer[..count]);
    }
    let text = String::from_utf8(bytes).unwrap();
    let (headers, body) = text.split_once("\r\n\r\n").unwrap();
    assert!(
        !headers
            .to_ascii_lowercase()
            .contains("transfer-encoding: chunked")
    );
    let status = headers.split_whitespace().nth(1).unwrap().parse().unwrap();
    (status, body.to_owned())
}

fn events(bytes: &[u8], complete: bool) -> Vec<Value> {
    assert!(bytes.len() <= 2 * 1024 * 1024);
    // Only a live final suffix may be incomplete. Every complete line must parse.
    let end = if complete {
        bytes.len()
    } else {
        bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |end| end + 1)
    };
    bytes[..end]
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect()
}

pub(super) fn execute() {
    let family = Family::author();
    let application = &family.application;
    application.author(HTTP);
    let cpu_artifact = application.build("work.lkja");
    let independent = Fixture::new("http");
    independent.host(&["check"], true);
    let independent_artifact = independent.build("independent.lkja");
    let mut health: Value = serde_json::from_slice(
        &std::fs::read(independent.project.join("service.deployment.json")).unwrap(),
    )
    .unwrap();
    health["artifact"] = json!("independent.lkja");
    health["runtime"]["request_deadline_milliseconds"] = json!(120_000);
    let mut cpu: Value = serde_json::from_str(SERVICE).unwrap();
    cpu["artifact"] = json!("work.lkja");
    let cpu_descriptor = application.input("cpu.json", &cpu.to_string());
    let health_descriptor = independent.input("health.json", &health.to_string());
    let original = [sha256(&cpu_artifact), sha256(&independent_artifact)];
    family.detach();
    independent.detach();

    let mut smoke = Smoke::new();
    let mut running = smoke.spawn(&[
        "serve",
        "--deployment",
        path(&cpu_descriptor),
        "--deployment",
        path(&health_descriptor),
    ]);
    let deadline = Instant::now() + Duration::from_secs(60);
    let ready = loop {
        if let Some(ready) = events(&std::fs::read(&running.stdout).unwrap(), false)
            .into_iter()
            .find(|event| event["event"] == "ready")
        {
            break ready;
        }
        let status = running.child.try_wait().unwrap();
        assert!(
            status.is_none(),
            "shared candidate failed before readiness: {status:?}; stdout={}; stderr={}",
            String::from_utf8_lossy(&std::fs::read(&running.stdout).unwrap()),
            String::from_utf8_lossy(&std::fs::read(&running.stderr).unwrap())
        );
        assert!(
            Instant::now() < deadline,
            "shared candidate readiness deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(ready["process_id"], json!(running.child.id()));
    assert_eq!(ready["instances"].as_array().unwrap().len(), 2);
    assert_eq!(ready["shared_runtime"]["instances"], 2);
    assert_eq!(
        ready["shared_runtime"]["programs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    #[cfg(target_os = "linux")]
    assert!(
        std::fs::read_to_string(format!(
            "/proc/{}/task/{}/children",
            running.child.id(),
            running.child.id()
        ))
        .unwrap()
        .trim()
        .is_empty(),
        "one shared host owns both instances without application children"
    );
    let address = |index: usize| {
        ready["instances"][index]["local_address"]
            .as_str()
            .unwrap()
            .parse::<SocketAddr>()
            .unwrap()
    };
    let cpu_address = address(0);
    let health_address = address(1);
    assert_ne!(cpu_address, health_address);
    assert_eq!(
        get(health_address, "/"),
        (200, "hello from lkjscript".to_owned())
    );
    let expected = expected(4096, 32);
    assert_eq!(expected["buffer_sum"], 1_069_109_248);
    std::thread::scope(|scope| {
        // Each blocked client has its own abortable start gate. If any later
        // native thread cannot start, unwinding drops all senders and releases
        // every earlier client before the scope joins it.
        let mut starts = Vec::new();
        let mut requests = Vec::new();
        for _ in 0..4 {
            let (start, ready) = mpsc::sync_channel(1);
            starts.push(start);
            let expected = &expected;
            requests.push(scope.spawn(move || {
                if ready.recv().is_err() {
                    return;
                }
                let (status, body) = get(cpu_address, "/work");
                assert_eq!(status, 200);
                assert_eq!(&serde_json::from_str::<Value>(&body).unwrap(), expected);
            }));
        }
        for start in starts {
            start.send(()).unwrap();
        }
        for _ in 0..4 {
            assert_eq!(
                get(health_address, "/"),
                (200, "hello from lkjscript".to_owned())
            );
        }
        // Join every owned client before inspecting its assertion result.
        let joined = requests
            .into_iter()
            .map(|request| request.join())
            .collect::<Vec<_>>();
        for request in joined {
            request.unwrap();
        }
    });
    for _ in 0..2 {
        let (status, body) = get(cpu_address, "/work");
        assert_eq!(status, 200);
        assert_eq!(serde_json::from_str::<Value>(&body).unwrap(), expected);
    }
    assert_eq!(
        get(health_address, "/"),
        (200, "hello from lkjscript".to_owned())
    );
    let pid = Pid::from_raw(i32::try_from(running.child.id()).unwrap()).unwrap();
    kill_process(pid, Signal::TERM).unwrap();
    let output = smoke.join(running, Duration::from_secs(20));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stderr.is_empty());
    let events = events(&output.stdout, true);
    assert_eq!(
        events
            .iter()
            .filter(|event| event["event"] == "ready")
            .count(),
        1
    );
    let stopped = events
        .iter()
        .filter(|event| event["event"] == "stopped")
        .collect::<Vec<_>>();
    assert_eq!(stopped.len(), 1);
    let stopped = stopped[0];
    assert_eq!(stopped["instances"].as_array().unwrap().len(), 2);
    let executor = &stopped["shared_runtime"]["executor"];
    assert_eq!(ready["shared_runtime"]["executor"]["dispatch_open"], true);
    for (name, expected) in [
        ("dispatch_open", json!(false)),
        ("active_dispatches", json!(0)),
        ("remaining_workers", json!(0)),
        ("workers_received", json!(0)),
        ("workers_handed_off", json!(0)),
    ] {
        assert_eq!(executor[name], expected, "{name}");
    }
    assert_eq!(executor["joined_workers"], executor["workers_started"]);
    let started = executor["workers_started"].as_u64().unwrap();
    if started > 0 {
        assert!(
            executor["completed_dispatches"].as_u64().unwrap() > started,
            "joined sequential requests reuse physical workers"
        );
    } else {
        assert_eq!(executor["completed_dispatches"], 0);
        assert!(executor["inline_fallbacks"].as_u64().unwrap() > 0);
    }
    for instance in stopped["instances"].as_array().unwrap() {
        let receipt = &instance["receipt"];
        assert_eq!(receipt["shutdown"]["admission_stopped"], true);
        assert_eq!(receipt["shutdown"]["remaining_tasks"], 0);
        assert_eq!(receipt["shutdown"]["cleanup_failures"], json!([]));
        for name in ["admission_permits", "worker_permits"] {
            assert_eq!(receipt["runtime"][name], 0);
        }
        for name in ["active", "queued"] {
            assert_eq!(receipt["runtime"]["resident"][name], 0);
        }
    }
    assert_eq!(
        [sha256(&cpu_artifact), sha256(&independent_artifact)],
        original
    );
    for fixture in [
        &family.contract,
        &family.carriers,
        &family.application,
        &independent,
    ] {
        fixture.assert_detached();
    }
    let mut fixtures = family.fixtures();
    fixtures.push(independent);
    smoke.finish("shared_parallel_http", fixtures);
}

const FOREGROUND: &str = r#"
    (type-alias WorkResult (record (payload Bytes) (buffer_sum I64) (scalar_sum I64)))
    (type-alias Result (record (work WorkResult) (view (record (length I64) (first I64))) (source Bytes)))
    (function create main (visibility private) (effect (task)) (returns Result)
      (body (let
        (binding work (type WorkResult) (call work::parallel (i64 5) (i64 3) (i64 0)))
        (binding source (type ByteBuffer) (call work-carriers::produce-octets (i64 3) (i64 7)))
        (binding view (as $foreground-view-scope)
          (type (read-view-application Probe (types) (lexical-read-scope main $foreground-view-scope)))
          (borrow-call (call make (scopes (lexical-read-scope main $foreground-view-scope)) (local source))
            (binding constructed (type (read-view-application Probe (types) (lexical-read-scope main $foreground-view-scope))))
            (in (view-move (local constructed)))))
        (binding observed (type (record (length I64) (first I64)))
          (call observe (scopes (lexical-read-scope main $foreground-view-scope)) (local view)))
        (binding payload (type Bytes) (sequence (view-drop (local view)) (call std::buffer-freeze (local source))))
        (in (record structural (field work (local work)) (field view (local observed)) (field source (local payload)))))))
    (component create command (visibility private)
      (port create main (type (task-function () Result (row))) (function main))))
  (target create boundary (component boundary::command) (runner command) (port boundary::command::main))
"#;

pub(super) fn foreground() {
    let family = Family::author();
    let application = &family.application;
    let source = substitute(
        &format!(
            "declarations.begin\n(units (use std builtin) (use work-carriers CARRIERS_PACKAGE CARRIERS_PACKAGE_REVISION) (module create boundary\n{VIEW}{FOREGROUND})\ndeclarations.end\n"
        ),
        "CARRIERS",
        &family.carriers_export,
    );
    application.author(&source);
    let artifact = application.build("foreground.lkja");
    let mut descriptor: Value = serde_json::from_slice(
        &std::fs::read(application.project.join("command.deployment.json")).unwrap(),
    )
    .unwrap();
    descriptor["artifact"] = json!("foreground.lkja");
    descriptor["target"] = json!("boundary");
    let descriptor = application.input("foreground.json", &descriptor.to_string());
    let result = application.root.path().join("foreground-result.json");
    let digest = sha256(&artifact);
    let descriptor_bytes = std::fs::read(&descriptor).unwrap();
    family.detach();
    let mut smoke = Smoke::new();
    let records = smoke.call(
        &[
            "run",
            "--deployment",
            path(&descriptor),
            "--result-file",
            path(&result),
        ],
        true,
    );
    assert_eq!(field(&records, "execution", "target"), "boundary");
    for (name, expected) in [
        ("execution-mode", "production"),
        ("verification", "not-performed"),
        ("execution-profile", "trusted-foreground"),
        ("instruction-limit", "absent"),
        ("allocation-limit", "absent"),
        ("collection-limit", "absent"),
        ("capability-call-limit", "absent"),
        ("deadline-milliseconds", "absent"),
    ] {
        assert_eq!(field(&records, "execution", name), expected);
    }
    assert!(
        !records
            .iter()
            .any(|record| matches!(record.operation.as_str(), "cache" | "compiler"))
    );
    let work = expected(5, 3);
    assert_eq!(work["buffer_sum"], 1635);
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(result).unwrap()).unwrap(),
        json!({"work":work,"view":{"length":3,"first":7},"source":{"$bytes":"BwgJ"}})
    );
    clean_execution(&records);
    let production: Value =
        serde_json::from_str(field(&records, "execution", "production-observation")).unwrap();
    assert_eq!(production["parallel_scopes"], 1);
    assert_eq!(production["capability_calls"], 0);
    assert_eq!(sha256(&artifact), digest);
    assert_eq!(std::fs::read(descriptor).unwrap(), descriptor_bytes);
    for fixture in [&family.contract, &family.carriers, &family.application] {
        fixture.assert_detached();
    }
    smoke.finish("detached_foreground", family.fixtures());
}
