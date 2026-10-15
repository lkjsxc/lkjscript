//! One owned public process; bounded readiness and joined shutdown evidence.
use super::*;
use rustix::process::{Pid, Signal, kill_process};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

pub(in super::super) struct Group {
    owner: Option<Owner>,
    output: PathBuf,
    errors: PathBuf,
    pub ready: Value,
}

enum Owner {
    Process(support::SpawnedChild),
    Host(super::super::native_http::resident::Resident),
}

fn events(path: &Path) -> Vec<Value> {
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.len() <= 2 * 1024 * 1024);
    bytes
        .split(|byte| *byte == b'\n')
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect()
}

impl Group {
    pub fn start(public: &Native, descriptors: &[Value], environment: &[(&str, &str)]) -> Self {
        Self::start_in(public, descriptors, environment, false)
    }

    /// PID and signal witnesses explicitly retain an actual process owner.
    pub fn start_process(
        public: &Native,
        descriptors: &[Value],
        environment: &[(&str, &str)],
    ) -> Self {
        Self::start_in(public, descriptors, environment, true)
    }

    fn start_in(
        public: &Native,
        descriptors: &[Value],
        environment: &[(&str, &str)],
        force_process: bool,
    ) -> Self {
        let output = public.root.path().join("shared.stdout");
        let errors = public.root.path().join("shared.stderr");
        let mut paths = Vec::new();
        let mut command = Command::new(&public.executable);
        command.arg("serve");
        for (index, descriptor) in descriptors.iter().enumerate() {
            let file = public.input(&format!("shared-{index}.json"), &descriptor.to_string());
            command.arg("--deployment").arg(&file);
            paths.push(file);
        }
        if !force_process
            && public.mode == crate::semantic_cli::Mode::HostSemantic
            && super::super::native_http::resident::eligible(&paths, environment)
        {
            let host = super::super::native_http::resident::Resident::start(
                paths,
                true,
                output.clone(),
                errors.clone(),
                Duration::from_secs(60),
                Duration::from_secs(20),
            );
            let ready = host.ready.clone();
            assert!(ready.get("process_id").is_none());
            assert_eq!(
                ready["instances"].as_array().unwrap().len(),
                descriptors.len()
            );
            return Self {
                owner: Some(Owner::Host(host)),
                output,
                errors,
                ready,
            };
        }
        let mut child = support::spawn(
            command
                .current_dir(public.root.path())
                .env_clear()
                .env("PATH", "")
                .envs(environment.iter().copied())
                .stdin(Stdio::null())
                .stdout(File::create(&output).unwrap())
                .stderr(File::create(&errors).unwrap()),
        )
        .unwrap();
        let until = Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(ready) = events(&output)
                .into_iter()
                .find(|event| event["event"] == "ready")
            {
                assert_eq!(ready["process_id"], json!(child.id()));
                assert_eq!(
                    ready["instances"].as_array().unwrap().len(),
                    descriptors.len()
                );
                #[cfg(target_os = "linux")]
                assert!(
                    std::fs::read_to_string(format!(
                        "/proc/{}/task/{}/children",
                        child.id(),
                        child.id()
                    ))
                    .unwrap()
                    .trim()
                    .is_empty(),
                    "shared host did not launch application processes"
                );
                return Self {
                    owner: Some(Owner::Process(child)),
                    output,
                    errors,
                    ready,
                };
            }
            assert!(
                child.try_wait().unwrap().is_none(),
                "failed before ready: {} / {}",
                String::from_utf8_lossy(&std::fs::read(&output).unwrap()),
                String::from_utf8_lossy(&std::fs::read(&errors).unwrap())
            );
            assert!(Instant::now() < until, "group readiness deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn address(&self, index: usize) -> SocketAddr {
        self.ready["instances"][index]["local_address"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap()
    }

    /// Host semantic teardown uses cooperative cancellation. A process remainder
    /// still receives the original explicitly selected operating-system signal.
    pub fn shutdown(mut self, process_signal: Signal) -> Value {
        match self.owner.take().unwrap() {
            Owner::Host(host) => {
                let actual = host.stop();
                let observed = self.terminal(true);
                assert_eq!(actual, observed);
                observed
            }
            Owner::Process(child) => {
                self.owner = Some(Owner::Process(child));
                self.stop(process_signal)
            }
        }
    }

    pub fn stop(mut self, signal: Signal) -> Value {
        let process = match self.owner.take().unwrap() {
            Owner::Process(child) => Some(child),
            Owner::Host(host) => {
                drop(host);
                None
            }
        };
        let mut child = process.expect("signal teardown requires an actual shared process owner");
        let pid = Pid::from_raw(i32::try_from(child.id()).unwrap()).unwrap();
        kill_process(pid, signal).unwrap();
        let until = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "group stop failed: {status}; {:?}; {}",
                    events(&self.output),
                    String::from_utf8_lossy(&std::fs::read(&self.errors).unwrap())
                );
                break;
            }
            assert!(Instant::now() < until, "group shutdown deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(child.wait_with_output().unwrap().status.success());
        self.terminal(false)
    }

    fn terminal(&self, host: bool) -> Value {
        assert!(std::fs::read(&self.errors).unwrap().is_empty());
        let events = events(&self.output);
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
        let mut ready_runtime = self.ready["shared_runtime"].clone();
        let mut stopped_runtime = stopped["shared_runtime"].clone();
        let ready_executor = ready_runtime
            .as_object_mut()
            .unwrap()
            .remove("executor")
            .unwrap();
        let stopped_executor = stopped_runtime
            .as_object_mut()
            .unwrap()
            .remove("executor")
            .unwrap();
        assert_eq!(stopped_runtime, ready_runtime);
        assert_eq!(ready_executor["dispatch_open"], true);
        assert_eq!(stopped_executor["dispatch_open"], false);
        assert_eq!(stopped_executor["active_dispatches"], 0);
        assert_eq!(stopped_executor["remaining_workers"], 0);
        if host {
            super::super::native_http::resident::check_executor(&stopped_executor).unwrap();
        } else {
            // The fixed CLI group has one executor owner in its isolated process.
            assert_eq!(stopped_executor["workers_received"], 0);
            assert_eq!(stopped_executor["workers_handed_off"], 0);
            assert_eq!(
                stopped_executor["joined_workers"],
                stopped_executor["workers_started"]
            );
        }
        for instance in stopped["instances"].as_array().unwrap() {
            let receipt = &instance["receipt"];
            assert_eq!(receipt["shutdown"]["admission_stopped"], true);
            assert_eq!(receipt["shutdown"]["remaining_tasks"], 0);
            assert_eq!(receipt["shutdown"]["cleanup_failures"], json!([]));
            if !receipt["runtime"].is_null() {
                for field in ["admission_permits", "worker_permits"] {
                    assert_eq!(receipt["runtime"][field], 0);
                }
                assert_eq!(receipt["runtime"]["resident"]["active"], 0);
                assert_eq!(receipt["runtime"]["resident"]["queued"], 0);
            }
            if !receipt["sessions"].is_null() {
                assert_eq!(receipt["sessions"]["active_sessions"], 0);
                assert_eq!(receipt["sessions"]["pending_handshakes"], 0);
            }
        }
        stopped.clone()
    }
}
