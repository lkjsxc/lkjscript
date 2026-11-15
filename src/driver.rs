// Command line and orchestration for the complete owned toolchain.
use crate::{
    ast, backend, check, driver_host, elaborate, embedded, interpreter, ir, ir_text, json, runtime,
    source, reuse, store, sha256,
};
use crate::{diagnostics, resolve};
use std::collections::BTreeMap;

#[derive(Clone)]
struct Options {
    command: String,
    path: String,
    entry: Option<String>,
    grants: Vec<String>,
    output: Option<String>,
    engine: String,
    engines: Vec<String>,
    stats: bool,
    profile: bool,
    roundtrip: bool,
    jobs: usize,
    witness: Option<String>,
    tool_arguments: Vec<String>,
    unchecked: bool,
    source_directory: Option<String>,
    library_directory: Option<String>,
    runtime_source: Option<String>,
    store_directory: Option<String>,
    store_key: Option<String>,
    no_library: bool,
    arguments: Vec<Vec<u8>>,
}

fn usage() -> &'static str {
    "lkjscript check|build|run|test|ir|authority <source.rs|program.lkjir> [options]\n\n  check  Check all source or input IR\n  build  Write a static Linux x86-64 executable\n  run    Execute main (or probe); bytes after -- are program arguments\n  test   Run #[test] functions through native and interpreter routes\n  ir     Write checked lkjscript IR\n  authority  Report entry grants and reachable checked-IR effects\n\n  --entry NAME                 Select an entry\n  --grant NAME[=VALUE]          Grant an owned entry capability (repeatable)\n  --engine native|interpreter  Select the execution route\n  -o PATH                     Write an artifact or IR\n  --stats                     Report phase times, counts and peak RSS\n  --profile                   Sample CPU to OUTPUT.samples.PID in built programs\n  profile SAMPLES             Report top self and inclusive functions\n  --jobs N                    Native test workers (1..8)\n  --witness RUSTC             Compare marked cases with optional rustc\n  --source-dir DIR            Grant a root for source reads\n  --library-dir DIR           Use explicit library sources\n  --no-library                Compile without the source library\n  --runtime-source PATH       Use an explicit source runtime\n  --store DIR                 Reuse authenticated work beneath this Directory\n  --store-key PATH            Grant a private 32-byte key outside source roots\n  --no-store                  Disable stored reuse\n  --unchecked-ir              Interpret raw IR for checker testing\n  runtime-ir                  Write the checked source runtime as IR\n  check-library               Check every source library body\n\nScalar runs print the value; f64 prints exact bits as f64:0x0123456789abcdef.\n"
}

fn options(bytes: Vec<Vec<u8>>) -> Result<Options, String> {
    let mut values: Vec<String> = Vec::new();
    let mut arguments: Vec<Vec<u8>> = Vec::new();
    let mut after = false;
    for value in bytes {
        if after {
            arguments.push(value);
        } else if value.len() == 2 && value[0] == b'-' && value[1] == b'-' {
            after = true;
        } else {
            match String::from_utf8(value) {
                Ok(text) => values.push(text),
                Err(_) => {
                    return Err(
                        "E_USAGE compiler arguments must be UTF-8; program bytes follow --"
                            .to_string(),
                    );
                }
            }
        }
    }
    let command = if values.is_empty() {
        "--help".to_string()
    } else {
        values[0].clone()
    };
    let mut out = Options {
        command,
        path: String::new(),
        entry: None,
        grants: Vec::new(),
        output: None,
        engine: "native".to_string(),
        engines: vec!["native".to_string(), "interpreter".to_string()],
        stats: false,
        profile: false,
        roundtrip: false,
        jobs: 1,
        witness: None,
        tool_arguments: Vec::new(),
        unchecked: false,
        source_directory: None,
        library_directory: None,
        runtime_source: None,
        store_directory: None,
        store_key: None,
        no_library: false,
        arguments,
    };
    if out.command == "generate" || out.command == "mutate" || out.command == "reduce" || out.command == "mutate-run" {
        out.tool_arguments = values;
        return Ok(out);
    }
    let mut at = 1usize;
    while at < values.len() {
        let option = values[at].clone();
        match option.as_str() {
            "--stats" => out.stats = true,
            "--profile" => out.profile = true,
            "--roundtrip" => out.roundtrip = true,
            "--unchecked-ir" => out.unchecked = true,
            "--no-library" => out.no_library = true,
            "--no-store" => { out.store_directory = None; out.store_key = None; },
            "--entry" | "-o" | "--engine" | "--engines" | "--source-dir" | "--library-dir"
            | "--runtime-source" | "--jobs" | "--witness" | "--grant" | "--store" | "--store-key" => {
                at += 1;
                if at == values.len() {
                    return Err(format!("E_USAGE {option} needs a value"));
                }
                let value = values[at].clone();
                match option.as_str() {
                    "--entry" => out.entry = Some(value),
                    "--grant" => out.grants.push(value),
                    "-o" => out.output = Some(value),
                    "--engine" => {
                        out.engine = value.clone();
                        out.engines = vec![value];
                    }
                    "--engines" => {
                        out.engines.clear();
                        for name in split(&value, b',', 0) {
                            out.engines.push(name);
                        }
                    }
                    "--jobs" => {
                        out.jobs = 0;
                        for byte in value.as_bytes() {
                            if *byte < b'0' || *byte > b'9' || out.jobs > 8 { return Err("E_USAGE --jobs must be between 1 and 8".to_string()); }
                            out.jobs = out.jobs * 10 + (*byte - b'0') as usize;
                        }
                        if out.jobs == 0 || out.jobs > 8 { return Err("E_USAGE --jobs must be between 1 and 8".to_string()); }
                    }
                    "--witness" => out.witness = Some(value),
                    "--source-dir" => out.source_directory = Some(value),
                    "--library-dir" => out.library_directory = Some(value),
                    "--store" => out.store_directory = Some(value),
                    "--store-key" => out.store_key = Some(value),
                    _ => out.runtime_source = Some(value),
                }
            }
            _ => {
                if option.starts_with('-') || !out.path.is_empty() {
                    return Err(format!("E_USAGE unexpected argument {option}"));
                }
                out.path = option.clone();
            }
        }
        at += 1;
    }
    if out.command == "--help" || out.command == "-h" {
        return Ok(out);
    }
    if out.command != "runtime-ir"
        && out.command != "check-library"
        && out.command != "check"
        && out.command != "build"
        && out.command != "run"
        && out.command != "test"
        && out.command != "ir"
        && out.command != "batch"
        && out.command != "profile"
        && out.command != "authority"
    {
        return Err("E_USAGE expected check|build|run|test|ir|authority".to_string());
    }
    if out.command == "test" && out.path.is_empty() { out.path = "tests/conformance".to_string(); }
    if out.witness.is_some() && out.command != "test" { return Err("E_USAGE --witness requires test".to_string()); }
    if !out.grants.is_empty() && out.command != "build" && out.command != "run" { return Err("E_USAGE --grant requires build or run".to_string()); }
    if out.profile && out.command != "build" { return Err("E_USAGE --profile requires build".to_string()); }
    if out.command != "runtime-ir" && out.command != "check-library" && out.path.is_empty() {
        return Err("E_USAGE a source or IR path is required".to_string());
    }
    if out.engine != "native" && out.engine != "interpreter" {
        return Err("E_USAGE engine is native or interpreter".to_string());
    }
    for (index, name) in out.engines.iter().enumerate() {
        if name != "native" && name != "interpreter" {
            return Err("E_USAGE engines are native,interpreter".to_string());
        }
        for earlier in &out.engines[..index] {
            if earlier == name {
                return Err("E_USAGE duplicate engine".to_string());
            }
        }
    }
    if out.engines.is_empty() {
        return Err("E_USAGE at least one engine is required".to_string());
    }
    if out.unchecked
        && (out.engine != "interpreter"
            || !out.path.ends_with(".lkjir")
            || (out.command != "run" && out.command != "test"))
    {
        return Err(
            "E_USAGE --unchecked-ir requires .lkjir input and interpreter run/test".to_string(),
        );
    }
    if !out.arguments.is_empty() && out.command != "run" && out.command != "test" {
        return Err("E_USAGE program arguments require run or test".to_string());
    }
    if out.no_library && out.command == "check-library" {
        return Err("E_USAGE check-library requires the source library".to_string());
    }
    if out.no_library && out.library_directory.is_some() {
        return Err("E_USAGE choose --no-library or --library-dir".to_string());
    }
    if out.command == "check-library" {
        out.path = "$toolchain/library-check.rs".to_string();
    }
    Ok(out)
}

