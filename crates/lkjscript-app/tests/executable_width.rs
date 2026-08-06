#![allow(clippy::expect_used, clippy::panic)]

use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, Op, ResourceLimitKind};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue};
use lkjscript_jit::{execute_forced, FailureCode, JitConfig, JitSession};
use lkjscript_vm::{run_chunk, run_chunk_auto, ExecutionInputs};

const WIDE_COUNT: usize = 300;
const STRESS_COUNT: usize = 1_024;
const WIDE_CONSTANT_COUNT: usize = 65_537;

fn wide_scalar_source(count: usize) -> String {
    let mut lines = vec![
        "def".to_string() + "/",
        "name/".into(),
        "select-high".into(),
        "/name".into(),
        "fn/".into(),
        "sig/".into(),
        "inputs/".into(),
    ];
    lines.extend((0..count).map(|_| "i64".to_string()));
    lines.extend([
        "/inputs".into(),
        "output/".into(),
        "i64".into(),
        "/output".into(),
        "/sig".into(),
        "params/".into(),
    ]);
    for index in 0..count {
        lines.push(format!("p{index}"));
        lines.push("i64".into());
    }
    lines.extend([
        "/params".into(),
        format!("p{}", count - 1),
        "/fn".into(),
        "/def".into(),
        "main/".into(),
        "sig/".into(),
        "inputs/".into(),
        "/inputs".into(),
        "output/".into(),
        "i64".into(),
        "/output".into(),
        "/sig".into(),
        "let/".into(),
    ]);
    for index in 0..count {
        lines.extend([
            "bind/".into(),
            format!("x{index}"),
            index.to_string(),
            "/bind".into(),
        ]);
    }
    lines.push("select-high/".into());
    lines.extend((0..count).map(|index| format!("x{index}")));
    lines.extend([
        "/select-high".into(),
        "/let".into(),
        "/main".into(),
        String::new(),
    ]);
    lines.join("\n")
}

fn wide_owned_parameter_source(count: usize) -> String {
    let mut lines = vec![
        "def/".into(),
        "name/".into(),
        "owned-high".into(),
        "/name".into(),
        "fn/".into(),
        "sig/".into(),
        "inputs/".into(),
    ];
    lines.extend((0..count - 1).map(|_| "i64".to_string()));
    lines.push("byte-vector".into());
    lines.extend([
        "/inputs".into(),
        "output/".into(),
        "i64".into(),
        "/output".into(),
        "/sig".into(),
        "params/".into(),
    ]);
    for index in 0..count - 1 {
        lines.push(format!("p{index}"));
        lines.push("i64".into());
    }
    lines.extend([
        format!("p{}", count - 1),
        "byte-vector".into(),
        "/params".into(),
        "byte-slice-length/".into(),
        "borrow/".into(),
        format!("p{}", count - 1),
        "/borrow".into(),
        "/byte-slice-length".into(),
        "/fn".into(),
        "/def".into(),
        "main/".into(),
        "sig/".into(),
        "inputs/".into(),
        "/inputs".into(),
        "output/".into(),
        "i64".into(),
        "/output".into(),
        "/sig".into(),
        "let/".into(),
        "bind/".into(),
        "bytes".into(),
        "new-byte-vector/".into(),
        "7".into(),
        "/new-byte-vector".into(),
        "/bind".into(),
        "owned-high/".into(),
    ]);
    lines.extend((0..count - 1).map(|index| index.to_string()));
    lines.extend([
        "move/".into(),
        "bytes".into(),
        "/move".into(),
        "/owned-high".into(),
        "/let".into(),
        "/main".into(),
        String::new(),
    ]);
    lines.join("\n")
}

fn many_owned_arguments_source(count: usize) -> String {
    let mut lines = vec![
        "def/".into(),
        "name/".into(),
        "drop-many-owned".into(),
        "/name".into(),
        "fn/".into(),
        "sig/".into(),
        "inputs/".into(),
    ];
    lines.extend((0..count).map(|_| "byte-vector".to_string()));
    lines.extend([
        "/inputs".into(),
        "output/".into(),
        "i64".into(),
        "/output".into(),
        "/sig".into(),
        "params/".into(),
    ]);
    for index in 0..count {
        lines.push(format!("p{index}"));
        lines.push("byte-vector".into());
    }
    lines.extend([
        "/params".into(),
        "7".into(),
        "/fn".into(),
        "/def".into(),
        "main/".into(),
        "sig/".into(),
        "inputs/".into(),
        "/inputs".into(),
        "output/".into(),
        "i64".into(),
        "/output".into(),
        "/sig".into(),
        "let/".into(),
    ]);
    for index in (0..count).rev() {
        lines.extend([
            "bind/".into(),
            format!("owned{index}"),
            "new-byte-vector/".into(),
            "1".into(),
            "/new-byte-vector".into(),
            "/bind".into(),
        ]);
    }
    lines.push("drop-many-owned/".into());
    for index in 0..count {
        lines.extend(["move/".into(), format!("owned{index}"), "/move".into()]);
    }
    lines.extend([
        "/drop-many-owned".into(),
        "/let".into(),
        "/main".into(),
        String::new(),
    ]);
    lines.join("\n")
}

