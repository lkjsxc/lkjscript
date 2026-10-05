//! Complete owned contract applications keep structural results and cleanup exact.
use super::super::{owned_i64_cell, owned_storage};
use super::*;

const SOURCE: &str = r#"declarations.begin
(units (module create worklist-runtime
  (type-alias Cells (owned-sequence OwnedI64Cell))
  (type-alias CellItem (owned-product (field rest Cells) (field value OwnedI64Cell)))
  (type-alias CellPop (owned-choice (case empty Cells) (case item CellItem)))
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create extract-cell (visibility private) (implementation core.cell.extract)
    (parameter create value (type OwnedI64Cell) (use consume)) (returns I64))
  (external create add (visibility private) (implementation core.i64.add)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (external create less (visibility private) (implementation core.i64.less)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns Bool))
  (external create divide (visibility private) (implementation core.i64.divide)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (owned-contract create Finish (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_a4000000000000000000000000000001 finish
      (parameters (Self consume)) (returns I64)))
  (owned-contract create Worklist (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Item (constraint owned))
    (method method_a4000000000000000000000000000002 push
      (parameters (Item consume) (Self consume)) (returns Self))
    (method method_a4000000000000000000000000000003 pop
      (parameters (Self consume))
      (returns (owned-choice (case empty Self)
        (case item (owned-product (field rest Self) (field value Item)))))))
  (function create append (visibility private) (effect pure)
    (type-parameter create Q (constraint owned))
    (type-parameter create I (constraint owned))
    (implementation-parameter implparam_a4000000000000000000000000000001 ops Worklist Q (types I))
    (parameter create value (type I) (use consume))
    (parameter create queue (type Q) (use consume)) (returns Q)
    (body (method-call parameter@append@implparam_a4000000000000000000000000000001
      Worklist method_a4000000000000000000000000000002 (local value) (local queue))))
  (function create drain (visibility private) (effect pure)
    (type-parameter create Q (constraint owned transferable))
    (type-parameter create I (constraint owned transferable))
    (implementation-parameter implparam_a4000000000000000000000000000002 ops Worklist Q (types I))
    (implementation-parameter implparam_a4000000000000000000000000000003 finish Finish I)
    (parameter create total (type I64))
    (parameter create queue (type Q) (use consume)) (returns I64)
    (body (let
      (binding outcome
        (type (owned-choice (case empty Q)
          (case item (owned-product (field rest Q) (field value I)))))
        (method-call parameter@drain@implparam_a4000000000000000000000000000002
          Worklist method_a4000000000000000000000000000003 (local queue)))
      (in (match-owned
        (type (owned-choice (case empty Q)
          (case item (owned-product (field rest Q) (field value I))))) (local outcome)
        (case empty (binding rest (type Q)) (in (local total)))
        (case item (binding item (type (owned-product (field rest Q) (field value I))))
          (in (unpack-owned (type (owned-product (field rest Q) (field value I))) (local item)
            (field rest (binding rest (type Q))) (field value (binding value (type I)))
            (in (let (binding n (type I64)
              (method-call parameter@drain@implparam_a4000000000000000000000000000003
                Finish method_a4000000000000000000000000000001 (local value)))
              (in (implementation-call drain (types Q I)
                (implementations
                  parameter@drain@implparam_a4000000000000000000000000000002
                  parameter@drain@implparam_a4000000000000000000000000000003)
                (call add (local total) (local n)) (local rest)))))))))))))
  (function create run-task (visibility private) (effect (task))
    (type-parameter create Q (constraint owned transferable))
    (type-parameter create I (constraint owned transferable))
    (implementation-parameter implparam_a4000000000000000000000000000004 ops Worklist Q (types I))
    (implementation-parameter implparam_a4000000000000000000000000000005 finish Finish I)
    (parameter create queue (type Q) (use consume)) (returns I64)
    (body (implementation-call drain (types Q I)
      (implementations
        parameter@run-task@implparam_a4000000000000000000000000000004
        parameter@run-task@implparam_a4000000000000000000000000000005)
      (i64 0) (local queue))))
  (function create push (visibility private) (effect pure)
    (parameter create value (type OwnedI64Cell) (use consume))
    (parameter create queue (type Cells) (use consume)) (returns Cells)
    (body (sequence-push (type Cells) (local value) (local queue))))
  (function create pop (visibility private) (effect pure)
    (parameter create queue (type Cells) (use consume)) (returns CellPop)
    (body (sequence-pop (type Cells) (local queue))))
  (function create finish (visibility private) (effect pure)
    (parameter create value (type OwnedI64Cell) (use consume)) (returns I64)
    (body (call extract-cell (local value))))
  (function create fail (visibility private) (effect pure)
    (parameter create value (type OwnedI64Cell) (use consume)) (returns I64)
    (body (let (binding n (type I64) (call extract-cell (local value)))
      (in (call divide (local n) (i64 0))))))
  (owned-implementation create Flat (visibility private) (contract Worklist)
    (self Cells) (types OwnedI64Cell)
    (method method_a4000000000000000000000000000002 push)
    (method method_a4000000000000000000000000000003 pop))
  (owned-implementation create Cell (visibility private) (contract Finish) (self OwnedI64Cell)
    (method method_a4000000000000000000000000000001 finish))
  (owned-implementation create Fault (visibility private) (contract Finish) (self OwnedI64Cell)
    (method method_a4000000000000000000000000000001 fail))
  (function create build (visibility private) (effect pure)
    (parameter create index (type I64)) (parameter create count (type I64))
    (parameter create queue (type Cells) (use consume)) (returns Cells)
    (body (if (call less (local index) (local count))
      (let (binding value (type OwnedI64Cell) (call new-cell (local index)))
        (binding next (type Cells) (implementation-call append (types Cells OwnedI64Cell)
          (implementations concrete@Flat) (local value) (local queue)))
        (in (call build (call add (local index) (i64 1)) (local count) (local next))))
      (local queue))))
  (function create main (visibility public) (effect pure)
    (parameter create count (type I64)) (returns I64)
    (body (let (binding empty (type Cells) (sequence-empty (type Cells)))
      (binding queue (type Cells) (call build (i64 0) (local count) (local empty)))
      (in (implementation-call drain (types Cells OwnedI64Cell)
        (implementations concrete@Flat concrete@Cell) (i64 0) (local queue))))))
  (function create parallel-main (visibility public) (effect (task))
    (parameter create count (type I64)) (returns (record (left I64) (right I64)))
    (body (let
      (binding a (type Cells) (sequence-empty (type Cells)))
      (binding a (type Cells) (call build (i64 0) (local count) (local a)))
      (binding b (type Cells) (sequence-empty (type Cells)))
      (binding b (type Cells) (call build (i64 0) (local count) (local b)))
      (in (parallel
        (implementation-call run-task (types Cells OwnedI64Cell)
          (implementations concrete@Flat concrete@Cell) (local a))
        (implementation-call run-task (types Cells OwnedI64Cell)
          (implementations concrete@Flat concrete@Cell) (local b)))))))))
