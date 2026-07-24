//! CLI entry for the lkjscript language runtime.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use lkjscript_compiler::{compile_path, compile_path_with_metrics, CompileMetrics};
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
    let metrics_enabled = metrics_enabled();
    let (program, compile_metrics) = if metrics_enabled {
        compile_path_with_metrics(&PathBuf::from(&options.file), &Limits::default())
            .map_err(|error| error.to_string())?
    } else {
        (
            compile_path(&PathBuf::from(&options.file), &Limits::default())
                .map_err(|error| error.to_string())?,
            CompileMetrics::default(),
        )
    };
    let execution = ExecutionConfig::default();
    let jit_config = JitConfig {
        auto_threshold: options.auto_threshold,
        auto_enabled: options.auto_enabled,
        retain_machine_code_diagnostics: diagnostics_enabled()
            || env::var_os("LKJSCRIPT_JIT_DUMP_DIR").is_some(),
        collect_metrics: metrics_enabled,
        ..JitConfig::default()
    };
    let engine_started = metrics_enabled.then(Instant::now);
    let mut vm_execution = Duration::ZERO;
    let (outcome, stats) = match options.engine {
        Engine::Vm => {
            let started = metrics_enabled.then(Instant::now);
            let outcome = run_chunk_with_args(program.bytecode(), &options.script_args, &execution);
            if let Some(started) = started {
                vm_execution = started.elapsed();
            }
            (outcome, None)
        }
        Engine::BaselineJit => {
            let execution = execute_forced(program.ssa(), &execution, jit_config)
                .map_err(|error| format!("engine error: {error}"))?;
            (execution.outcome, Some(execution.stats))
        }
        Engine::Auto => {
            let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), jit_config);
            let started = metrics_enabled.then(Instant::now);
            let (outcome, stats) = run_chunk_auto(
                program.bytecode(),
                &options.script_args,
                &execution,
                session,
            );
            if let Some(started) = started {
                vm_execution = started.elapsed();
            }
            (outcome, Some(stats))
        }
    };
    let engine_execution = engine_started.map_or(Duration::ZERO, |started| started.elapsed());
    if diagnostics_enabled() {
        if let Some(stats) = &stats {
            print_jit_diagnostics(program.ssa(), stats);
        }
    }
    if metrics_enabled {
        emit_metrics(MetricReport {
            engine: options.engine,
            configured_threshold: options.auto_threshold,
            auto_enabled: options.auto_enabled,
            compile: &compile_metrics,
            vm_execution,
            engine_execution,
            outcome: &outcome,
            stats: stats.as_ref(),
        })?;
    }
    outcome_exit_code(outcome)
}

fn parse_run_options(args: &[String]) -> Result<RunOptions, String> {
    let mut index = 1_usize;
    let mut engine = Engine::Auto;
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

fn metrics_enabled() -> bool {
    env::var_os("LKJSCRIPT_METRICS").is_some() || env::var_os("LKJSCRIPT_METRICS_FILE").is_some()
}

fn diagnostics_enabled() -> bool {
    env::var_os("LKJSCRIPT_JIT_DIAGNOSTICS").is_some()
        || env::var_os("LKJSCRIPT_JIT_DUMP_DIR").is_some()
}

struct MetricReport<'a> {
    engine: Engine,
    configured_threshold: u64,
    auto_enabled: bool,
    compile: &'a CompileMetrics,
    vm_execution: Duration,
    engine_execution: Duration,
    outcome: &'a ExecutionOutcome,
    stats: Option<&'a JitStats>,
}

