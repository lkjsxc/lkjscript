//! CLI entry for the lkjscript language runtime.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use lkjscript_compiler::compile_path;
use lkjscript_core::{
    DecodedInstruction, ExecutionConfig, ExecutionOutcome, FunctionProto, Limits, Op,
    ValidatedChunk,
};
use lkjscript_vm::run_chunk_with_args;

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("lkjscript: {error}");
            ExitCode::from(1)
        }
    }
}

fn real_main() -> Result<ExitCode, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version" | "-V") if args.len() == 1 => {
            println!("lkjscript {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
        None | Some("--help" | "-h") => {
            print_help();
            Ok(ExitCode::SUCCESS)
        }
        Some("run") => run_command(&args),
        Some("disasm") => disasm_command(&args),
        Some(other) => Err(format!("unknown command: {other}")),
    }
}

fn run_command(args: &[String]) -> Result<ExitCode, String> {
    let file = args
        .get(1)
        .ok_or_else(|| "run needs a .lkjscript path".to_string())?;
    let script_arg_start = if args.get(2).map(String::as_str) == Some("--") {
        3
    } else {
        2
    };
    let script_args = args.get(script_arg_start..).unwrap_or_default().to_vec();
    let program = compile_path(&PathBuf::from(file), &Limits::default())
        .map_err(|error| error.to_string())?;
    let outcome = run_chunk_with_args(
        program.bytecode(),
        &script_args,
        &ExecutionConfig::default(),
    );
    match outcome {
        ExecutionOutcome::Returned(_) => Ok(ExitCode::SUCCESS),
        ExecutionOutcome::Exited(code) => {
            let portable = u8::try_from(code.rem_euclid(256))
                .map_err(|_| format!("invalid process exit code {code}"))?;
            Ok(ExitCode::from(portable))
        }
        ExecutionOutcome::Trapped(trap) => Err(format!("trap: {trap}")),
        ExecutionOutcome::DeadlineExceeded => Err("execution deadline exceeded".to_string()),
        ExecutionOutcome::ResourceLimitExceeded(kind) => {
            Err(format!("execution resource limit exceeded: {kind:?}"))
        }
        ExecutionOutcome::HostFailure(error) => Err(format!("host failure: {error}")),
    }
}

fn disasm_command(args: &[String]) -> Result<ExitCode, String> {
    let file = args
        .get(1)
        .ok_or_else(|| "disasm needs a .lkjscript path".to_string())?;
    if args.len() != 2 {
        return Err("disasm accepts exactly one .lkjscript path".to_string());
    }
    let program = compile_path(&PathBuf::from(file), &Limits::default())
        .map_err(|error| error.to_string())?;
    disassemble(program.bytecode())?;
    Ok(ExitCode::SUCCESS)
}

fn disassemble(chunk: &ValidatedChunk) -> Result<(), String> {
    println!("constants ({}):", chunk.constants().len());
    for (index, constant) in chunk.constants().iter().enumerate() {
        println!("  {index:04} {constant:?}");
    }
    println!("globals ({}):", chunk.global_names().len());
    for (index, name) in chunk.global_names().iter().enumerate() {
        println!("  {index:04} {name}");
    }
    println!("products ({}):", chunk.products().len());
    for (index, product) in chunk.products().iter().enumerate() {
        println!(
            "  {index:04} {} ({})",
            product.name,
            product.fields.join(", ")
        );
    }
    println!("product fields ({}):", chunk.product_fields().len());
    for index in 0..chunk.product_fields().len() {
        let (product, field) = product_field(chunk, index)
            .ok_or_else(|| "validated product descriptor became inconsistent".to_string())?;
        println!("  {index:04} {product}.{field}");
    }
    disassemble_function(chunk, chunk.main(), chunk.main_instructions())?;
    for (index, function) in chunk.protos().iter().enumerate() {
        let instructions = chunk
            .proto_instructions(index)
            .ok_or_else(|| "validated function decode metadata is missing".to_string())?;
        disassemble_function(chunk, function, instructions)?;
    }
    Ok(())
}

fn disassemble_function(
    chunk: &ValidatedChunk,
    function: &FunctionProto,
    instructions: &[DecodedInstruction],
) -> Result<(), String> {
    println!();
    println!(
        "fn {} arity={} locals={} bytes={}",
        function.name,
        function.arity,
        function.locals,
        function.code.len()
    );
    for instruction in instructions {
        let offset = instruction.offset();
        let op = instruction.op();
        let operand = instruction.operand();
        let annotation = operand_annotation(chunk, op, operand);
        if let Some(operand) = operand {
            println!("  {offset:04} {op:?} {operand}{annotation}");
        } else {
            println!("  {offset:04} {op:?}");
        }
    }
    Ok(())
}

fn product_field(chunk: &ValidatedChunk, index: usize) -> Option<(&str, &str)> {
    let field_ref = chunk.product_fields().get(index)?;
    let product = chunk.products().get(field_ref.product.index())?;
    let field = product.fields.get(usize::from(field_ref.field))?;
    Some((&product.name, field))
}

fn operand_annotation(chunk: &ValidatedChunk, op: Op, operand: Option<u16>) -> String {
    let Some(index) = operand.map(usize::from) else {
        return String::new();
    };
    match op {
        Op::LoadConst => chunk
            .constants()
            .get(index)
            .map(|constant| format!(" ; {constant:?}"))
            .unwrap_or_default(),
        Op::LoadGlobal | Op::StoreGlobal => chunk
            .global_names()
            .get(index)
            .map(|name| format!(" ; {name}"))
            .unwrap_or_default(),
        Op::MakeClosure => format!(" ; captures {index}"),
        Op::MakeProduct => chunk
            .products()
            .get(index)
            .map(|product| format!(" ; {}", product.name))
            .unwrap_or_default(),
        Op::LoadProductField | Op::WithProductField => product_field(chunk, index)
            .map(|(product, field)| format!(" ; {product}.{field}"))
            .unwrap_or_default(),
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use lkjscript_core::{
        validate_chunk, Chunk, Op, ProductFieldRef, ProductId, ProductMetadata, ValidationLimits,
    };

    use super::{operand_annotation, product_field};

    #[test]
    fn product_annotations_only_receive_validated_metadata() {
        let mut chunk = Chunk::new();
        chunk.products.push(ProductMetadata {
            id: ProductId::new(0),
            name: "Point".into(),
            fields: vec!["x".into()],
        });
        chunk.product_fields.push(ProductFieldRef {
            product: ProductId::new(0),
            field: 0,
        });
        chunk.main.emit(Op::Unit);
        chunk.main.emit_op_u16(Op::MakeProduct, 0);
        chunk.main.emit_op_u16(Op::LoadProductField, 0);
        chunk.main.emit(Op::Return);
        let chunk = validate_chunk(chunk, &ValidationLimits::default())
            .expect("product disassembly chunk validates");
        assert_eq!(
            operand_annotation(&chunk, Op::MakeProduct, Some(0)),
            " ; Point"
        );
        assert_eq!(
            operand_annotation(&chunk, Op::LoadProductField, Some(0)),
            " ; Point.x"
        );
        assert_eq!(product_field(&chunk, 0), Some(("Point", "x")));
    }
}
