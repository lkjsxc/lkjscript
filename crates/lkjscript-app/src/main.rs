//! CLI entry for the lkjscript language runtime.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use lkjscript_compiler::compile_path;
use lkjscript_core::Limits;
use lkjscript_vm::run_chunk_with_args;

fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lkjscript: {e}");
            ExitCode::from(1)
        }
    }
}

fn real_main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.as_slice() == ["--version"] || args.as_slice() == ["-V"] {
        println!("lkjscript {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }
    let limits = Limits::default();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    match cmd {
        "run" => {
            let file = args
                .get(1)
                .ok_or_else(|| "run needs a .lkjml path".to_string())?
                .clone();
            let script_args: Vec<String> = args.iter().skip(2).cloned().collect();
            let path = PathBuf::from(&file);
            let chunk = compile_path(&path, &limits).map_err(|e| e.to_string())?;
            run_chunk_with_args(&chunk, &script_args).map_err(|e| e.to_string())?;
            Ok(())
        }
        "disasm" => {
            let file = args
                .get(1)
                .ok_or_else(|| "disasm needs a .lkjml path".to_string())?;
            let path = PathBuf::from(file);
            let chunk = compile_path(&path, &limits).map_err(|e| e.to_string())?;
            println!("globals: {:?}", chunk.global_names);
            println!("protos: {}", chunk.protos.len());
            Ok(())
        }
        "repl" => Err("repl not implemented yet".into()),
        other => Err(format!("unknown command: {other}")),
    }
}

fn print_help() {
    println!("lkjscript — typed LKJML language runtime");
    println!();
    println!("Usage:");
    println!("  lkjscript run <file.lkjml> [script-args…]");
    println!("  lkjscript disasm <file.lkjml>");
    println!("  lkjscript --help");
    println!("  lkjscript --version");
    println!();
    println!("Environment:");
    println!("  LKJSCRIPT_ROOT  installed root containing src/std and src/lib");
}