fn emit_metrics(report: MetricReport<'_>) -> Result<(), String> {
    let engine = match report.engine {
        Engine::Vm => "vm",
        Engine::Auto => "auto",
        Engine::BaselineJit => "baseline-jit",
    };
    let mut native_lowering = Duration::ZERO;
    let mut installation = Duration::ZERO;
    if let Some(stats) = report.stats {
        for object in &stats.code_objects {
            native_lowering =
                native_lowering.saturating_add(object.compile_stats.lowering_and_encoding());
            installation = installation.saturating_add(object.compile_stats.installation());
        }
    }
    let time_to_first_native = report
        .stats
        .and_then(|stats| stats.time_to_first_native_entry)
        .map_or_else(|| "null".to_string(), duration_ns);
    let first_native = report
        .stats
        .and_then(|stats| stats.first_native_call)
        .map_or_else(|| "null".to_string(), duration_ns);
    let native_execution = report
        .stats
        .map_or(Duration::ZERO, |stats| stats.native_execution);
    let jit = report
        .stats
        .map_or_else(|| "null".to_string(), jit_metrics_json);
    let json = format!(
        "{{\"schema\":\"lkjscript.metrics.v1\",\"engine\":{engine},\"configured_auto_threshold\":{configured_threshold},\"auto_enabled\":{auto_enabled},\"outcome\":{outcome},\"timings_ns\":{{\"compile_total\":{compile_total},\"source_loading\":{source_loading},\"parse\":{parsing},\"hir_analysis\":{hir_analysis},\"effect_analysis\":{effect_analysis},\"ssa_construction\":{ssa_construction},\"ssa_verification\":{ssa_verification},\"normalization\":{normalization},\"reference_bytecode_lowering\":{bytecode_lowering},\"reference_bytecode_validation\":{bytecode_validation},\"native_lowering_encoding\":{native_lowering},\"relocation_wx_installation\":{installation},\"time_to_first_native_entry\":{time_to_first_native},\"first_native_call\":{first_native},\"native_execution\":{native_execution},\"vm_execution\":{vm_execution},\"engine_execution\":{engine_execution}}},\"source_files\":{source_files},\"jit\":{jit}}}",
        engine = json_string(engine),
        configured_threshold = report.configured_threshold,
        auto_enabled = report.auto_enabled,
        outcome = outcome_json(report.outcome),
        compile_total = report.compile.total.as_nanos(),
        source_loading = report.compile.source_loading.as_nanos(),
        parsing = report.compile.parsing.as_nanos(),
        hir_analysis = report.compile.hir_analysis.as_nanos(),
        effect_analysis = report.compile.effect_analysis.as_nanos(),
        ssa_construction = report.compile.ssa_construction.as_nanos(),
        ssa_verification = report.compile.ssa_verification.as_nanos(),
        normalization = report.compile.normalization.as_nanos(),
        bytecode_lowering = report.compile.bytecode_lowering.as_nanos(),
        bytecode_validation = report.compile.bytecode_validation.as_nanos(),
        native_lowering = native_lowering.as_nanos(),
        installation = installation.as_nanos(),
        native_execution = native_execution.as_nanos(),
        vm_execution = report.vm_execution.as_nanos(),
        engine_execution = report.engine_execution.as_nanos(),
        source_files = report.compile.source_files,
    );
    let line = format!("LKJSCRIPT_METRICS {json}\n");
    if let Some(path) = env::var_os("LKJSCRIPT_METRICS_FILE") {
        std::fs::write(PathBuf::from(path), line)
            .map_err(|error| format!("write metrics file: {error}"))?;
    } else {
        eprint!("{line}");
    }
    Ok(())
}

fn duration_ns(duration: Duration) -> String {
    duration.as_nanos().to_string()
}

fn outcome_json(outcome: &ExecutionOutcome) -> String {
    match outcome {
        ExecutionOutcome::Returned(value) => {
            let (kind, exact) = if value.is_unit() {
                ("unit", "unit".to_string())
            } else if value.is_empty_list() {
                ("empty-list", "empty-list".to_string())
            } else if value.is_none() {
                ("none", "none".to_string())
            } else if let Some(value) = value.as_bool() {
                ("bool", value.to_string())
            } else if let Some(value) = value.as_i64() {
                ("i64", value.to_string())
            } else if let Some(value) = value.as_f64() {
                ("f64-bits", format!("0x{:016x}", value.to_bits()))
            } else if let Some(value) = value.as_str() {
                ("str-or-symbol", value.to_string())
            } else if let Some(value) = value.as_handle() {
                ("handle", value.to_string())
            } else if let Some(value) = value.product_id() {
                ("product", value.raw().to_string())
            } else {
                ("owned-value", format!("{value:?}"))
            };
            format!(
                "{{\"kind\":\"returned\",\"value_kind\":{},\"exact\":{}}}",
                json_string(kind),
                json_string(&exact)
            )
        }
        ExecutionOutcome::Exited(code) => {
            format!("{{\"kind\":\"exited\",\"code\":{code}}}")
        }
        ExecutionOutcome::Trapped(trap) => format!(
            "{{\"kind\":\"trapped\",\"detail\":{}}}",
            json_string(trap.as_str())
        ),
        ExecutionOutcome::DeadlineExceeded => "{\"kind\":\"deadline-exceeded\"}".to_string(),
        ExecutionOutcome::ResourceLimitExceeded(kind) => format!(
            "{{\"kind\":\"resource-limit-exceeded\",\"resource\":{}}}",
            json_string(&format!("{kind:?}"))
        ),
        ExecutionOutcome::HostFailure(error) => format!(
            "{{\"kind\":\"host-failure\",\"detail\":{},\"prior\":{}}}",
            json_string(error.as_str()),
            error
                .prior_outcome()
                .map_or_else(|| "null".to_string(), json_string)
        ),
    }
}