fn wide_distinct_constants_source(count: usize) -> String {
    let mut source = String::from(
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nvar/\nname/\nx\n/name\ntype/\ni64\n/type\n0\ndo/\n",
    );
    for value in 1..count {
        source.push_str("set/\nx\n");
        source.push_str(&value.to_string());
        source.push_str("\n/set\n");
    }
    source.push_str("x\n/do\n/var\n/main\n");
    source
}

fn wide_branch_source(body_updates: usize) -> String {
    let mut source = String::from(
        "def/\nname/\nwide-branch\n/name\nfn/\nsig/\ninputs/\nbool\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nflag\nbool\n/params\nvar/\nname/\nx\n/name\ntype/\ni64\n/type\n0\nif/\nflag\ndo/\n",
    );
    for _ in 0..body_updates {
        source.push_str("set/\nx\nadd/\nx\n1\n/add\n/set\n");
    }
    source.push_str(
        "x\n/do\n22\n/if\n/var\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nadd/\nwide-branch/\ntrue\n/wide-branch\nwide-branch/\nfalse\n/wide-branch\n/add\n/main\n",
    );
    source
}

fn logical_cleanup_actions(proto: &lkjscript_core::FunctionProto) -> usize {
    let mut lengths = Vec::with_capacity(proto.failure_cleanups.len());
    for node in &proto.failure_cleanups {
        let tail = node
            .next
            .map(|next| lengths[next.index().expect("validated cleanup link")])
            .unwrap_or(0_usize);
        lengths.push(tail.checked_add(1).expect("test cleanup length fits usize"));
    }
    proto
        .failure_cleanup_ranges
        .iter()
        .flat_map(|range| {
            range
                .plan
                .into_iter()
                .flat_map(lkjscript_core::FailureCleanupRoots::ids)
                .chain(range.unentered_plan)
        })
        .map(|root| lengths[root.index().expect("validated cleanup root")])
        .sum()
}

fn compile_wide() -> lkjscript_compiler::ExecutableProgram {
    compile_source(
        &wide_scalar_source(WIDE_COUNT),
        "generated-wide-executable.lkjscript",
    )
    .expect("compile generated wide executable through HIR, memory plan, SSA, and bytecode")
}

fn returned_i64(outcome: ExecutionOutcome) -> i64 {
    match outcome {
        ExecutionOutcome::Returned(value) => value.as_i64().expect("returned I64"),
        other => panic!("wide executable did not return: {other:?}"),
    }
}

#[test]
#[ignore = "release-only generated 65,537-distinct-constant executable-width stress"]
fn generated_source_crosses_constant_memory_plan_and_bytecode_widths() {
    let program = compile_source(
        &wide_distinct_constants_source(WIDE_CONSTANT_COUNT),
        "generated-wide-constants.lkjscript",
    )
    .expect("compile wide constants through HIR memory plan, SSA, and bytecode validation");
    assert!(program.prepared_identity().is_bound());
    assert_eq!(program.memory_plan().constants.len(), WIDE_CONSTANT_COUNT);
    assert!(program.memory_plan().entries.len() > usize::from(u16::MAX));
    assert!(program.memory_plan().work.verifier_steps > u64::from(u16::MAX));
    assert!(!program.bytecode().constants().is_empty());
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    assert_eq!(
        returned_i64(outcome),
        i64::try_from(WIDE_CONSTANT_COUNT - 1).expect("test width fits i64")
    );
}

