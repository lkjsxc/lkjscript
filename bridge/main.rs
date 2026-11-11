//! Rust-only bootstrap host. Replace by lkjscript World once the compiler self-compiles.
use lkjscript::{ast::Diagnostic, check, compiler, k, k_text, source, wasm};
mod library;
use std::fs;
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

    pub fn restore_in_child(command: &mut Command, original: Limits) {
        // Only an async-signal-safe syscall and error capture run after fork.
        // Node must not inherit an enormous default pthread stack reservation.
        unsafe {
            command.pre_exec(move || {
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
fn compile(path: &str) -> Result<k::CheckedProgram, String> {
    let source = fs::read_to_string(path).map_err(|e| format!("{path}:1:1: E_IO {e}"))?;
    if Path::new(path).extension().is_some_and(|ext| ext == "k") {
        return check::check(k_text::read(&source).map_err(|e| report(path, e))?)
            .map_err(|e| report(path, e));
    }
    let library = library::standard_library();
    let parsed = source::load(path, Some("$std/lib.rs"), &mut |file| {
        if let Some(input) = library.iter().find(|input| input.path == file) {
            return Ok(Some(input.text.clone()));
        }
        match fs::read_to_string(file) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    })
    .map_err(|e| report(path, e))?;
    compiler::check_source(&parsed).map_err(|problem| report(path, problem))
}
fn option(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}
fn execute(
    module: &Path,
    entry: &str,
    kind: &str,
    child_stack: host_stack::Limits,
) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut command = Command::new("node");
    command
        .arg(root.join("bridge/run.mjs"))
        .arg(module)
        .arg(entry)
        .arg(kind);
    host_stack::restore_in_child(&mut command, child_stack);
    let output = command.output().map_err(|e| format!("E_HOST Node: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    print!("{}", String::from_utf8_lossy(&output.stdout));
    Ok(())
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
fn run(child_stack: host_stack::Limits) -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && args[1] == "trusted" {
        println!(
            "source syntax: src/ast.rs, src/syntax.rs\nsource loading and resolution: src/source.rs, src/resolve.rs, src/names.rs\ngeneric contracts: src/generics.rs, src/elaborate/generic_check.rs\nsource lowering: src/elaborate.rs, src/elaborate/static_lifetimes.rs\nK representation and checker: src/k.rs, src/check.rs\nlowering, memory and collections: src/wasm.rs\nbinary encoding: src/encoding.rs\ncompiler-core boundary: src/compiler.rs\nbootstrap host and inputs: bridge/main.rs, bridge/library.rs\nNode host: bridge/run.mjs"
        );
        return Ok(());
    }
    if args.len() < 3 {
        return Err(
            "usage: lkjscript check|run|test|build|k <source.rs> [--entry name] [-o module.wasm]"
                .to_string(),
        );
    }
    let verb = args[1].as_str();
    if !["check", "run", "test", "build", "k"].contains(&verb) {
        return Err(format!("E_USAGE unknown command {verb}"));
    }
    let checked = compile(&args[2])?;
    if verb == "check" {
        return Ok(());
    }
    if verb == "k" {
        print!("{}", k_text::write(checked.program()));
        return Ok(());
    }
    let bytes = wasm::emit(&checked).map_err(|e| format!("{}:1:1: E_EMIT {e}", args[2]))?;
    let temporary = verb != "build";
    let output = if temporary {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/run");
        fs::create_dir_all(&dir).map_err(|e| format!("E_IO {e}"))?;
        dir.join(format!("{}.wasm", std::process::id()))
    } else {
        PathBuf::from(option(&args, "-o").unwrap_or_else(|| {
            Path::new(&args[2])
                .with_extension("wasm")
                .to_string_lossy()
                .into_owned()
        }))
    };
    fs::write(&output, bytes).map_err(|e| format!("E_IO {}: {e}", output.display()))?;
    let program = checked.program();
    let result = if verb == "test" {
        let mut result = Ok(());
        for function in &program.functions {
            if function.test {
                if let Err(e) = execute(&output, &function.name, "unit", child_stack) {
                    result = Err(e);
                    break;
                }
            }
        }
        result
    } else if verb == "run" {
        let entry = option(&args, "--entry").unwrap_or_else(|| {
            if program.functions.iter().any(|f| f.name == "main") {
                "main".to_string()
            } else {
                "probe".to_string()
            }
        });
        let function = program
            .functions
            .iter()
            .find(|f| f.name == entry)
            .ok_or_else(|| format!("E_ENTRY unknown function {entry}"))?;
        if function.params != 0 {
            Err("E_ENTRY entry arguments are not implemented yet".to_string())
        } else {
            execute(&output, &entry, result_kind(program, function), child_stack)
        }
    } else {
        Ok(())
    };
    if temporary {
        let _ = fs::remove_file(&output);
    }
    result
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
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