fn io(path: &str, operation: &str, errno: i64) -> String {
    format!("{path}:1:1: E_IO {operation} failed (errno {errno})")
}

fn location(path: &str) -> (String, String) {
    let mut last = None;
    for (index, byte) in path.as_bytes().iter().enumerate() {
        if *byte == b'/' {
            last = Some(index);
        }
    }
    match last {
        Some(0) => ("/".to_string(), path[1..].to_string()),
        Some(index) => (path[..index].to_string(), path[index + 1..].to_string()),
        None => (".".to_string(), path.to_string()),
    }
}

fn read(directory: &mut Directory, path: &str) -> Result<Option<Vec<u8>>, String> {
    let mut file = match directory.open(path) {
        Ok(file) => file,
        Err(errno) => {
            if errno == 2 || errno == 20 {
                return Ok(None);
            }
            return Err(io(path, "open", errno));
        }
    };
    Ok(Some(
        file.read_to_end()
            .map_err(|errno: i64| io(path, "read", errno))?,
    ))
}

fn read_path(world: &mut World, path: &str) -> Result<Vec<u8>, String> {
    let (parent, name) = location(path);
    let mut directory = world
        .directory(&parent)
        .map_err(|errno: i64| io(&parent, "open directory", errno))?;
    read(&mut directory, &name)?.ok_or_else(|| io(path, "open", 2))
}

fn text(bytes: Vec<u8>, path: &str) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|_| format!("{path}:1:1: E_IO input is not valid UTF-8"))
}

fn report(path: &str, problem: ast::Diagnostic) -> String {
    let label = if problem.path.is_empty() {
        path.to_string()
    } else {
        problem.path
    };
    format!(
        "{label}:{}:{}: {} {}",
        problem.line, problem.column, problem.code, problem.message
    )
}
fn located(path: &str, program: &ast::Program, mut problem: ast::Diagnostic) -> String {
    if problem.path.is_empty() {
        if let Some(file) = program.files.get(problem.file) {
            problem.path = file.path.clone();
        }
    }
    report(path, problem)
}

fn join_lines(lines: &[String]) -> String {
    let mut text = String::new();
    for (index, line) in lines.iter().enumerate() {
        if index != 0 {
            text.push('\n');
        }
        text.push_str(line);
    }
    text
}

fn analysis_failure(
    path: &str,
    source: &mut source::SourceReport,
    problems: Vec<ast::Diagnostic>,
    blocked: &str,
) -> String {
    for mut problem in problems {
        if problem.path.is_empty() {
            if let Some(file) = source.program.files.get(problem.file) {
                problem.path = file.path.clone();
            }
        }
        source.diagnostics.push(problem);
    }
    diagnostics::complete_unsupported(source);
    let mut messages: Vec<String> = Vec::new();
    for problem in &source.diagnostics {
        messages.push(report(path, problem.clone()));
    }
    for region in &source.blocked {
        messages.push(format!(
            "{}:{}:{}: E_BLOCKED {}: {}",
            region.path, region.span.line, region.span.column, region.kind, region.reason
        ));
    }
    if !blocked.is_empty() {
        messages.push(format!(
            "{path}:1:1: E_BLOCKED remaining semantic analysis: {blocked}"
        ));
    }
    join_lines(&messages)
}
fn ir_failure(path: &str, problems: Vec<ast::Diagnostic>) -> String {
    let mut messages = Vec::new();
    for problem in problems {
        messages.push(report(path, problem));
    }
    join_lines(&messages)
}

fn milliseconds(nanoseconds: usize) -> String {
    format!(
        "{}.{:03}",
        nanoseconds / 1000000,
        (nanoseconds % 1000000) / 1000
    )
}
fn split(text: &str, separator: u8, fields: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for (index, byte) in text.as_bytes().iter().enumerate() {
        if *byte == separator && (fields == 0 || out.len() + 1 < fields) {
            out.push(text[start..index].to_string());
            start = index + 1;
        }
    }
    out.push(text[start..].to_string());
    out
}

struct Stats {
    enabled: bool,
    clock: Clock,
    started: usize,
    previous: usize,
    times: BTreeMap<String, usize>,
    counts: BTreeMap<String, usize>,
}
impl Stats {
    fn new(world: &mut World, enabled: bool) -> Result<Stats, String> {
        let mut clock = world.clock();
        let started = if enabled {
            clock
                .nanoseconds()
                .map_err(|errno: i64| io("clock", "read", errno))?
        } else {
            0
        };
        Ok(Stats {
            enabled,
            clock,
            started,
            previous: started,
            times: BTreeMap::new(),
            counts: BTreeMap::new(),
        })
    }
    fn mark(&mut self, stage: &str) -> Result<(), String> {
        if self.enabled {
            let now = self
                .clock
                .nanoseconds()
                .map_err(|errno: i64| io("clock", "read", errno))?;
            let key = stage.to_string();
            let before = self.times.get(&key).copied().unwrap_or(0);
            self.times.insert(key, before + now - self.previous);
            self.previous = now;
        }
        Ok(())
    }
    fn count(&mut self, name: &str, value: usize) {
        if self.enabled {
            self.counts.insert(name.to_string(), value);
        }
    }
    fn finish(&mut self, world: &mut World, stopped: &str) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        let now = self
            .clock
            .nanoseconds()
            .map_err(|errno: i64| io("clock", "read", errno))?;
        let mut out = format!(
            "LKJSCRIPT_STATS {{\"route\":\"compiler\",\"stopped_stage\":\"{stopped}\",\"total_ms\":{}",
            milliseconds(now - self.started)
        );
        for (name, value) in &self.times {
            out.push_str(&format!(",\"{name}_ms\":{}", milliseconds(*value)));
        }
        for (name, value) in &self.counts {
            out.push_str(&format!(",\"{name}\":{value}"));
        }
        if let Ok(bytes) = read_path(world, "/proc/self/status") {
            if let Ok(status) = String::from_utf8(bytes) {
                for line in split(&status, b'\n', 0) {
                    if line.starts_with("VmHWM:") {
                        let mut kib = 0usize;
                        for byte in line.as_bytes() {
                            if *byte >= b'0' && *byte <= b'9' {
                                kib = kib * 10 + (*byte - b'0') as usize;
                            }
                        }
                        out.push_str(&format!(",\"peak_rss_bytes\":{}", kib * 1024));
                    }
                }
            }
        }
        out.push_str("}\n");
        world.stderr().write(out.as_bytes());
        Ok(())
    }
}