declarations.end"#;

fn invoke(
    reference: bool,
    source: &crate::platform::kernel::KernelSnapshot,
    program: &NormalizedProgram,
    count: i64,
    policy: NormalizedRunPolicy,
    control: &ExecutionControl,
) -> Result<(NormalizedValue, usize, u64), ExecutionError> {
    let entry = declaration_named(source, "main");
    if reference {
        NormalizedReferenceInterpreter::new(source, program, policy)
            .invoke(entry, vec![NormalizedValue::I64(count)], None, control)
            .map(|(value, work)| (value, work.maximum_call_depth, work.tail_transfers))
    } else {
        NormalizedVm::for_test(program, policy)
            .invoke(entry, vec![NormalizedValue::I64(count)], None, control)
            .map(|(value, work)| (value, work.maximum_call_depth, work.tail_transfers))
    }
}

#[test]
fn owned_contract_application_structural_results_tail_forward_exact_arguments() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        let cells = owned_i64_cell::StorageObservation::start();
        let composites = owned_storage::StorageObservation::start();
        for count in [0, 1, 17, 513] {
            let (value, depth, transfers) = invoke(
                reference,
                &source,
                &program,
                count,
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled(),
            )
            .unwrap();
            assert_eq!(value, NormalizedValue::I64(count * (count - 1) / 2));
            assert!(
                depth < 16,
                "reference={reference} count={count} depth={depth}"
            );
            if count == 513 {
                assert!(
                    transfers >= 1026,
                    "reference={reference} transfers={transfers}"
                );
            }
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(composites.live(), (0, 0));
        }
    }
}

