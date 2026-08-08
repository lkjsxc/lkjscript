#![allow(clippy::expect_used, clippy::panic)]

use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_jit::{attempt_baseline, BaselineAttempt, JitConfig};
use lkjscript_vm::{run_chunk, ExecutionInputs};

const SMALL_STACK_BYTES: usize = 128 * 1024;

fn alternating_chain_source(depth: usize) -> String {
    format!(
        concat!(
            "product/\nname/\nlink\n/name\nfields/\n",
            "field/\nname/\nnext\n/name\ntype/\nchain/\n/chain\n/type\n/field\n",
            "/fields\n/product\n",
            "enum/\nname/\nchain\n/name\nvariants/\n",
            "variant/\nname/\nleaf\n/name\nfields/\n",
            "variant-field/\nname/\nvalue\n/name\ntype/\ni64\n/type\n/variant-field\n",
            "/fields\n/variant\n",
            "variant/\nname/\nbranch\n/name\nfields/\n",
            "variant-field/\nname/\nnext\n/name\ntype/\nproduct\nlink\n/type\n/variant-field\n",
            "/fields\n/variant\n/variants\n/enum\n",
            "def/\nname/\nbuild-chain\n/name\nfn/\nsig/\ninputs/\ni64\n/inputs\n",
            "output/\nchain/\n/chain\n/output\n/sig\nparams/\ndepth\ni64\n/params\n",
            "if/\nequal-value/\ndepth\n0\n/equal-value\n",
            "variant-value/\ntype/\nchain/\n/chain\n/type\nvariant/\nleaf\n/variant\nfields/\n",
            "variant-field/\nname/\nvalue\n/name\n41\n/variant-field\n",
            "/fields\n/variant-value\n",
            "variant-value/\ntype/\nchain/\n/chain\n/type\nvariant/\nbranch\n/variant\nfields/\n",
            "variant-field/\nname/\nnext\n/name\nproduct-value/\nlink\n",
            "field/\nnext\nbuild-chain/\nsubtract/\ndepth\n1\n/subtract\n/build-chain\n/field\n",
            "/product-value\n/variant-field\n/fields\n/variant-value\n",
            "/if\n/fn\n/def\n",
            "main/\nsig/\ninputs/\n/inputs\noutput/\nchain/\n/chain\n/output\n/sig\n",
            "build-chain/\n{}\n/build-chain\n/main\n",
        ),
        depth
    )
}

fn compile_and_run(depth: usize) -> Result<(), String> {
    let source = alternating_chain_source(depth);
    let program = compile_source(&source, "deep-owned-structural.lkjscript")
        .map_err(|error| error.to_string())?;

    let decline = match attempt_baseline(
        program.ssa(),
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    ) {
        BaselineAttempt::Declined(decline) => decline,
        BaselineAttempt::Executed(_) => {
            return Err("recursive structural construction entered baseline native".into())
        }
        BaselineAttempt::EnteredFailure(failure) => {
            return Err(format!(
                "recursive structural native attempt failed after entry: {}",
                failure.error
            ))
        }
    };
    if decline.reason.stage() != "lowering"
        || decline.reason.code() != "native-stack-boundary"
        || !decline.reason.detail().contains("recursive call graph")
    {
        return Err(format!("unexpected native decline: {}", decline.reason));
    }
    if decline
        .stats
        .as_ref()
        .is_some_and(|stats| !stats.code_objects.is_empty() || stats.native_entries != 0)
    {
        return Err("pre-entry recursive decline published or entered native code".into());
    }

    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    let ExecutionOutcome::Returned(value) = outcome else {
        return Err(format!(
            "VM did not return the deep structural value: {outcome:?}"
        ));
    };
    let depth = u64::try_from(depth).map_err(|_| "test depth exceeds u64")?;
    let metrics = value
        .structural_snapshot_metrics()
        .ok_or_else(|| "VM return lacks structural metrics".to_string())?;
    if metrics.nodes != depth * 2 + 2
        || metrics.fields != depth * 2 + 1
        || metrics.encode_work != depth * 4 + 3
    {
        return Err(format!("unexpected VM structural metrics: {metrics:?}"));
    }
    let debug = format!("{value:?}");
    if debug.len() > 128 || !debug.contains("owned-structural-enum") {
        return Err(format!("VM structural debug is not concise: {debug}"));
    }
    let cloned = value.clone();
    if cloned != value {
        return Err("VM returned structural clone differs".into());
    }
    drop(cloned);
    drop(value);
    drop(program);
    Ok(())
}

#[test]
fn vm_returns_and_cleans_a_deep_alternating_value_on_a_small_stack() {
    std::thread::Builder::new()
        .name("vm-owned-structural-small-stack".into())
        .stack_size(SMALL_STACK_BYTES)
        .spawn(|| compile_and_run(512))
        .expect("spawn VM structural worker")
        .join()
        .expect("VM structural worker panicked")
        .expect("VM structural product path succeeds");
}

#[test]
#[ignore = "opt-in 20,000-level VM structural return and cleanup stress geometry"]
fn vm_returns_and_cleans_twenty_thousand_alternating_levels() {
    std::thread::Builder::new()
        .name("vm-owned-structural-stress-small-stack".into())
        .stack_size(SMALL_STACK_BYTES)
        .spawn(|| compile_and_run(10_000))
        .expect("spawn VM structural stress worker")
        .join()
        .expect("VM structural stress worker panicked")
        .expect("VM structural stress product path succeeds");
}
