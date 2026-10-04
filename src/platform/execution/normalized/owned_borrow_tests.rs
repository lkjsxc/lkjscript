//! Native scoped child reads retain custody across both evaluators and every exit.
use super::super::{byte_buffer, owned_i64_cell, owned_product};
use super::*;

const SOURCE: &str = r#"declarations.begin
(units (module create borrowed
  (type-alias CellPacket (owned-product (field payload OwnedI64Cell) (field tag I64)))
  (type-alias NestedPacket (owned-product (field payload CellPacket) (field tag I64)))
  (type-alias Outcome (owned-choice (case accepted I64) (case rejected OwnedI64Cell)))
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create read-cell (visibility private) (implementation core.cell.read)
    (parameter create c (type OwnedI64Cell) (use borrow)) (returns I64))
  (external create finish-cell (visibility private) (implementation core.cell.extract)
    (parameter create c (type OwnedI64Cell) (use consume)) (returns I64))
  (external create probe (visibility private) (implementation core.i64.add)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (external create divide (visibility private) (implementation core.i64.divide)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (external create less (visibility private) (implementation core.i64.less)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns Bool))
  (external create subtract (visibility private) (implementation core.i64.subtract)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (external create empty-buffer (visibility private) (implementation core.buffer.empty)
    (returns ByteBuffer))
  (external create push-buffer (visibility private) (implementation core.buffer.push)
    (parameter create n (type I64))
    (parameter create b (type ByteBuffer) (use consume)) (returns ByteBuffer))
  (external create read-buffer (visibility private) (implementation core.buffer.get)
    (parameter create n (type I64))
    (parameter create b (type ByteBuffer) (use borrow)) (returns I64))
  (external create freeze-buffer (visibility private) (implementation core.buffer.freeze)
    (parameter create b (type ByteBuffer) (use consume)) (returns Bytes))
  (owned-contract create Observe (visibility private)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_61000000000000000000000000000001 read
      (parameters (Self borrow)) (returns I64)))
  (function create real-read (visibility private) (effect pure)
    (parameter create c (type OwnedI64Cell) (use borrow)) (returns I64)
    (body (call read-cell (local c))))
  (function create alternate-read (visibility private) (effect pure)
    (parameter create c (type OwnedI64Cell) (use borrow)) (returns I64)
    (body (i64 99)))
  (owned-implementation create Real (visibility private) (contract Observe) (self OwnedI64Cell)
    (method method_61000000000000000000000000000001 real-read))
  (owned-implementation create Alternate (visibility private) (contract Observe) (self OwnedI64Cell)
    (method method_61000000000000000000000000000001 alternate-read))
  (function create packet (visibility private) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create tag (type I64)) (parameter create child (type T) (use consume))
    (returns (owned-product (field payload T) (field tag I64)))
    (body (pack-owned (type (owned-product (field payload T) (field tag I64)))
      (field payload (local child)) (field tag (local tag)))))
  (function create take-packet (visibility private) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create p (type (owned-product (field payload T) (field tag I64))) (use consume))
    (returns T)
    (body (unpack-owned (type (owned-product (field payload T) (field tag I64))) (local p)
      (field payload (binding child (type T))) (field tag (binding tag (type I64)))
      (in (local child)))))
  (function create inspect-depth (visibility private) (effect pure)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_61000000000000000000000000000001 ops Observe T)
    (parameter create depth (type I64))
    (parameter create p (type (owned-product (field payload T) (field tag I64))) (use borrow)) (returns I64)
    (body (borrow-owned-field (type (owned-product (field payload T) (field tag I64))) (local p)
      (field payload (binding view (type T)))
      (in (if (call less (local depth) (i64 1))
        (method-call parameter@inspect-depth@implparam_61000000000000000000000000000001
          Observe method_61000000000000000000000000000001 (local view))
        (implementation-call inspect-depth (types T)
          (implementations parameter@inspect-depth@implparam_61000000000000000000000000000001)
          (call subtract (local depth) (i64 1)) (local p)))))))
  (function create combine (visibility private) (effect pure)
    (parameter create prefix (type I64))
    (parameter create n (type I64))
    (parameter create p (type CellPacket) (use borrow))
    (returns I64)
    (body (sequence (local prefix) (field (local p) (name tag)) (local n))))
  (function create product-main (visibility public) (effect pure)
    (parameter create n (type I64)) (returns (record
      (alternate I64) (original I64) (pending I64) (real I64) (recursive I64) (unrelated I64)))
    (body (let
      (binding child (type OwnedI64Cell) (call new-cell (local n)))
      (binding p (type CellPacket) (call packet (types OwnedI64Cell) (i64 7) (local child)))
      (binding real (type I64) (implementation-call inspect-depth (types OwnedI64Cell)
        (implementations concrete@Real) (i64 0) (local p)))
      (binding alternate (type I64) (implementation-call inspect-depth (types OwnedI64Cell)
        (implementations concrete@Alternate) (i64 0) (local p)))
      (binding recursive (type I64) (implementation-call inspect-depth (types OwnedI64Cell)
        (implementations concrete@Real) (i64 12) (local p)))
      (binding pending (type I64) (call combine (i64 63)
        (borrow-owned-field (type CellPacket) (local p)
          (field payload (binding view (type OwnedI64Cell)))
          (in (call read-cell (local view)))) (local p)))
      (binding unrelated (type OwnedI64Cell) (borrow-owned-field (type CellPacket) (local p)
        (field payload (binding view (type OwnedI64Cell)))
        (in (sequence (call read-cell (local view)) (call new-cell (i64 77))))))
      (binding unrelated-value (type I64) (call finish-cell (local unrelated)))
      (binding child (type OwnedI64Cell) (call take-packet (types OwnedI64Cell) (local p)))
      (binding original (type I64) (call finish-cell (local child)))
      (in (record structural (field alternate (local alternate)) (field original (local original))
        (field pending (local pending)) (field real (local real))
        (field recursive (local recursive)) (field unrelated (local unrelated-value)))))))
  (function create choice-main (visibility public) (effect pure)
    (parameter create n (type I64)) (parameter create accepted (type Bool))
    (returns (record (first I64) (original I64) (second I64)))
    (body (let
      (binding child (type OwnedI64Cell) (call new-cell (local n)))
      (binding outcome (type Outcome) (if (local accepted)
        (choose-owned (type Outcome) (case accepted) (local n))
        (choose-owned (type Outcome) (case rejected) (local child))))
      (binding first (type I64) (match-borrowed-owned (type Outcome) (local outcome)
        (case rejected (binding view (type OwnedI64Cell)) (in (call real-read (local view))))
        (case accepted (binding value (type I64)) (in (local value)))))
      (binding second (type I64) (match-borrowed-owned (type Outcome) (local outcome)
        (case accepted (binding value (type I64)) (in (local value)))
        (case rejected (binding view (type OwnedI64Cell)) (in (call read-cell (local view))))))
      (binding original (type I64) (match-owned (type Outcome) (local outcome)
        (case accepted (binding value (type I64)) (in (local value)))
        (case rejected (binding child (type OwnedI64Cell)) (in (call finish-cell (local child))))))
      (in (record structural (field first (local first)) (field original (local original))
        (field second (local second)))))))
  (function create buffer-main (visibility public) (effect pure)
    (parameter create n (type I64)) (returns Bytes)
    (body (let
      (binding empty (type ByteBuffer) (call empty-buffer))
      (binding child (type ByteBuffer) (call push-buffer (local n) (local empty)))
      (binding p (type (owned-product (field payload ByteBuffer) (field tag I64)))
        (call packet (types ByteBuffer) (i64 9) (local child)))
      (binding observed (type I64) (borrow-owned-field
        (type (owned-product (field payload ByteBuffer) (field tag I64))) (local p)
        (field payload (binding view (type ByteBuffer)))
        (in (sequence (call read-buffer (i64 0) (local view))
          (call read-buffer (i64 0) (local view))))))
      (binding child (type ByteBuffer) (call take-packet (types ByteBuffer) (local p)))
      (in (call freeze-buffer (local child))))))
  (function create nested-main (visibility public) (effect pure)
    (parameter create fail (type Bool)) (returns I64)
    (body (let
      (binding cell (type OwnedI64Cell) (call new-cell (i64 137)))
      (binding inner (type CellPacket) (call packet (types OwnedI64Cell) (i64 19) (local cell)))
      (binding outer (type NestedPacket) (call packet (types CellPacket) (i64 23) (local inner)))
      (binding observed (type I64) (borrow-owned-field (type NestedPacket) (local outer)
        (field payload (binding inner-view (type CellPacket)))
        (in (borrow-owned-field (type CellPacket) (local inner-view)
          (field payload (binding cell-view (type OwnedI64Cell)))
          (in (sequence (field (local outer) (name tag)) (field (local inner-view) (name tag))
            (call read-cell (local cell-view))
            (let (binding transient (type OwnedI64Cell) (call new-cell (i64 99)))
              (in (call finish-cell (local transient))))
            (if (local fail) (call divide (i64 1) (i64 0)) (call probe (i64 31) (i64 42)))))))))
      (binding inner (type CellPacket) (call take-packet (types CellPacket) (local outer)))
      (binding cell (type OwnedI64Cell) (call take-packet (types OwnedI64Cell) (local inner)))
      (binding discarded (type I64) (call finish-cell (local cell)))
      (in (local observed)))))
  (function create tail-source (visibility private) (effect pure)
    (parameter create p (type CellPacket) (use borrow)) (returns I64)
    (body (borrow-owned-field (type CellPacket) (local p)
      (field payload (binding view (type OwnedI64Cell))) (in (call real-read (local view))))))
  (function create tail-main (visibility public) (effect pure) (returns I64)
    (body (let
      (binding cell (type OwnedI64Cell) (call new-cell (i64 137)))
      (binding p (type CellPacket) (call packet (types OwnedI64Cell) (i64 7) (local cell)))
      (binding observed (type I64) (call tail-source (local p)))
      (binding cell (type OwnedI64Cell) (call take-packet (types OwnedI64Cell) (local p)))
      (binding discarded (type I64) (call finish-cell (local cell)))
      (in (local observed)))))))