#[test]
fn owned_contract_application_failure_and_interruption_join_cleanup() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&source);
    let failing = byte_buffer_tests::author_only(&SOURCE.replace(
        "(implementations concrete@Flat concrete@Cell) (i64 0)",
        "(implementations concrete@Flat concrete@Fault) (i64 0)",
    ))
    .unwrap();
    let failing_program = prepare_snapshot(&failing);
    for reference in [false, true] {
        let cells = owned_i64_cell::StorageObservation::start();
        let composites = owned_storage::StorageObservation::start();
        assert_eq!(
            invoke(
                reference,
                &failing,
                &failing_program,
                4,
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled()
            )
            .unwrap_err()
            .class,
            crate::platform::execution::ExecutionFailureClass::Trap
        );
        assert_eq!(cells.live(), (0, 0));
        assert_eq!(composites.live(), (0, 0));
        let mut cancelled = 0;
        let mut completed = 0;
        for checks in [0, 32, 128, 512, 2048, 8192, 32768, 131072] {
            match invoke(
                reference,
                &source,
                &program,
                4,
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::cancel_after_checks(checks),
            ) {
                Ok((value, _, _)) => {
                    assert_eq!(value, NormalizedValue::I64(6));
                    completed += 1;
                }
                Err(error) => {
                    assert_eq!(
                        error.class,
                        crate::platform::execution::ExecutionFailureClass::Cancelled
                    );
                    cancelled += 1;
                }
            }
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(composites.live(), (0, 0));
        }
        assert!(cancelled > 0 && completed > 0);
        let mut exhausted = 0;
        for quota in [0, 256, 1024, 4096, 16384, 1048576] {
            let policy = NormalizedRunPolicy {
                maximum_allocated_bytes: Some(quota),
                ..NormalizedRunPolicy::foreground()
            };
            match invoke(
                reference,
                &source,
                &program,
                4,
                policy,
                &ExecutionControl::uncancelled(),
            ) {
                Ok((value, _, _)) => assert_eq!(value, NormalizedValue::I64(6)),
                Err(error) => {
                    assert_eq!(
                        error.class,
                        crate::platform::execution::ExecutionFailureClass::Resource
                    );
                    exhausted += 1;
                }
            }
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(composites.live(), (0, 0));
        }
        assert!(exhausted > 0);
    }
}

#[test]
fn owned_contract_application_parallel_children_keep_exact_bindings() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "parallel-main");
    let expected = NormalizedValue::Record(super::super::value::NormalizedRecord::Structural {
        fields: Arc::new(vec![
            (Name::new("left").unwrap(), NormalizedValue::I64(136)),
            (Name::new("right").unwrap(), NormalizedValue::I64(136)),
        ]),
    });
    for reference in [false, true] {
        let cells = owned_i64_cell::StorageObservation::start();
        let composites = owned_storage::StorageObservation::start();
        let actual = if reference {
            NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .invoke(
                entry,
                vec![NormalizedValue::I64(17)],
                None,
                &ExecutionControl::uncancelled(),
            )
            .unwrap()
            .0
        } else {
            NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                .invoke(
                    entry,
                    vec![NormalizedValue::I64(17)],
                    None,
                    &ExecutionControl::uncancelled(),
                )
                .unwrap()
                .0
        };
        assert_eq!(actual, expected);
        assert_eq!(cells.live(), (0, 0));
        assert_eq!(composites.live(), (0, 0));
    }
}

#[test]
fn owned_contract_application_vm_and_transfer_reject_wrong_or_missing_item() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&source);
    let flat = declaration_named(&source, "Flat");
    let drain = declaration_named(&source, "drain");
    let run_task = declaration_named(&source, "run-task");
    let target = program
        .functions
        .iter()
        .find(|function| {
            function.declaration == run_task && !function.implementation_arguments.is_empty()
        })
        .unwrap();
    let application = &target.implementation_arguments[0];
    let types = Arc::from([application.self_type, application.type_arguments[0]]);
    let control = ExecutionControl::uncancelled();
    let index = super::super::value::FunctionIndex(
        program
            .functions
            .iter()
            .position(|function| std::ptr::eq(function, target))
            .unwrap() as u32,
        program.value_origin,
    );
    super::super::vm::transfer::TaskApplication::bind(
        &program,
        index,
        Arc::clone(&types),
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    for fault in 0..3 {
        let mut forged = program.clone();
        let mut changed = 0;
        for function in Arc::make_mut(&mut forged.functions) {
            if function.declaration != drain && function.declaration != run_task {
                continue;
            }
            for application in Arc::make_mut(&mut function.implementation_arguments) {
                if application.implementation != flat {
                    continue;
                }
                changed += 1;
                application.type_arguments = match fault {
                    0 => Arc::from([]),
                    1 => Arc::from([application.self_type]),
                    _ => Arc::from([application.type_arguments[0], application.type_arguments[0]]),
                };
            }
        }
        assert!(changed > 0);
        let cells = owned_i64_cell::StorageObservation::start();
        let composites = owned_storage::StorageObservation::start();
        let error = invoke(
            false,
            &source,
            &forged,
            4,
            NormalizedRunPolicy::foreground(),
            &control,
        )
        .unwrap_err();
        assert!(
            error
                .message
                .contains("implementation contract type argument"),
            "{error:?}"
        );
        assert_eq!(
            cells.created(),
            4,
            "invocation admitted only after all producer values exist"
        );
        assert_eq!(cells.live(), (0, 0));
        assert_eq!(composites.live(), (0, 0));
        assert!(
            super::super::vm::transfer::TaskApplication::bind(
                &forged,
                index,
                Arc::clone(&types),
                &control,
                &mut |_| Ok(()),
            )
            .is_err()
        );
    }
}
