//! Copied-product resident quotas, operational cancellation and detached recovery.
use super::*;
use serde_json::json;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

const SOURCE: &str = include_str!("../fixtures/resident-policy.lkjc");
const QUOTAS: [&str; 4] = [
    "instruction_fuel",
    "maximum_allocated_bytes",
    "maximum_collection_items",
    "maximum_capability_calls",
];

struct Server {
    child: support::SpawnedChild,
    output: PathBuf,
    errors: PathBuf,
    address: SocketAddr,
}

fn events(path: &Path) -> Vec<Value> {
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.len() <= 1024 * 1024, "bounded fixture server output");
    bytes
        .split(|byte| *byte == b'\n')
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect()
}

impl Server {
    fn start(public: &Native, label: &str, descriptor: &Value) -> Self {
        let deployment = public.input(&format!("{label}.json"), &descriptor.to_string());
        let output = public.root.path().join(format!("{label}.stdout"));
        let errors = public.root.path().join(format!("{label}.stderr"));
        let mut child = support::spawn(
            Command::new(&public.executable)
                .args(["serve", "--deployment", path(&deployment)])
                .current_dir(public.root.path())
                .env_clear()
                .env("PATH", "")
                .stdin(Stdio::null())
                .stdout(File::create(&output).unwrap())
                .stderr(File::create(&errors).unwrap()),
        )
        .unwrap();
        let until = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(event) = events(&output)
                .into_iter()
                .find(|event| event["event"] == "ready")
            {
                let address = event["local_address"].as_str().unwrap().parse().unwrap();
                return Self {
                    child,
                    output,
                    errors,
                    address,
                };
            }
            assert!(
                child.try_wait().unwrap().is_none(),
                "server failed before ready: {} / {}",
                String::from_utf8_lossy(&std::fs::read(&output).unwrap()),
                String::from_utf8_lossy(&std::fs::read(&errors).unwrap())
            );
            assert!(Instant::now() < until, "server readiness timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn get(&self, path: &str) -> (u16, String, String) {
        let timeout = Duration::from_secs(120);
        let mut connection = TcpStream::connect_timeout(&self.address, timeout).unwrap();
        connection.set_read_timeout(Some(timeout)).unwrap();
        connection.set_write_timeout(Some(timeout)).unwrap();
        write!(
            connection,
            "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut bytes = Vec::new();
        connection.take(65537).read_to_end(&mut bytes).unwrap();
        assert!(bytes.len() <= 65536, "bounded fixture HTTP response");
        let text = String::from_utf8(bytes).unwrap();
        let (headers, body) = text.split_once("\r\n\r\n").unwrap();
        let status = headers.split_whitespace().nth(1).unwrap().parse().unwrap();
        assert!(
            !headers
                .to_ascii_lowercase()
                .contains("transfer-encoding: chunked")
        );
        (status, headers.to_ascii_lowercase(), body.to_owned())
    }

    fn expect(&self, path: &str, status: u16, body: &str) {
        let response = self.get(path);
        assert_eq!(
            (response.0, response.2.as_str()),
            (status, body),
            "{path}: {response:?}"
        );
    }

    fn stop(mut self) -> Value {
        // The direct child remains owned and unreaped until after signaling.
        let pid = rustix::process::Pid::from_raw(i32::try_from(self.child.id()).unwrap()).unwrap();
        rustix::process::kill_process(pid, rustix::process::Signal::INT).unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "unjoined server: {status}");
                break;
            }
            assert!(Instant::now() < until, "server shutdown timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(self.child.wait_with_output().unwrap().status.success());
        assert!(std::fs::read(&self.errors).unwrap().is_empty());
        let stopped = events(&self.output)
            .into_iter()
            .find(|event| event["event"] == "stopped")
            .unwrap();
        assert_eq!(stopped["receipt"]["shutdown"]["remaining_tasks"], 0);
        assert_eq!(stopped["receipt"]["runtime"]["resident"]["active"], 0);
        assert_eq!(stopped["receipt"]["runtime"]["resident"]["queued"], 0);
        stopped
    }
}

fn author() -> (Native, Value) {
    let public = Native::template("http");
    let input = public.input(
        "work.lkjc",
        &SOURCE.replacen("base=BASE", &format!("base={}", public.revision()), 1),
    );
    let plan = public.plan(&input, true);
    public.apply(&input, &plan, true);
    let check = public.cli(&["check"], true);
    assert_eq!(
        compact_field(compact_record(&check, "tests"), "differential"),
        "equal"
    );
    let mut descriptor: Value = serde_json::from_slice(
        &std::fs::read(public.project.join("service.deployment.json")).unwrap(),
    )
    .unwrap();
    for name in QUOTAS {
        assert_eq!(descriptor["execution"].get(name), Some(&Value::Null));
    }
    descriptor["artifact"] = json!("work.lkja");
    descriptor["target"] = json!("web");
    // This cost witness also runs with a debug executable in the workspace suite.
    // Choose a test-local operational deadline; the recipe's 30-second default is unchanged.
    descriptor["runtime"]["request_deadline_milliseconds"] = json!(90_000);
    // Copy only the existing authority revision; create a separately named exact clock grant.
    let authority = descriptor["grants"][0]["authority_revision"].clone();
    descriptor["grants"].as_array_mut().unwrap().push(json!({"requirement":"clock",
        "sharing_domain":"policy-clock", "authority_revision":authority, "adapter":{"kind":"wall_clock"}}));
    public.cli(
        &[
            "build",
            "--output",
            path(&public.root.path().join("work.lkja")),
        ],
        true,
    );
    (public, descriptor)
}

#[test]
fn detached_resident_quotas_remain_optional_and_operational_deadlines_still_join() {
    let (public, descriptor) = author();
    let mut command = descriptor.clone();
    command["target"] = json!("count");
    for name in ["listen", "http", "session", "worker"] {
        command[name] = Value::Null;
    }
    command["grants"] = json!([]);
    for name in ["execution", "runtime"] {
        command.as_object_mut().unwrap().remove(name);
    }
    let command = public.input("count.json", &command.to_string());
    // Remove only fixture-owned authoring inputs. Runtime has no project dependency.
    std::fs::remove_dir_all(&public.project).unwrap();
    std::fs::remove_file(public.root.path().join("work.lkjc")).unwrap();
    let output = public.cli(
        &[
            "run",
            "--deployment",
            path(&command),
            "--arguments",
            "[1000000]",
        ],
        true,
    );
    let execution = compact_record(&output, "execution");
    assert_eq!(compact_field(execution, "value"), "1000000");
    let work: Value =
        serde_json::from_str(compact_field(execution, "production-observation")).unwrap();
    assert!(
        work["instructions"].as_u64().unwrap() > 10_000_000,
        "fixture must cross the old instruction quota: {work}"
    );

    assert!(work["allocated_bytes"].as_u64().unwrap() > 268_435_456);
    eprintln!("resident quota work witness: {work}");
    let server = Server::start(&public, "unmetered", &descriptor);
    server.expect("/large", 200, "1000000");
    server.expect("/collections", 200, "100");
    let clock = server.get("/clock");
    assert_eq!(clock.0, 200);
    assert!(matches!(clock.2.as_str(), "1" | "2"));
    server.expect("/health", 200, "7");
    server.stop();

    for (field, maximum, route, code) in [
        (
            "instruction_fuel",
            10_000_000,
            "/large",
            "normalized_instruction_steps",
        ),
        (
            "maximum_allocated_bytes",
            1_000_000,
            "/large",
            "normalized_allocation",
        ),
        (
            "maximum_collection_items",
            64,
            "/collections",
            "normalized_collection_items",
        ),
        (
            "maximum_capability_calls",
            1,
            "/clock",
            "normalized_capability_calls",
        ),
    ] {
        let mut bounded = descriptor.clone();
        bounded["execution"][field] = json!(maximum);
        let server = Server::start(&public, field, &bounded);
        let failed = server.get(route);
        assert_eq!(failed.0, 500, "{field}: {failed:?}");
        assert!(
            failed.1.contains("x-lkjscript-failure-class: resource"),
            "{field}: {failed:?}"
        );
        assert!(
            failed
                .1
                .contains(&format!("x-lkjscript-failure-code: {code}")),
            "{field}: {failed:?}"
        );
        server.expect("/health", 200, "7");
        server.stop();
    }
    let mut timed = descriptor.clone();
    timed["runtime"]["maximum_concurrent_tasks"] = json!(1);
    timed["runtime"]["request_deadline_milliseconds"] = json!(100);
    let server = Server::start(&public, "deadline", &timed);
    let failed = server.get("/deadline");
    assert_eq!(failed.0, 503, "{failed:?}");
    assert!(failed.1.contains("x-lkjscript-failure-class: cancelled"));
    server.expect("/health", 200, "7");
    let stopped = server.stop();
    assert!(
        stopped["receipt"]["runtime"]["resident"]["cancelled"]
            .as_u64()
            .unwrap()
            >= 1
    );
}

#[test]
fn invalid_resident_quota_configuration_rejects_before_artifact_secret_or_listener_access() {
    let public = Native::template("http");
    let mut descriptor: Value = serde_json::from_slice(
        &std::fs::read(public.project.join("service.deployment.json")).unwrap(),
    )
    .unwrap();
    descriptor["artifact"] = json!("absent.lkja");
    descriptor["secrets"] =
        json!([{"name":"must-not-read", "variable":"LKJSCRIPT_ABSENT_RESIDENT_POLICY_SECRET"}]);
    let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    descriptor["listen"] = json!(occupied.local_addr().unwrap().to_string());
    for (index, field) in QUOTAS.iter().enumerate() {
        let mut invalid = descriptor.clone();
        invalid["execution"][field] = json!(0);
        let path = public.input(&format!("invalid-{index}.json"), &invalid.to_string());
        let result = command_at(
            &public.executable,
            public.root.path(),
            &["serve", "--deployment", super::path(&path)],
        );
        assert!(!result.status.success());
        let output = String::from_utf8(result.stdout).unwrap();
        assert!(output.contains("deployment_execution_limit"), "{output}");
        assert!(!output.contains("\"event\":\"ready\""));
        assert!(!public.root.path().join("absent.lkja").exists());
    }
    for field in [
        "instruction_fuel",
        "maximum_call_depth",
        "maximum_value_stack",
    ] {
        let mut invalid = descriptor.clone();
        invalid["execution"].as_object_mut().unwrap().remove(field);
        let path = public.input(&format!("missing-{field}.json"), &invalid.to_string());
        let result = command_at(
            &public.executable,
            public.root.path(),
            &["serve", "--deployment", super::path(&path)],
        );
        assert!(!result.status.success());
        assert!(
            String::from_utf8(result.stdout)
                .unwrap()
                .contains("deployment_json")
        );
    }
}

#[test]
fn foreground_policy_reporting_does_not_hide_partial_cumulative_limits() {
    let public = Native::template("command");
    let mut descriptor: Value = serde_json::from_slice(
        &std::fs::read(public.project.join("command.deployment.json")).unwrap(),
    )
    .unwrap();
    descriptor["artifact"] = json!("hello.lkja");
    public.cli(
        &[
            "build",
            "--output",
            path(&public.root.path().join("hello.lkja")),
        ],
        true,
    );
    descriptor["execution"] = json!({"instruction_fuel":null,"maximum_call_depth":4096,
        "maximum_value_stack":1000000,"maximum_allocated_bytes":null,
        "maximum_collection_items":null,"maximum_capability_calls":null});
    for selection in 0..=4 {
        let mut selected = descriptor.clone();
        if selection < QUOTAS.len() {
            selected["execution"][QUOTAS[selection]] = json!(1000000);
        }
        let deployment = public.input(&format!("profile-{selection}.json"), &selected.to_string());
        let records = public.cli(&["run", "--deployment", path(&deployment)], true);
        let execution = compact_record(&records, "execution");
        assert_eq!(
            compact_field(execution, "execution-profile"),
            if selection == QUOTAS.len() {
                "trusted-foreground"
            } else {
                "bounded"
            }
        );
    }
    let discovery = command_at(
        &public.executable,
        public.root.path(),
        &["capabilities", "--section", "deployment"],
    );
    assert!(discovery.status.success());
    let records = parse_records("deployment discovery", &discovery.stdout).unwrap();
    let quotas = compact_record(&records, "deployment.cumulative-quotas");
    assert_eq!(
        compact_field(quotas, "new-http-recipe"),
        "four-explicit-nulls"
    );
    for name in QUOTAS {
        let expected = format!("execution.{name}");
        let field = records
            .iter()
            .find(|record| {
                record.operation == "deployment.field"
                    && super::super::compact_field(record, "path") == Some(expected.as_str())
            })
            .unwrap();
        assert_eq!(compact_field(field, "scalar"), "null|u64");
    }
}