declarations.end"#;

fn source() -> crate::platform::kernel::KernelSnapshot {
    byte_buffer_tests::author_only(SOURCE).unwrap()
}

fn reader(source: &crate::platform::kernel::KernelSnapshot) -> FaultedReferenceRead<'_> {
    FaultedReferenceRead {
        source,
        schema: Arc::new(
            super::super::reference_schema::NormalizedReferenceSchema::reconstruct([source])
                .unwrap(),
        ),
    }
}

#[derive(Debug)]
struct Work {
    allocated: u64,
    depth: usize,
}

fn run_clean(
    reference: bool,
    reader: &FaultedReferenceRead<'_>,
    program: &NormalizedProgram,
    name: &str,
    arguments: Vec<NormalizedValue>,
    policy: NormalizedRunPolicy,
    control: &ExecutionControl,
) -> (Result<NormalizedValue, ExecutionError>, Work) {
    let products = owned_product::StorageObservation::start();
    let cells = owned_i64_cell::StorageObservation::start();
    let buffers = byte_buffer::StorageObservation::start();
    let entry = declaration_named(reader.source, name);
    let (result, work) = if reference {
        let observation = Mutex::new(None);
        let result = NormalizedReferenceInterpreter::from_reader(reader, program, policy)
            .observing_checked(&observation)
            .invoke(entry, arguments, None, control)
            .map(|pair| pair.0);
        let work = if let Some(observed) = observation.into_inner().unwrap() {
            assert_eq!(observed.live_call_frames_after, 0);
            assert_eq!(observed.live_transactions_after, 0);
            assert_eq!(observed.live_handles_after, 0);
            Work {
                allocated: observed.allocated_bytes,
                depth: observed.maximum_call_depth,
            }
        } else {
            assert!(
                result.is_err(),
                "only preflight failure can omit execution observation"
            );
            Work {
                allocated: 0,
                depth: 0,
            }
        };
        (result, work)
    } else {
        let observation = Mutex::new(None);
        let result = NormalizedVm::for_test(program, policy)
            .observing_checked(&observation)
            .invoke(entry, arguments, None, control)
            .map(|pair| pair.0);
        let work = if let Some(observed) = observation.into_inner().unwrap() {
            assert_eq!(observed.live_call_frames_after, 0);
            assert_eq!(observed.live_operands_after, 0);
            assert_eq!(observed.live_transactions_after, 0);
            assert_eq!(observed.live_handles_after, 0);
            Work {
                allocated: observed.allocated_bytes,
                depth: observed.maximum_call_depth,
            }
        } else {
            assert!(
                result.is_err(),
                "only preflight failure can omit execution observation"
            );
            Work {
                allocated: 0,
                depth: 0,
            }
        };
        (result, work)
    };
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
    assert_eq!(buffers.live(), (0, 0));
    products.assert_owners_released_after_loans();
    if result.is_ok() && products.created() != 0 {
        products.assert_transfers_preserve_allocations();
    }
    (result, work)
}

