#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "../bridge/library.rs"]
mod bootstrap_library;
use lkjscript::{bootstrap_runtime, compiler, lower, native, source};
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
        let mut command = Command::new("prlimit");
        command.arg("--as=268435456").arg(&self.0);
        command
    }
    fn new(name: &str, program: &lower::Program, entry: usize, arguments: Vec<i64>) -> Self {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/native-platform");
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("{}-{name}.elf", std::process::id()));
        let bytes = native::encode(
            program,
            native::Entry {
                function: entry,
                arguments,
                result_fd: Some(2),
            },
        )
        .unwrap();
        std::fs::write(&path, bytes).unwrap();
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
        let program = lower::lower(&checked, bootstrap_runtime::get().unwrap()).unwrap();
        let function = program
            .exports
            .iter()
            .find(|e| e.name == format!("fn:{entry}"))
            .unwrap()
            .function;
        let arguments = if entry == "main" { vec![1] } else { vec![] };
        Self::new(name, &program, function, arguments)
    }
}
impl Drop for Image {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn packet(output: Output) -> [u64; 3] {
    assert_eq!(
        output.stderr.len(),
        24,
        "native exit lacks result packet: {output:?}"
    );
    let words = std::array::from_fn(|i| {
        u64::from_le_bytes(output.stderr[i * 8..i * 8 + 8].try_into().unwrap())
    });
    assert_eq!(
        output.status.code(),
        Some(i32::from(words[0] != 0)),
        "native process died by a signal: {output:?}"
    );
    words
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
        .args([
            "-c",
            "ulimit -v 262144; exec 1>&-; exec \"$1\"",
            "native-output",
        ])
        .arg(&image.0)
        .output()
        .unwrap();
    for output in [broken, closed] {
        let words = packet(output);
        assert_eq!((words[0], words[2]), (7, 5));
    }
}

#[test]
fn conditional_block_results_preserve_the_not_taken_operand_stack() {
    let _execution = EXECUTION.lock().unwrap();
    use lower::{
        Code, Function, FunctionRole, Global, Instruction as I, Layouts, Program, ScalarOp,
        Signature, ValueType,
    };
    let program = Program {
        layout: Layouts { types: vec![] },
        imports: vec![],
        functions: vec![Function {
            name: "branch_result".to_string(),
            signature: Signature {
                parameters: vec![ValueType::I32],
                result: Some(ValueType::I64),
            },
            role: FunctionRole::Source,
            code: Code {
                locals: vec![],
                instructions: vec![
                    I::Block(Some(ValueType::I64)),
                    I::I64(11),
                    I::I64(31),
                    I::LocalGet(0),
                    I::Branch {
                        depth: 0,
                        conditional: true,
                    },
                    I::Scalar(ScalarOp::I64Add),
                    I::End,
                    I::End,
                ],
            },
        }],
        globals: (0..2)
            .map(|_| Global {
                initial: 0,
                mutable: true,
            })
            .collect(),
        exports: vec![],
        table: vec![],
        data: vec![],
        data_offset: 8,
        minimum_memory_pages: 1,
        trap_code_global: 0,
        trap_line_global: 1,
        trap_messages: vec![],
    };
    for (condition, expected) in [(0, 42), (1, 31), (1i64 << 32, 42)] {
        let image = Image::new("branch", &program, 0, vec![condition]);
        assert_eq!(
            packet(Command::new(&image.0).output().unwrap()),
            [0, expected, 0]
        );
    }
}

#[test]
fn prepared_source_runtime_probes_run_through_the_native_encoder() {
    let _execution = EXECUTION.lock().unwrap();
    let text = format!(
        "{}\n{}\n{}",
        include_str!("../library/runtime.rs"),
        include_str!("runtime/allocator.rs"),
        include_str!("runtime/drop_walk.rs")
    );
    let runtime = lkjscript::runtime::prepare(&source::SourceBundle {
        root: "runtime.rs".to_string(),
        library: None,
        files: vec![source::SourceInput {
            path: "runtime.rs".to_string(),
            text,
        }],
    })
    .unwrap();
    let step = runtime.function("model_drop_step").unwrap();
    let program = runtime.model_program(&[step]).unwrap();
    for (name, status, value) in [
        ("probe_allocator", 0, 42),
        ("probe_drop", 0, 42),
        ("probe_overflow", 6, 0),
    ] {
        let entry = program
            .exports
            .iter()
            .find(|e| e.name == format!("fn:{name}"))
            .unwrap()
            .function;
        let image = Image::new(name, &program, entry, vec![]);
        let result = packet(Command::new(&image.0).output().unwrap());
        assert_eq!((result[0], result[1]), (status, value), "{name}");
    }
}

fn instruction_program(functions: Vec<lower::Function>, table: Vec<usize>) -> lower::Program {
    lower::Program {
        layout: lower::Layouts { types: vec![] },
        imports: vec![],
        functions,
        globals: vec![
            lower::Global {
                initial: 0,
                mutable: true,
            },
            lower::Global {
                initial: 0,
                mutable: true,
            },
        ],
        exports: vec![],
        table,
        data: vec![],
        data_offset: 8,
        minimum_memory_pages: 1,
        trap_code_global: 0,
        trap_line_global: 1,
        trap_messages: vec![],
    }
}

fn instruction_function(
    name: &str,
    parameters: Vec<lower::ValueType>,
    locals: Vec<lower::ValueType>,
    instructions: Vec<lower::Instruction>,
) -> lower::Function {
    lower::Function {
        name: name.to_string(),
        signature: lower::Signature {
            parameters,
            result: Some(lower::ValueType::I64),
        },
        role: lower::FunctionRole::Source,
        code: lower::Code {
            locals,
            instructions,
        },
    }
}

#[test]
fn pending_operands_survive_local_updates_calls_and_region_merges() {
    let _execution = EXECUTION.lock().unwrap();
    use lower::{Instruction as I, ScalarOp as S, ValueType as V};
    let program = instruction_program(
        vec![
            instruction_function(
                "pending_boundaries",
                vec![V::I32],
                vec![V::I64],
                vec![
                    I::I64(11),
                    I::LocalSet(1),
                    I::LocalGet(1),
                    I::I64(31),
                    I::LocalSet(1),
                    I::LocalGet(1),
                    I::Scalar(S::I64Add),
                    I::LocalTee(1),
                    I::Call(1),
                    I::LocalGet(0),
                    I::If(None),
                    I::I64(99),
                    I::LocalSet(1),
                    I::Else,
                    I::I32(-1),
                    I::Scalar(S::I64ExtendI32U),
                    I::LocalSet(1),
                    I::End,
                    I::Return,
                    I::End,
                ],
            ),
            instruction_function(
                "identity",
                vec![V::I64],
                vec![],
                vec![I::LocalGet(0), I::End],
            ),
        ],
        vec![],
    );
    for condition in [0, 1] {
        let image = Image::new("pending", &program, 0, vec![condition]);
        assert_eq!(packet(image.command().output().unwrap()), [0, 42, 0]);
    }
}

fn table_program() -> lower::Program {
    use lower::{Instruction as I, Signature};
    instruction_program(
        vec![
            instruction_function(
                "entry",
                vec![],
                vec![],
                vec![I::I64(1), I::CallIndirect(Signature::words(0)), I::End],
            ),
            // Public lowered IR is not a second safety boundary. Its dead local
            // instruction is omitted; mandatory K checking precedes real lowering.
            instruction_function("dead", vec![], vec![], vec![I::LocalGet(99), I::End]),
            instruction_function("table_target", vec![], vec![], vec![I::Call(3), I::End]),
            instruction_function("direct_callee", vec![], vec![], vec![I::I64(42), I::End]),
            instruction_function("other_table_slot", vec![], vec![], vec![I::I64(73), I::End]),
        ],
        vec![4, 2],
    )
}

#[test]
fn reachability_keeps_table_slots_and_their_direct_callees() {
    let _execution = EXECUTION.lock().unwrap();
    use lower::{Instruction as I, Signature};
    let mut program = table_program();
    let image = Image::new("table_second", &program, 0, vec![]);
    assert_eq!(packet(image.command().output().unwrap()), [0, 42, 0]);
    let original = std::fs::read(&image.0).unwrap();
    program.functions[1].code.instructions = vec![I::I64(0), I::End];
    let changed_dead_body = native::encode(
        &program,
        native::Entry {
            function: 0,
            arguments: vec![],
            result_fd: Some(2),
        },
    )
    .unwrap();
    assert_eq!(
        original, changed_dead_body,
        "dead bodies must not contribute code"
    );
    program.functions[0].code.instructions =
        vec![I::I64(0), I::CallIndirect(Signature::words(0)), I::End];
    let image = Image::new("table_first", &program, 0, vec![]);
    assert_eq!(packet(image.command().output().unwrap()), [0, 73, 0]);
}

#[test]
fn reachability_rejects_invalid_roots_table_entries_and_dead_direct_calls() {
    use lower::Instruction as I;
    let encode = |program: &lower::Program, function| {
        native::encode(
            program,
            native::Entry {
                function,
                arguments: vec![],
                result_fd: Some(2),
            },
        )
    };
    let mut program = table_program();
    assert_eq!(
        encode(&program, 99).unwrap_err(),
        "native entry is not a function"
    );
    assert_eq!(encode(&program, 1).unwrap_err(), "invalid lowered local");
    program.table.push(99);
    assert_eq!(
        encode(&program, 0).unwrap_err(),
        "invalid lowered table function"
    );
    program.table.pop();
    program.functions[1].code.instructions = vec![I::Call(99), I::End];
    assert_eq!(encode(&program, 0).unwrap_err(), "invalid lowered call");
}
