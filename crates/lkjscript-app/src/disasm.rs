use std::path::PathBuf;
use std::process::ExitCode;

use lkjscript_compiler::compile_path;
use lkjscript_core::{DecodedInstruction, FunctionProto, Op, ValidatedChunk};

pub fn command(args: &[String]) -> Result<ExitCode, String> {
    let file = request(args)?;
    let source = PathBuf::from(file);
    lkjscript_compiler::package::verify(&source).map_err(|error| error.to_string())?;
    let program = compile_path(&source).map_err(|error| error.to_string())?;
    disassemble(program.bytecode())?;
    Ok(ExitCode::SUCCESS)
}

fn request(args: &[String]) -> Result<&str, String> {
    match args {
        [command, file] if command == "disasm" => Ok(file),
        _ => Err("usage: disasm <file.lkjscript>".to_string()),
    }
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
            "  {index:04} {} region={} routes={:?} ({})",
            product.name,
            product.region,
            product.region_fields,
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
        match operand {
            lkjscript_core::DecodedOperand::None => println!("  {offset:04} {op:?}"),
            lkjscript_core::DecodedOperand::U16(operand) => {
                println!("  {offset:04} {op:?} {operand}{annotation}");
            }
            lkjscript_core::DecodedOperand::Index(operand) => {
                println!("  {offset:04} {op:?} {operand}{annotation}");
            }
            lkjscript_core::DecodedOperand::PlaceLocal { place, local } => {
                println!("  {offset:04} {op:?} place={place} local={local}{annotation}");
            }
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

fn operand_annotation(
    chunk: &ValidatedChunk,
    op: Op,
    operand: lkjscript_core::DecodedOperand,
) -> String {
    let Some(index) = operand.index() else {
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use lkjscript_core::{
        validate_chunk, Chunk, DecodedOperand, Op, ProductFieldRef, ProductId, ProductMetadata,
        ValidationPolicy,
    };

    use super::{operand_annotation, product_field, request};

    #[test]
    fn removed_resource_profile_flag_is_rejected() {
        assert!(request(&["disasm".into(), "main.lkjscript".into()]).is_ok());
        assert!(request(&[
            "disasm".into(),
            "--resource-profile".into(),
            "sandbox".into(),
            "main.lkjscript".into(),
        ])
        .is_err());
    }

    #[test]
    fn product_annotations_only_receive_validated_metadata() {
        let mut chunk = Chunk::new();
        let plan = lkjscript_core::MemoryPlanId::new([2; 32]);
        chunk.memory_plan = Some(plan);
        chunk.main.memory_plan = Some(plan);
        chunk.products.push(ProductMetadata {
            id: ProductId::new(0),
            identity: lkjscript_core::runtime_product_contract_identity(plan, "Point")
                .expect("canonical product identity"),
            region: true,
            name: "Point".into(),
            fields: vec!["x".into()],
            region_fields: vec![lkjscript_core::RegionProductFieldKind::Unit],
        });
        chunk.product_fields.push(ProductFieldRef {
            product: ProductId::new(0),
            field: 0,
        });
        chunk.main.emit(Op::Unit);
        chunk.main.emit_op_u16(Op::MakeProduct, 0);
        chunk.main.emit_op_u16(Op::LoadProductField, 0);
        chunk.main.emit(Op::Return);
        let chunk = validate_chunk(chunk, ValidationPolicy::Unrestricted)
            .expect("product disassembly chunk validates");
        assert_eq!(
            operand_annotation(&chunk, Op::MakeProduct, DecodedOperand::U16(0)),
            " ; Point"
        );
        assert_eq!(
            operand_annotation(&chunk, Op::LoadProductField, DecodedOperand::U16(0)),
            " ; Point.x"
        );
        assert_eq!(product_field(&chunk, 0), Some(("Point", "x")));
    }
}
