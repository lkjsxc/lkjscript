//! Independent source oracles plus a bounded native-stack fault control.
use super::*;
use crate::platform::execution::normalized::prepare::{
    NormalizedFunctionBody, NormalizedInstruction,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn closure_fixture()
-> std::result::Result<(crate::platform::kernel::KernelSnapshot, NormalizedProgram), String> {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/native-closures.lkjc"
    ))?;
    let program = prepare_snapshot(&source);
    Ok((source, program))
}

fn bounded_native(bytes: &[u8], arguments: &[i64]) -> std::result::Result<Output, String> {
    let directory = tempfile::tempdir().map_err(|e| e.to_string())?;
    let binary = directory.path().join("callable");
    crate::platform::owned_output::publish_executable_create_new(
        &binary,
        bytes,
        None,
        "native closure stack control",
    )
    .map_err(|e| format!("publish: {e:?}"))?;
    crate::release_container::inspect_static_elf_bytes(bytes).map_err(|e| format!("ELF: {e:?}"))?;
    // The explicit stack bound distinguishes proper frame reuse from an
    // implementation which merely gives a correct answer on small examples.
    let mut child = Command::new("/usr/bin/prlimit")
        .env_clear()
        .args(["--stack=262144", "--core=0", "--"])
        .arg(binary)
        .args(arguments.iter().map(ToString::to_string))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().map_err(|e| e.to_string()),
            Ok(None) if started.elapsed() < Duration::from_secs(10) => {
                std::thread::sleep(Duration::from_millis(2));
            }
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("native closure test deadline/wait: {result:?}"));
            }
        }
    }
}

fn require_output(output: &Output, expected: i64) -> std::result::Result<(), String> {
    if !output.status.success()
        || !output.stderr.is_empty()
        || output.stdout != expected.to_string().as_bytes()
    {
        return Err(format!("expected I64 {expected}, got {output:?}"));
    }
    Ok(())
}

#[test]
fn native_elf_callable_source_controls_agree_with_vm_and_canonical_reference()
-> std::result::Result<(), String> {
    let (source, program) = closure_fixture()?;
    let policy = NormalizedRunPolicy {
        maximum_call_depth: 8,
        ..NormalizedRunPolicy::default()
    };
    for (name, arguments, expected) in [
        ("indirect", vec![0], 100),
        ("indirect", vec![101], 201),
        ("direct", vec![100], 200),
        ("borrowed", vec![101], 0),
        ("empty-unbound", vec![], 42),
        ("empty-bound", vec![], 42),
        ("empty-factory", vec![], 42),
        ("full", vec![], 42),
        ("terminal-external", vec![], 42),
        ("held-invoke", vec![], 42),
        ("held-call", vec![], 42),
    ] {
        let declaration = declaration_named(&source, name);
        let index = program
            .function(declaration)
            .ok_or("callable fixture function absent")?
            .0 as usize;
        let values = arguments
            .iter()
            .copied()
            .map(NormalizedValue::I64)
            .collect::<Vec<_>>();
        let control = ExecutionControl::uncancelled();
        let vm = NormalizedVm::for_test(&program, policy)
            .invoke(declaration, values.clone(), None, &control)
            .map_err(|e| format!("{name}: VM: {e:?}"))?
            .0;
        let reference = NormalizedReferenceInterpreter::new(&source, &program, policy)
            .invoke(declaration, values, None, &control)
            .map_err(|e| format!("{name}: reference: {e:?}"))?
            .0;
        if !matches!(&vm, NormalizedValue::I64(value) if *value == expected)
            || !matches!(&reference, NormalizedValue::I64(value) if *value == expected)
        {
            return Err(format!(
                "{name}: independent expected {expected}, VM {vm:?}, reference {reference:?}"
            ));
        }
        let image = elf_x86_64::compile_function(&program, index)
            .map_err(|e| format!("{name}: compile: {e:?}"))?
            .0;
        require_output(&bounded_native(&image, &arguments)?, expected)?;
    }
    Ok(())
}

