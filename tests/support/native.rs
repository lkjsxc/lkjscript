//! Compare scalar observations from checked IR with the Rust test oracle.
#![allow(dead_code)]
use lkjscript::{bootstrap_runtime, compiler, ir, native, source};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
static EXECUTION: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn check(bundle: &source::SourceBundle) -> ir::CheckedProgram {
    compiler::check_bundle(bundle).unwrap_or_else(|error| panic!("{error:?}"))
}
pub fn assert_bundle(bundle: &source::SourceBundle, entries: &[(&str, i64)]) {
    assert_values(&check(bundle), entries);
}
pub fn assert_values(checked: &ir::CheckedProgram, entries: &[(&str, i64)]) {
    let calls = entries
        .iter()
        .map(|(name, expected)| (*name, &[][..], *expected))
        .collect::<Vec<_>>();
    assert_calls(checked, &calls);
}
pub fn assert_rows(checked: &ir::CheckedProgram, rows: &str) {
    let values = rows
        .lines()
        .map(|row| {
            let fields = row.split_whitespace().collect::<Vec<_>>();
            assert_eq!(fields.len(), 2, "invalid oracle row: {row}");
            (fields[0], fields[1].parse().unwrap())
        })
        .collect::<Vec<_>>();
    assert_values(checked, &values);
}
pub fn assert_calls(checked: &ir::CheckedProgram, calls: &[(&str, &[i64], i64)]) {
    let runtime = bootstrap_runtime::get().unwrap();
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/semantic-tests");
    std::fs::create_dir_all(&directory).unwrap();
    for (name, arguments, expected) in calls {
        let function = checked
            .program()
            .functions
            .iter()
            .position(|function| function.name == *name)
            .unwrap_or_else(|| panic!("missing test entry {name}"));
        let image = native::encode(
            checked,
            runtime,
            native::Entry {
                function,
                arguments: arguments.to_vec(),
                result_fd: Some(2),
            },
        )
        .unwrap();
        let path = directory.join(format!(
            "{}-{}.elf",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let output = {
            let _execution = EXECUTION.lock().unwrap();
            let mut child = Command::new("/bin/sh")
                .args([
                    "-c",
                    "cat > \"$1\" && chmod 700 \"$1\" && exec \"$1\"",
                    "native-semantic-test",
                ])
                .arg(&path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child.stdin.take().unwrap().write_all(&image.bytes).unwrap();
            let output = child.wait_with_output().unwrap();
            std::fs::remove_file(path).unwrap();
            output
        };
        assert_eq!(
            output.stderr.len(),
            24,
            "missing native result packet for {name}: {output:?}"
        );
        let word =
            |offset| u64::from_le_bytes(output.stderr[offset..offset + 8].try_into().unwrap());
        assert!(
            output.status.success() && word(0) == 0,
            "entry {name} trapped with R{:04} at line {}: {:?}",
            word(0),
            word(16),
            output.status
        );
        assert_eq!(word(8) as i64, *expected, "entry {name}");
    }
}
