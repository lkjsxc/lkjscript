//! CLI entry for the lkjscript language runtime.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use lkjscript_compiler::compile_path;
use lkjscript_core::{Chunk, FunctionProto, Limits, Op, ProductMetadata, MAX_PRODUCT_FIELDS};
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
    println!("products ({}):", chunk.products.len());
    for (index, product) in chunk.products.iter().enumerate() {
        if valid_product(chunk, index).is_some() {
            println!(
                "  {index:04} {} ({})",
                product.name,
                product.fields.join(", ")
            );
        } else {
            println!("  {index:04} INVALID product metadata");
        }
    }
    println!("product fields ({}):", chunk.product_fields.len());
    for index in 0..chunk.product_fields.len() {
        let annotation = valid_product_field(chunk, index)
            .map(|(product, field)| format!("{product}.{field}"))
            .unwrap_or_else(|| "INVALID product field".to_string());
        println!("  {index:04} {annotation}");
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

fn valid_product(chunk: &Chunk, index: usize) -> Option<&ProductMetadata> {
    let raw = u16::try_from(index).ok()?;
    chunk
        .products
        .get(index)
        .filter(|product| product.id.raw() == raw && product.fields.len() <= MAX_PRODUCT_FIELDS)
}

fn valid_product_field(chunk: &Chunk, index: usize) -> Option<(&str, &str)> {
    let field_ref = chunk.product_fields.get(index)?;
    let product = valid_product(chunk, field_ref.product.index())?;
    let field = product.fields.get(usize::from(field_ref.field))?;
    Some((&product.name, field))
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
        Op::MakeProduct => valid_product(chunk, index)
            .map(|product| format!(" ; {}", product.name))
            .unwrap_or_else(|| " ; INVALID product index or metadata".to_string()),
        Op::LoadProductField | Op::WithProductField => valid_product_field(chunk, index)
            .map(|(product, field)| format!(" ; {product}.{field}"))
            .unwrap_or_else(|| " ; INVALID product field index or metadata".to_string()),
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
mod tests {
    use lkjscript_core::{Chunk, Op, ProductFieldRef, ProductId, ProductMetadata};

    use super::{operand_annotation, valid_product, valid_product_field};

    #[test]
    fn product_disassembly_annotations_reject_malformed_metadata() {
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
        assert_eq!(
            operand_annotation(&chunk, Op::MakeProduct, Some(0)),
            " ; Point"
        );
        assert_eq!(
            operand_annotation(&chunk, Op::LoadProductField, Some(0)),
            " ; Point.x"
        );
        assert!(valid_product(&chunk, 0).is_some());
        assert_eq!(valid_product_field(&chunk, 0), Some(("Point", "x")));

        chunk.products[0].id = ProductId::new(1);
        assert!(operand_annotation(&chunk, Op::MakeProduct, Some(0)).contains("INVALID"));
        assert!(operand_annotation(&chunk, Op::LoadProductField, Some(0)).contains("INVALID"));

        chunk.products[0].id = ProductId::new(0);
        chunk.product_fields[0].field = 1;
        assert!(operand_annotation(&chunk, Op::WithProductField, Some(0)).contains("INVALID"));
    }
}