fn compile(
    world: &mut World,
    options: &Options,
    stats: &mut Stats,
    memo: &mut Memo,
) -> Result<reuse::Checked, String> {
    memo.inputs.clear();
    memo.valid_inputs = true;
    if options.path.ends_with(".lkjir") {
        let bytes = read_path(world, &options.path)?;
        if memo.store.is_some() {
            let (parent, name) = location(&options.path);
            memo.inputs.push(store::input(&parent, &name, &Some(bytes.clone())));
        }
        let input = text(bytes, &options.path)?;
        let meaning = ir_text::read(&input).map_err(|problem| report(&options.path, problem))?;
        stats.mark("parse_ir")?;
        let checked = check_reusing(meaning, memo, options.roundtrip)
            .map_err(|problems: Vec<ast::Diagnostic>| ir_failure(&options.path, problems))?;
        stats.mark("check")?;
        return Ok(checked);
    }
    let (directory_path, root): (String, String) = match &options.source_directory {
        Some(path) => (path.to_string(), options.path.clone()),
        None => {
            if options.path.starts_with('/') {
                ("/".to_string(), options.path.clone())
            } else {
                (".".to_string(), options.path.clone())
            }
        }
    };
    let mut directory = world
        .directory(&directory_path)
        .map_err(|errno: i64| io(&directory_path, "open directory", errno))?;
    let mut library: Option<Directory> = None;
    if let Some(path) = &options.library_directory {
        library = Some(match world.directory(path) {
            Ok(directory) => directory,
            Err(errno) => return Err(io(path, "open library", errno)),
        });
    }
    let standard = if options.no_library {
        None
    } else {
        Some("$std/lib.rs")
    };
    let absolute_source = options.source_directory.is_none() && options.path.starts_with('/');
    let mut loaded = source::load_report(&root, standard, &mut |path: &str| {
        if options.command == "check-library" && path == "$toolchain/library-check.rs" {
            Ok(Some("fn main() {}".as_bytes().to_vec()))
        } else if path.starts_with("$std/") {
            match library.as_mut() {
                Some(directory) => {
                    let result = read(directory, &path[5..]);
                    if memo.store.is_some() {
                        match &result {
                            Ok(bytes) => memo.inputs.push(store::input(options.library_directory.as_ref().unwrap(), &path[5..], bytes)),
                            Err(_) => memo.valid_inputs = false,
                        }
                    }
                    result
                },
                None => Ok(embedded::library(path)),
            }
        } else if absolute_source && path.starts_with('/') {
            let result = read(&mut directory, &path[1..]);
            if memo.store.is_some() {
                match &result {
                    Ok(bytes) => memo.inputs.push(store::input(&directory_path, &path[1..], bytes)),
                    Err(_) => memo.valid_inputs = false,
                }
            }
            result
        } else {
            let result = read(&mut directory, path);
            if memo.store.is_some() {
                match &result {
                    Ok(bytes) => memo.inputs.push(store::input(&directory_path, path, bytes)),
                    Err(_) => memo.valid_inputs = false,
                }
            }
            result
        }
    });
    stats.mark("load_parse")?;
    if !loaded.diagnostics.is_empty() {
        return Err(analysis_failure(&options.path, &mut loaded, Vec::new(), ""));
    }
    let resolved = match resolve::resolve(&loaded.program) {
        Ok(resolved) => resolved,
        Err(problem) => {
            return Err(analysis_failure(
                &options.path,
                &mut loaded,
                vec![problem],
                "declaration resolution failed; dependent definition and IR analysis was not attempted",
            ));
        }
    };
    stats.mark("resolve")?;
    let inferred = if options.command == "check-library" {
        elaborate::infer_resolved_report_all_library(&resolved)
    } else {
        elaborate::infer_resolved_report(&resolved)
    };
    let definitions = match inferred {
        Ok(definitions) => definitions,
        Err(problems) => {
            return Err(analysis_failure(
                &options.path,
                &mut loaded,
                problems,
                "definition analysis failed; dependent IR construction and checking was not attempted",
            ));
        }
    };
    stats.mark("type")?;
    let meaning = match elaborate::lower_definitions(definitions) {
        Ok(meaning) => meaning,
        Err(problems) => {
            return Err(analysis_failure(
                &options.path,
                &mut loaded,
                problems,
                "failed bodies are blocked; independent typed bodies were constructed and checked",
            ));
        }
    };
    stats.mark("build_ir")?;
    let checked = match check_reusing(meaning, memo, options.roundtrip) {
        Ok(checked) => checked,
        Err(problems) => {
            return Err(analysis_failure(
                &options.path,
                &mut loaded,
                problems,
                "each invalid IR body stops at its diagnostic; no artifact is produced",
            ));
        }
    };
    stats.mark("check")?;
    stats.count("ir_functions", checked.program().functions.len());
    let mut instructions = 0usize;
    for function in &checked.program().functions {
        instructions += function.body.len();
    }
    stats.count("ir_instructions", instructions);
    Ok(checked)
}

pub(crate) fn load_runtime(
    world: &mut World,
    runtime_source: &Option<String>,
) -> Result<runtime::Runtime, String> {
    let code = match runtime_source {
        Some(path) => text(read_path(world, path)?, path)?,
        None => embedded::runtime()
            .ok_or("E_RUNTIME this seed needs --runtime-source library/runtime.rs")?,
    };
    prepare_runtime(code)
}
fn prepare_runtime(code: String) -> Result<runtime::Runtime, String> {
    let parsed = source::load("$runtime/runtime.rs", None, &mut |path: &str| {
        if path == "$runtime/runtime.rs" {
            Ok(Some(code.as_bytes().to_vec()))
        } else {
            Ok(None)
        }
    })
    .map_err(|problem| report("$runtime/runtime.rs", problem))?;
    let meaning = elaborate::elaborate(&parsed)
        .map_err(|problem| located("$runtime/runtime.rs", &parsed, problem))?;
    let checked =
        check::check(meaning).map_err(|problem| report("$runtime/runtime.rs", problem))?;
    runtime::seal(checked).map_err(|problem| report("$runtime/runtime.rs", problem))
}

