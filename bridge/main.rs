//! Rust-only bootstrap host. Replace by lkjscript World once the compiler self-compiles.
use lkjscript::{
    ast::Diagnostic, check, diagnostics, elaborate, k, k_text, machine, resolve, source,
};
mod batch;
mod library;
mod native;
mod timing;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Bootstrap child lifetime management; source programs cannot call these operations.
#[allow(unsafe_code)]
mod host_process {
    use std::io;
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    unsafe extern "C" {
        fn prctl(option: i32, ...) -> i32;
        fn getppid() -> i32;
        fn prlimit(pid: i32, resource: u32, new: *const Limit, old: *mut Limit) -> i32;
    }
    #[repr(C)]
    struct Limit {
        current: u64,
        maximum: u64,
    }

    /// A runner budget is host policy, not a property of the accepted source.
    /// An explicit environment setting can enlarge it or inherit Linux's full
    /// extent; a tighter limit imposed by the operator always remains effective.
    pub fn prepare_native_child(command: &mut Command) -> Result<(), String> {
        prepare_budgeted_child(command, 268435456).map(|_| ())
    }
    fn prepare_budgeted_child(command: &mut Command, default: u64) -> Result<u64, String> {
        let text = std::env::var("LKJ_RUN_MEMORY_BYTES").unwrap_or_else(|_| default.to_string());
        let budget = if text == "unlimited" {
            u64::MAX
        } else {
            text.parse::<u64>().ok().filter(|n| *n > 0).ok_or_else(|| {
                "E_HOST LKJ_RUN_MEMORY_BYTES must be positive bytes or unlimited".to_string()
            })?
        };
        prepare_child(command);
        unsafe {
            command.pre_exec(move || {
                let mut limit = Limit {
                    current: 0,
                    maximum: 0,
                };
                if prlimit(0, 9, std::ptr::null(), &mut limit) != 0 {
                    return Err(io::Error::last_os_error());
                }
                let no_core = Limit {
                    current: 0,
                    maximum: 0,
                };
                if prlimit(0, 4, &no_core, std::ptr::null_mut()) != 0 {
                    return Err(io::Error::last_os_error());
                }
                limit.current = limit.current.min(budget);
                if prlimit(0, 9, &limit, std::ptr::null_mut()) != 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        Ok(budget)
    }

    pub fn prepare_child(command: &mut Command) {
        let parent = std::process::id() as i32;
        // Only async-signal-safe syscalls run after fork. Process-group timeout
        // handling normally kills children too; this also covers parent death.
        unsafe {
            command.pre_exec(move || {
                if prctl(1, 15_usize, 0_usize, 0_usize, 0_usize) != 0 {
                    return Err(io::Error::last_os_error());
                }
                if getppid() != parent {
                    return Err(io::Error::from_raw_os_error(3));
                }
                Ok(())
            });
        }
    }

    /// The Rust K model runs in a fresh process so its allocator cannot consume
    /// the compiler host. Compile/check-only commands have no runner budget.
    /// Batch keeps its streaming protocol and gets one process envelope.
    pub fn model_worker() -> Result<Option<std::process::ExitCode>, String> {
        use std::io::Write;
        use std::os::unix::process::ExitStatusExt;
        use std::process::{ExitCode, Stdio};
        if std::env::var("LKJ_MODEL_WORKER").as_deref() == Ok("1") {
            return Ok(None);
        }
        let argv = std::env::args().skip(1).collect::<Vec<_>>();
        let Some(verb) = argv.first() else {
            return Ok(None);
        };
        let selected = argv.windows(2).find_map(|pair| {
            if pair[0] == "--engine" || pair[0] == "--engines" {
                Some(pair[1].as_str())
            } else {
                None
            }
        });
        let runs_model = if verb == "batch" {
            selected
                .unwrap_or("k")
                .split(',')
                .any(|engine| engine == "k")
        } else {
            (verb == "run" || verb == "test") && selected == Some("k")
        };
        if !runs_model {
            return Ok(None);
        }
        let executable = std::env::current_exe().map_err(|error| format!("E_HOST {error}"))?;
        let mut command = Command::new(executable);
        command
            .args(&argv)
            .env("LKJ_MODEL_WORKER", "1")
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::piped());
        let budget = prepare_budgeted_child(&mut command, 536870912)?;
        let output = command
            .output()
            .map_err(|error| format!("E_HOST K worker: {error}"))?;
        std::io::stderr()
            .write_all(&output.stderr)
            .map_err(|error| format!("E_HOST {error}"))?;
        if let Some(signal) = output.status.signal() {
            let text = String::from_utf8_lossy(&output.stderr);
            if signal == 6
                && text.contains("memory allocation of ")
                && text.contains(" bytes failed")
            {
                return Err(format!(
                    "E_HOST K worker exhausted its Linux address-space budget ({budget} bytes); Rust allocator failure has no source location"
                ));
            }
            return Err(format!(
                "E_HOST K worker terminated by signal {signal} under Linux address-space budget {budget} bytes"
            ));
        }
        Ok(Some(
            ExitCode::from(output.status.code().unwrap_or(1) as u8),
        ))
    }
}

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
fn compile(
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
}
fn arguments(args: &[String]) -> Result<Arguments, String> {
    if args.len() < 2 {
        return Err("usage: lkjscript check|run|test|build|k <source> [--census] [--engine native|k] [--entry name] [-o artifact]".to_string());
    }
    let mut result = Arguments {
        verb: args[1].clone(),
        path: String::new(),
        entry: None,
        output: None,
        engine: "native".to_string(),
        census: false,
        unchecked: false,
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
struct StandardOutput;
impl machine::Host for StandardOutput {
    fn write_stdout(&mut self, bytes: &[u8]) -> Result<(), String> {
        std::io::stdout()
            .write_all(bytes)
            .map_err(|e| e.to_string())
    }
}
fn execute_machine(
    program: &k::Program,
    args: &Arguments,
    timings: &mut timing::Timings,
) -> Result<(), String> {
    for entry in entries(program, args)? {
        let started = std::time::Instant::now();
        let result = machine::run(program, entry, &[], &mut StandardOutput);
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
        if args.verb != "test" {
            std::io::stdout()
                .write_all(scalar(result.value).as_bytes())
                .map_err(|error| format!("E_IO {error}"))?;
        }
    }
    Ok(())
}
fn run() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() == 2 && argv[1] == "runtime-k" {
        print!(
            "{}",
            k_text::write(lkjscript::bootstrap_runtime::get()?.checked_program())
        );
        return Ok(());
    }
    if argv.len() == 2 && argv[1] == "trusted" {
        println!(
            "source meaning: src/syntax.rs, src/resolve.rs, src/generics.rs, src/elaborate/\nK safety gate: src/k.rs, src/check.rs\nlayout and lowering: src/lower.rs, src/lower/ir.rs\nsource runtime and sealed raw binding: library/runtime.rs, src/runtime.rs\nnative ELF encoder and Linux entry: src/native.rs\nindependent models: src/machine.rs, src/machine/scalar.rs, src/runtime_memory.rs\nRust host and declared source provisioning: bridge/\nRust output oracle: bridge/output.rs\nphysical size ledger: node tools/trust.mjs"
        );
        return Ok(());
    }
    if argv.get(1).is_some_and(|verb| verb == "batch") {
        return batch::run(&argv[2..]);
    }
    let args = arguments(&argv)?;
    let mut timings = timing::Timings::new();
    if args.unchecked {
        let text = fs::read_to_string(&args.path).map_err(|e| format!("E_IO {e}"))?;
        let program = timings
            .measure("parse_k", || k_text::read(&text))
            .map_err(|e| report(&args.path, e))?;
        execute_machine(&program, &args, &mut timings)?;
        timings.done();
        return Ok(());
    }
    let checked = compile(&args.path, args.census, &mut timings)?;
    if args.verb == "check" {
        timings.done();
        return Ok(());
    }
    if args.verb == "k" {
        print!("{}", k_text::write(checked.program()));
        timings.done();
        return Ok(());
    }
    if args.engine == "k" && args.verb != "build" {
        execute_machine(checked.program(), &args, &mut timings)?;
        timings.done();
        return Ok(());
    }
    native::run(&checked, &args, &mut timings)?;
    timings.done();
    Ok(())
}

fn main() -> ExitCode {
    match host_process::model_worker() {
        Ok(Some(status)) => return status,
        Ok(None) => {}
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    }
    let result = lkjscript::compiler_stack::run(run)
        .map_err(|error| format!("E_HOST {error}"))
        .and_then(|result| result);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if !error.is_empty() {
                eprintln!("{error}");
            }
            ExitCode::FAILURE
        }
    }
}
