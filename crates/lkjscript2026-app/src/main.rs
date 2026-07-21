//! CLI entry for the lkjscript2026 language runtime.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use lkjscript2026_compiler::compile_path;
use lkjscript2026_core::Limits;
use lkjscript2026_vm::run_chunk_with_args;

fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lkjscript2026: {e}");
            ExitCode::from(1)
        }
    }
}

fn real_main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
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
                .ok_or_else(|| "run needs a .lkjscript path".to_string())?
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
                .ok_or_else(|| "disasm needs a .lkjscript path".to_string())?;
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
    println!("lkjscript2026 — typed slash-grammar language runtime");
    println!();
    println!("Usage:");
    println!("  lkjscript2026 run <file.lkjscript> [script-args…]");
    println!("  lkjscript2026 disasm <file.lkjscript>");
    println!("  lkjscript2026 --help");
}