fn assert_record(value: NormalizedValue, expected: &[(&str, i64)]) {
    let NormalizedValue::Record(super::super::value::NormalizedRecord::Structural { fields }) =
        value
    else {
        panic!("expected ordinary result record");
    };
    let expected = expected
        .iter()
        .map(|(name, value)| (Name::new(*name).unwrap(), NormalizedValue::I64(*value)))
        .collect::<Vec<_>>();
    assert_eq!(fields.as_slice(), expected.as_slice());
}

#[test]
fn owned_borrow_native_products_choices_and_recursive_borrowed_sources() {
    let source = source();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let reader = reader(&source);
    for reference in [false, true] {
        // The ordinary prefix argument of combine remains pending while its
        // next ordinary argument evaluates a child loan; owned suffixes stay locals.
        for n in [-401, 0, 137] {
            let (result, work) = run_clean(
                reference,
                &reader,
                &program,
                "product-main",
                vec![NormalizedValue::I64(n)],
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled(),
            );
            assert_record(
                result.unwrap(),
                &[
                    ("alternate", 99),
                    ("original", n),
                    ("pending", n),
                    ("real", n),
                    ("recursive", n),
                    ("unrelated", 77),
                ],
            );
            assert!(work.depth >= 13, "reference={reference} {work:?}");
            for accepted in [false, true] {
                let (result, _) = run_clean(
                    reference,
                    &reader,
                    &program,
                    "choice-main",
                    vec![NormalizedValue::I64(n), NormalizedValue::Bool(accepted)],
                    NormalizedRunPolicy::foreground(),
                    &ExecutionControl::uncancelled(),
                );
                assert_record(
                    result.unwrap(),
                    &[("first", n), ("original", n), ("second", n)],
                );
            }
        }
        for n in [0, 127, 255] {
            let (result, _) = run_clean(
                reference,
                &reader,
                &program,
                "buffer-main",
                vec![NormalizedValue::I64(n)],
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled(),
            );
            assert_eq!(result.unwrap(), NormalizedValue::bytes(vec![n as u8]));
        }
        let (result, _) = run_clean(
            reference,
            &reader,
            &program,
            "product-main",
            vec![NormalizedValue::I64(137)],
            NormalizedRunPolicy {
                maximum_call_depth: 8,
                ..NormalizedRunPolicy::foreground()
            },
            &ExecutionControl::uncancelled(),
        );
        assert_eq!(result.unwrap_err().class, ExecutionFailureClass::Resource);
    }
}

