use std::sync::atomic::{AtomicU64, Ordering};

use crate::canonical::{compile, execution, f64_loop};
use lkjscript_core::{ExecutionConfig, ResourceProfile, ResourceProfileName};
use lkjscript_jit::{
    execute_forced, execute_optimizing, CacheContext, CacheStatus, JitConfig, JitSession,
};
use lkjscript_vm::run_chunk_auto;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Root(std::path::PathBuf);

impl Root {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "lkjscript-jit-cache-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).expect("cache test root");
        Self(path)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn config(root: &Root) -> JitConfig {
    JitConfig {
        cache: Some(CacheContext {
            package_root: root.0.clone(),
            module_path: "src/cache.lkjscript".into(),
            source_sha256: [1; 32],
            module_sha256: [2; 32],
            package_sha256: [3; 32],
            lock_sha256: [4; 32],
            profile: ResourceProfile::new(ResourceProfileName::Default).identity(),
        }),
        ..JitConfig::default()
    }
}

fn scalar_source(value: i64) -> String {
    format!("main/\nsig/\n->\nI64\n/sig\n+/\n{value}\n1\n/+\n/main\n")
}

#[test]
fn baseline_warm_hit_skips_lowering_but_reinstalls_wx_image() {
    let root = Root::new();
    let program = compile(&scalar_source(40), "cache-baseline.lkjscript");
    let cold = execute_forced(program.ssa(), &ExecutionConfig::default(), config(&root))
        .expect("cold baseline cache miss");
    assert_eq!(execution(cold.outcome), crate::canonical::Scalar::I64(41));
    assert_eq!(cold.stats.cache_hits, 0);
    assert_eq!(cold.stats.cache_misses, 1);
    assert_eq!(cold.stats.cache_publications, 1);
    assert_eq!(
        cold.stats.code_objects[0].compile_stats.cache_status(),
        CacheStatus::MissNotFound
    );

    let warm = execute_forced(program.ssa(), &ExecutionConfig::default(), config(&root))
        .expect("warm baseline cache hit");
    assert_eq!(warm.stats.cache_hits, 1);
    assert_eq!(
        warm.stats.code_objects[0].compile_stats.cache_status(),
        CacheStatus::Hit
    );
    assert_eq!(
        warm.stats.code_objects[0]
            .compile_stats
            .lowering_and_encoding(),
        std::time::Duration::ZERO
    );
    assert!(warm.stats.code_objects[0].wx_transition_verified);
    assert_eq!(warm.stats.vm_fallbacks, 0);
}

#[test]
fn optimizing_warm_hit_reruns_proof_checker() {
    let root = Root::new();
    let program = compile(&scalar_source(8), "cache-optimizing.lkjscript");
    execute_optimizing(program.ssa(), &ExecutionConfig::default(), config(&root))
        .expect("cold optimizing cache miss");
    let warm = execute_optimizing(program.ssa(), &ExecutionConfig::default(), config(&root))
        .expect("warm optimizing cache hit");
    assert_eq!(warm.stats.cache_hits, 1);
    assert_eq!(warm.stats.optimization_checker_passes, 1);
    assert_eq!(warm.stats.optimization_validation_passes, 17);
    assert_eq!(
        warm.stats.code_objects[0].compile_stats.cache_status(),
        CacheStatus::Hit
    );
    assert_eq!(warm.stats.vm_fallbacks, 0);
}

#[test]
fn freshly_verified_ssa_identity_prevents_stale_image_hits() {
    let root = Root::new();
    let first = compile(&scalar_source(1), "cache-ssa-first.lkjscript");
    execute_forced(first.ssa(), &ExecutionConfig::default(), config(&root))
        .expect("publish first SSA image");
    let changed = compile(&scalar_source(9), "cache-ssa-changed.lkjscript");
    let changed_execution =
        execute_forced(changed.ssa(), &ExecutionConfig::default(), config(&root))
            .expect("changed SSA compiles independently");
    assert_eq!(changed_execution.stats.cache_hits, 0);
    assert_eq!(changed_execution.stats.cache_misses, 1);
    assert_eq!(
        execution(changed_execution.outcome),
        crate::canonical::Scalar::I64(10)
    );
}

#[test]
fn automatic_tiering_uses_verified_warm_images_without_semantic_change() {
    let root = Root::new();
    let program = compile(&f64_loop(), "cache-auto.lkjscript");
    let mut cold_config = config(&root);
    cold_config.auto_threshold = 1;
    let cold = JitSession::new_auto(program.ssa(), program.bytecode_links(), cold_config);
    let (expected, cold_stats) = run_chunk_auto(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
        cold,
    );
    assert!(cold_stats.cache_publications > 0);

    let mut warm_config = config(&root);
    warm_config.auto_threshold = 1;
    let warm = JitSession::new_auto(program.ssa(), program.bytecode_links(), warm_config);
    let (actual, warm_stats) = run_chunk_auto(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
        warm,
    );
    assert_eq!(execution(actual), execution(expected));
    assert!(warm_stats.cache_hits > 0);
    assert_eq!(warm_stats.compile_failures, 0);
}

#[test]
fn corrupt_exact_entry_is_a_miss_and_never_forced_vm_fallback() {
    let root = Root::new();
    let program = compile(&scalar_source(2), "cache-corrupt.lkjscript");
    execute_forced(program.ssa(), &ExecutionConfig::default(), config(&root))
        .expect("publish baseline image");
    let objects = root.0.join("target/lkjscript/native-cache/objects");
    let object = std::fs::read_dir(objects)
        .expect("object directory")
        .next()
        .expect("cache object")
        .expect("object entry")
        .path();
    std::fs::write(object, b"corrupt").expect("corrupt object");
    let rebuilt = execute_forced(program.ssa(), &ExecutionConfig::default(), config(&root))
        .expect("corruption rebuilds native image");
    assert_eq!(rebuilt.stats.cache_corruptions, 1);
    assert_eq!(rebuilt.stats.cache_hits, 0);
    assert_eq!(rebuilt.stats.vm_fallbacks, 0);
    assert!(rebuilt.stats.native_entries > 0);
}
