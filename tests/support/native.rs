//! Test-only scalar execution. Source inputs remain explicit; expected values are
//! compared in Rust after one native image returns every observation as bytes.
// Each integration-test crate uses a different subset of this shared helper.
#![allow(dead_code)]
use lkjscript::{bootstrap_runtime, compiler, k, lower, native, source};
use lower::{
    Code, Function, FunctionRole, HostFunction, Import, Instruction as I, RuntimeFunction,
    Signature,
};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
// Keep each native observation batch together inside this test process.
static EXECUTION: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn check(bundle: &source::SourceBundle) -> k::CheckedProgram {
    compiler::check_bundle(bundle).unwrap_or_else(|error| panic!("{error:?}"))
}
pub fn assert_bundle(bundle: &source::SourceBundle, entries: &[(&str, i64)]) {
    assert_values(&check(bundle), entries);
}
pub fn assert_values(checked: &k::CheckedProgram, entries: &[(&str, i64)]) {
    let calls = entries
        .iter()
        .map(|(name, expected)| (*name, &[][..], *expected))
        .collect::<Vec<_>>();
    assert_calls(checked, &calls);
}
pub fn assert_rows(checked: &k::CheckedProgram, rows: &str) {
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
pub fn assert_calls(checked: &k::CheckedProgram, calls: &[(&str, &[i64], i64)]) {
    let mut program = lower::lower(checked, bootstrap_runtime::get().unwrap()).unwrap();
    let stdout = match program
        .imports
        .iter()
        .position(|import| import.host == HostFunction::StdoutWrite)
    {
        Some(index) => index,
        None => {
            let index = program.imports.len();
            program.imports.push(Import {
                host: HostFunction::StdoutWrite,
                signature: Signature::words(3),
            });
            for function in &mut program.functions {
                for instruction in &mut function.code.instructions {
                    if let I::Call(target) = instruction {
                        if *target >= index {
                            *target += 1;
                        }
                    }
                }
            }
            for export in &mut program.exports {
                if export.function >= index {
                    export.function += 1;
                }
            }
            for target in &mut program.table {
                if *target >= index {
                    *target += 1;
                }
            }
            index
        }
    };
    let runtime = |role| {
        program.imports.len()
            + program
                .functions
                .iter()
                .position(|function| function.role == FunctionRole::Runtime(role))
                .unwrap()
    };
    let allocate = runtime(RuntimeFunction::Allocate);
    let release = runtime(RuntimeFunction::Release);
    let length = i64::try_from(calls.len().checked_mul(8).unwrap()).unwrap();
    let mut instructions = vec![I::I64(length), I::Call(allocate), I::LocalSet(0)];
    for (index, (name, arguments, _)) in calls.iter().enumerate() {
        let function = program
            .exports
            .iter()
            .find(|export| export.name == format!("fn:{name}"))
            .unwrap_or_else(|| panic!("missing test entry {name}"));
        instructions.push(I::LocalGet(0));
        instructions.extend(arguments.iter().map(|value| I::I64(*value)));
        instructions.push(I::Call(function.function));
        instructions.push(I::Store {
            bytes: 8,
            offset: index as u64 * 8,
        });
    }
    instructions.extend([
        I::LocalGet(0),
        I::I64(length),
        I::I64(0),
        I::Call(stdout),
        I::Drop,
        I::LocalGet(0),
        I::Call(release),
        I::Drop,
        I::I64(0),
        I::End,
    ]);
    let entry = program.imports.len() + program.functions.len();
    program.functions.push(Function {
        name: "$test_observations".to_string(),
        signature: Signature::words(0),
        role: FunctionRole::Source,
        code: Code {
            locals: vec![lower::ValueType::I64],
            instructions,
        },
    });
    let bytes = native::encode(
        &program,
        native::Entry {
            function: entry,
            arguments: vec![],
            result_fd: Some(2),
        },
        bootstrap_runtime::resources().unwrap(),
    )
    .unwrap();
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/semantic-tests");
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!(
        "{}-{}.elf",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let output = {
        let _execution = EXECUTION.lock().unwrap();
        // Other tests can fork rustc while this helper runs. Publish in a child
        // so those forks never inherit a writable ELF handle (ETXTBSY at exec).
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
        child.stdin.take().unwrap().write_all(&bytes).unwrap();
        let output = child.wait_with_output().unwrap();
        std::fs::remove_file(path).unwrap();
        output
    };
    assert_eq!(
        output.stderr.len(),
        24,
        "missing native result packet: {output:?}"
    );
    let word = |offset| u64::from_le_bytes(output.stderr[offset..offset + 8].try_into().unwrap());
    assert!(
        output.status.success() && word(0) == 0,
        "native test trapped with R{:04} at line {}: {:?}",
        word(0),
        word(16),
        output.status
    );
    assert_eq!(
        output.stdout.len(),
        calls.len() * 8,
        "native observation count differs"
    );
    for (index, (name, _, expected)) in calls.iter().enumerate() {
        let actual =
            i64::from_le_bytes(output.stdout[index * 8..index * 8 + 8].try_into().unwrap());
        assert_eq!(actual, *expected, "entry {name}");
    }
}