#[test]
fn owned_borrow_nested_trap_cancellation_and_quota_release_loans_before_owners() {
    let source = source();
    let program = prepare_snapshot(&source);
    let reader = reader(&source);
    for reference in [false, true] {
        let (failure, _) = run_clean(
            reference,
            &reader,
            &program,
            "nested-main",
            vec![NormalizedValue::Bool(true)],
            NormalizedRunPolicy::foreground(),
            &ExecutionControl::uncancelled(),
        );
        assert_eq!(failure.unwrap_err().class, ExecutionFailureClass::Trap);
        let (success, baseline) = run_clean(
            reference,
            &reader,
            &program,
            "nested-main",
            vec![NormalizedValue::Bool(false)],
            NormalizedRunPolicy::foreground(),
            &ExecutionControl::uncancelled(),
        );
        assert_eq!(success.unwrap(), NormalizedValue::I64(73));
        let mut cancelled = 0;
        let mut complete = 0;
        for checks in (0..8192).chain([32768]) {
            let (result, _) = run_clean(
                reference,
                &reader,
                &program,
                "nested-main",
                vec![NormalizedValue::Bool(false)],
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::cancel_after_checks(checks),
            );
            match result {
                Ok(value) => {
                    assert_eq!(value, NormalizedValue::I64(73));
                    complete += 1;
                    break;
                }
                Err(error) => {
                    assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                    cancelled += 1;
                }
            }
        }
        assert!(cancelled > 0 && complete > 0);
        let mut exhausted = 0;
        let mut complete = 0;
        for limit in [
            0,
            16,
            64,
            256,
            1024,
            4096,
            baseline.allocated / 2,
            baseline.allocated.saturating_sub(1),
            baseline.allocated,
            baseline.allocated + 1,
        ]
        .into_iter()
        .chain(
            (0..2048)
                .step_by(8)
                .map(|offset| baseline.allocated.saturating_sub(offset)),
        ) {
            let (result, _) = run_clean(
                reference,
                &reader,
                &program,
                "nested-main",
                vec![NormalizedValue::Bool(false)],
                NormalizedRunPolicy {
                    maximum_allocated_bytes: Some(limit),
                    ..NormalizedRunPolicy::foreground()
                },
                &ExecutionControl::uncancelled(),
            );
            match result {
                Ok(value) => {
                    assert_eq!(value, NormalizedValue::I64(73));
                    complete += 1;
                }
                Err(error) => {
                    assert_eq!(error.class, ExecutionFailureClass::Resource);
                    exhausted += 1;
                }
            }
        }
        assert!(exhausted > 0 && complete > 0);
    }
}

