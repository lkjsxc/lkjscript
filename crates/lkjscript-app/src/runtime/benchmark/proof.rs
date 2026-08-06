use std::error::Error;
use std::time::Instant;

use lkjscript_compiler::compile_source;
use lkjscript_core::ExecutionConfig;
use lkjscript_jit::{execute_optimizing, JitConfig};

pub(super) fn run(samples: usize) -> Result<(), Box<dyn Error>> {
    let source = include_str!("../../../tests/fixtures/optimizing-loop.lkjscript");
    let program = compile_source(source, "proof-benchmark.lkjscript")?;
    println!(concat!(
        "proof-workers\tsamples\tp50-ns\tp95-ns\tp99-ns\t",
        "certificate-records\tcertificate-bytes\tnative-entries\tvm-fallbacks"
    ));
    for workers in [1_u16, 2, 4] {
        let mut elapsed = Vec::with_capacity(samples);
        let mut evidence = None;
        for sample in 0..samples + 3 {
            let config = JitConfig {
                proof_discovery_workers: workers,
                ..JitConfig::default()
            };
            let start = Instant::now();
            let execution = execute_optimizing(program.ssa(), &ExecutionConfig::default(), config)?;
            let duration = start.elapsed().as_nanos();
            if execution.stats.optimizing_native_entries == 0 || execution.stats.vm_fallbacks != 0 {
                return Err("proof benchmark did not execute optimizing native code".into());
            }
            if sample >= 3 {
                elapsed.push(duration);
                evidence = Some((
                    execution.stats.optimization_certificate_records,
                    execution.stats.optimization_certificate_bytes_estimate,
                    execution.stats.optimizing_native_entries,
                    execution.stats.vm_fallbacks,
                ));
            }
        }
        elapsed.sort_unstable();
        let percentile = |value: usize| elapsed[(elapsed.len() - 1) * value / 100];
        let evidence = evidence.ok_or("proof benchmark has no samples")?;
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            workers,
            samples,
            percentile(50),
            percentile(95),
            percentile(99),
            evidence.0,
            evidence.1,
            evidence.2,
            evidence.3,
        );
    }
    Ok(())
}
