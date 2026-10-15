//! Independent bounded HTTP/1.1 fixture; it imports no production HTTP codec.
use super::*;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

#[path = "native_resident.rs"]
pub(super) mod resident;

pub(super) struct Reply {
    pub status: u16,
    pub headers: String,
    pub body: String,
}

pub(super) struct Server {
    owner: Owner,
    output: PathBuf,
    errors: PathBuf,
    pub address: SocketAddr,
}

enum Owner {
    Process(support::SpawnedChild),
    Host(resident::Resident),
}

fn events(path: &Path) -> Vec<Value> {
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.len() <= 2 * 1024 * 1024);
    bytes
        .split(|byte| *byte == b'\n')
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect()
}

impl Server {
    pub fn start(
        public: &Native,
        label: &str,
        descriptor: &Value,
        environment: &[(&str, &str)],
    ) -> Self {
        let deployment = public.input(&format!("{label}.json"), &descriptor.to_string());
        Self::start_file(public, label, &deployment, environment)
    }

    pub fn start_file(
        public: &Native,
        label: &str,
        deployment: &Path,
        environment: &[(&str, &str)],
    ) -> Self {
        let output = public.root.path().join(format!("{label}.stdout"));
        let errors = public.root.path().join(format!("{label}.stderr"));
        if public.mode == crate::semantic_cli::Mode::HostSemantic
            && resident::eligible(&[deployment.to_path_buf()], environment)
        {
            let host = resident::Resident::start(
                vec![deployment.to_path_buf()],
                false,
                output.clone(),
                errors.clone(),
                Duration::from_secs(30),
                Duration::from_secs(15),
            );
            let address = host.ready["local_address"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            return Self {
                owner: Owner::Host(host),
                output,
                errors,
                address,
            };
        }
        let mut child = support::spawn(
            Command::new(&public.executable)
                .args(["serve", "--deployment", path(deployment)])
                .current_dir(public.root.path())
                .env_clear()
                .env("PATH", "")
                .envs(environment.iter().copied())
                .stdin(Stdio::null())
                .stdout(File::create(&output).unwrap())
                .stderr(File::create(&errors).unwrap()),
        )
        .unwrap();
        let until = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(ready) = events(&output)
                .into_iter()
                .find(|event| event["event"] == "ready")
            {
                let address = ready["local_address"].as_str().unwrap().parse().unwrap();
                return Self {
                    owner: Owner::Process(child),
                    output,
                    errors,
                    address,
                };
            }
            assert!(
                child.try_wait().unwrap().is_none(),
                "native HTTP server failed before readiness: {} / {}",
                String::from_utf8_lossy(&std::fs::read(&output).unwrap()),
                String::from_utf8_lossy(&std::fs::read(&errors).unwrap())
            );
            assert!(Instant::now() < until, "native HTTP readiness deadline");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn stop(self) {
        match self.owner {
            Owner::Host(host) => Self::checked_stop(host.stop(), true),
            Owner::Process(child) => Self::stop_process(
                child,
                &self.output,
                &self.errors,
                rustix::process::Signal::INT,
            ),
        }
    }

    pub fn stop_with_signal(self, signal: rustix::process::Signal) {
        let process = match self.owner {
            Owner::Process(child) => Some(child),
            Owner::Host(host) => {
                drop(host); // Join actual owned resources before failing this boundary assertion.
                None
            }
        };
        let child = process.expect("stop_with_signal requires an actual process owner");
        Self::stop_process(child, &self.output, &self.errors, signal);
    }

    fn stop_process(
        mut child: support::SpawnedChild,
        output: &Path,
        errors: &Path,
        signal: rustix::process::Signal,
    ) {
        let pid = rustix::process::Pid::from_raw(i32::try_from(child.id()).unwrap()).unwrap();
        rustix::process::kill_process(pid, signal).unwrap();
        let until = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "native HTTP server failed to join: {status}"
                );
                break;
            }
            assert!(Instant::now() < until, "native HTTP shutdown deadline");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(child.wait_with_output().unwrap().status.success());
        assert!(std::fs::read(errors).unwrap().is_empty());
        let stopped = events(output)
            .into_iter()
            .find(|event| event["event"] == "stopped")
            .unwrap();
        Self::checked_stop(stopped, false);
    }

    fn checked_stop(stopped: Value, host: bool) {
        assert_eq!(stopped["receipt"]["shutdown"]["admission_stopped"], true);
        assert_eq!(stopped["receipt"]["shutdown"]["remaining_tasks"], 0);
        assert_eq!(
            stopped["receipt"]["shutdown"]["cleanup_failures"],
            serde_json::json!([])
        );
        assert_eq!(stopped["receipt"]["runtime"]["admission_permits"], 0);
        assert_eq!(stopped["receipt"]["runtime"]["worker_permits"], 0);
        assert_eq!(stopped["receipt"]["runtime"]["resident"]["active"], 0);
        assert_eq!(stopped["receipt"]["runtime"]["resident"]["queued"], 0);
        assert_eq!(stopped["executor"]["dispatch_open"], false);
        assert_eq!(stopped["executor"]["active_dispatches"], 0);
        assert_eq!(stopped["executor"]["remaining_workers"], 0);
        if host {
            resident::check_executor(&stopped["executor"]).unwrap();
        } else {
            assert_eq!(stopped["executor"]["workers_received"], 0);
            assert_eq!(stopped["executor"]["workers_handed_off"], 0);
            assert_eq!(
                stopped["executor"]["joined_workers"],
                stopped["executor"]["workers_started"]
            );
        }
    }
}

pub(super) fn send(
    address: SocketAddr,
    method: &str,
    route: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> Reply {
    let timeout = Duration::from_secs(120);
    let mut stream = TcpStream::connect_timeout(&address, timeout).unwrap();
    stream.set_read_timeout(Some(timeout)).unwrap();
    stream.set_write_timeout(Some(timeout)).unwrap();
    write!(stream, "{method} {route} HTTP/1.1\r\nConnection: close\r\n").unwrap();
    for (name, value) in headers {
        write!(stream, "{name}: {value}\r\n").unwrap();
    }
    write!(stream, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    let mut bytes = Vec::new();
    stream.take(262145).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() <= 262144, "bounded native HTTP response");
    let text = String::from_utf8(bytes).unwrap();
    let (headers, body) = text.split_once("\r\n\r\n").expect("complete HTTP response");
    let status = headers.split_whitespace().nth(1).unwrap().parse().unwrap();
    let headers = headers.to_ascii_lowercase();
    assert!(!headers.contains("transfer-encoding: chunked"));
    Reply {
        status,
        headers,
        body: body.to_owned(),
    }
}