fn jit_metrics_json(stats: &JitStats) -> String {
    let functions = stats
        .functions
        .iter()
        .map(|function| {
            format!(
                "{{\"id\":{},\"name\":{},\"state\":{},\"calls\":{},\"attempts\":{},\"failure\":{},\"code_object\":{},\"epoch\":{},\"native_entries\":{}}}",
                function.function().raw(),
                json_string(function.name()),
                json_string(&format!("{:?}", function.state())),
                function.call_count(),
                function.attempts(),
                function.last_failure().map_or_else(
                    || "null".to_string(),
                    |failure| json_string(&format!("{failure:?}"))
                ),
                function.code_object().map_or_else(|| "null".to_string(), |id| id.to_string()),
                function.epoch(),
                function.native_entries(),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let objects = stats
        .code_objects
        .iter()
        .map(|object| {
            format!(
                "{{\"identity\":{},\"functions\":{},\"code_bytes\":{},\"metadata_bytes\":{},\"accounted_allocation_bytes\":{},\"relocations\":{},\"safepoints\":{},\"lowering_encoding_ns\":{},\"installation_ns\":{},\"work_units\":{},\"native_entries\":{},\"wx_verified\":{}}}",
                object.identity,
                object.functions.len(),
                object.code_bytes,
                object.metadata_bytes,
                object.accounted_allocation_bytes,
                object.relocation_count,
                object.safepoint_count,
                object.compile_stats.lowering_and_encoding().as_nanos(),
                object.compile_stats.installation().as_nanos(),
                object.compile_stats.work_units(),
                object.native_entry_count,
                object.wx_transition_verified,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"compile_failures\":{},\"vm_fallbacks\":{},\"native_entries\":{},\"direct_native_calls\":{},\"poll_v1_calls\":{},\"native_invocations\":{},\"auto_threshold\":{},\"auto_enabled\":{},\"code_cache_peak_objects\":{},\"code_cache_peak_bytes\":{},\"metadata_cache_peak_bytes\":{},\"accounted_allocation_peak_bytes\":{},\"allocations\":{},\"allocation_bytes\":{},\"collections\":{},\"peak_live_heap_bytes\":{},\"maximum_roots\":{},\"runtime_heap_calls\":{},\"barrier_count\":{},\"peak_native_frame_depth\":{},\"vm_to_native_transitions\":{},\"native_to_vm_transitions\":{},\"functions\":[{}],\"objects\":[{}]}}",
        stats.compile_failures,
        stats.vm_fallbacks,
        stats.native_entries,
        stats.direct_native_calls,
        stats.poll_v1_calls,
        stats.native_invocations,
        stats.auto_threshold,
        stats.auto_enabled,
        stats.code_cache_peak_objects,
        stats.code_cache_peak_bytes,
        stats.metadata_cache_peak_bytes,
        stats.accounted_allocation_peak_bytes,
        stats.allocations,
        stats.allocation_bytes,
        stats.collections,
        stats.peak_live_heap_bytes,
        stats.maximum_roots,
        stats.runtime_heap_calls,
        stats.barrier_count,
        stats.peak_native_frame_depth,
        stats.vm_to_native_transitions,
        stats.native_to_vm_transitions,
        functions,
        objects,
    )
}

fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len().saturating_add(2));
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", u32::from(character)));
            }
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
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
    println!("                 default: auto at 64 function entries; explicit vm is deterministic");
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
    println!("  LKJSCRIPT_METRICS          emit one low-overhead JSON metrics line to stderr");
    println!("  LKJSCRIPT_METRICS_FILE     write that metrics line to an explicit file instead");
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use lkjscript_core::{
        validate_chunk, Chunk, Op, ProductFieldRef, ProductId, ProductMetadata, ValidationLimits,
    };

    use super::{operand_annotation, parse_run_options, product_field, Engine};

    #[test]
    fn ordinary_run_defaults_to_auto_and_explicit_vm_remains_available() {
        let default =
            parse_run_options(&["run".into(), "main.lkjscript".into()]).expect("parse default run");
        assert_eq!(default.engine, Engine::Auto);
        assert_eq!(default.auto_threshold, 64);

        let explicit_vm = parse_run_options(&[
            "run".into(),
            "--engine".into(),
            "vm".into(),
            "main.lkjscript".into(),
        ])
        .expect("parse explicit VM run");
        assert_eq!(explicit_vm.engine, Engine::Vm);
    }

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