fn ensure_runtime(
    world: &mut World,
    options: &Options,
    cache: &mut Option<runtime::Runtime>,
    memo: &mut Memo,
) -> Result<(), String> {
    if cache.is_none() {
        let runtime = match &options.runtime_source {
            Some(path) => {
                let bytes = read_path(world, path)?;
                if memo.store.is_some() {
                    let (parent, name) = location(path);
                    memo.runtime_input = Some(store::input(&parent, &name, &Some(bytes.clone())));
                }
                prepare_runtime(text(bytes, path)?)?
            }
            None => load_runtime(world, &None)?,
        };
        *cache = Some(runtime);
    }
    Ok(())
}

pub(crate) fn interpreter_image(
    world: &mut World,
    checked: &ir::CheckedProgram,
    index: usize,
    runtime_source: &Option<String>,
) -> Result<Vec<u8>, String> {
    let runtime = load_runtime(world, runtime_source)?;
    backend::emit_for_run(checked, &runtime, &checked.program().functions[index].name)
        .map_err(|problem| report("interpreter executable", problem))
}

fn entry(program: &ir::Program, requested: &Option<String>) -> Result<usize, String> {
    let mut main = "probe".to_string();
    for function in &program.functions {
        if function.name == "main" {
            main = "main".to_string();
            break;
        }
    }
    let name = requested.as_ref().unwrap_or(&main);
    for (index, function) in program.functions.iter().enumerate() {
        if &function.name == name {
            return Ok(index);
        }
    }
    Err(format!("E_ENTRY unknown function {name}"))
}

fn validate_entry(program: &ir::Program, index: usize) -> Result<(), String> {
    crate::entry_grants::validate(program, index)
}
fn checked_entry(program: &ir::Program, requested: &Option<String>) -> Result<usize, String> {
    let index = entry(program, requested)?;
    validate_entry(program, index)?;
    Ok(index)
}

pub(crate) fn scalar(value: interpreter::Scalar) -> String {
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
                let shift = 64u32 - bits as u32;
                format!("{}\n", ((value << shift) as i64) >> shift)
            } else {
                format!("{value}\n")
            }
        }
        interpreter::Scalar::F64 { bits } => format!("f64:0x{bits:016x}\n"),
    }
}
pub(crate) fn raw_scalar(program: &ir::Program, index: usize, bytes: &[u8]) -> Result<String, String> {
    if bytes.len() != 8 {
        return Err(format!(
            "E_HOST expected eight scalar bytes, got {}",
            bytes.len()
        ));
    }
    let mut value = 0u64;
    for index in 0..8usize {
        value |= (bytes[index] as u64) << (index * 8);
    }
    let result = &program.types[program.functions[index].result].kind;
    match result {
        ir::TypeKind::Unit => Ok(String::new()),
        ir::TypeKind::Bool => {
            if value > 1 {
                return Err("E_HOST invalid bool result".to_string());
            }
            Ok(format!("{}\n", value != 0))
        }
        ir::TypeKind::Char => {
            if value > 0x10ffff {
                return Err("E_HOST invalid char result".to_string());
            }
            let character = char::from_u32(value as u32).ok_or("E_HOST invalid char result")?;
            Ok(format!("{character}\n"))
        }
        ir::TypeKind::Int { bits, signed } => Ok(scalar(interpreter::Scalar::Int {
            value,
            bits: *bits,
            signed: *signed,
        })),
        ir::TypeKind::F64 => Ok(scalar(interpreter::Scalar::F64 { bits: value })),
        _ => Err("E_ENTRY host entry result must be unit or a scalar".to_string()),
    }
}

#[derive(Clone, Copy)]
enum Completion {
    Success,
    SemanticTrap,
    Other,
}
fn semantic_trap(code: u64) -> bool {
    // Fixed language traps and message-table traps are deterministic. Resource,
    // output and reserved trap codes cannot authorize a reusable observation.
    (code >= 1 && code <= 5) || code >= 16
}
#[derive(Clone)]
struct Outcome {
    status: u8,
    completion: Completion,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    diagnostic: String,
}
fn interpret(
    world: &mut World,
    program: &ir::Program,
    index: usize,
    options: &Options,
    capture: bool,
    checked: Option<&ir::CheckedProgram>,
) -> Outcome {
    let grants = if checked.is_none() && options.grants.is_empty() {
        None
    } else {
        match crate::entry_grants::prepare(program, index, &options.grants) {
            Ok(grants) => Some(grants),
            Err(problem) => {
                return Outcome { status: 1, completion: Completion::Other, stdout: Vec::new(), stderr: Vec::new(), diagnostic: problem };
            }
        }
    };
    let mut host = match driver_host::Host::new(
        world,
        options.arguments.clone(),
        capture,
        checked,
        index,
        options.runtime_source.clone(),
    ) {
        Ok(host) => host,
        Err(problem) => {
            return Outcome {
                status: 1,
                completion: Completion::Other,
                stdout: Vec::new(),
                stderr: Vec::new(),
                diagnostic: problem,
            };
        }
    };
    let arguments: Vec<u64> = Vec::new();
    let result = match &grants {
        Some(grants) => interpreter::run_granted(program, index, grants, &mut host),
        None => interpreter::run(program, index, &arguments, &mut host),
    };
    if let Err(problem) = host.finish() {
        return Outcome {
            status: 1,
            completion: Completion::Other,
            stdout: host.stdout,
            stderr: host.stderr,
            diagnostic: problem,
        };
    }
    let mut status = host.status;
    let mut completion = if status == 0 { Completion::Success } else { Completion::Other };
    let diagnostic = match result {
        Ok(result) => {
            host.stdout
                .extend_from_slice(scalar(result.value).as_bytes());
            String::new()
        }
        Err(problem) => {
            status = 1;
            completion = Completion::Other;
            if let interpreter::ErrorKind::Trap(code) = &problem.kind {
                if semantic_trap(*code) { completion = Completion::SemanticTrap; }
            }
            format!(
                "{}:{}: {} {}",
                options.path,
                problem.line,
                problem.code(),
                problem.message
            )
        }
    };
    Outcome {
        status,
        completion,
        stdout: host.stdout,
        stderr: host.stderr,
        diagnostic,
    }
}

