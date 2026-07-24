//! CLI entry for the lkjscript language runtime.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use lkjscript_compiler::compile_path;
use lkjscript_core::{
    DecodedInstruction, ExecutionConfig, ExecutionOutcome, FunctionProto, Limits, Op,
    ValidatedChunk,
};
use lkjscript_jit::{execute_forced, JitConfig, JitSession, JitStats};
use lkjscript_vm::{run_chunk_auto, run_chunk_with_args};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Engine {
    Vm,
    Auto,
    BaselineJit,
}

struct RunOptions {
    engine: Engine,
    auto_threshold: u64,
    auto_enabled: bool,
    file: String,
    script_args: Vec<String>,
}

fn run_command(args: &[String]) -> Result<ExitCode, String> {
    let options = parse_run_options(args)?;
    let program = compile_path(&PathBuf::from(&options.file), &Limits::default())
        .map_err(|error| error.to_string())?;
    let execution = ExecutionConfig::default();
    let jit_config = JitConfig {
        auto_threshold: options.auto_threshold,
        auto_enabled: options.auto_enabled,
        retain_machine_code_diagnostics: diagnostics_enabled()
            || env::var_os("LKJSCRIPT_JIT_DUMP_DIR").is_some(),
        ..JitConfig::default()
    };
    let (outcome, stats) = match options.engine {
        Engine::Vm => (
            run_chunk_with_args(program.bytecode(), &options.script_args, &execution),
            None,
        ),
        Engine::BaselineJit => {
            let execution = execute_forced(program.ssa(), &execution, jit_config)
                .map_err(|error| format!("engine error: {error}"))?;
            (execution.outcome, Some(execution.stats))
        }
        Engine::Auto => {
            let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), jit_config);
            let (outcome, stats) = run_chunk_auto(
                program.bytecode(),
                &options.script_args,
                &execution,
                session,
            );
            (outcome, Some(stats))
        }
    };
    if diagnostics_enabled() {
        if let Some(stats) = &stats {
            print_jit_diagnostics(program.ssa(), stats);
        }
    }
    outcome_exit_code(outcome)
}

fn parse_run_options(args: &[String]) -> Result<RunOptions, String> {
    let mut index = 1_usize;
    let mut engine = Engine::Vm;
    let mut auto_threshold = JitConfig::default().auto_threshold;
    let mut auto_enabled = true;
    while let Some(argument) = args.get(index).map(String::as_str) {
        match argument {
            "--engine" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--engine needs vm, auto, or baseline-jit".to_string())?;
                engine = match value.as_str() {
                    "vm" => Engine::Vm,
                    "auto" => Engine::Auto,
                    "baseline-jit" => Engine::BaselineJit,
                    other => return Err(format!("unknown execution engine: {other}")),
                };
                index += 2;
            }
            "--auto-jit-threshold" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--auto-jit-threshold needs a positive integer".to_string())?;
                auto_threshold = value
                    .parse::<u64>()
                    .ok()
                    .filter(|value| *value > 0)
                    .ok_or_else(|| "--auto-jit-threshold needs a positive integer".to_string())?;
                index += 2;
            }
            "--disable-auto-jit" => {
                auto_enabled = false;
                index += 1;
            }
            _ => break,
        }
    }
    let file = args
        .get(index)
        .ok_or_else(|| "run needs a .lkjscript path".to_string())?
        .clone();
    index += 1;
    if args.get(index).map(String::as_str) == Some("--") {
        index += 1;
    }
    let script_args = args.get(index..).unwrap_or_default().to_vec();
    Ok(RunOptions {
        engine,
        auto_threshold,
        auto_enabled,
        file,
        script_args,
    })
}

fn diagnostics_enabled() -> bool {
    env::var_os("LKJSCRIPT_JIT_DIAGNOSTICS").is_some()
}

fn print_jit_diagnostics(program: &lkjscript_ir::VerifiedProgram, stats: &JitStats) {
    eprintln!("jit.pre_native_ssa={:?}", program.program());
    eprintln!("jit.post_native_ssa={:?}", program.program());
    eprintln!(
        "jit.native_entries={} jit.direct_native_calls={} jit.poll_v1_calls={} jit.vm_fallbacks={} jit.compile_failures={}",
        stats.native_entries,
        stats.direct_native_calls,
        stats.poll_v1_calls,
        stats.vm_fallbacks,
        stats.compile_failures
    );
    for object in &stats.code_objects {
        eprintln!("jit.code_object={object:?}");
        if let Some(bytes) = &object.diagnostic_machine_code {
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            eprintln!("jit.machine_code.{}={hex}", object.identity);
            if let Some(directory) = env::var_os("LKJSCRIPT_JIT_DUMP_DIR") {
                let directory = PathBuf::from(directory);
                let path = directory.join(format!("code-object-{}.bin", object.identity));
                match std::fs::create_dir_all(&directory)
                    .and_then(|()| std::fs::write(&path, bytes))
                {
                    Ok(()) => eprintln!(
                        "jit.objdump_hint=objdump -D -b binary -m i386:x86-64 -M intel {}",
                        path.display()
                    ),
                    Err(error) => {
                        eprintln!("jit.diagnostic_error=write {}: {error}", path.display())
                    }
                }
            }
        }
    }
    for function in &stats.functions {
        eprintln!("jit.function={function:?}");
    }
}

fn outcome_exit_code(outcome: ExecutionOutcome) -> Result<ExitCode, String> {
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
    println!("  lkjscript run [--engine vm|auto|baseline-jit] [--auto-jit-threshold N]");
    println!("                 [--disable-auto-jit] <file.lkjscript> [--] [script-args...]");
    println!("  lkjscript disasm <file.lkjscript>");
    println!("  lkjscript --help");
    println!("  lkjscript --version");
    println!();
    println!("Environment:");
    println!("  LKJSCRIPT_ROOT             installed root containing src/std and src/lib");
    println!(
        "  LKJSCRIPT_JIT_DIAGNOSTICS  emit SSA, bytes, relocations, metadata, counts to stderr"
    );
    println!("  LKJSCRIPT_JIT_DUMP_DIR     write generated .bin files for external objdump");
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