#[derive(Clone, Copy, Debug)]
enum Fault {
    None,
    Error,
    Panic,
    Forgery,
    Cancel,
}

struct Inspect<'a> {
    products: &'a owned_product::StorageObservation,
    cells: &'a owned_i64_cell::StorageObservation,
    fault: Fault,
}
impl Inspect<'_> {
    fn answer(&self, control: &ExecutionControl) -> Result<NormalizedValue, ExecutionError> {
        // The outer guard has one loan; the projected inner product has a view
        // and its own guard. The cell has its direct child view.
        assert_eq!(self.products.live(), (2, 3));
        assert_eq!(self.cells.live(), (1, 1));
        match self.fault {
            Fault::None => Ok(NormalizedValue::I64(73)),
            Fault::Error => Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "test_owned_borrow_host",
                "host refused under nested child loans",
            )),
            Fault::Panic => panic!("host unwind under nested child loans"),
            Fault::Forgery => Ok(NormalizedValue::OwnedI64Cell(
                owned_i64_cell::OwnedI64Cell::new(
                    super::super::value::ValueOrigin::fresh().unwrap(),
                    0,
                ),
            )),
            Fault::Cancel => {
                control.cancel();
                Ok(NormalizedValue::I64(73))
            }
        }
    }
}
impl super::super::vm::NormalizedHost for Inspect<'_> {
    fn call(
        &self,
        _: &NormalizedProgram,
        _: &super::super::prepare::NormalizedFunction,
        _: &ImplementationName,
        _: &[TypeObjectDigest],
        _: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.answer(control)
    }
}
impl super::super::reference::NormalizedReferenceHost for Inspect<'_> {
    fn call(
        &self,
        _: &dyn super::super::value_schema::NormalizedValueSchema,
        _: &super::super::reference::ReferenceSignature,
        _: &ImplementationName,
        _: &[TypeObjectDigest],
        _: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.answer(control)
    }
}

