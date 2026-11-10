//! Ordinary task control transfers, with canonical expectations independent of dispatch.
use super::*;

fn control_context(context: &str, request: &mut String, add: &str, equal: &str) -> String {
    match context {
        "direct" => String::new(),
        "task-to-pure" => {
            *request = request.replace("when-true=$acc", "when-true=$finish-call");
            "expression.call as=$finish-call function=$finish\nexpression.argument parent=$finish-call index=0 expression=$acc\nexpression.local as=$finished value=$finish-value\ncreate.function as=$finish module=$module name=finish visibility=private result=i64 effect=pure body=$finished\nadd.parameter as=$finish-value function=$finish name=value type=i64\n".into()
        }
        "let" => "expression.i64 as=$seed value=7\nexpression.let as=$wrapped body=$again\nexpression.binding parent=$wrapped index=0 as=$ignored name=ignored type=i64 value=$seed\n".into(),
        "sequence" => "expression.unit as=$ignored\nexpression.sequence as=$wrapped\nexpression.argument parent=$wrapped index=0 expression=$ignored\nexpression.argument parent=$wrapped index=1 expression=$again\n".into(),
        "match" => "create.variant as=$phase module=$module name=phase visibility=private\nadd.case as=$continue variant=$phase name=continue\nexpression.variant as=$phase-value case=$continue\nexpression.match as=$wrapped value=$phase-value\nexpression.match-arm parent=$wrapped index=0 case=$continue body=$again\n".into(),
        "non-tail" => format!("expression.i64 as=$suffix value=1\nexpression.call as=$wrapped function={add}\nexpression.argument parent=$wrapped index=0 expression=$again\nexpression.argument parent=$wrapped index=1 expression=$suffix\n"),
        "initializer" => "expression.local as=$after value=$pending\nexpression.let as=$wrapped body=$after\nexpression.binding parent=$wrapped index=0 as=$pending name=pending type=i64 value=$again\n".into(),
        "earlier-sequence" => "expression.i64 as=$after value=1\nexpression.sequence as=$wrapped\nexpression.argument parent=$wrapped index=0 expression=$again\nexpression.argument parent=$wrapped index=1 expression=$after\n".into(),
        "condition" => format!("expression.i64 as=$test-zero value=0\nexpression.call as=$pending function={equal}\nexpression.argument parent=$pending index=0 expression=$again\nexpression.argument parent=$pending index=1 expression=$test-zero\nexpression.i64 as=$yes value=1\nexpression.i64 as=$no value=2\nexpression.if as=$wrapped condition=$pending when-true=$yes when-false=$no\n"),
        "constructor-projection" => "expression.record as=$record\nexpression.record-field parent=$record index=0 name=value value=$again\nexpression.field as=$wrapped value=$record name=value\n".into(),
        "scrutinee" => "create.variant as=$phase module=$module name=phase visibility=private\nadd.case as=$done variant=$phase name=done payload=i64\nexpression.variant as=$phase-value case=$done payload=$again\nexpression.match as=$wrapped value=$phase-value\nexpression.local as=$result value=$matched\nexpression.match-arm parent=$wrapped index=0 case=$done as=$matched name=matched type=i64 body=$result\n".into(),
        "invoke" | "partial" | "fully-bound" => {
            *request = request.replace("expression.call as=$again function=$loop\nexpression.argument parent=$again index=0 expression=$decrement\nexpression.argument parent=$again index=1 expression=$sum\n", "");
            match context {
                "invoke" => "expression.function-value as=$callee function=$loop\nexpression.invoke as=$again function=$callee\nexpression.argument parent=$again index=0 expression=$decrement\nexpression.argument parent=$again index=1 expression=$sum\n".into(),
                "partial" => "expression.function-value as=$callee function=$loop\nexpression.bind as=$bound callee=$callee\nexpression.argument parent=$bound index=0 expression=$decrement\nexpression.invoke as=$again function=$bound\nexpression.argument parent=$again index=0 expression=$sum\n".into(),
                _ => "expression.function-value as=$callee function=$loop\nexpression.bind as=$bound callee=$callee\nexpression.argument parent=$bound index=0 expression=$decrement\nexpression.argument parent=$bound index=1 expression=$sum\nexpression.invoke as=$again function=$bound\n".into(),
            }
        }
        "mutual" => {
            *request = request.replace("expression.call as=$again function=$loop", "expression.call as=$again function=$other");
            "expression.local as=$other-n-value value=$other-n\nexpression.local as=$other-acc-value value=$other-acc\nexpression.call as=$other-body function=$loop\nexpression.argument parent=$other-body index=0 expression=$other-n-value\nexpression.argument parent=$other-body index=1 expression=$other-acc-value\ncreate.function as=$other module=$module name=other visibility=private result=i64 effect=task body=$other-body\nadd.parameter as=$other-n function=$other name=n type=i64\nadd.parameter as=$other-acc function=$other name=acc type=i64\n".into()
        }
        _ => panic!("unknown control fixture"),
    }
}