#[test]
fn native_elf_live_owner_tail_fallback_reaches_cleanup_continuation()
-> std::result::Result<(), String> {
    let (source, program) = closure_fixture()?;
    let empty = program
        .function(declaration_named(&source, "empty"))
        .ok_or("buffer constructor absent")?;
    let plus = program
        .function(declaration_named(&source, "plus"))
        .ok_or("call target absent")?;
    for (name, indirect, sentinel) in [("held-call", false, 4301), ("held-invoke", true, 4302)] {
        let index = program
            .function(declaration_named(&source, name))
            .ok_or("held fixture absent")?
            .0 as usize;
        let mut observed = program.clone();
        let NormalizedFunctionBody::Code(code) =
            &mut Arc::make_mut(&mut observed.functions)[index].body
        else {
            return Err(format!("{name}: expected graph code"));
        };
        let instructions = code.instructions.as_ref();
        if code.parameter_count != 0
            || code.local_count != 1
            || !matches!(instructions.first(), Some(NormalizedInstruction::Call {
                function, arguments: 0, ..
            }) if *function == empty)
            || !matches!(
                instructions.get(1),
                Some(NormalizedInstruction::StoreLocal(0))
            )
        {
            return Err(format!(
                "{name}: fixture lost its real initialized ByteBuffer local"
            ));
        }
        let pc = instructions
            .iter()
            .position(|instruction| match instruction {
                NormalizedInstruction::TailInvoke { arguments: 2 } => indirect,
                NormalizedInstruction::TailCall {
                    function,
                    arguments: 2,
                    ..
                } => !indirect && *function == plus,
                _ => false,
            })
            .ok_or_else(|| format!("{name}: exact derived tail hint absent"))?;
        if !matches!(
            &instructions[pc + 1..],
            [
                NormalizedInstruction::Unit,
                NormalizedInstruction::StoreLocal(0),
                NormalizedInstruction::Return,
            ]
        ) {
            return Err(format!(
                "{name}: exact lexical cleanup/Return suffix absent"
            ));
        }

        // Deliberate test-only normalized mutation: keep the actual live owner,
        // tail hint and exact cleanup, then observe reaching the continuation.
        // This is a native dispatch-wiring control, not raw-source-stage evidence.
        let mut instructions = instructions.to_vec();
        let returned = instructions.len() - 1;
        instructions.insert(returned, NormalizedInstruction::I64(sentinel));
        instructions.insert(returned, NormalizedInstruction::Drop);
        code.instructions = instructions.into();
        let image = elf_x86_64::compile_function(&observed, index)
            .map_err(|e| format!("{name}: live-owner continuation: {e:?}"))?
            .0;
        require_output(&bounded_native(&image, &[])?, sentinel)?;

        // Sensitivity pair: clear that one owner immediately before the hint.
        // Actual tail transfer must now skip the sentinel and return the callee's
        // original 42. Always-CALL and always-tail implementations both fail.
        let mut released = observed.clone();
        let NormalizedFunctionBody::Code(code) =
            &mut Arc::make_mut(&mut released.functions)[index].body
        else {
            return Err(format!("{name}: sensitivity body absent"));
        };
        let mut instructions = code.instructions.to_vec();
        instructions.insert(pc, NormalizedInstruction::StoreLocal(0));
        instructions.insert(pc, NormalizedInstruction::Unit);
        code.instructions = instructions.into();
        let bypass = elf_x86_64::compile_function(&released, index)
            .map_err(|e| format!("{name}: released-owner sensitivity: {e:?}"))?
            .0;
        require_output(&bounded_native(&bypass, &[])?, 42)?;
    }
    Ok(())
}

#[test]
fn native_elf_mutual_tail_calls_and_invokes_reuse_stack_and_detect_forced_call_growth()
-> std::result::Result<(), String> {
    let (source, program) = closure_fixture()?;
    let mut faulty = program.clone();
    let mut direct = 0usize;
    let mut indirect = 0usize;
    for function in Arc::make_mut(&mut faulty.functions) {
        if let NormalizedFunctionBody::Code(code) = &mut function.body {
            for instruction in Arc::make_mut(&mut code.instructions) {
                *instruction = match instruction {
                    NormalizedInstruction::TailCall {
                        read_scope_arguments,
                        requirement_arguments,
                        effect_arguments,
                        function,
                        type_arguments,
                        arguments,
                    } => {
                        direct += 1;
                        NormalizedInstruction::Call {
                            read_scope_arguments: Arc::clone(read_scope_arguments),
                            requirement_arguments: Arc::clone(requirement_arguments),
                            effect_arguments: Arc::clone(effect_arguments),
                            function: *function,
                            type_arguments: type_arguments.clone(),
                            arguments: *arguments,
                        }
                    }
                    NormalizedInstruction::TailInvoke { arguments } => {
                        indirect += 1;
                        NormalizedInstruction::Invoke {
                            arguments: *arguments,
                        }
                    }
                    _ => continue,
                };
            }
        }
    }
    if direct == 0 || indirect == 0 {
        return Err("tail fixture did not exercise both dispatch families".to_owned());
    }
    for (name, large_expected, small_expected) in [
        ("direct", 100_100, 110),
        ("indirect", 100_100, 110),
        ("borrowed", 0, 0),
    ] {
        let declaration = declaration_named(&source, name);
        let index = program
            .function(declaration)
            .ok_or("tail fixture entry absent")?
            .0 as usize;
        let original = elf_x86_64::compile_function(&program, index)
            .map_err(|e| format!("original: {e:?}"))?
            .0;
        let mutant = elf_x86_64::compile_function(&faulty, index)
            .map_err(|e| format!("forced calls: {e:?}"))?
            .0;
        require_output(&bounded_native(&original, &[100_000])?, large_expected)?;
        // The safe private-IR fault retains exactly the same ordinary semantics.
        require_output(&bounded_native(&mutant, &[10])?, small_expected)?;
        let overflow = bounded_native(&mutant, &[100_000])?;
        use std::os::unix::process::ExitStatusExt;
        if overflow.status.signal() != Some(11) {
            return Err(format!(
                "{name}: explicit stack limit did not detect forced CALL growth: {overflow:?}"
            ));
        }
    }
    Ok(())
}