#[test]
fn owned_borrow_host_failures_and_unwind_preserve_nested_cleanup_order() {
    let source = source();
    let program = prepare_snapshot(&source);
    let reader = reader(&source);
    let entry = declaration_named(&source, "nested-main");
    for reference in [false, true] {
        for fault in [
            Fault::None,
            Fault::Error,
            Fault::Panic,
            Fault::Forgery,
            Fault::Cancel,
            Fault::None,
        ] {
            let products = owned_product::StorageObservation::start();
            let cells = owned_i64_cell::StorageObservation::start();
            let buffers = byte_buffer::StorageObservation::start();
            let host = Inspect {
                products: &products,
                cells: &cells,
                fault,
            };
            let control = ExecutionControl::uncancelled();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if reference {
                    let observation = Mutex::new(None);
                    NormalizedReferenceInterpreter::from_reader(
                        &reader,
                        &program,
                        NormalizedRunPolicy::foreground(),
                    )
                    .observing(&observation, &host)
                    .invoke(entry, vec![NormalizedValue::Bool(false)], None, &control)
                    .map(|pair| pair.0)
                } else {
                    let observation = Mutex::new(None);
                    NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                        .observing(&observation, &host)
                        .invoke(entry, vec![NormalizedValue::Bool(false)], None, &control)
                        .map(|pair| pair.0)
                }
            }));
            match fault {
                Fault::None => assert_eq!(result.unwrap().unwrap(), NormalizedValue::I64(73)),
                Fault::Panic => assert!(result.is_err()),
                Fault::Cancel => assert_eq!(
                    result.unwrap().unwrap_err().class,
                    ExecutionFailureClass::Cancelled
                ),
                Fault::Error | Fault::Forgery => assert!(result.unwrap().is_err()),
            }
            assert_eq!(products.live(), (0, 0), "reference={reference} {fault:?}");
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(buffers.live(), (0, 0));
            products.assert_owners_released_after_loans();
        }
    }
}

