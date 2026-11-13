use lkjscript::compiler;
use lkjscript::source::{SourceBundle, SourceInput};
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bundle(text: String) -> SourceBundle {
    SourceBundle {
        root: "stack/main.rs".to_string(),
        library: None,
        files: vec![SourceInput {
            path: "stack/main.rs".to_string(),
            text,
        }],
    }
}

fn file(name: &str, extension: &str) -> PathBuf {
    let directory = root().join("target/stack-tests");
    fs::create_dir_all(&directory).unwrap();
    directory.join(format!("{}-{name}.{extension}", std::process::id()))
}

fn run_source(name: &str, text: &str, verb: &str) -> Output {
    let path = file(name, "rs");
    fs::write(&path, text).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .arg(verb)
        .arg(&path)
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();
    output
}

#[test]
fn compiler_bridge_respects_a_host_address_space_envelope() {
    const CHILD: &str = "LKJSCRIPT_ADDRESS_SPACE_CHILD";
    if std::env::var_os(CHILD).is_some() {
        compiler::compile(&bundle("fn probe() -> i64 { 42 }".to_string())).unwrap();
        return;
    }
    let output = Command::new("bash").arg("-c")
        .arg("ulimit -v 1048576; exec \"$1\" --exact compiler_bridge_respects_a_host_address_space_envelope --nocapture")
        .arg("stack-probe").arg(std::env::current_exe().unwrap())
        .env(CHILD, "1").output().unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn run_and_test_execute_owned_recursion_and_recursive_drop() {
    let output = run_source(
        "recursive-run",
        "
        fn direct(n: i64) -> i64 { if n == 0 { 0 } else { 1 + direct(n - 1) } }
        fn left(n: i64) -> i64 { if n == 0 { 0 } else { 1 + right(n - 1) } }
        fn right(n: i64) -> i64 { if n == 0 { 0 } else { 1 + left(n - 1) } }
        fn probe() -> i64 { direct(100000) + left(100000) }
    ",
        "run",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"200000\n");
    let output = run_source(
        "recursive-drop-test",
        "
        struct Node { depth: i64 }
        fn destroy(depth: i64) { let node = Node { depth: depth }; }
        impl Drop for Node {
            fn drop(&mut self) { if self.depth > 0 { destroy(self.depth - 1); } }
        }
        #[test] fn deep_cleanup() { destroy(30000); }
    ",
        "test",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
}

struct KillOnDrop(Child);
impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn killed_cli_does_not_leave_its_native_child_running() {
    let path = file("cancelled-run", "rs");
    fs::write(&path, "fn main() { loop {} }").unwrap();
    let mut child = KillOnDrop(
        Command::new(env!("CARGO_BIN_EXE_lkjscript"))
            .arg("run")
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let pid = child.0.id();
    let started = Instant::now();
    let guest = loop {
        // The compiler runs on its resource-sized thread; Linux records forked
        // children under the creating task, not necessarily the main task.
        let tasks = fs::read_dir(format!("/proc/{pid}/task")).unwrap();
        let mut found = None;
        for task in tasks.flatten() {
            if let Ok(children) = fs::read_to_string(task.path().join("children")) {
                if let Some(child) = children.split_whitespace().next() {
                    found = Some(child.to_string());
                    break;
                }
            }
        }
        if let Some(child) = found {
            break child;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "CLI never started its native child"
        );
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "CLI finished before cancellation"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    loop {
        let state = fs::read_to_string(format!("/proc/{guest}/status"));
        if state.is_err()
            || state
                .unwrap()
                .lines()
                .any(|line| line.starts_with("State:\tZ"))
        {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "Native child survived its compiler parent"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    fs::remove_file(path).unwrap();
    for entry in fs::read_dir(root().join("target/run")).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&format!("{pid}-")) && name.ends_with(".elf") {
            let _ = fs::remove_file(entry.path());
        }
    }
}