#[test]
fn three_hundred_parameters_arguments_and_live_lexical_locals_execute_in_vm() {
    let program = compile_wide();
    let function = program
        .bytecode()
        .protos()
        .iter()
        .find(|proto| proto.arity == WIDE_COUNT)
        .expect("wide function prototype");
    assert_eq!(function.arity, WIDE_COUNT);
    assert!(function.locals >= WIDE_COUNT);
    assert!(program.bytecode().main().locals > usize::from(u8::MAX));

    let call = program
        .bytecode()
        .main_instructions()
        .iter()
        .find(|instruction| instruction.op() == Op::Call)
        .expect("wide call instruction");
    assert_eq!(call.operand().index(), Some(WIDE_COUNT));
    assert!(program
        .bytecode()
        .main_instructions()
        .iter()
        .any(|instruction| {
            matches!(instruction.op(), Op::LoadLocal | Op::TakeUniqueLocal)
                && instruction
                    .operand()
                    .index()
                    .is_some_and(|slot| slot > usize::from(u8::MAX))
        }));

    let expected = i64::try_from(WIDE_COUNT - 1).expect("test width fits i64");
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(expected))
    );
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    assert_eq!(returned_i64(outcome), expected);
}

#[test]
fn one_thousand_parameters_arguments_and_live_locals_execute_in_vm() {
    let source = wide_scalar_source(STRESS_COUNT);
    let program = compile_source(&source, "wide-executable-stress.lkjscript")
        .expect("compile stress-width scalar source");
    let function = &program.bytecode().protos()[0];
    assert_eq!(function.arity, STRESS_COUNT);
    assert!(function.locals > usize::from(u8::MAX));
    assert!(program
        .bytecode()
        .proto_instructions(0)
        .expect("decoded stress function")
        .iter()
        .any(|instruction| {
            instruction.op() == Op::LoadLocal
                && instruction.operand().index() == Some(STRESS_COUNT - 1)
        }));
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    assert_eq!(
        returned_i64(outcome),
        i64::try_from(STRESS_COUNT - 1).expect("test width fits i64")
    );
}

#[test]
fn owned_parameter_above_byte_width_executes_and_cleans_up() {
    let program = compile_source(
        &wide_owned_parameter_source(WIDE_COUNT),
        "generated-wide-owned-parameter.lkjscript",
    )
    .expect("compile wide owned parameter through the production pipeline");
    let function = program
        .bytecode()
        .protos()
        .iter()
        .find(|proto| proto.arity == WIDE_COUNT)
        .expect("wide owned function prototype");
    assert_eq!(function.parameter_unique_places[WIDE_COUNT - 1], Some(0));
    assert_eq!(function.unique_places, 1);
    assert!(function.failure_cleanups.iter().any(|node| {
        match node.action {
            lkjscript_core::FailureCleanupAction::EndBorrow { local, .. }
            | lkjscript_core::FailureCleanupAction::DropUnique { local, .. } => {
                local > usize::from(u8::MAX)
            }
            _ => false,
        }
    }));
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    assert_eq!(returned_i64(outcome), 7);
}