#[test]
fn owned_borrow_vm_retains_borrowed_source_frame_on_counterfeit_tail_call() {
    let source = source();
    let mut program = prepare_snapshot(&source);
    let target = declaration_named(&source, "tail-source");
    let function = Arc::make_mut(&mut program.functions)
        .iter_mut()
        .find(|f| f.declaration == target)
        .unwrap();
    let NormalizedFunctionBody::Code(code) = &mut function.body else {
        panic!("graph helper code")
    };
    let instructions = Arc::make_mut(&mut code.instructions);
    let mut changed = 0;
    for index in 0..instructions.len().saturating_sub(1) {
        if !matches!(
            instructions[index + 1],
            NormalizedInstruction::EndOwnedBorrow { .. }
        ) {
            continue;
        }
        if let NormalizedInstruction::Call {
            requirement_arguments,
            effect_arguments,
            function,
            type_arguments,
            arguments,
        } = instructions[index].clone()
        {
            instructions[index] = NormalizedInstruction::TailCall {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
            };
            changed += 1;
        }
    }
    assert_eq!(changed, 1, "one call directly inside the live child scope");
    let reader = reader(&source);
    let (result, work) = run_clean(
        false,
        &reader,
        &program,
        "tail-main",
        vec![],
        NormalizedRunPolicy::foreground(),
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(result.unwrap(), NormalizedValue::I64(137));
    assert!(
        work.depth >= 3,
        "lexical source, helper and owner frames remain live: {work:?}"
    );
}

#[test]
fn owned_borrow_counterfeit_source_consumption_and_move_keep_custody_until_loan_cleanup() {
    let mut source = source();
    let mut program = prepare_snapshot(&source);
    let schema = reader(&source).schema;
    let entry = declaration_named(&source, "nested-main");
    let function = Arc::make_mut(&mut program.functions)
        .iter_mut()
        .find(|function| function.declaration == entry)
        .unwrap();
    let NormalizedFunctionBody::Code(code) = &mut function.body else {
        panic!("nested graph helper");
    };
    let instructions = Arc::make_mut(&mut code.instructions);
    let source_local = instructions
        .iter()
        .find_map(|instruction| match instruction {
            NormalizedInstruction::BorrowOwnedField { source_local, .. } => Some(*source_local),
            _ => None,
        })
        .unwrap();
    let mut active_scopes = 0;
    let mut changed = false;
    for instruction in instructions {
        match instruction {
            NormalizedInstruction::BorrowOwnedField { .. } => active_scopes += 1,
            NormalizedInstruction::LoadLocal { local, use_mode }
                if active_scopes == 2 && *local == source_local =>
            {
                assert_eq!(*use_mode, crate::platform::kernel::ParameterUse::Borrow);
                *use_mode = crate::platform::kernel::ParameterUse::Consume;
                changed = true;
                break;
            }
            _ => {}
        }
    }
    assert!(
        changed,
        "counterfeit root consume under two live projections"
    );
    let (outer_source, inner_body) = source
        .owners
        .values()
        .find_map(|owner| {
            let OwnerRecord::Expression(expression) = owner else {
                return None;
            };
            let ExpressionOperation::BorrowOwnedField {
                source: outer_source,
                body,
                ..
            } = expression.operation
            else {
                return None;
            };
            let Some(OwnerRecord::Expression(inner)) =
                source.owners.get(&OwnerKey::Expression(body))
            else {
                return None;
            };
            let ExpressionOperation::BorrowOwnedField {
                body: inner_body, ..
            } = inner.operation
            else {
                return None;
            };
            Some((outer_source, inner_body))
        })
        .unwrap();
    let OwnerRecord::Expression(parent_local) = &source.owners[&OwnerKey::Expression(outer_source)]
    else {
        panic!("exact root local");
    };
    let ExpressionOperation::Local { value } = parent_local.operation else {
        panic!("root local source");
    };
    let OwnerRecord::Expression(body) = source
        .owners
        .get_mut(&OwnerKey::Expression(inner_body))
        .unwrap()
    else {
        panic!("nested body");
    };
    body.operation = ExpressionOperation::Local { value };
    let reader = FaultedReferenceRead {
        source: &source,
        schema,
    };
    for reference in [false, true] {
        let (result, _) = run_clean(
            reference,
            &reader,
            &program,
            "nested-main",
            vec![NormalizedValue::Bool(false)],
            NormalizedRunPolicy::foreground(),
            &ExecutionControl::uncancelled(),
        );
        assert_eq!(result.unwrap_err().class, ExecutionFailureClass::Resource);
    }
    let function = Arc::make_mut(&mut program.functions)
        .iter_mut()
        .find(|function| function.declaration == entry)
        .unwrap();
    let NormalizedFunctionBody::Code(code) = &mut function.body else {
        panic!("nested graph helper");
    };
    let forged = Arc::make_mut(&mut code.instructions)
        .iter_mut()
        .find(|instruction| {
            matches!(instruction, NormalizedInstruction::LoadLocal {
            local, use_mode: crate::platform::kernel::ParameterUse::Consume,
        } if *local == source_local)
        })
        .unwrap();
    *forged = NormalizedInstruction::MoveLocal(source_local);
    let (result, _) = run_clean(
        false,
        &reader,
        &program,
        "nested-main",
        vec![NormalizedValue::Bool(false)],
        NormalizedRunPolicy::foreground(),
        &ExecutionControl::uncancelled(),
    );
    assert_eq!(result.unwrap_err().code, "normalized_local_resource_use");
}

#[test]
fn owned_borrow_rejects_structured_parallel_child_receiving_a_read_view() {
    const TRANSFER: &str = r#"declarations.begin
(units (module create loan-transfer
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create value (type I64)) (returns OwnedI64Cell))
  (function create consume-child (visibility private) (effect (task))
    (parameter create value (type OwnedI64Cell) (use consume))
    (returns Unit) (body (unit)))
  (function create ordinary-child (visibility private) (effect (task))
    (returns Unit) (body (unit)))
  (function create transfer (visibility private) (effect (task))
    (parameter create packet (type (owned-product (field payload OwnedI64Cell))) (use consume))
    (parameter create extra (type OwnedI64Cell) (use consume))
    (returns Unit)
    (body (sequence
      (borrow-owned-field (type (owned-product (field payload OwnedI64Cell))) (local packet)
        (field payload (binding view (type OwnedI64Cell)))
        (in (sequence
          (parallel (call consume-child (local view)) (call ordinary-child))
          (unit))))
      (unpack-owned (type (owned-product (field payload OwnedI64Cell))) (local packet)
        (field payload (binding retained (type OwnedI64Cell))) (in (unit))))))
  (function create transfer-main (visibility private) (effect (task)) (returns Unit)
    (body (let
      (binding value (type OwnedI64Cell) (call new-cell (i64 71)))
      (binding packet (type (owned-product (field payload OwnedI64Cell)))
        (pack-owned (type (owned-product (field payload OwnedI64Cell))) (field payload (local value))))
      (binding extra (type OwnedI64Cell) (call new-cell (i64 83)))
      (in (call transfer (local packet) (local extra))))))))