fn trap_number(text: &str) -> Option<u64> {
    if text.is_empty() || text.as_bytes().iter().any(|byte: &u8| *byte < b'0' || *byte > b'9') {
        return None;
    }
    text.parse::<u64>().ok()
}
fn native_trap(path: &str, raw: &[u8]) -> Option<(u64, String, String)> {
    let text = String::from_utf8(raw.to_vec()).ok()?;
    if !text.ends_with("\n") { return None; }
    let parts = split(&text, b' ', 7);
    if parts.len() != 7 || parts[1] != "at" || parts[2] != "line" || parts[4] != "file" {
        return None;
    }
    if parts[0].len() < 5 || !parts[0].starts_with("R") || !parts[5].ends_with(":") {
        return None;
    }
    let code = trap_number(&parts[0][1..])?;
    trap_number(&parts[3])?;
    trap_number(&parts[5][..parts[5].len() - 1])?;
    let message = &parts[6];
    let message = if message.ends_with("\n") {
        &message[..message.len() - 1]
    } else {
        message.as_str()
    };
    Some((code, format!("{path}:{}: {} {message}", parts[3], parts[0]), message.to_string()))
}
pub(crate) fn normalize_trap(path: &str, raw: &[u8]) -> Option<String> {
    native_trap(path, raw).map(|trap: (u64, String, String)| trap.1)
}
fn complete_semantic_trap(program: &ir::Program, code: u64, message: &str) -> bool {
    // A refused diagnostic write can leave a syntactically valid prefix. Match
    // the entire message, including embedded newlines, before reusing the result.
    match code {
        1 => return message == "integer overflow",
        2 => return message == "division by zero",
        3 => return message == "index out of bounds",
        4 => return message == "explicit trap",
        5 => return message == "assertion failed",
        _ => {}
    }
    if code < 16 { return false; }
    let mut messages = Vec::new();
    for function in &program.functions {
        for instruction in &function.body {
            if let ir::Op::Trap { message: expected } = &instruction.op {
                if !messages.contains(expected) {
                    if messages.len() as u64 == code - 16 { return expected.as_str() == message; }
                    messages.push(expected.clone());
                }
            }
        }
    }
    false
}

fn native_run(
    world: &mut World,
    checked: &reuse::Checked,
    runtime: &runtime::Runtime,
    index: usize,
    options: &Options,
    capture: bool,
    stats: &mut Stats,
    memo: &mut Memo,
) -> Result<Outcome, String> {
    let name = &checked.program().functions[index].name;
    let grants = crate::entry_grants::prepare(checked.program(), index, &options.grants)?;
    let bytes = emit_reusing(checked, runtime, name, &grants, Some(3), None, memo)
        .map_err(|problem| report(&options.path, problem))?;
    stats.count("emitted_bytes", bytes.len());
    stats.mark("native_encode")?;
    let mut executable = world
        .temporary_file()
        .map_err(|errno: i64| io("artifact", "create", errno))?;
    executable
        .write_all(&bytes)
        .map_err(|errno: i64| io("artifact", "write", errno))?;
    executable
        .set_executable()
        .map_err(|errno: i64| io("artifact", "set executable", errno))?;
    let input = if capture {
        world.temporary_file()
    } else {
        world.stdin_file()
    }
    .map_err(|errno: i64| io("stdin", "derive stream", errno))?;
    let mut output = if capture {
        world.temporary_file()
    } else {
        world.stdout_file()
    }
    .map_err(|errno: i64| io("stdout", "derive stream", errno))?;
    let mut errors = if capture {
        world.temporary_file()
    } else {
        world.stderr_file()
    }
    .map_err(|errno: i64| io("stderr", "derive stream", errno))?;
    let mut result = world
        .temporary_file()
        .map_err(|errno: i64| io("result", "create capture", errno))?;
    let mut process = world.process();
    let mut child = process
        .spawn_file(
            &executable,
            &options.arguments,
            &input,
            &output,
            &errors,
            &result,
        )
        .map_err(|errno: i64| io("artifact", "spawn", errno))?;
    let status = child
        .wait()
        .map_err(|errno: i64| io("artifact", "wait", errno))? as u8;
    stats.mark("native_run")?;
    let stdout = if capture {
        output
            .rewind()
            .map_err(|errno: i64| io("stdout", "rewind", errno))?;
        output
            .read_to_end()
            .map_err(|errno: i64| io("stdout", "read", errno))?
    } else {
        Vec::new()
    };
    let stderr = if capture {
        errors
            .rewind()
            .map_err(|errno: i64| io("stderr", "rewind", errno))?;
        errors
            .read_to_end()
            .map_err(|errno: i64| io("stderr", "read", errno))?
    } else {
        Vec::new()
    };
    result
        .rewind()
        .map_err(|errno: i64| io("result", "rewind", errno))?;
    let raw = result
        .read_to_end()
        .map_err(|errno: i64| io("result", "read", errno))?;
    let mut outcome = Outcome {
        status,
        completion: if status == 0 { Completion::Success } else { Completion::Other },
        stdout,
        stderr: Vec::new(),
        diagnostic: String::new(),
    };
    if raw.len() != 8 && status != 0 {
        outcome.diagnostic = match native_trap(&options.path, &raw) {
            Some((code, diagnostic, message)) => {
                if status == 1 && complete_semantic_trap(checked.program(), code, &message) {
                    outcome.completion = Completion::SemanticTrap;
                }
                diagnostic
            }
            None => format!("E_HOST native program exited with status {status} without a scalar result"),
        };
    } else {
        outcome
            .stdout
            .extend_from_slice(raw_scalar(checked.program(), index, &raw)?.as_bytes());
    }
    outcome.stderr = stderr;
    Ok(outcome)
}

fn publish(world: &mut World, path: &str, bytes: &[u8], executable: bool) -> Result<(), String> {
    let (parent, name) = location(path);
    let mut directory = world
        .directory(&parent)
        .map_err(|errno: i64| io(&parent, "open output directory", errno))?;
    let mut clock = world.clock();
    let stamp = clock
        .nanoseconds()
        .map_err(|errno: i64| io("clock", "read", errno))?;
    let mut destination = 14695981039346656037u64;
    for byte in name.as_bytes() {
        destination = (destination ^ *byte as u64).wrapping_mul(1099511628211u64);
    }
    let temporary = format!(".lkjscript-{destination:x}-{stamp}.tmp");
    let result = {
        let mut file = directory
            .create(&temporary)
            .map_err(|errno: i64| io(path, "create temporary", errno))?;
        match file.write_all(bytes) {
            Err(errno) => Err(io(path, "write", errno)),
            Ok(()) => {
                if executable {
                    file.set_executable()
                        .map_err(|errno: i64| io(path, "set executable", errno))
                } else {
                    Ok(())
                }
            }
        }
    };
    if let Err(problem) = result {
        let _ = directory.remove(&temporary);
        return Err(problem);
    }
    if let Err(errno) = directory.rename(&temporary, &name) {
        let _ = directory.remove(&temporary);
        return Err(io(path, "rename", errno));
    }
    Ok(())
}

fn deliver(world: &mut World, outcome: Outcome, show_value: bool) -> u8 {
    if show_value {
        world.stdout().write(&outcome.stdout);
    }
    world.stderr().write(&outcome.stderr);
    if !outcome.diagnostic.is_empty() {
        world
            .stderr()
            .write(format!("{}\n", outcome.diagnostic).as_bytes());
    }
    outcome.status
}

