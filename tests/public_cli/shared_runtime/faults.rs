//! Negative startup boundaries and busy sibling isolation.
use super::*;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

fn command(public: &Native, descriptors: &[Value]) -> Command {
    let mut command = Command::new(&public.executable);
    command.arg("serve");
    for (index, descriptor) in descriptors.iter().enumerate() {
        command
            .arg("--deployment")
            .arg(public.input(&format!("failed-{index}.json"), &descriptor.to_string()));
    }
    command
        .current_dir(public.root.path())
        .env_clear()
        .env("PATH", "");
    command
}

fn failure(output: Output) -> String {
    assert!(!output.status.success());
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!text.contains("\"event\":\"ready\""), "{text}");
    text
}

#[test]
fn shared_runtime_admits_all_before_secrets_and_never_partially_announces_startup() {
    let (public, descriptor) = super::super::resident_termination::author("http");
    let mut first = descriptor.clone();
    first["secrets"] = json!([{"name":"unused-secret","variable":"SHARED_FIXTURE_ABSENT_SECRET"}]);
    let mut invalid = descriptor.clone();
    invalid["target"] = json!("missing-target");
    let text = failure(support::output(&mut command(&public, &[first, invalid])).unwrap());
    assert!(text.contains("deployment_target_missing"), "{text}");

    let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
    let reserve = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = reserve.local_addr().unwrap();
    drop(reserve);
    let mut first = descriptor.clone();
    let mut second = descriptor.clone();
    first["listen"] = json!(address.to_string());
    second["listen"] = json!(occupied.local_addr().unwrap().to_string());
    let text = failure(support::output(&mut command(&public, &[first, second])).unwrap());
    assert!(text.contains("serve_bind"), "{text}");
    let released = TcpListener::bind(address).unwrap();
    drop(released);
    drop(occupied);

    let mut damaged = std::fs::read(public.root.path().join("termination.lkja")).unwrap();
    *damaged.last_mut().unwrap() ^= 1;
    std::fs::write(public.root.path().join("damaged.lkja"), damaged).unwrap();
    let mut second = descriptor.clone();
    second["artifact"] = json!("damaged.lkja");
    failure(support::output(&mut command(&public, &[descriptor.clone(), second])).unwrap());

    // A broken readiness stream is established before the process can write.
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let child = support::spawn(
        command(&public, &[descriptor.clone(), descriptor.clone()])
            .stdout(writer)
            .stderr(Stdio::piped()),
    )
    .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cli_output"));

    // The owned binary still starts cleanly after every rejected group.
    let group = Group::start(&public, &[descriptor.clone(), descriptor], &[]);
    group.stop(Signal::TERM);
}

#[test]
fn shared_runtime_parse_and_aggregate_limits_reject_without_fallback() {
    let public = Native::template("http");
    for arguments in [
        vec!["serve", "--deployment", "missing", "--deployment"],
        vec!["serve", "--deployment", "missing", "--unknown", "other"],
        vec!["serve", "--deployment", "missing", "--deployment", ""],
    ] {
        let text = failure(
            support::output(
                Command::new(&public.executable)
                    .args(arguments)
                    .current_dir(public.root.path())
                    .env_clear(),
            )
            .unwrap(),
        );
        assert!(text.contains("cli_usage"), "{text}");
    }
    let mut too_many = Command::new(&public.executable);
    too_many
        .arg("serve")
        .current_dir(public.root.path())
        .env_clear();
    for _ in 0..65 {
        too_many.args(["--deployment", "must-not-be-opened"]);
    }
    assert!(failure(support::output(&mut too_many).unwrap()).contains("cli_usage"));
    build(&public, "application.lkja");
    let mut descriptor = descriptor(&public);
    descriptor["artifact"] = json!("application.lkja");
    descriptor["runtime"]["maximum_concurrent_tasks"] = json!(4096);
    let text =
        failure(support::output(&mut command(&public, &[descriptor.clone(), descriptor])).unwrap());
    assert!(text.contains("shared_serve_limit"), "{text}");
}

#[test]
fn shared_runtime_busy_instance_does_not_block_peer_and_signals_join_both() {
    let (public, descriptor) = super::super::resident_termination::author("http");
    for signal in [Signal::TERM, Signal::INT] {
        let group = Group::start(&public, &[descriptor.clone(), descriptor.clone()], &[]);
        let mut busy =
            TcpStream::connect_timeout(&group.address(0), Duration::from_secs(5)).unwrap();
        busy.set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        busy.set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        busy.write_all(b"GET /busy HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .unwrap();
        let mut byte = [0];
        assert!(matches!(busy.read(&mut byte), Err(error)
            if matches!(error.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut)));
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            let reply = http::send(
                group.address(0),
                "GET",
                "/health",
                &[("Host", "localhost")],
                "",
            );
            if reply.status == 503 {
                assert!(reply.headers.contains("resident_overloaded"));
                break;
            }
            assert_eq!(reply.status, 200);
            assert!(Instant::now() < until);
        }
        assert_eq!(
            http::send(
                group.address(1),
                "GET",
                "/health",
                &[("Host", "localhost")],
                ""
            )
            .body,
            "7"
        );
        let stopped = group.stop(signal);
        assert!(
            stopped["instances"][0]["receipt"]["runtime"]["resident"]["cancelled"]
                .as_u64()
                .unwrap()
                >= 1
        );
        drop(busy);
    }
}