declarations.end"#;
    let rejected = byte_buffer_tests::author_only(TRANSFER).unwrap_err();
    assert!(rejected.contains("kernel_buffer_ownership"), "{rejected}");
    // A task may retain the projection while transferring an unrelated owner
    // into the same structured child. The loan does not erase task authority.
    let positive = TRANSFER.replace("(local view)", "(local extra)");
    let accepted = byte_buffer_tests::author_only(&positive).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(
        &accepted
    ));
    let mut program = prepare_snapshot(&accepted);
    let reader = reader(&accepted);
    for reference in [false, true] {
        let (result, _) = run_clean(
            reference,
            &reader,
            &program,
            "transfer-main",
            vec![],
            NormalizedRunPolicy::foreground(),
            &ExecutionControl::uncancelled(),
        );
        assert_eq!(result.unwrap(), NormalizedValue::Unit);
    }
    let transfer = declaration_named(&accepted, "transfer");
    let function = Arc::make_mut(&mut program.functions)
        .iter_mut()
        .find(|function| function.declaration == transfer)
        .unwrap();
    let NormalizedFunctionBody::Code(code) = &mut function.body else {
        panic!("task graph helper");
    };
    let instructions = Arc::make_mut(&mut code.instructions);
    let view = instructions
        .iter()
        .find_map(|instruction| match instruction {
            NormalizedInstruction::BorrowOwnedField { binding_local, .. } => Some(*binding_local),
            _ => None,
        })
        .unwrap();
    let parallel = instructions
        .iter()
        .position(|instruction| matches!(instruction, NormalizedInstruction::Parallel { .. }))
        .unwrap();
    assert!(matches!(
        instructions[parallel - 1],
        NormalizedInstruction::LoadLocal {
            use_mode: crate::platform::kernel::ParameterUse::Consume,
            ..
        }
    ));
    // Bypass local consume rejection to reach the actual structured transfer
    // boundary with a real sealed read token as the child's argument.
    instructions[parallel - 1] = NormalizedInstruction::LoadLocal {
        local: view,
        use_mode: crate::platform::kernel::ParameterUse::Borrow,
    };
    let children = super::super::vm::ChildProbe::start(program.value_origin);
    let (rejected, _) = run_clean(
        false,
        &reader,
        &program,
        "transfer-main",
        vec![],
        NormalizedRunPolicy::foreground(),
        &ExecutionControl::uncancelled(),
    );
    assert!(rejected.is_err());
    assert!(
        children.observed().is_empty(),
        "a read view must be rejected before either child starts"
    );
}
