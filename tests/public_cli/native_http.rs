//! Independent bounded HTTP/1.1 fixture; it imports no production HTTP codec.
use super::*;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

pub(super) struct Reply {
    pub status: u16,
    pub headers: String,
    pub body: String,
}

pub(super) struct Server {
    child: support::SpawnedChild,
    output: PathBuf,
    errors: PathBuf,
    pub address: SocketAddr,
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
                    child,
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

    pub fn stop(mut self) {
        let pid = rustix::process::Pid::from_raw(i32::try_from(self.child.id()).unwrap()).unwrap();
        rustix::process::kill_process(pid, rustix::process::Signal::INT).unwrap();
        let until = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "native HTTP server failed to join: {status}"
                );
                break;
            }
            assert!(Instant::now() < until, "native HTTP shutdown deadline");
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
