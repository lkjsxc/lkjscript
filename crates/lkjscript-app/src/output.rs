use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use lkjscript_core::ExecutionOutcome;
use lkjscript_jit::JitStats;

pub fn diagnostics_enabled() -> bool {
    env::var_os("LKJSCRIPT_JIT_DIAGNOSTICS").is_some()
        || env::var_os("LKJSCRIPT_JIT_DUMP_DIR").is_some()
}

pub fn print_jit_diagnostics(program: &lkjscript_ir::VerifiedProgram, stats: &JitStats) {
    eprintln!("jit.verified_baseline_ssa={:?}", program.program());
    if stats.optimizing_code_objects > 0 {
        match lkjscript_ir::optimize(program, lkjscript_ir::OptimizationLimits::default()) {
            Ok(optimized) => eprintln!("jit.verified_optimized_ssa={:?}", optimized.program()),
            Err(error) => eprintln!("jit.optimized_ssa_diagnostic_error={error}"),
        }
    }
    eprintln!(
        "jit.native_entries={} jit.baseline_entries={} jit.optimizing_entries={} jit.direct_native_calls={} jit.poll_v1_calls={} jit.vm_fallbacks={} jit.compile_failures={} jit.algebraic_rewrites={} jit.gvn_rewrites={} jit.checked_i64_rewrites={}",
        stats.native_entries,
        stats.baseline_native_entries,
        stats.optimizing_native_entries,
        stats.direct_native_calls,
        stats.poll_v1_calls,
        stats.vm_fallbacks,
        stats.compile_failures,
        stats.algebraic_rewrites,
        stats.gvn_rewrites,
        stats.checked_i64_rewrites,
    );
    for object in &stats.code_objects {
        eprintln!("jit.code_object={object:?}");
        if let Some(certificate) = &object.optimization_certificate {
            eprintln!("jit.optimization_certificate={certificate:?}");
        }
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
                        eprintln!("jit.diagnostic_error=write {}: {error}", path.display());
                    }
                }
            }
        }
    }
    for function in &stats.functions {
        eprintln!("jit.function={function:?}");
    }
}

pub fn outcome_exit_code(outcome: ExecutionOutcome) -> Result<ExitCode, String> {
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
