//! CLI entry for the lkjscript language runtime.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use lkjscript_compiler::compile_path;
use lkjscript_core::{Chunk, FunctionProto, Limits, Op};
use lkjscript_vm::run_chunk_with_args;

fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lkjscript: {error}");
            ExitCode::from(1)
        }
    }
}

fn real_main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version" | "-V") if args.len() == 1 => {
            println!("lkjscript {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        None | Some("--help" | "-h") => {
            print_help();
            Ok(())
        }
        Some("run") => run_command(&args),
        Some("disasm") => disasm_command(&args),
        Some(other) => Err(format!("unknown command: {other}")),
    }
}

fn run_command(args: &[String]) -> Result<(), String> {
    let file = args
        .get(1)
        .ok_or_else(|| "run needs a .lkjscript path".to_string())?;
    let script_arg_start = if args.get(2).map(String::as_str) == Some("--") {
        3
    } else {
        2
    };
    let script_args = args.get(script_arg_start..).unwrap_or_default().to_vec();
    let chunk = compile_path(&PathBuf::from(file), &Limits::default())
        .map_err(|error| error.to_string())?;
    run_chunk_with_args(&chunk, &script_args)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn disasm_command(args: &[String]) -> Result<(), String> {
    let file = args
        .get(1)
        .ok_or_else(|| "disasm needs a .lkjscript path".to_string())?;
    if args.len() != 2 {
        return Err("disasm accepts exactly one .lkjscript path".to_string());
    }
    let chunk = compile_path(&PathBuf::from(file), &Limits::default())
        .map_err(|error| error.to_string())?;
    disassemble(&chunk)
}

fn disassemble(chunk: &Chunk) -> Result<(), String> {
    println!("constants ({}):", chunk.constants.len());
    for (index, constant) in chunk.constants.iter().enumerate() {
        println!("  {index:04} {constant:?}");
    }
    println!("globals ({}):", chunk.global_names.len());
    for (index, name) in chunk.global_names.iter().enumerate() {
        println!("  {index:04} {name}");
    }
    disassemble_function(chunk, &chunk.main)?;
    for function in &chunk.protos {
        disassemble_function(chunk, function)?;
    }
    Ok(())
}

fn disassemble_function(chunk: &Chunk, function: &FunctionProto) -> Result<(), String> {
    println!();
    println!(
        "fn {} arity={} locals={} bytes={}",
        function.name,
        function.arity,
        function.locals,
        function.code.len()
    );
    let mut offset = 0;
    while offset < function.code.len() {
        let instruction_offset = offset;
        let byte = function.code[offset];
        let op = Op::from_byte(byte).ok_or_else(|| {
            format!(
                "{}: unknown opcode {byte} at byte {instruction_offset}",
                function.name
            )
        })?;
        offset += 1;
        let operand = match op.operand_width() {
            0 => None,
            1 => {
                let value = function.code.get(offset).copied().ok_or_else(|| {
                    format!(
                        "{}: truncated {op:?} operand at byte {instruction_offset}",
                        function.name
                    )
                })?;
                offset += 1;
                Some(u16::from(value))
            }
            2 => {
                let low = function.code.get(offset).copied().ok_or_else(|| {
                    format!(
                        "{}: truncated {op:?} operand at byte {instruction_offset}",
                        function.name
                    )
                })?;
                let high = function.code.get(offset + 1).copied().ok_or_else(|| {
                    format!(
                        "{}: truncated {op:?} operand at byte {instruction_offset}",
                        function.name
                    )
                })?;
                offset += 2;
                Some(u16::from_le_bytes([low, high]))
            }
            width => {
                return Err(format!(
                    "{}: unsupported operand width {width} for {op:?}",
                    function.name
                ));
            }
        };
        let annotation = operand_annotation(chunk, op, operand);
        if let Some(operand) = operand {
            println!("  {instruction_offset:04} {op:?} {operand}{annotation}");
        } else {
            println!("  {instruction_offset:04} {op:?}");
        }
    }
    Ok(())
}

fn operand_annotation(chunk: &Chunk, op: Op, operand: Option<u16>) -> String {
    let Some(index) = operand.map(usize::from) else {
        return String::new();
    };
    match op {
        Op::LoadConst => chunk
            .constants
            .get(index)
            .map(|constant| format!(" ; {constant:?}"))
            .unwrap_or_else(|| " ; INVALID constant index".to_string()),
        Op::LoadGlobal | Op::StoreGlobal => chunk
            .global_names
            .get(index)
            .map(|name| format!(" ; {name}"))
            .unwrap_or_else(|| " ; INVALID global index".to_string()),
        Op::MakeClosure => chunk
            .protos
            .get(index)
            .map(|function| format!(" ; {}", function.name))
            .unwrap_or_else(|| " ; INVALID prototype index".to_string()),
        Op::Jump | Op::JumpIfFalse => format!(" ; target byte {index}"),
        Op::LoadLocal | Op::StoreLocal => format!(" ; local {index}"),
        Op::Call => format!(" ; argc {index}"),
        _ => String::new(),
    }
}

fn print_help() {
    println!("lkjscript - typed line-oriented language runtime");
    println!();
    println!("Usage:");
    println!("  lkjscript run <file.lkjscript> [--] [script-args...]");
    println!("  lkjscript disasm <file.lkjscript>");
    println!("  lkjscript --help");
    println!("  lkjscript --version");
    println!();
    println!("Environment:");
    println!("  LKJSCRIPT_ROOT  installed root containing src/std and src/lib");
}
