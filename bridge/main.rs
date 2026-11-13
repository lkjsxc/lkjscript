//! Rust-only bootstrap host. Replace by lkjscript World once the compiler self-compiles.
use lkjscript::{
    ast::Diagnostic, check, diagnostics, elaborate, k, k_text, machine, resolve, source, wasm,
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
    let semantic = diagnostics::semantic_census(source);
    for (kind, count) in &semantic {
        println!("{kind}\t{count}\tsemantic");
    }
    println!("TOTAL\t{}\tsemantic", semantic.values().sum::<usize>());
    if !source.blocked.is_empty() {
        println!(
            "Method and trait body resolution remains incomplete in blocked regions. Missing imports may account for dependent names."
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
        return Err("usage: lkjscript check|run|test|build|k <source> [--census] [--engine native|k|node] [--entry name] [-o artifact]".to_string());
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
    if !["node", "k", "native"].contains(&result.engine.as_str()) {
        return Err("E_USAGE engine is node, k or native".to_string());
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
fn execute(module: &Path, entry: &str, kind: &str, world: bool) -> Result<(), String> {
    let mut command = Command::new("node");
    command
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("bridge/run.mjs"))
        .arg(module)
        .arg(entry)
        .arg(kind);
    if world {
        command.arg("1");
    }
    host_process::prepare_child(&mut command);
    // Inherited output preserves streaming/backpressure and detects host failure
    // inside the write operation. A failed engine already reports its diagnostic.
    let status = command.status().map_err(|e| format!("E_HOST Node: {e}"))?;
    if !status.success() {
        return Err(String::new());
    }
    Ok(())
}
fn scalar(value: machine::Scalar) -> String {
    match value {
        machine::Scalar::Unit => String::new(),
        machine::Scalar::Bool(value) => format!("{value}\n"),
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
fn run() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() == 2 && argv[1] == "trusted" {
        println!(
            "source meaning: src/syntax.rs, src/resolve.rs, src/generics.rs, src/elaborate/\nK safety gate: src/k.rs, src/check.rs\nlayout and lowering: src/lower.rs, src/lower/ir.rs\nsource runtime and sealed raw binding: library/runtime.rs, src/runtime.rs\nnative ELF encoder and Linux entry: src/native.rs\nWebAssembly encoder: src/wasm.rs\nindependent models: src/machine.rs, src/machine/scalar.rs, src/runtime/memory.rs\nRust host and declared source provisioning: bridge/\nRust output oracle: bridge/output.rs\nphysical size ledger: node tools/trust.mjs"
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
    if args.engine == "native" {
        native::run(&checked, &args, &mut timings)?;
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
