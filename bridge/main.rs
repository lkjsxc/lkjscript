//! Rust-only bootstrap host. Replace by lkjscript World once the compiler self-compiles.
use lkjscript::{
    ast::Diagnostic, check, diagnostics, elaborate, k, k_text, machine, resolve, source,
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
fn print_census(source: &source::SourceReport) {
    let census = diagnostics::census(source);
    println!("kind\tcount\tcategory");
    for (kind, count) in &census.unsupported {
        println!("{kind}\t{count}\tunsupported");
    }
    for (kind, count) in &census.blocked {
        println!("{kind}\t{count}\tblocked");
    }
    println!(
        "TOTAL\t{}\tunsupported",
        census.unsupported.values().sum::<usize>()
    );
    println!("TOTAL\t{}\tblocked", census.blocked.values().sum::<usize>());
    println!("\nsemantic dependency\tcount\tcategory");
    let report = diagnostics::semantic_report(source);
    let semantic = report.counts();
    for (kind, count) in &semantic {
        println!("{kind}\t{count}\tsemantic");
    }
    println!("TOTAL\t{}\tsemantic", semantic.values().sum::<usize>());
    println!("\nsemantic phase\tcount\tcategory");
    for (phase, count) in &report.coverage {
        println!("{phase}\t{count}\tcoverage");
    }
    for (phase, count) in report.blocked_counts() {
        println!("{phase}\t{count}\tblocked");
    }
    if !source.blocked.is_empty() || !report.blocked.is_empty() {
        println!(
            "Coverage counts each phase separately. Blocked analysis makes no claim that a name, method, or trait is absent."
        );
    }
}
fn analysis_failure(
    path: &str,
    source: &mut source::SourceReport,
    problems: Vec<Diagnostic>,
    census: bool,
    blocked: Option<&str>,
) -> String {
    source.diagnostics.extend(
        problems
            .into_iter()
            .map(|problem| located(&source.program, problem)),
    );
    if let Some(reason) = blocked {
        source.blocked.push(diagnostics::BlockedRegion {
            domain: None,
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
    if census {
        print_census(source);
    }
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
// Reserve the bootstrap compiler stack only while compiler work is live.
// Execution and stream capture continue after the thread has been joined.
fn compiler_work<T: Send>(work: impl FnOnce() -> Result<T, String> + Send) -> Result<T, String> {
    lkjscript::compiler_stack::run(work).map_err(|error| format!("E_HOST {error}"))?
}
fn compile(
    path: &str,
    census: bool,
    timings: &mut timing::Timings,
) -> Result<k::CheckedProgram, String> {
    compiler_work(|| compile_inner(path, census, timings))
}
fn compile_inner(
    path: &str,
    census: bool,
    timings: &mut timing::Timings,
) -> Result<k::CheckedProgram, String> {
    if Path::new(path).extension().is_some_and(|ext| ext == "k") {
        let text = fs::read_to_string(path).map_err(|e| format!("{path}:1:1: E_IO {e}"))?;
        timings.count("source_bytes", text.len());
        let meaning = timings
            .measure("parse_k", || k_text::read(&text))
            .map_err(|e| report(path, e))?;
        timings.count(
            "k_instructions",
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
        return Err(analysis_failure(
            path,
            &mut loaded,
            Vec::new(),
            census,
            None,
        ));
    }
    let resolved = match timings.measure("resolve", || resolve::resolve(&loaded.program)) {
        Ok(value) => value,
        Err(problem) => {
            return Err(analysis_failure(
                path,
                &mut loaded,
                vec![problem],
                census,
                Some(
                    "declaration resolution failed; dependent definition and K analysis was not attempted",
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
                census,
                Some(
                    "definition analysis failed; dependent K construction and checking was not attempted",
                ),
            ));
        }
    };
    let meaning = match timings.measure("build_k", || elaborate::lower_definitions(definitions)) {
        Ok(value) => value,
        Err(problems) => {
            return Err(analysis_failure(
                path,
                &mut loaded,
                problems,
                census,
                Some(
                    "failed bodies are blocked; independent typed bodies were constructed and checked",
                ),
            ));
        }
    };
    timings.count(
        "k_instructions",
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
                census,
                Some("each invalid K body stops at its diagnostic; no artifact is produced"),
            ));
        }
    };
    if census {
        print_census(&loaded);
    }
    Ok(checked)
}
struct Arguments {
    verb: String,
    path: String,
    entry: Option<String>,
    output: Option<String>,
    engine: String,
    census: bool,
    unchecked: bool,
    program_arguments: Vec<OsString>,
}
fn arguments(args: &[String]) -> Result<Arguments, String> {
    if args.len() < 2 {
        return Err("usage: lkjscript check|run|test|build|k <source> [--census] [--engine native|k] [--entry name] [-o artifact] [-- program arguments]".to_string());
    }
    let mut result = Arguments {
        verb: args[1].clone(),
        path: String::new(),
        entry: None,
        output: None,
        engine: "native".to_string(),
        census: false,
        unchecked: false,
        program_arguments: Vec::new(),
    };
    let mut at = 2;
    while at < args.len() {
        match args[at].as_str() {
            "--census" => result.census = true,
            "--unchecked-k" => result.unchecked = true,
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
        || !["check", "run", "test", "build", "k"].contains(&result.verb.as_str())
    {
        return Err("E_USAGE expected check|run|test|build|k and a source path".to_string());
    }
    if !["k", "native"].contains(&result.engine.as_str()) {
        return Err("E_USAGE engine is native or k".to_string());
    }
    if result.unchecked
        && (result.engine != "k"
            || !["run", "test"].contains(&result.verb.as_str())
            || Path::new(&result.path)
                .extension()
                .is_none_or(|ext| ext != "k"))
    {
        return Err(
            "E_USAGE --unchecked-k requires a .k input, --engine k, and run/test".to_string(),
        );
    }
    if result.census && result.verb != "check" {
        return Err("E_USAGE --census is a check option".to_string());
    }
    Ok(result)
}
fn entry_world(program: &k::Program, function: &k::Function) -> Result<bool, String> {
    if !matches!(
        program.types[function.result].kind,
        k::TypeKind::Unit
            | k::TypeKind::Bool
            | k::TypeKind::Char
            | k::TypeKind::Int { .. }
            | k::TypeKind::F64
    ) {
        return Err("E_ENTRY host entry result must be unit or a scalar".to_string());
    }
    if function.params == 0 {
        return Ok(false);
    }
    if function.params == 1
        && matches!(
            program.types[function.locals[0].ty].kind,
            k::TypeKind::World
        )
    {
        return Ok(true);
    }
    Err("E_ENTRY entry takes no arguments or exactly one World".to_string())
}
fn entries(program: &k::Program, args: &Arguments) -> Result<Vec<usize>, String> {
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
fn scalar(value: machine::Scalar) -> String {
    match value {
        machine::Scalar::Unit => String::new(),
        machine::Scalar::Bool(value) => format!("{value}\n"),
        machine::Scalar::Char(value) => format!("{value}\n"),
        machine::Scalar::Int {
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
        machine::Scalar::F64 { bits } => format!("{}\n", f64::from_bits(bits)),
    }
}
fn execute_machine(
    program: &k::Program,
    args: &Arguments,
    timings: &mut timing::Timings,
) -> Result<u8, String> {
    let mut status = 0;
    let resources = host_process::machine_resources()?;
    timings.count("machine_stack_extent_bytes", resources.stack_bytes);
    for entry in entries(program, args)? {
        let started = std::time::Instant::now();
        let mut host = world::Host::stdio(&args.program_arguments);
        let result = machine::run(program, entry, &[], &mut host, resources);
        if status == 0 {
            status = host.status;
        }
        if std::env::var("LKJSCRIPT_TIMINGS").as_deref() == Ok("1") {
            eprintln!(
                "LKJ_TIMING {{\"route\":\"k\",\"run_ms\":{},\"stopped_stage\":\"{}\"}}",
                started.elapsed().as_secs_f64() * 1000.0,
                if result.is_ok() { "done" } else { "run" }
            );
        }
        let result =
            result.map_err(|e| format!("{}:{}: {} {}", args.path, e.line, e.code(), e.message))?;
        timings.count("machine_peak_frames", result.peak_frames);
        timings.count("machine_peak_cells", result.peak_cells);
        timings.count("machine_peak_stack_bytes", result.peak_stack_bytes);
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
    if argv.len() == 2 && argv[1] == "runtime-k" {
        let text = compiler_work(|| {
            Ok(k_text::write(
                lkjscript::bootstrap_runtime::get()?.checked_program(),
            ))
        })?;
        print!("{text}");
        return Ok(0);
    }
    if argv.len() == 2 && argv[1] == "trusted" {
        println!(
            "source meaning: src/syntax.rs, src/resolve.rs, src/generics.rs, src/elaborate/\nK safety gate: src/k.rs, src/check.rs\nlayout and lowering: src/lower.rs, src/lower/\nsource runtime and sealed raw binding: library/runtime.rs, src/runtime.rs\nnative ELF encoder and Linux entry: src/native.rs, src/native/\nindependent models: src/machine.rs, src/machine/, src/runtime_memory.rs\nRust host and declared source provisioning: bridge/\nRust output oracle: bridge/output.rs\nphysical size ledger: node tools/trust.mjs"
        );
        return Ok(0);
    }
    if argv.get(1).is_some_and(|verb| verb == "batch") {
        return batch::run(&argv[2..]).map(|_| 0);
    }
    let mut args = arguments(&argv)?;
    args.program_arguments = program_arguments;
    let mut timings = timing::Timings::new();
    if args.unchecked {
        let text = fs::read_to_string(&args.path).map_err(|e| format!("E_IO {e}"))?;
        let program = compiler_work(|| {
            timings
                .measure("parse_k", || k_text::read(&text))
                .map_err(|e| report(&args.path, e))
        })?;
        let status = execute_machine(&program, &args, &mut timings)?;
        timings.done();
        return Ok(status);
    }
    let checked = compile(&args.path, args.census, &mut timings)?;
    if args.verb == "check" {
        timings.done();
        return Ok(0);
    }
    if args.verb == "k" {
        let text = compiler_work(|| Ok(k_text::write(checked.program())))?;
        print!("{text}");
        timings.done();
        return Ok(0);
    }
    if args.engine == "k" && args.verb != "build" {
        let status = execute_machine(checked.program(), &args, &mut timings)?;
        timings.done();
        return Ok(status);
    }
    let status = native::run(&checked, &args, &mut timings)?;
    timings.done();
    Ok(status)
}

fn main() -> ExitCode {
    world::restore_closed_standard_streams();
    match host_process::execution_worker() {
        Ok(Some(status)) => return status,
        Ok(None) => {}
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{error}");
            return ExitCode::FAILURE;
        }
    }
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
