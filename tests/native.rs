#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "../bridge/library.rs"]
mod bootstrap_library;
use lkjscript::{bootstrap_runtime, compiler, ir, native, source};
use std::os::fd::OwnedFd;
use std::os::unix::{fs::PermissionsExt, net::UnixStream};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

// Fork can inherit another test thread's writable image descriptor before exec.
// Serialize publication/execution so the host cannot transiently return ETXTBSY.
static EXECUTION: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct Image(PathBuf);
impl Image {
    fn command(&self) -> Command {
        Command::new(&self.0)
    }
    fn new(name: &str, program: &ir::CheckedProgram, entry: usize, arguments: Vec<i64>) -> Self {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/native-platform");
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("{}-{name}.elf", std::process::id()));
        let image = native::encode(
            program,
            bootstrap_runtime::get().unwrap(),
            native::Entry {
                function: entry,
                arguments,
                result_fd: Some(2),
            },
        )
        .unwrap();
        std::fs::write(&path, image.bytes).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
    fn source(name: &str, text: &str, entry: &str) -> Self {
        let mut files = bootstrap_library::standard_library();
        files.push(source::SourceInput {
            path: name.to_string(),
            text: text.to_string(),
        });
        let checked = compiler::check_bundle(&source::SourceBundle {
            root: name.to_string(),
            library: Some("$std/lib.rs".to_string()),
            files,
        })
        .unwrap();
        let function = checked
            .program()
            .functions
            .iter()
            .position(|function| function.name == entry)
            .unwrap();
        let arguments = if entry == "main" { vec![1] } else { vec![] };
        Self::new(name, &checked, function, arguments)
    }
}
impl Drop for Image {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn assert_trap(output: Output, code: &str, line: u64) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "native process died by a signal: {output:?}"
    );
    let diagnostic = String::from_utf8(output.stderr).unwrap();
    assert!(
        diagnostic.starts_with(&format!("{code} at line {line} file 0:")),
        "{diagnostic:?}"
    );
    assert!(diagnostic.ends_with("\n"), "{diagnostic:?}");
}

#[test]
fn broken_and_closed_output_return_the_source_line_without_sigpipe() {
    let _execution = EXECUTION.lock().unwrap();
    let image = Image::source("output", include_str!("native/output.rs"), "main");
    let (writer, reader) = UnixStream::pair().unwrap();
    drop(reader);
    let writer: OwnedFd = writer.into();
    let broken = image
        .command()
        .stdout(Stdio::from(writer))
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let closed = Command::new("sh")
        .args(["-c", "exec 1>&-; exec \"$1\"", "native-output"])
        .arg(&image.0)
        .output()
        .unwrap();
    for output in [broken, closed] {
        assert_trap(output, "R0007", 5);
    }
}
