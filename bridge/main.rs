//! Rust-only bootstrap host. Replace by lkjscript World once the compiler self-compiles.
use lkjscript::{
    ast::Diagnostic, check, diagnostics, elaborate, k, k_text, machine, resolve, source, wasm,
};
mod library;
mod timing;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Linux-only bootstrap host code; source programs cannot call these operations.
#[allow(unsafe_code)]
mod host_stack {
    use std::io;
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    #[derive(Clone, Copy)]
    #[repr(C)]
    pub struct Limits {
        current: u64,
        maximum: u64,
    }
    const RLIMIT_STACK: i32 = 3;
    unsafe extern "C" {
        fn getrlimit(resource: i32, limits: *mut Limits) -> i32;
        fn setrlimit(resource: i32, limits: *const Limits) -> i32;
        fn prctl(option: i32, ...) -> i32;
        fn getppid() -> i32;
    }

    pub fn configure(memory: usize) -> io::Result<Limits> {
        let mut original = Limits {
            current: 0,
            maximum: 0,
        };
        // Linux x86-64 rlimit is two u64 fields; the pointer is valid and writable.
        if unsafe { getrlimit(RLIMIT_STACK, &mut original) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let growing = Limits {
            current: (memory as u64).max(original.current).min(original.maximum),
            maximum: original.maximum,
        };
        // Preserve the host's hard extent. Linux commits stack pages on growth.
        if unsafe { setrlimit(RLIMIT_STACK, &growing) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(original)
    }

    pub fn prepare_child(command: &mut Command, original: Limits) {
        let parent = std::process::id() as i32;
        // Only async-signal-safe syscalls and error capture run after fork.
        // Node must not inherit an enormous default pthread stack reservation.
        unsafe {
            command.pre_exec(move || {
                // PR_SET_PDEATHSIG=1, SIGTERM=15. Killing a timed-out CLI must
                // also terminate Node and all of its in-process workers.
                if prctl(1, 15_usize, 0_usize, 0_usize, 0_usize) != 0 {
                    return Err(io::Error::last_os_error());
                }
                // Close the race where the parent died before prctl ran.
                if getppid() != parent {
                    return Err(io::Error::from_raw_os_error(3));
                }
                if setrlimit(RLIMIT_STACK, &original) == 0 {
                    Ok(())
                } else {
                    Err(io::Error::last_os_error())
                }
            });
        }
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
    if census {
        print_census(source);
    }
    for problem in source.diagnostics.drain(..) {
        eprintln!("{}", report(path, problem));
    }
    for blocked in &source.blocked {
        eprintln!(
            "{}:{}:{}: E_BLOCKED {}: {}",
            blocked.path, blocked.span.line, blocked.span.column, blocked.kind, blocked.reason
        );
    }
    String::new()
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
        if census {
            print_census(&loaded);
        }
        for problem in loaded.diagnostics {
            eprintln!("{}", report(path, problem));
        }
        for blocked in loaded.blocked {
            eprintln!(
                "{}:{}:{}: E_BLOCKED {}: {}",
                blocked.path, blocked.span.line, blocked.span.column, blocked.kind, blocked.reason
            );
        }
        return Err(String::new());
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
    let definitions = match timings.measure("type", || elaborate::infer_resolved_all(&resolved)) {
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
                    "K construction stopped at its diagnostic; dependent analysis was not attempted",
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
        return Err("usage: lkjscript check|run|test|build|k <source> [--census] [--engine node|k] [--entry name] [-o module.wasm]".to_string());
    }
    let mut result = Arguments {
        verb: args[1].clone(),
        path: String::new(),
        entry: None,
        output: None,
        engine: "node".to_string(),
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
    if !["node", "k"].contains(&result.engine.as_str()) {
        return Err("E_USAGE engine is node or k".to_string());
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
    if args.verb == "test" {
        return Ok(program
            .functions
            .iter()
            .enumerate()
            .filter_map(|(id, f)| f.test.then_some(id))
            .collect());
    }
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
        .ok_or_else(|| format!("E_ENTRY unknown function {entry}"))
}
fn result_kind(program: &k::Program, function: &k::Function) -> &'static str {
    match program.types[function.result].kind {
        k::TypeKind::Unit => "unit",
        k::TypeKind::Bool => "bool",
        k::TypeKind::F64 => "f64",
        k::TypeKind::Int { signed: false, .. } => "u64",
        _ => "i64",
    }
}
fn execute(
    module: &Path,
    entry: &str,
    kind: &str,
    world: bool,
    child_stack: host_stack::Limits,
) -> Result<(), String> {
    let mut command = Command::new("node");
    command
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("bridge/run.mjs"))
        .arg(module)
        .arg(entry)
        .arg(kind);
    if world {
        command.arg("1");
    }
    host_stack::prepare_child(&mut command, child_stack);
    // Inherited output preserves streaming/backpressure and detects host failure
    // inside the write operation. A failed worker already reports its diagnostic.
    let status = command.status().map_err(|e| format!("E_HOST Node: {e}"))?;
    if !status.success() {
        return Err(String::new());
    }
    Ok(())
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
            match result.value {
                machine::Scalar::Unit => {}
                machine::Scalar::Bool(value) => println!("{value}"),
                machine::Scalar::Int {
                    value,
                    bits,
                    signed,
                } => {
                    if signed {
                        let shift = 64 - u32::from(bits);
                        println!("{}", ((value << shift) as i64) >> shift);
                    } else {
                        println!("{value}");
                    }
                }
                machine::Scalar::F64 { bits } => println!("{}", f64::from_bits(bits)),
            }
        }
    }
    Ok(())
}
fn run(child_stack: host_stack::Limits) -> Result<(), String> {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() == 2 && argv[1] == "trusted" {
        println!(
            "source/resolve/type/K: src/\nindependent K interpreter: src/machine.rs, src/machine/scalar.rs\nRust host, resource setup and source packing: bridge/lib.rs, bridge/compiler_stack.rs, bridge/main.rs, bridge/library.rs\nNode host and byte boundary: bridge/run.mjs, bridge/output.mjs\nRust output oracle: bridge/output.rs"
        );
        return Ok(());
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
    let bytes = timings
        .measure("emit", || wasm::emit(&checked))
        .map_err(|e| format!("{}:1:1: E_EMIT {e}", args.path))?;
    timings.count("emitted_bytes", bytes.len());
    let temporary = args.verb != "build";
    let output = if temporary {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/run");
        fs::create_dir_all(&directory).map_err(|e| format!("E_IO {e}"))?;
        directory.join(format!("{}.wasm", std::process::id()))
    } else {
        PathBuf::from(args.output.clone().unwrap_or_else(|| {
            Path::new(&args.path)
                .with_extension("wasm")
                .to_string_lossy()
                .into_owned()
        }))
    };
    fs::write(&output, bytes).map_err(|e| format!("E_IO {}: {e}", output.display()))?;
    let result: Result<(), String> = if args.verb == "build" {
        Ok(())
    } else {
        let program = checked.program();
        (|| {
            for entry in entries(program, &args)? {
                let function = &program.functions[entry];
                execute(
                    &output,
                    &function.name,
                    if args.verb == "test" {
                        "unit"
                    } else {
                        result_kind(program, function)
                    },
                    entry_world(program, function)?,
                    child_stack,
                )?;
            }
            Ok(())
        })()
    };
    if temporary {
        let _ = fs::remove_file(&output);
    }
    result?;
    timings.done();
    Ok(())
}
fn main() -> ExitCode {
    // Grow the main Linux stack from physical memory and swap, preserving the
    // host hard extent. Unlike a pthread stack, its mapping expands on demand.
    let memory = fs::read_to_string("/proc/meminfo").ok().and_then(|s| {
        let mut total = 0_usize;
        for line in s.lines() {
            if line.starts_with("MemTotal:") || line.starts_with("SwapTotal:") {
                total = total.checked_add(
                    line.split_whitespace()
                        .nth(1)?
                        .parse::<usize>()
                        .ok()?
                        .checked_mul(1024)?,
                )?;
            }
        }
        if total == 0 { None } else { Some(total) }
    });
    let result = memory
        .ok_or_else(|| "E_HOST cannot read memory and swap extents from /proc/meminfo".to_string())
        .and_then(|memory| host_stack::configure(memory).map_err(|error| format!("E_HOST {error}")))
        .and_then(run);
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
