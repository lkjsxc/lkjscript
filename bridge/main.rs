//! Rust-only bootstrap host. Replace by lkjscript World once the compiler self-compiles.
use lkjscript::{ast::Diagnostic, check, elaborate, k, k_text, syntax, wasm};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn report(path: &str, problem: Diagnostic) -> String {
    format!(
        "{path}:{}:{}: {} {}",
        problem.line, problem.column, problem.code, problem.message
    )
}
fn compile(path: &str) -> Result<k::CheckedProgram, String> {
    let mut source = fs::read_to_string(path).map_err(|e| format!("{path}:1:1: E_IO {e}"))?;
    if Path::new(path).extension().is_some_and(|ext| ext == "k") {
        return check::check(k_text::read(&source).map_err(|e| report(path, e))?)
            .map_err(|e| report(path, e));
    }
    source.push('\n');
    source.push_str(include_str!("../library/prelude.rs"));
    source.push('\n');
    source.push_str(include_str!("../library/sequence.rs"));
    let parsed = syntax::parse(&source).map_err(|e| report(path, e))?;
    let program = elaborate::elaborate(&parsed).map_err(|e| report(path, e))?;
    check::check(program).map_err(|e| report(path, e))
}
fn option(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}
fn execute(module: &Path, entry: &str, kind: &str) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("node")
        .arg(root.join("bridge/run.mjs"))
        .arg(module)
        .arg(entry)
        .arg(kind)
        .output()
        .map_err(|e| format!("E_HOST Node: {e}"))?;
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
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && args[1] == "trusted" {
        println!(
            "K checker: src/check.rs\nmemory and collections: src/wasm.rs\nbootstrap host: bridge/main.rs\nNode host: bridge/run.mjs"
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
                if let Err(e) = execute(&output, &function.name, "unit") {
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
            execute(&output, &entry, result_kind(program, function))
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
    // Reserve a native bootstrap stack from this host's physical memory and swap.
    // Reservation is virtual; pages are committed by the operating system on use.
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
    let mut thread = std::thread::Builder::new().name("lkjscript".to_string());
    if let Some(memory) = memory {
        thread = thread.stack_size(memory);
    }
    match thread.spawn(run) {
        Ok(job) => match job.join() {
            Ok(Ok(())) => ExitCode::SUCCESS,
            Ok(Err(e)) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
            Err(_) => {
                eprintln!("E_HOST compiler thread failed");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("E_HOST {e}");
            ExitCode::FAILURE
        }
    }
}
