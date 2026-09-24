use super::*;

fn fixture(root: &Path, name: &str, script: &str) -> ProcessSpec {
    ProcessSpec {
        command: vec!["/bin/sh".to_owned(), "-c".to_owned(), script.to_owned()],
        cwd: root.to_path_buf(),
        environment: BTreeMap::from([("PATH".to_owned(), "/usr/bin:/bin".to_owned())]),
        timeout: Duration::from_millis(80),
        maximum_stdout_bytes: 4096,
        maximum_stderr_bytes: 4096,
        stdout_path: root.join(format!("{name}.stdout")),
        stderr_path: root.join(format!("{name}.stderr")),
        unavailable_exit_code: None,
    }
}

#[test]
fn inherited_stdout_cannot_outlive_the_deadline_as_a_pass() {
    let root = tempfile::tempdir().expect("owned inherited-output fixture");
    // Finite even with the old broken implementation: no orphan survives this test.
    let spec = fixture(root.path(), "inherited", "sleep 0.6 & printf 'parent-done'");
    let result = run(&spec, root.path());
    assert_ne!(result.status, ProcessStatus::Passed, "{result:?}");
    assert!(result.elapsed_nanoseconds < 500_000_000, "{result:?}");
    assert_eq!(std::fs::read(spec.stdout_path).unwrap(), b"parent-done");
}

#[test]
fn inherited_stderr_cannot_outlive_the_deadline_as_a_pass() {
    let root = tempfile::tempdir().expect("owned inherited-error fixture");
    let spec = fixture(
        root.path(),
        "inherited",
        "sleep 0.6 >/dev/null & printf 'parent-error' >&2",
    );
    let result = run(&spec, root.path());
    assert_ne!(result.status, ProcessStatus::Passed, "{result:?}");
    assert!(result.elapsed_nanoseconds < 500_000_000, "{result:?}");
    assert_eq!(std::fs::read(spec.stderr_path).unwrap(), b"parent-error");
}

fn assert_stopped(pid: u32) {
    if let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        let state = stat.rsplit_once(") ").unwrap().1.chars().next().unwrap();
        assert!(
            matches!(state, 'Z' | 'X'),
            "owned PID {pid} still live: {stat}"
        );
    }
}

#[test]
fn exact_limits_and_final_bursts_preserve_both_streams() {
    let root = tempfile::tempdir().unwrap();
    for (name, count, maximum) in [
        ("empty", 0, 0),
        ("zero", 1, 0),
        ("exact", 4, 4),
        ("over", 5, 4),
        ("large", 131072, 131072),
    ] {
        let script = format!("head -c {count} /dev/zero; head -c {count} /dev/zero >&2");
        let mut spec = fixture(root.path(), name, &script);
        spec.timeout = Duration::from_secs(3);
        spec.maximum_stdout_bytes = maximum;
        spec.maximum_stderr_bytes = maximum;
        let result = run(&spec, root.path());
        assert_eq!(
            result.status,
            if count > maximum {
                ProcessStatus::OutputExhausted
            } else {
                ProcessStatus::Passed
            },
            "{result:?}"
        );
        let stdout = std::fs::read(spec.stdout_path).unwrap();
        let stderr = std::fs::read(spec.stderr_path).unwrap();
        assert!(stdout.iter().chain(&stderr).all(|byte| *byte == 0));
        assert_eq!(stdout.len(), count.min(maximum) as usize);
        if count <= maximum {
            assert_eq!(stderr.len(), count as usize);
            assert!(!result.stdout_limit_exhausted && !result.stderr_limit_exhausted);
        } else {
            assert!(result.stdout_limit_exhausted);
        }
    }
}

#[test]
fn all_shared_routes_clean_sampled_separate_groups_and_reap_the_direct_child() {
    let root = tempfile::tempdir().unwrap();
    for mode in 0..4 {
        let script =
            format!("setsid sleep 0.8 & printf '%s %s' $$ $! > ids-{mode}; sleep 0.06; exit 0");
        let mut spec = fixture(root.path(), &format!("route-{mode}"), &script);
        spec.timeout = Duration::from_millis(180);
        let result = match mode {
            0 => run(&spec, root.path()),
            1 => run_selected(&spec, root.path(), Some(Path::new("/bin/sh"))),
            2 => run_controlled(&spec, root.path(), &ProcessControl::default()),
            _ => {
                let stdin = root.path().join("input");
                std::fs::write(&stdin, b"").unwrap();
                run_with_stdin_file(&spec, root.path(), &stdin, 0)
            }
        };
        assert_eq!(result.status, ProcessStatus::Timeout, "{result:?}");
        let ids = std::fs::read_to_string(root.path().join(format!("ids-{mode}"))).unwrap();
        let pids: Vec<u32> = ids
            .split_whitespace()
            .map(|id| id.parse().unwrap())
            .collect();
        assert!(
            !Path::new(&format!("/proc/{}", pids[0])).exists(),
            "direct child not reaped"
        );
        assert_stopped(pids[1]);
    }
    let spec = fixture(root.path(), "recovered", "printf healthy");
    assert_eq!(run(&spec, root.path()).status, ProcessStatus::Passed);
}

#[test]
fn cancellation_remains_observable_after_direct_child_exit() {
    let root = tempfile::tempdir().unwrap();
    let mut spec = fixture(
        root.path(),
        "cancel",
        "printf '%s' $$ > direct; sleep 0.8 & exit 0",
    );
    spec.timeout = Duration::from_secs(3);
    let direct = root.path().join("direct");
    let control = ProcessControl::default();
    let trigger = control.clone();
    let watcher = std::thread::spawn(move || {
        let began = Instant::now();
        loop {
            if let Ok(pid) = std::fs::read_to_string(&direct)
                && let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat"))
                && stat.rsplit_once(") ").unwrap().1.starts_with('Z')
            {
                trigger.kill();
                return;
            }
            assert!(
                began.elapsed() < Duration::from_secs(2),
                "direct child was not kept waitable"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    });
    let result = run_controlled(&spec, root.path(), &control);
    watcher.join().unwrap();
    assert_eq!(result.status, ProcessStatus::Signaled, "{result:?}");
    assert_eq!(result.reason.as_deref(), Some("control_kill"));
    assert!(result.elapsed_nanoseconds < 500_000_000, "{result:?}");
}

#[test]
fn silent_sampled_survivor_is_not_a_clean_success() {
    let root = tempfile::tempdir().unwrap();
    let spec = fixture(
        root.path(),
        "silent",
        "setsid sleep 0.8 >/dev/null 2>&1 & echo $! > survivor; sleep 0.04; exit 0",
    );
    let result = run(&spec, root.path());
    assert_eq!(
        result.status,
        ProcessStatus::InfrastructureFailure,
        "{result:?}"
    );
    assert!(result.reason.unwrap().contains("descendants survived"));
    assert_stopped(
        std::fs::read_to_string(root.path().join("survivor"))
            .unwrap()
            .trim()
            .parse()
            .unwrap(),
    );
}

#[test]
fn closed_stdout_is_closed_before_even_an_immediate_write() {
    let root = tempfile::tempdir().unwrap();
    for attempt in 0..24 {
        let spec = fixture(
            root.path(),
            &format!("closed-{attempt}"),
            "trap '' PIPE; if printf x; then exit 99; else exit 0; fi",
        );
        let result = run_closed_stdout(&spec, root.path());
        assert_eq!(
            result.status,
            ProcessStatus::Passed,
            "attempt {attempt}: {result:?}"
        );
        assert_eq!(result.stdout.bytes, Some(0));
        assert!(result.stderr.bytes.unwrap() > 0);
    }
}