fn plain_task_program(
    context: &str,
) -> (crate::platform::kernel::KernelSnapshot, NormalizedProgram) {
    let standard = GraphRepository::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard"),
    )
    .unwrap()
    .view_current()
    .unwrap()
    .reconstruct_full_oracle()
    .unwrap()
    .value;
    let equal = declaration_named(&standard, "i64-equal").declaration;
    let subtract = declaration_named(&standard, "subtract").declaration;
    let add = declaration_named(&standard, "add").declaration;
    let temporary = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&temporary.path().join("plain"), &standard, None)
        .unwrap()
        .repository;
    let base = repository.view_current().unwrap().revision();
    let mut request = format!(
        r#"request base={base}
create.module as=$module name=plain-task-control
expression.local as=$n value=$loop-n
expression.i64 as=$zero value=0
expression.call as=$stop function={equal}
expression.argument parent=$stop index=0 expression=$n
expression.argument parent=$stop index=1 expression=$zero
expression.local as=$acc value=$loop-acc
expression.local as=$next-n value=$loop-n
expression.i64 as=$one value=1
expression.call as=$decrement function={subtract}
expression.argument parent=$decrement index=0 expression=$next-n
expression.argument parent=$decrement index=1 expression=$one
expression.local as=$old-acc value=$loop-acc
expression.local as=$add-n value=$loop-n
expression.call as=$sum function={add}
expression.argument parent=$sum index=0 expression=$old-acc
expression.argument parent=$sum index=1 expression=$add-n
expression.call as=$again function=$loop
expression.argument parent=$again index=0 expression=$decrement
expression.argument parent=$again index=1 expression=$sum
expression.if as=$body condition=$stop when-true=$acc when-false=$again
create.function as=$loop module=$module name=plain-task-loop visibility=public result=i64 effect=task body=$body
add.parameter as=$loop-n function=$loop name=n type=i64
add.parameter as=$loop-acc function=$loop name=acc type=i64
"#
    );
    let continuation = control_context(context, &mut request, &add.to_string(), &equal.to_string());
    if continuation.contains("as=$wrapped") {
        request = request.replace("when-false=$again", "when-false=$wrapped");
    }
    request.push_str(&continuation);
    let decoded =
        crate::platform::control::decode_compact_change("plain-task", request.as_bytes()).unwrap();
    let prepared = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    assert!(matches!(
        repository.publish(&prepared.publication).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    let snapshot = repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    let program = prepare_snapshot(&snapshot);
    (snapshot, program)
}

#[test]
fn task_tail_chain_has_constant_control_state() {
    let (snapshot, program) = plain_task_program("direct");
    let entry = declaration_named(&snapshot, "plain-task-loop");
    let policy = NormalizedRunPolicy {
        ..Default::default()
    };
    let mut accepted = true;
    for reference in [false, true] {
        for count in [0_i64, 8, 32, 128, 4097, 8192] {
            let args = vec![NormalizedValue::I64(count), NormalizedValue::I64(0)];
            let control = ExecutionControl::uncancelled();
            let (result, work) = if reference {
                let sink = Mutex::new(None);
                let result = NormalizedReferenceInterpreter::new(&snapshot, &program, policy)
                    .observing_checked(&sink)
                    .invoke(entry, args, None, &control);
                (
                    result.map(|(value, _)| value),
                    serde_json::to_value(sink.into_inner().unwrap().unwrap()).unwrap(),
                )
            } else {
                let sink = Mutex::new(None);
                let result = NormalizedVm::for_test(&program, policy)
                    .observing_checked(&sink)
                    .invoke(entry, args, None, &control);
                (
                    result.map(|(value, _)| value),
                    serde_json::to_value(sink.into_inner().unwrap().unwrap()).unwrap(),
                )
            };
            let expected = count * (count + 1) / 2;
            let good = matches!(&result, Ok(NormalizedValue::I64(value)) if *value == expected)
                && work["maximum_call_depth"].as_u64().unwrap() <= 2
                && work["maximum_live_locals"].as_u64().unwrap() <= 4
                && work["maximum_live_type_bindings"] == 0;
            println!(
                "{}",
                serde_json::json!({"reference":reference,"count":count,"expected":expected,"result":format!("{result:?}"),"work":work,"constant_control":good})
            );
            accepted &= good;
        }
    }
    assert!(
        accepted,
        "ordinary task terminal chains must retain constant control state"
    );
}

#[test]
fn task_tail_contexts_and_bound_mutual_transitions_preserve_pending_work() {
    let contexts = [
        "let",
        "sequence",
        "match",
        "invoke",
        "partial",
        "fully-bound",
        "mutual",
        "task-to-pure",
        "non-tail",
        "initializer",
        "earlier-sequence",
        "condition",
        "constructor-projection",
        "scrutinee",
    ];
    std::thread::scope(|scope| {
        for worker in 0..4 {
            scope.spawn(move || {
                for context in contexts.into_iter().skip(worker).step_by(4) {
                    assert_task_tail_context(context);
                }
            });
        }
    });
}

fn assert_task_tail_context(context: &str) {
    let terminal = matches!(
        context,
        "let"
            | "sequence"
            | "match"
            | "invoke"
            | "partial"
            | "fully-bound"
            | "mutual"
            | "task-to-pure"
    );
    let (snapshot, program) = plain_task_program(context);
    let entry = declaration_named(&snapshot, "plain-task-loop");
    let policy = NormalizedRunPolicy {
        ..Default::default()
    };
    for reference in [false, true] {
        println!("control-context={context} reference={reference}");
        let args = vec![NormalizedValue::I64(8192), NormalizedValue::I64(0)];
        let control = ExecutionControl::uncancelled();
        let result = if reference {
            NormalizedReferenceInterpreter::new(&snapshot, &program, policy)
                .invoke(entry, args, None, &control)
                .map(|(v, o)| (v, o.maximum_call_depth))
        } else {
            NormalizedVm::for_test(&program, policy)
                .invoke(entry, args, None, &control)
                .map(|(v, o)| (v, o.maximum_call_depth))
        };
        let (value, depth) = result.unwrap_or_else(|e| panic!("{context}/{reference}: {e:?}"));
        let expected = match context {
            "non-tail" => 33_558_528 + 8192,
            "earlier-sequence" => 1,
            "condition" => 2,
            _ => 33_558_528,
        };
        assert!(
            matches!(value, NormalizedValue::I64(actual) if actual == expected),
            "{context}/{reference}"
        );
        if terminal {
            assert!(depth <= 2, "{context}/{reference}: {depth}");
        } else {
            assert!(depth > 4096, "{context}/{reference}: {depth}");
        }
    }
}

#[test]
fn task_control_oracle_detects_disabled_production_transfers() {
    let (snapshot, mut program) = plain_task_program("direct");
    let entry = declaration_named(&snapshot, "plain-task-loop");
    let index = program.function(entry).unwrap();
    let NormalizedFunctionBody::Code(code) =
        &mut Arc::make_mut(&mut program.functions)[index.0 as usize].body
    else {
        panic!("graph")
    };
    for instruction in Arc::make_mut(&mut code.instructions) {
        if let NormalizedInstruction::TailCall {
            read_scope_arguments,
            function,
            type_arguments,
            effect_arguments,
            requirement_arguments,
            arguments,
        } = instruction
        {
            *instruction = NormalizedInstruction::Call {
                read_scope_arguments: read_scope_arguments.clone(),
                requirement_arguments: requirement_arguments.clone(),
                function: *function,
                type_arguments: type_arguments.clone(),
                effect_arguments: effect_arguments.clone(),
                arguments: *arguments,
            };
        }
    }
    let policy = NormalizedRunPolicy::default();
    for count in [2048_i64, 4096, 8192] {
        let started = std::time::Instant::now();
        let args = vec![NormalizedValue::I64(count), NormalizedValue::I64(0)];
        let (value, work) = NormalizedVm::for_test(&program, policy)
            .invoke(entry, args.clone(), None, &ExecutionControl::uncancelled())
            .unwrap();
        assert!(matches!(value, NormalizedValue::I64(actual) if actual == count * (count + 1) / 2));
        assert_eq!(work.maximum_call_depth, count as usize + 1);
        assert_eq!(work.live_call_frames_after, 0);
        assert_eq!(work.live_operands_after, 0);
        let (value, reference) = NormalizedReferenceInterpreter::new(&snapshot, &program, policy)
            .invoke(entry, args, None, &ExecutionControl::uncancelled())
            .unwrap();
        assert!(matches!(value, NormalizedValue::I64(actual) if actual == count * (count + 1) / 2));
        assert!(reference.maximum_call_depth <= 2);
        eprintln!(
            "call-stack scale input={count} frames={} elapsed_ms={}",
            work.maximum_call_depth,
            started.elapsed().as_millis()
        );
    }
    let observed = std::sync::Mutex::new(None);
    let cancelled = NormalizedVm::for_test(&program, policy)
        .observing_checked(&observed)
        .invoke(
            entry,
            vec![NormalizedValue::I64(16384), NormalizedValue::I64(0)],
            None,
            &ExecutionControl::cancel_after_checks(1000),
        )
        .unwrap_err();
    assert_eq!(
        cancelled.class,
        crate::platform::execution::ExecutionFailureClass::Cancelled
    );
    let work = observed.into_inner().unwrap().unwrap();
    assert_eq!(work.live_call_frames_after, 0);
    assert_eq!(work.live_operands_after, 0);
}

#[test]
#[ignore = "scale: grow operand stack beyond the former one-million-value default"]
fn vm_operand_stack_scale_and_cancellation_cleanup() {
    let (snapshot, mut program) = plain_task_program("direct");
    let entry = declaration_named(&snapshot, "plain-task-loop");
    let index = program.function(entry).unwrap();
    for width in [500_000, 1_000_000, 2_000_000] {
        let started = std::time::Instant::now();
        let NormalizedFunctionBody::Code(code) =
            &mut Arc::make_mut(&mut program.functions)[index.0 as usize].body
        else {
            panic!("graph")
        };
        code.instructions = std::iter::repeat_n(NormalizedInstruction::Unit, width)
            .chain(std::iter::repeat_n(NormalizedInstruction::Drop, width))
            .chain([NormalizedInstruction::I64(7), NormalizedInstruction::Return])
            .collect::<Vec<_>>()
            .into();
        let (value, work) = NormalizedVm::for_test(&program, NormalizedRunPolicy::default())
            .invoke(
                entry,
                vec![NormalizedValue::I64(0), NormalizedValue::I64(0)],
                None,
                &ExecutionControl::uncancelled(),
            )
            .unwrap();
        assert!(matches!(value, NormalizedValue::I64(7)));
        assert_eq!(work.maximum_value_stack, width);
        assert_eq!(work.live_operands_after, 0);
        assert_eq!(work.live_call_frames_after, 0);
        eprintln!(
            "operand-stack scale values={width} elapsed_ms={}",
            started.elapsed().as_millis()
        );
        let observed = std::sync::Mutex::new(None);
        let error = NormalizedVm::for_test(&program, NormalizedRunPolicy::default())
            .observing_checked(&observed)
            .invoke(
                entry,
                vec![NormalizedValue::I64(0), NormalizedValue::I64(0)],
                None,
                &ExecutionControl::cancel_after_checks(width as u64 / 2),
            )
            .unwrap_err();
        assert_eq!(
            error.class,
            crate::platform::execution::ExecutionFailureClass::Cancelled
        );
        let work = observed.into_inner().unwrap().unwrap();
        assert_eq!(work.live_operands_after, 0);
        assert_eq!(work.live_call_frames_after, 0);
    }
}
