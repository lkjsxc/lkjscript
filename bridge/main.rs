//! Rust-only bootstrap host. Replace by lkjscript World once the compiler self-compiles.
use lkjscript::{
    ast::Diagnostic, check, diagnostics, elaborate, interpreter, ir, ir_text, resolve, source,
};
mod batch;
mod library;
mod native;
mod timing;
mod world;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

#[allow(unsafe_code)]
#[path = "process.rs"]
mod host_process;

fn report(path: &str, problem: Diagnostic) -> String {
    let path = if problem.path.is_empty() {
        path
    } else {
        &problem.path
    };
    format!(
        "{path}:{}:{}: {} {}",
        problem.line, problem.column, problem.code, problem.message
    )
}
fn located(program: &lkjscript::ast::Program, mut problem: Diagnostic) -> Diagnostic {
    if problem.path.is_empty() {
        if let Some(file) = program.files.get(problem.file) {
            problem.path = file.path.clone();
        }
    }
    problem
}
fn analysis_failure(
    path: &str,
    source: &mut source::SourceReport,
    problems: Vec<Diagnostic>,
    blocked: Option<&str>,
) -> String {
    source.diagnostics.extend(
        problems
            .into_iter()
            .map(|problem| located(&source.program, problem)),
    );
    if let Some(reason) = blocked {
        source.blocked.push(diagnostics::BlockedRegion {
            path: path.to_string(),
            span: lkjscript::ast::Span {
                file: 0,
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
            kind: "remaining semantic analysis".to_string(),
            reason: reason.to_string(),
        });
    }
    diagnostics::complete_unsupported(source);
    let mut messages: Vec<String> = source
        .diagnostics
        .drain(..)
        .map(|problem| report(path, problem))
        .collect();
    for blocked in &source.blocked {
        messages.push(format!(
            "{}:{}:{}: E_BLOCKED {}: {}",
            blocked.path, blocked.span.line, blocked.span.column, blocked.kind, blocked.reason
        ));
    }
    messages.join("\n")
}
fn compiler_work<T: Send>(work: impl FnOnce() -> Result<T, String> + Send) -> Result<T, String> {
    work()
}
fn compile(path: &str, timings: &mut timing::Timings) -> Result<ir::CheckedProgram, String> {
    compiler_work(|| compile_inner(path, timings))
}
fn compile_inner(path: &str, timings: &mut timing::Timings) -> Result<ir::CheckedProgram, String> {
    if Path::new(path)
        .extension()
        .is_some_and(|ext| ext == "lkjir")
    {
        let text = fs::read_to_string(path).map_err(|e| format!("{path}:1:1: E_IO {e}"))?;
        timings.count("source_bytes", text.len());
        let meaning = timings
            .measure("parse_ir", || ir_text::read(&text))
            .map_err(|e| report(path, e))?;
        timings.count(
            "ir_instructions",
            meaning.functions.iter().map(|f| f.body.len()).sum(),
        );
        return timings
            .measure("check", || check::check_all(meaning))
            .map_err(|problems| {
                problems
                    .into_iter()
                    .map(|problem| report(path, problem))
                    .collect::<Vec<_>>()
                    .join("\n")
            });
    }
    let library = library::standard_library();
    let mut sizes = std::collections::BTreeMap::new();
    let mut loaded = timings.measure("load_parse", || {
        source::load_report(path, Some("$std/lib.rs"), &mut |file| {
            let text = if let Some(input) = library.iter().find(|input| input.path == file) {
                Some(input.text.clone())
            } else {
                match fs::read_to_string(file) {
                    Ok(text) => Some(text),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(error) => return Err(error.to_string()),
                }
            };
            if let Some(text) = &text {
                sizes.insert(file.to_string(), text.len());
            }
            Ok(text)
        })
    });
    timings.count("source_bytes", sizes.values().sum());
    if !loaded.diagnostics.is_empty() {
        return Err(analysis_failure(path, &mut loaded, Vec::new(), None));
    }
    let resolved = match timings.measure("resolve", || resolve::resolve(&loaded.program)) {
        Ok(value) => value,
        Err(problem) => {
            return Err(analysis_failure(
                path,
                &mut loaded,
                vec![problem],
                Some(
                    "declaration resolution failed; dependent definition and IR analysis was not attempted",
                ),
            ));
        }
    };
    let definitions = match timings.measure("type", || elaborate::infer_resolved_report(&resolved))
    {
        Ok(value) => value,
        Err(problems) => {
            return Err(analysis_failure(
                path,
                &mut loaded,
                problems,
                Some(
                    "definition analysis failed; dependent IR construction and checking was not attempted",
                ),
            ));
        }
    };
    let meaning = match timings.measure("build_ir", || elaborate::lower_definitions(definitions)) {
        Ok(value) => value,
        Err(problems) => {
            return Err(analysis_failure(
                path,
                &mut loaded,
                problems,
                Some(
                    "failed bodies are blocked; independent typed bodies were constructed and checked",
                ),
            ));
        }
    };
    timings.count(
        "ir_instructions",
        meaning.functions.iter().map(|f| f.body.len()).sum(),
    );
    timings.count("instantiated_bodies", meaning.functions.len());
    let checked = match timings.measure("check", || check::check_all(meaning)) {
        Ok(value) => value,
        Err(problems) => {
            return Err(analysis_failure(
                path,
                &mut loaded,
                problems,
                Some("each invalid IR body stops at its diagnostic; no artifact is produced"),
            ));
        }
    };
    Ok(checked)
}
struct Arguments {
    verb: String,
    path: String,
    entry: Option<String>,
    output: Option<String>,
    engine: String,
    unchecked: bool,
    stats: bool,
    program_arguments: Vec<OsString>,
}
fn arguments(args: &[String]) -> Result<Arguments, String> {
    if args.len() < 2 {
        return Err("usage: lkjscript check|run|test|build|ir <source> [--stats] [--engine native|interpreter] [--entry name] [-o artifact] [-- program arguments]".to_string());
    }
    let mut result = Arguments {
        verb: args[1].clone(),
        path: String::new(),
        entry: None,
        output: None,
        engine: "native".to_string(),
        unchecked: false,
        stats: false,
        program_arguments: Vec::new(),
    };
    let mut at = 2;
    while at < args.len() {
        match args[at].as_str() {
            "--unchecked-ir" => result.unchecked = true,
            "--stats" => result.stats = true,
            "--entry" | "--engine" | "-o" => {
                let option = args[at].as_str();
                at += 1;
                let value = args
                    .get(at)
                    .ok_or_else(|| format!("E_USAGE {option} needs a value"))?
                    .clone();
                match option {
                    "--entry" => result.entry = Some(value),
                    "--engine" => result.engine = value,
                    _ => result.output = Some(value),
                }
            }
            value if !value.starts_with('-') && result.path.is_empty() => {
                result.path = value.to_string()
            }
            value => return Err(format!("E_USAGE unexpected argument {value}")),
        }
        at += 1;
    }
    if result.path.is_empty()
        || !["check", "run", "test", "build", "ir"].contains(&result.verb.as_str())
    {
        return Err("E_USAGE expected check|run|test|build|ir and a source path".to_string());
    }
    if !["interpreter", "native"].contains(&result.engine.as_str()) {
        return Err("E_USAGE engine is native or interpreter".to_string());
    }
    if result.unchecked
        && (result.engine != "interpreter"
            || !["run", "test"].contains(&result.verb.as_str())
            || Path::new(&result.path)
                .extension()
                .is_none_or(|ext| ext != "lkjir"))
    {
        return Err(
            "E_USAGE --unchecked-ir requires a .lkjir input, --engine interpreter, and run/test"
                .to_string(),
        );
    }
    Ok(result)
}
fn entry_world(program: &ir::Program, function: &ir::Function) -> Result<bool, String> {
    if !matches!(
        program.types[function.result].kind,
        ir::TypeKind::Unit
            | ir::TypeKind::Bool
            | ir::TypeKind::Char
            | ir::TypeKind::Int { .. }
            | ir::TypeKind::F64
    ) {
        return Err("E_ENTRY host entry result must be unit or a scalar".to_string());
    }
    if function.params == 0 {
        return Ok(false);
    }
    if function.params == 1
        && matches!(
            program.types[function.locals[0].ty].kind,
            ir::TypeKind::World
        )
    {
        return Ok(true);
    }
    Err("E_ENTRY entry takes no arguments or exactly one World".to_string())
}
fn entries(program: &ir::Program, args: &Arguments) -> Result<Vec<usize>, String> {
    let selected: Vec<_> = if args.verb == "test" {
        program
            .functions
            .iter()
            .enumerate()
            .filter_map(|(id, f)| f.test.then_some(id))
            .collect()
    } else {
        let entry = args.entry.clone().unwrap_or_else(|| {
            if program.functions.iter().any(|f| f.name == "main") {
                "main".to_string()
            } else {
                "probe".to_string()
            }
        });
        program
            .functions
            .iter()
            .position(|f| f.name == entry)
            .map(|id| vec![id])
            .ok_or_else(|| format!("E_ENTRY unknown function {entry}"))?
    };
    if !args.unchecked {
        for index in &selected {
            entry_world(program, &program.functions[*index])?;
        }
    }
    Ok(selected)
}
fn scalar(value: interpreter::Scalar) -> String {
    match value {
        interpreter::Scalar::Unit => String::new(),
        interpreter::Scalar::Bool(value) => format!("{value}\n"),
        interpreter::Scalar::Char(value) => format!("{value}\n"),
        interpreter::Scalar::Int {
            value,
            bits,
            signed,
        } => {
            if signed {
                let shift = 64 - u32::from(bits);
                format!("{}\n", ((value << shift) as i64) >> shift)
            } else {
                format!("{value}\n")
            }
        }
        interpreter::Scalar::F64 { bits } => format!("{}\n", f64::from_bits(bits)),
    }
}
fn execute_interpreter(
    program: &ir::Program,
    args: &Arguments,
    timings: &mut timing::Timings,
) -> Result<u8, String> {
    let mut status = 0;
    for entry in entries(program, args)? {
        let mut host = world::Host::stdio(&args.program_arguments);
        let result = timings.measure("interpreter", || {
            interpreter::run(program, entry, &[], &mut host)
        });
        if status == 0 {
            status = host.status;
        }
        let result =
            result.map_err(|e| format!("{}:{}: {} {}", args.path, e.line, e.code(), e.message))?;
        timings.count("interpreter_peak_frames", result.peak_frames);
        timings.count("interpreter_peak_cells", result.peak_cells);
        timings.count("interpreter_peak_stack_bytes", result.peak_stack_bytes);
        if args.verb != "test" {
            std::io::stdout()
                .write_all(scalar(result.value).as_bytes())
                .map_err(|error| format!("E_IO {error}"))?;
        }
    }
    Ok(status)
}
fn run() -> Result<u8, String> {
    let mut argv = vec!["lkjscript".to_string()];
    let mut program_arguments = Vec::new();
    let mut after_separator = false;
    for argument in std::env::args_os().skip(1) {
        if after_separator {
            program_arguments.push(argument);
        } else if argument == "--" {
            after_separator = true;
        } else {
            argv.push(argument.into_string().map_err(|_| {
                "E_USAGE compiler arguments must be UTF-8; program bytes follow --".to_string()
            })?);
        }
    }
    if !program_arguments.is_empty()
        && !argv
            .get(1)
            .is_some_and(|verb| verb == "run" || verb == "test")
    {
        return Err("E_USAGE program arguments after -- require run or test".to_string());
    }
    if argv.len() == 2 && (argv[1] == "--help" || argv[1] == "-h") {
        println!(
            "lkjscript check|build|run|test|ir <source.rs|program.lkjir> [options]\n\n  check  Check source or IR without emission\n  build  Write a static Linux x86-64 executable (-o PATH)\n  run    Execute one entry; arguments after -- belong to the program\n  test   Execute functions marked #[test]\n  ir     Write checked IR to stdout\n\n  --entry NAME                Select an entry (default main, then probe)\n  --engine native|interpreter Select execution route\n  --stats                     Report phase times, peak RSS and IR/code sizes\n  --unchecked-ir              Exercise the independent interpreter on unchecked IR\n  runtime-ir                  Write the sealed source runtime as IR\n\nEntry functions take no arguments or one World and return unit or a scalar.\nWorld derives arguments, stream and directory capabilities; file paths stay beneath the granted directory."
        );
        return Ok(0);
    }
    if argv.len() == 2 && argv[1] == "runtime-ir" {
        let text = compiler_work(|| {
            Ok(ir_text::write(
                lkjscript::bootstrap_runtime::get()?.checked_program(),
            ))
        })?;
        print!("{text}");
        return Ok(0);
    }
    if argv.get(1).is_some_and(|verb| verb == "batch") {
        return batch::run(&argv[2..]).map(|_| 0);
    }
    let mut args = arguments(&argv)?;
    args.program_arguments = program_arguments;
    let mut timings = timing::Timings::new(args.stats);
    if args.unchecked {
        let text = fs::read_to_string(&args.path).map_err(|e| format!("E_IO {e}"))?;
        let program = compiler_work(|| {
            timings
                .measure("parse_ir", || ir_text::read(&text))
                .map_err(|e| report(&args.path, e))
        })?;
        let status = execute_interpreter(&program, &args, &mut timings)?;
        timings.done();
        return Ok(status);
    }
    let checked = compile(&args.path, &mut timings)?;
    if timings.enabled() {
        timings.count("ir_bytes", ir_text::write(checked.program()).len());
    }
    if args.verb == "check" {
        timings.done();
        return Ok(0);
    }
    if args.verb == "ir" {
        let text = compiler_work(|| Ok(ir_text::write(checked.program())))?;
        print!("{text}");
        timings.done();
        return Ok(0);
    }
    if args.engine == "interpreter" && args.verb != "build" {
        let status = execute_interpreter(checked.program(), &args, &mut timings)?;
        timings.done();
        return Ok(status);
    }
    let status = native::run(&checked, &args, &mut timings)?;
    timings.done();
    Ok(status)
}

fn main() -> ExitCode {
    world::restore_closed_standard_streams();
    let result = run();
    match result {
        Ok(status) => ExitCode::from(status),
        Err(error) => {
            if !error.is_empty() {
                let _ = writeln!(std::io::stderr(), "{error}");
            }
            ExitCode::FAILURE
        }
    }
}