fn assert_many_owned_arguments(count: usize) {
    let source = many_owned_arguments_source(count);
    let program = compile_source(&source, "generated-many-owned-arguments.lkjscript")
        .expect("compile owned arguments through HIR, memory plan, SSA, and publication");
    let function = program
        .bytecode()
        .protos()
        .iter()
        .find(|proto| proto.arity == count)
        .expect("many-owned function prototype");
    assert_eq!(function.unique_places, count);
    assert!(program.bytecode().main().code.len() > usize::from(u16::MAX));
    assert!(function.failure_cleanups.iter().any(|node| {
        matches!(
            node.action,
            lkjscript_core::FailureCleanupAction::DropUnique {
                local,
                place: Some(place),
                ..
            } if local > usize::from(u8::MAX) && place > usize::from(u8::MAX)
        )
    }));

    let geometry: Vec<_> = std::iter::once(program.bytecode().main())
        .chain(program.bytecode().protos())
        .map(|proto| {
            (
                proto.name.as_str(),
                logical_cleanup_actions(proto),
                proto.failure_cleanups.len(),
                proto
                    .failure_cleanups
                    .iter()
                    .filter(|node| match node.action {
                        lkjscript_core::FailureCleanupAction::DropUnique { place, .. }
                        | lkjscript_core::FailureCleanupAction::DropResource { place, .. }
                        | lkjscript_core::FailureCleanupAction::DropStructural { place, .. } => {
                            place.is_some()
                        }
                        lkjscript_core::FailureCleanupAction::EndBorrow { .. }
                        | lkjscript_core::FailureCleanupAction::EndStructuralBorrow { .. }
                        | lkjscript_core::FailureCleanupAction::AbortStructuralDestination {
                            ..
                        } => false,
                    })
                    .count(),
            )
        })
        .collect();
    let (logical, physical) = geometry.iter().fold(
        (0_usize, 0_usize),
        |(logical, physical), (_, proto_logical, proto_physical, _)| {
            (
                logical
                    .checked_add(*proto_logical)
                    .expect("test logical cleanup geometry fits usize"),
                physical
                    .checked_add(*proto_physical)
                    .expect("test physical cleanup geometry fits usize"),
            )
        },
    );
    assert!(logical > 65_535, "logical cleanup actions: {logical}");
    assert!(
        physical <= count.saturating_mul(12),
        "physical cleanup nodes {physical} are not near-linear for {count} owners ({logical} logical actions): {geometry:?}",
    );
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(7))
    );

    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    assert_eq!(returned_i64(outcome), 7);

    let session = JitSession::new_auto(
        program.ssa(),
        program.bytecode_links(),
        JitConfig {
            auto_threshold: 1,
            ..JitConfig::default()
        },
    );
    let (auto_outcome, stats) = run_chunk_auto(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
        session,
    );
    assert_eq!(returned_i64(auto_outcome), 7);
    let tier = stats
        .functions
        .iter()
        .find(|item| item.name().ends_with("drop-many-owned"))
        .expect("many-owned function tier record");
    assert!(!tier.auto_entry_eligible());
    assert_eq!(tier.native_entries(), 0);
    assert!(stats.vm_fallbacks > 0);

    let instructions = program.bytecode().main_instructions();
    let call_index = instructions
        .iter()
        .position(|instruction| instruction.op() == Op::Call)
        .expect("many-owned call instruction");
    let ranges = &program.bytecode().main().failure_cleanup_ranges;
    let boundaries_before_call = instructions[..call_index]
        .iter()
        .filter(|instruction| {
            let offset = u64::try_from(instruction.offset()).expect("test offset fits u64");
            ranges
                .iter()
                .find(|range| range.start <= offset && offset < range.end)
                .is_none_or(|range| range.start == offset)
        })
        .count();
    let fuel = u64::try_from(boundaries_before_call).expect("test boundary count fits u64");
    let limited = ExecutionConfig {
        instruction_fuel: fuel,
        ..ExecutionConfig::default()
    };
    assert_eq!(
        run_chunk(program.bytecode(), &ExecutionInputs::default(), &limited,),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::InstructionFuel),
        "call-unentered cleanup must run before ordinary frame cleanup without leaks or failures",
    );
}

#[test]
fn one_thousand_twenty_four_owned_parameters_and_arguments_publish_and_execute_with_shared_cleanup()
{
    assert_many_owned_arguments(STRESS_COUNT);
}

#[test]
fn generated_wide_jump_executes_both_source_branch_paths() {
    const BODY_UPDATES: usize = 4_000;
    let program = compile_source(
        &wide_branch_source(BODY_UPDATES),
        "generated-wide-jump.lkjscript",
    )
    .expect("compile and prepare a source branch beyond the former jump boundary");
    let function = program
        .bytecode()
        .protos()
        .iter()
        .find(|proto| proto.name.ends_with("wide-branch"))
        .expect("wide branch function prototype");
    assert!(
        function.code.len() > usize::from(u16::MAX),
        "wide branch emitted {} code bytes",
        function.code.len()
    );
    let instructions = program
        .bytecode()
        .proto_instructions(0)
        .expect("wide branch instructions");
    assert!(instructions.iter().any(|instruction| {
        matches!(instruction.op(), Op::Jump | Op::JumpIfFalse)
            && instruction
                .operand()
                .index()
                .is_some_and(|target| target > usize::from(u16::MAX))
    }));
    assert_eq!(
        returned_i64(run_chunk(
            program.bytecode(),
            &ExecutionInputs::default(),
            &ExecutionConfig::default(),
        )),
        i64::try_from(BODY_UPDATES).expect("test update count fits i64") + 22,
    );
}

#[test]
fn automatic_engine_keeps_high_signature_on_the_generic_vm_path() {
    let program = compile_wide();
    let config = JitConfig {
        auto_threshold: 1,
        ..JitConfig::default()
    };
    let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), config);
    let (outcome, stats) = run_chunk_auto(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
        session,
    );
    assert_eq!(
        returned_i64(outcome),
        i64::try_from(WIDE_COUNT - 1).expect("test width fits i64")
    );
    let function = stats
        .functions
        .iter()
        .find(|function| function.name() == "select-high")
        .expect("wide function tier record");
    assert!(!function.auto_entry_eligible());
    assert_eq!(function.native_entries(), 0);

    let error = execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect_err("forced native mode reports the unsupported high signature");
    assert_eq!(error.code(), FailureCode::UnsupportedSignature);
}
