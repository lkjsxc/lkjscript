use super::*;

#[test]
fn supervised_cancellation_and_timeout_clean_separate_process_groups_then_recover() {
    let root = tempfile::tempdir().expect("owned test root");
    for kill in [false, true] {
        let control = process::ProcessControl::default();
        let spec = process::ProcessSpec {
            command: vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                "setsid sleep 60 & echo $!; printf '%s\\n' '{\"status\":\"fresh passed\"}' > provisional.json; wait".to_owned(),
            ],
            cwd: root.path().to_path_buf(),
            environment: BTreeMap::from([("PATH".to_owned(), "/usr/bin:/bin".to_owned())]),
            timeout: if kill { Duration::from_secs(3) } else { Duration::from_millis(350) },
            maximum_stdout_bytes: 1024,
            maximum_stderr_bytes: 1024,
            stdout_path: root.path().join(format!("{kill}.stdout")),
            stderr_path: root.path().join(format!("{kill}.stderr")),
            unavailable_exit_code: None,
        };
        let trigger = control.clone();
        let provisional_path = root.path().join("provisional.json");
        let ready = provisional_path.clone();
        let thread = std::thread::spawn(move || {
            if kill {
                let started = Instant::now();
                while !ready.exists() {
                    assert!(
                        started.elapsed() < Duration::from_secs(2),
                        "child never recorded its provisional result"
                    );
                    std::thread::sleep(Duration::from_millis(10));
                }
                // Give the existing supervisor one sampling interval to observe the owned group.
                std::thread::sleep(Duration::from_millis(30));
                trigger.kill();
            }
        });
        let observation = process::run_supervised(&spec, root.path(), Some(&control));
        thread.join().expect("joined cancellation");
        assert_eq!(
            fs::read_to_string(&provisional_path).expect("provisional result"),
            "{\"status\":\"fresh passed\"}\n",
        );
        fs::remove_file(provisional_path)
            .expect("remove owned provisional result before next attempt");
        assert_eq!(
            observation.status,
            if kill {
                process::ProcessStatus::Signaled
            } else {
                process::ProcessStatus::Timeout
            },
            "{observation:?}"
        );
        assert_eq!(
            observation.reason.as_deref(),
            Some(if kill { "control_kill" } else { "timeout" })
        );
        let pid = fs::read_to_string(&spec.stdout_path)
            .expect("child identity")
            .trim()
            .parse::<u32>()
            .expect("pid");
        if let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) {
            assert!(
                stat.rsplit_once(") ")
                    .expect("proc stat")
                    .1
                    .starts_with('Z')
            );
        }
    }
    let spec = process::ProcessSpec {
        command: vec!["/bin/true".to_owned()],
        cwd: root.path().to_path_buf(),
        environment: BTreeMap::new(),
        timeout: Duration::from_secs(5),
        maximum_stdout_bytes: 1024,
        maximum_stderr_bytes: 1024,
        stdout_path: root.path().join("healthy.stdout"),
        stderr_path: root.path().join("healthy.stderr"),
        unavailable_exit_code: None,
    };
    assert_eq!(
        process::run_supervised(&spec, root.path(), None).status,
        process::ProcessStatus::Passed
    );
}
