use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

fn file(name: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/recursion-tests");
    fs::create_dir_all(&directory).unwrap();
    directory.join(format!("{}-{name}.rs", std::process::id()))
}

fn run_source(name: &str, text: &str, verb: &str) -> Output {
    let path = file(name);
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

struct NativeImage(PathBuf);
impl Drop for NativeImage {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn allocation_image(name: &str) -> NativeImage {
    let runtime = lkjscript::bootstrap_runtime::get().unwrap();
    let image = lkjscript::native::encode_runtime(
        runtime,
        lkjscript::native::Entry {
            function: runtime.entry(lkjscript::runtime::Entry::Allocate),
            arguments: vec![1i64 << 62],
            result_fd: None,
        },
    )
    .unwrap_or_else(|problem| panic!("{}", problem.message));
    let path = file(name).with_extension("elf");
    fs::write(&path, image.bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    NativeImage(path)
}

#[test]
fn kernel_heap_mapping_refusal_is_a_located_trap() {
    let image = allocation_image("heap-refusal");
    let output = Command::new(&image.0).output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("R0006 at line 1 file 0"),
        "{output:?}"
    );
}

#[test]
fn kernel_stack_mapping_refusal_is_a_located_trap() {
    let checked = lkjscript::compiler::check_bundle(&lkjscript::source::SourceBundle {
        root: "stack-refusal.rs".to_string(),
        library: None,
        files: vec![lkjscript::source::SourceInput {
            path: "stack-refusal.rs".to_string(),
            text: "fn deep(n:usize)->i64 {
    let frame = [n as i64; 1024];
    if n == 0 { return frame[0]; }
    deep(n - 1) + frame[n % 1024]
}
pub fn probe()->i64 { deep(100000) }"
                .to_string(),
        }],
    })
    .unwrap();
    let function = checked
        .program()
        .functions
        .iter()
        .position(|function| function.name == "probe")
        .unwrap();
    let bytes = lkjscript::native::encode(
        &checked,
        lkjscript::bootstrap_runtime::get().unwrap(),
        lkjscript::native::Entry {
            function,
            arguments: vec![],
            result_fd: None,
        },
    )
    .unwrap()
    .bytes;
    let path = file("stack-refusal").with_extension("elf");
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let image = NativeImage(path);
    let output = Command::new("prlimit")
        .args(["--as=67108864", "--"])
        .arg(&image.0)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("R0008 at line "),
        "{output:?}"
    );
}