fn quoted(value: &str) -> String {
    let mut bytes = Vec::new();
    json::write_string(&mut bytes, value);
    String::from_utf8(bytes).unwrap()
}
fn hex(bytes: &[u8]) -> String {
    let digits = b"0123456789abcdef";
    let mut text = String::new();
    for byte in bytes {
        text.push(digits[(*byte >> 4) as usize] as char);
        text.push(digits[(*byte & 15) as usize] as char);
    }
    text
}
fn record(world: &mut World, index: usize, engine: &str, checked: bool, outcome: &Outcome) {
    let code = if checked { 0 } else { 1 };
    let text = format!(
        "{{\"index\":{index},\"engine\":{},\"checkStatus\":{code},\"status\":{},\"stdoutHex\":\"{}\",\"stderrHex\":\"{}\",\"stderr\":{}}}\n",
        quoted(engine),
        outcome.status,
        hex(&outcome.stdout),
        hex(&outcome.stderr),
        quoted(&outcome.diagnostic)
    );
    world.stdout().write(text.as_bytes());
}
fn batch(world: &mut World, options: &Options, stats: &mut Stats, memo: &mut Memo) -> Result<u8, String> {
    let input = text(read_path(world, &options.path)?, &options.path)?;
    let mut runtime = None;
    for (index, row) in split(&input, b'\n', 0).iter().enumerate() {
        if row.is_empty() {
            continue;
        }
        let columns = split(row, b'\t', 3);
        if columns.len() != 3 || (columns[0] != "check" && columns[0] != "run") {
            return Err(format!("E_USAGE invalid batch request {}", index + 1));
        }
        let mut request = options.clone();
        request.command = columns[0].to_string();
        request.path = columns[2].to_string();
        request.arguments.clear();
        request.entry = if columns[1].is_empty() {
            None
        } else {
            Some(columns[1].to_string())
        };
        if request.runtime_source.is_some() {
            runtime = None;
            memo.runtime_input = None;
        }
        if let Some(bytes) = memo.lookup(world, &request, "test") {
            if let Some((accepted, outcomes)) = decode_outcomes(&bytes, options.engines.len()) {
                for (route, outcome) in outcomes.iter().enumerate() {
                    record(world, index, &options.engines[route], accepted, outcome);
                }
                memo.test_reused += 1;
                continue;
            }
        }
        let checked = match compile(world, &request, stats, memo) {
            Ok(checked) => checked,
            Err(problem) => {
                let outcome = Outcome {
                    status: 1,
                    completion: Completion::Other,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                    diagnostic: problem,
                };
                for engine in &options.engines {
                    record(world, index, engine, false, &outcome);
                }
                if !outcome.diagnostic.contains("E_IO") {
                    let mut outcomes = Vec::new();
                    for _ in &options.engines { outcomes.push(outcome.clone()); }
                    memo.remember(&request, "test", &encode_outcomes(false, &outcomes));
                }
                continue;
            }
        };
        if options.roundtrip {
            let encoded = ir_text::write(checked.program());
            let decoded = match ir_text::read(&encoded) {
                Ok(decoded) => decoded,
                Err(problem) => return Err(report(&request.path, problem)),
            };
            let replay = match check::check(decoded) {
                Ok(replay) => replay,
                Err(problem) => return Err(report(&request.path, problem)),
            };

            if ir_text::write(replay.program()) != encoded {
                return Err(format!("E_TEST IR roundtrip changed {}", request.path));
            }
            if let Ok(selected) = checked_entry(checked.program(), &request.entry) {
                // A check-only case need not grant an executable entry. Always
                // recheck its IR above; compare native bytes only when the
                // invocation supplies every required capability.
                if let Ok(grants) = crate::entry_grants::prepare(
                    checked.program(), selected, &request.grants,
                ) {
                    ensure_runtime(world, &request, &mut runtime, memo)?;
                    let name = &checked.program().functions[selected].name;
                    let original = match backend::emit_granted(
                        checked.checked(), runtime.as_ref().unwrap(), name, &grants, None, None,
                    ) {
                        Ok(bytes) => bytes,
                        Err(problem) => return Err(report(&request.path, problem)),
                    };
                    let repeated = match backend::emit_granted(
                        &replay, runtime.as_ref().unwrap(), name, &grants, None, None,
                    ) {
                        Ok(bytes) => bytes,
                        Err(problem) => return Err(report(&request.path, problem)),
                    };
                    if original != repeated { return Err(format!("E_TEST IR roundtrip changed native bytes for {}", request.path)); }
                    if memo.store.is_some() {
                        // The second call reloads the entries flushed by the
                        // first, proving warm byte identity in an empty-store gate.
                        for pass in 0..2usize {
                            let reused = match emit_reusing(
                                &checked, runtime.as_ref().unwrap(), name, &grants, None, None, memo,
                            ) {
                                Ok(bytes) => bytes,
                                Err(problem) => return Err(report(&request.path, problem)),
                            };
                            if original != reused {
                                let phase = if pass == 0 { "first" } else { "warm" };
                                return Err(format!("E_TEST {phase} store emission changed native bytes for {}", request.path));
                            }
                        }
                    }
                }
            }
        }
        let mut outcomes = Vec::new();
        let reusable = request.command == "check" || (request.runtime_source.is_none()
            && checked_entry(checked.program(), &request.entry)
                .map(|selected: usize| checked.program().functions[selected].params == 0).unwrap_or(false));
        for engine in &options.engines {
            let outcome = if request.command == "check" {
                Outcome {
                    status: 0,
                    completion: Completion::Success,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                    diagnostic: String::new(),
                }
            } else {
                match checked_entry(checked.program(), &request.entry) {
                    Err(problem) => Outcome {
                        status: 1,
                        completion: Completion::Other,
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                        diagnostic: problem,
                    },
                    Ok(selected) => {
                        if engine == "interpreter" {
                            interpret(
                                world,
                                checked.program(),
                                selected,
                                &request,
                                true,
                                Some(checked.checked()),
                            )
                        } else {
                            ensure_runtime(world, &request, &mut runtime, memo)?;
                            match native_run(
                                world,
                                &checked,
                                runtime.as_ref().unwrap(),
                                selected,
                                &request,
                                true,
                                stats,
                                memo,
                            ) {
                                Ok(outcome) => outcome,
                                Err(problem) => Outcome {
                                    status: 1,
                                    completion: Completion::Other,
                                    stdout: Vec::new(),
                                    stderr: Vec::new(),
                                    diagnostic: problem,
                                },
                            }
                        }
                    }
                }
            };
            record(world, index, engine, true, &outcome);
            outcomes.push(outcome);
        }
        let mut completed = reusable;
        for outcome in &outcomes {
            if !matches!(outcome.completion, Completion::Success | Completion::SemanticTrap) {
                completed = false;
            }
        }
        if completed {
            memo.remember(&request, "test", &encode_outcomes(true, &outcomes));
        }
    }
    Ok(0)
}

