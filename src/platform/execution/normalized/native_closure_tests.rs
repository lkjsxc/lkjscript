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
    let policy = NormalizedRunPolicy::default();
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

#[test]
fn native_elf_boxed_fallback_jump_zero_skips_parameter_reboxing() -> std::result::Result<(), String>
{
    use crate::platform::execution::normalized::prepare::NormalizedInstruction as I;
    use crate::platform::kernel::{FunctionEffect, ParameterUse, ResultRelation, TypeForm};

    const SOURCE: &str = r#"declarations.begin
(units (module create native-zero-jump
  (function create round-trip (visibility private) (effect pure)
    (parameter create value (type I64)) (parameter create again (type Bool)) (returns I64)
    (body (if (local again)
      (field (record structural (field value (local value))) (name value))
      (local value))))
  (function create jump-once (visibility public) (effect pure)
    (parameter create value (type I64)) (returns I64)
    (body (call round-trip (local value) (bool true))))
  (function create jump-never (visibility public) (effect pure)
    (parameter create value (type I64)) (returns I64)
    (body (call round-trip (local value) (bool false))))))
declarations.end"#;

    let source = byte_buffer_tests::author_only(SOURCE)?;
    let program = prepare_snapshot(&source);
    let helper = program
        .function(declaration_named(&source, "round-trip"))
        .ok_or("Jump(0) helper absent")?;
    let helper_index = helper.0 as usize;
    let function = &program.functions[helper_index];
    if !function.graph_function
        || function.effect != FunctionEffect::Pure
        || function.parameter_count != 2
        || function.parameters.len() != 2
        || !matches!(function.result.relation, ResultRelation::Value)
        || function.parameters.iter().any(|parameter| {
            parameter.use_mode != ParameterUse::Unrestricted
                || parameter.resource_requirement.is_some()
        })
        || !matches!(
            program
                .types
                .get(&function.parameters[0].ty)
                .map(|ty| &ty.form),
            Some(TypeForm::I64)
        )
        || !matches!(
            program
                .types
                .get(&function.parameters[1].ty)
                .map(|ty| &ty.form),
            Some(TypeForm::Bool)
        )
        || !matches!(
            program.types.get(&function.result.ty).map(|ty| &ty.form),
            Some(TypeForm::I64)
        )
    {
        return Err("Jump(0) helper lost its ordinary (I64, Bool) -> I64 signature".to_owned());
    }
    let NormalizedFunctionBody::Code(original) = &function.body else {
        return Err("Jump(0) helper lacks admitted graph code".to_owned());
    };
    if original.parameter_count != 2 || original.local_count != 2 {
        return Err("Jump(0) helper lost its exact two-parameter frame".to_owned());
    }
    let mut records = original
        .instructions
        .iter()
        .filter(|i| matches!(i, I::Record { .. }));
    let record = records
        .next()
        .ok_or("Jump(0) source record absent")?
        .clone();
    let mut fields = original
        .instructions
        .iter()
        .filter(|i| matches!(i, I::Field(_)));
    let field = fields.next().ok_or("Jump(0) source field absent")?.clone();
    if records.next().is_some() || fields.next().is_some() {
        return Err("Jump(0) source round trip is not unique".to_owned());
    }
    let (
        I::Record {
            layout: None,
            type_arguments,
            fields,
        },
        I::Field(selector),
    ) = (&record, &field)
    else {
        return Err("Jump(0) source round trip is not structural".to_owned());
    };
    if !type_arguments.is_empty() || fields.as_ref() != std::slice::from_ref(selector) {
        return Err("Jump(0) source record and selector disagree".to_owned());
    }

    // Behavior-preserving test-only lowering of the admitted identity function.
    // The true path performs its original record/Field round trip once, clears
    // the flag, then executes Jump(0). Entry local 0 is raw I64; the backedge
    // writes the boxed Field result, so current representation flow declines
    // the whole body. The backedge must skip raw-parameter boxing at entry.
    // This proves derived-code wiring, not a new raw-source compilation route.
    let mut rewritten = program.clone();
    let NormalizedFunctionBody::Code(code) =
        &mut Arc::make_mut(&mut rewritten.functions)[helper_index].body
    else {
        return Err("Jump(0) copied helper lacks graph code".to_owned());
    };
    let load = |local| I::LoadLocal {
        local,
        use_mode: ParameterUse::Unrestricted,
    };
    code.instructions = vec![
        load(1),
        I::JumpIfFalse(9),
        load(0),
        record,
        field,
        I::StoreLocal(0),
        I::Bool(false),
        I::StoreLocal(1),
        I::Jump(0),
        load(0),
        I::Return,
    ]
    .into();
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let values = [i64::MIN, -2, -1, 0, 1, 255, 256, i64::MAX];
    for (name, again) in [("jump-once", true), ("jump-never", false)] {
        let declaration = declaration_named(&source, name);
        let index = program
            .function(declaration)
            .ok_or("Jump(0) entry absent")?
            .0 as usize;
        let NormalizedFunctionBody::Code(wrapper) = &program.functions[index].body else {
            return Err(format!("{name}: source wrapper lacks graph code"));
        };
        if !matches!(wrapper.instructions.as_ref(), [
            first,
            I::Bool(flag),
            I::TailCall { function, arguments: 2, .. },
            I::Return,
        ] if matches!(first, I::LoadLocal { local: 0, use_mode: ParameterUse::Unrestricted } | I::MoveLocal(0)) && *flag == again && *function == helper)
        {
            return Err(format!("{name}: exact admitted terminal call is absent"));
        }
        for expected in values {
            let arguments = vec![NormalizedValue::I64(expected)];
            let vm = NormalizedVm::for_test(&program, policy)
                .invoke(declaration, arguments.clone(), None, &control)
                .map_err(|e| format!("{name}/{expected}: original VM: {e:?}"))?
                .0;
            let reference = NormalizedReferenceInterpreter::new(&source, &program, policy)
                .invoke(declaration, arguments, None, &control)
                .map_err(|e| format!("{name}/{expected}: canonical reference: {e:?}"))?
                .0;
            if !matches!(&vm, NormalizedValue::I64(value) if *value == expected)
                || !matches!(&reference, NormalizedValue::I64(value) if *value == expected)
            {
                return Err(format!(
                    "{name}: independent expected I64 {expected}, original VM {vm:?}, reference {reference:?}"
                ));
            }
        }
        for ordinary in [false, true] {
            let route = if ordinary { "Call" } else { "TailCall" };
            let mut candidate = rewritten.clone();
            if ordinary {
                let NormalizedFunctionBody::Code(code) =
                    &mut Arc::make_mut(&mut candidate.functions)[index].body
                else {
                    return Err(format!("{name}: copied wrapper lacks graph code"));
                };
                let I::TailCall {
                    read_scope_arguments,
                    requirement_arguments,
                    effect_arguments,
                    function,
                    type_arguments,
                    arguments,
                } = code.instructions[2].clone()
                else {
                    return Err(format!("{name}: copied tail-call identity changed"));
                };
                Arc::make_mut(&mut code.instructions)[2] = I::Call {
                    read_scope_arguments,
                    requirement_arguments,
                    effect_arguments,
                    function,
                    type_arguments,
                    arguments,
                };
            }
            let image = elf_x86_64::compile_function(&candidate, index)
                .map_err(|e| format!("{name}/{route}: Jump(0) native compilation: {e:?}"))?
                .0;
            for expected in values {
                let vm = NormalizedVm::for_test(&candidate, policy)
                    .invoke(
                        declaration,
                        vec![NormalizedValue::I64(expected)],
                        None,
                        &control,
                    )
                    .map_err(|e| format!("{name}/{route}/{expected}: rewritten VM: {e:?}"))?
                    .0;
                if !matches!(&vm, NormalizedValue::I64(value) if *value == expected) {
                    return Err(format!(
                        "{name}/{route}: rewritten VM expected I64 {expected}, got {vm:?}"
                    ));
                }
                require_output(&bounded_native(&image, &[expected])?, expected).map_err(|e| {
                    format!("{name}/{route}: Jump(0) parameter representation: {e}")
                })?;
            }
        }
    }
    Ok(())
}