fn execute(world: &mut World, options: &Options, stats: &mut Stats, memo: &mut Memo) -> Result<u8, String> {
    if options.command == "--help" || options.command == "-h" {
        world.stdout().write(usage().as_bytes());
        return Ok(0);
    }
    if !options.tool_arguments.is_empty() {
        return crate::test_generators::run(world, &options.tool_arguments);
    }
    if options.command == "profile" {
        let bytes = read_path(world, &options.path)?;
        let report = crate::native::profiling::report(&bytes)?;
        world.stdout().write(report.as_bytes());
        return Ok(0);
    }
    if options.command == "test" {
        if let Some(rustc) = &options.witness {
            return crate::test_runner::witness(world, &options.path, rustc);
        }
    }
    if options.command == "test" && crate::test_runner::has_cases(world, &options.path)? {
        return crate::test_runner::run_stored(world, &options.path, options.jobs, &options.engines,
            &options.store_directory, &options.store_key, options.stats);
    }
    if options.command == "batch" {
        return batch(world, options, stats, memo);
    }
    let mut runtime = None;
    if options.command == "runtime-ir" {
        ensure_runtime(world, options, &mut runtime, memo)?;
        let text = ir_text::write(runtime.as_ref().unwrap().checked_program());
        world.stdout().write(text.as_bytes());
        return Ok(0);
    }
    if options.unchecked {
        let input = text(read_path(world, &options.path)?, &options.path)?;
        let program = ir_text::read(&input).map_err(|problem| report(&options.path, problem))?;
        let selected = entry(&program, &options.entry)?;
        let outcome = interpret(world, &program, selected, options, false, None);
        return Ok(deliver(world, outcome, true));
    }
    if options.command == "build" {
        if let Some(bytes) = memo.lookup(world, options, "image") {
            let path: String = match &options.output { Some(path) => path.clone(), None => format!("{}.elf", options.path) };
            publish(world, &path, &bytes, true)?;
            memo.build_reused += 1;
            stats.count("emitted_bytes", bytes.len());
            stats.mark("stored_build")?;
            return Ok(0);
        }
    }
    let checked = compile(world, options, stats, memo)?;
    if options.command == "check" || options.command == "check-library" {
        return Ok(0);
    }
    if options.command == "ir" {
        let text = ir_text::write(checked.program());
        stats.count("ir_bytes", text.len());
        stats.mark("ir_write")?;
        match &options.output {
            Some(path) => publish(world, path, text.as_bytes(), false)?,
            None => world.stdout().write(text.as_bytes()),
        }
        return Ok(0);
    }
    if options.command == "authority" {
        let selected = checked_entry(checked.program(), &options.entry)?;
        let analysis = crate::authority::analyze(checked.checked(), selected)?;
        let output = crate::authority::render(checked.checked(), &analysis);
        world.stdout().write(output.as_bytes());
        return Ok(0);
    }
    if options.command == "build" {
        let selected = checked_entry(checked.program(), &options.entry)?;
        let grants = crate::entry_grants::prepare(checked.program(), selected, &options.grants)?;
        ensure_runtime(world, options, &mut runtime, memo)?;
        stats.mark("runtime")?;
        let path: String = match &options.output {
            Some(path) => path.to_string(),
            None => format!("{}.elf", options.path),
        };
        let name = &checked.program().functions[selected].name;
        let bytes = if options.profile {
            let profile_path = format!("{path}.samples");
            emit_reusing(&checked, runtime.as_ref().unwrap(), name, &grants, None, Some(profile_path.as_str()), memo)
        } else {
            emit_reusing(&checked, runtime.as_ref().unwrap(), name, &grants, None, None, memo)
        }.map_err(|problem| report(&options.path, problem))?;
        stats.count("emitted_bytes", bytes.len());
        stats.mark("native_encode")?;
        publish(world, &path, &bytes, true)?;
        memo.remember(options, "image", &bytes);
        return Ok(0);
    }
    if options.command == "test" {
        let mut count = 0usize;
        let mut failed = false;
        for (index, function) in checked.program().functions.iter().enumerate() {
            if !function.test {
                continue;
            }
            validate_entry(checked.program(), index)?;
            for engine in &options.engines {
                let outcome = if engine == "interpreter" {
                    interpret(
                        world,
                        checked.program(),
                        index,
                        options,
                        true,
                        Some(checked.checked()),
                    )
                } else {
                    ensure_runtime(world, options, &mut runtime, memo)?;
                    native_run(
                        world,
                        &checked,
                        runtime.as_ref().unwrap(),
                        index,
                        options,
                        true,
                        stats,
                        memo,
                    )?
                };
                count += 1;
                if outcome.status != 0 || !outcome.diagnostic.is_empty() {
                    failed = true;
                    world.stderr().write(
                        format!("FAIL {} {} {engine}\n", options.path, function.name).as_bytes(),
                    );
                    deliver(world, outcome, true);
                }
            }
        }
        if failed {
            return Ok(1);
        }
        world
            .stdout()
            .write(format!("ok {count} tests\n").as_bytes());
        return Ok(0);
    }
    let selected = checked_entry(checked.program(), &options.entry)?;
    let outcome = if options.engine == "interpreter" {
        interpret(
            world,
            checked.program(),
            selected,
            options,
            false,
            Some(checked.checked()),
        )
    } else {
        ensure_runtime(world, options, &mut runtime, memo)?;
        stats.mark("runtime")?;
        native_run(
            world,
            &checked,
            runtime.as_ref().unwrap(),
            selected,
            options,
            false,
            stats,
            memo,
        )?
    };
    Ok(deliver(world, outcome, true))
}

pub fn main(mut world: World) {
    let result = match options(world.args()) {
        Err(problem) => Err(problem),
        Ok(options) => match Stats::new(&mut world, options.stats) {
            Err(problem) => Err(problem),
            Ok(mut stats) => {
                let mut memo = Memo::new(&mut world, &options);
                let result = execute(&mut world, &options, &mut stats, &mut memo);
                memo.stats(&mut stats);
                let stopped = if result.is_ok() { "done" } else { "failed" };
                let measured = stats.finish(&mut world, stopped);
                match measured {
                    Err(problem) => Err(problem),
                    Ok(()) => result,
                }
            }
        },
    };
    match result {
        Ok(status) => world.set_exit_status(status),
        Err(problem) => {
            world.stderr().write(format!("{problem}\n").as_bytes());
            world.set_exit_status(1);
        }
    }
}

struct Memo {
    store: Option<store::Store>,
    inputs: Vec<store::Input>,
    valid_inputs: bool,
    runtime_input: Option<store::Input>,
    checked_reused: usize,
    emitted_reused: usize,
    test_reused: usize,
    build_reused: usize,
}
impl Memo {
    fn new(world: &mut World, options: &Options) -> Self {
        Self {
            store: store::Store::open(world, &options.store_directory, &options.store_key),
            inputs: Vec::new(), valid_inputs: true, runtime_input: None,
            checked_reused: 0, emitted_reused: 0, test_reused: 0, build_reused: 0,
        }
    }
    fn stats(&self, stats: &mut Stats) {
        stats.count("checked_reused", self.checked_reused);
        stats.count("emitted_reused", self.emitted_reused);
        stats.count("test_reused", self.test_reused);
        stats.count("build_reused", self.build_reused);
        if let Some(store) = &self.store {
            stats.count("store_hits", store.hits);
            stats.count("store_misses", store.misses);
            stats.count("store_rejected", store.rejected);
            stats.count("store_writes", store.writes);
        }
    }
    fn lookup(&mut self, world: &mut World, options: &Options, kind: &str) -> Option<Vec<u8>> {
        let store = self.store.as_mut()?;
        let request = request_key(options);
        let plan = store.get(&format!("inputs-{kind}"), &request)?;
        let mut reader = store::Reader::new(&plan);
        let inputs = reader.bytes()?;
        let result = reader.text()?;
        if !reader.done() || !store::inputs_match(world, &inputs) { return None; }
        store.get(kind, &result)
    }
    fn remember(&mut self, options: &Options, kind: &str, bytes: &[u8]) {
        if !self.valid_inputs { return; }
        if let Some(store) = self.store.as_mut() {
            let request = request_key(options);
            let mut closure = self.inputs.clone();
            if let Some(input) = &self.runtime_input { closure.push(input.clone()); }
            let inputs = store::encode_inputs(&closure);
            let mut identity = Vec::new();
            store::push_bytes(&mut identity, request.as_bytes());
            store::push_bytes(&mut identity, &inputs);
            let result = sha256::hex_digest(&identity);
            store.put(kind, &result, bytes);
            let mut plan = Vec::new();
            store::push_bytes(&mut plan, &inputs);
            store::push_bytes(&mut plan, result.as_bytes());
            store.put(&format!("inputs-{kind}"), &request, &plan);
        }
    }
}
fn optional_key(out: &mut Vec<u8>, value: &Option<String>) {
    match value {
        Some(value) => { store::push_word(out, 1); store::push_bytes(out, value.as_bytes()); }
        None => store::push_word(out, 0),
    }
}
fn request_key(options: &Options) -> String {
    let mut out = Vec::new();
    store::push_bytes(&mut out, b"lkjscript-input-closure-1");
    store::push_bytes(&mut out, options.command.as_bytes());
    store::push_bytes(&mut out, options.path.as_bytes());
    optional_key(&mut out, &options.entry);
    optional_key(&mut out, &options.source_directory);
    optional_key(&mut out, &options.library_directory);
    optional_key(&mut out, &options.runtime_source);
    store::push_word(&mut out, options.no_library as usize);
    store::push_word(&mut out, options.profile as usize);
    store::push_word(&mut out, options.roundtrip as usize);
    if options.profile { optional_key(&mut out, &options.output); }
    store::push_word(&mut out, options.grants.len());
    for grant in &options.grants { store::push_bytes(&mut out, grant.as_bytes()); }
    store::push_word(&mut out, options.engines.len());
    for engine in &options.engines { store::push_bytes(&mut out, engine.as_bytes()); }
    store::push_word(&mut out, options.arguments.len());
    for argument in &options.arguments { store::push_bytes(&mut out, argument); }
    sha256::hex_digest(&out)
}
fn check_reusing(
    program: ir::Program, memo: &mut Memo, verify: bool,
) -> Result<reuse::Checked, Vec<ast::Diagnostic>> {
    if let Some(store) = memo.store.as_mut() {
        // The corpus roundtrip proves identical verdicts, including complete
        // diagnostics, before its authenticated test observation is reusable.
        let expected: Option<Option<Vec<ast::Diagnostic>>> = if verify {
            Some(check::check_all(program.clone()).err())
        } else { None };
        let prepared = reuse::Prepared::new(program);
        let mut records = store.records("check", prepared.keys().all());
        let checked = check::check_all_cached(prepared, &mut records);
        memo.checked_reused += records.hits;
        store.flush(&records);
        if let Some(expected) = expected {
            let identical = match (&expected, &checked) {
                (None, Ok(_)) => true,
                (Some(left), Err(right)) => left == right,
                _ => false,
            };
            if !identical {
                memo.valid_inputs = false;
                return Err(vec![ast::Diagnostic {
                    path: String::new(), file: 0, line: 1, column: 1,
                    code: "E_TEST".to_string(),
                    message: "authenticated reuse changed the checker verdict".to_string(),
                }]);
            }
        }
        checked
    } else {
        check::check_all(program).map(|checked: ir::CheckedProgram| reuse::Checked::uncached(checked))
    }
}
fn emit_reusing(
    checked: &reuse::Checked,
    runtime: &runtime::Runtime,
    entry: &str,
    grants: &[ir::EntryGrant],
    result_fd: Option<u32>,
    profile_path: Option<&str>,
    memo: &mut Memo,
) -> Result<Vec<u8>, ast::Diagnostic> {
    if let Some(keys) = checked.keys() {
        if let Some(store) = memo.store.as_mut() {
            let namespace = backend::cache_namespace(checked, runtime, grants, profile_path);
            let mut records = store.records(&namespace, keys.all());
            let result = backend::emit_granted_reusing(
                checked, runtime, entry, grants, result_fd, profile_path, &namespace, &mut records,
            );
            memo.emitted_reused += records.hits;
            store.flush(&records);
            return result;
        }
    }
    backend::emit_granted(checked.checked(), runtime, entry, grants, result_fd, profile_path)
}
fn encode_outcomes(accepted: bool, outcomes: &[Outcome]) -> Vec<u8> {
    let mut bytes = Vec::new();
    store::push_word(&mut bytes, accepted as usize);
    store::push_word(&mut bytes, outcomes.len());
    for outcome in outcomes {
        store::push_word(&mut bytes, outcome.status as usize);
        store::push_bytes(&mut bytes, &outcome.stdout);
        store::push_bytes(&mut bytes, &outcome.stderr);
        store::push_bytes(&mut bytes, outcome.diagnostic.as_bytes());
    }
    bytes
}
fn decode_outcomes(bytes: &[u8], count: usize) -> Option<(bool, Vec<Outcome>)> {
    let mut reader = store::Reader::new(bytes);
    let accepted = reader.word()?;
    if accepted > 1 || reader.word()? != count { return None; }
    let mut outcomes = Vec::new();
    for _ in 0..count {
        let status = reader.word()?;
        if status > 255 { return None; }
        outcomes.push(Outcome {
            status: status as u8, completion: Completion::Other,
            stdout: reader.bytes()?, stderr: reader.bytes()?, diagnostic: reader.text()?,
        });
    }
    if !reader.done() { return None; }
    Some((accepted != 0, outcomes))
}
