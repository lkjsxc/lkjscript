//! Invocation-scoped read custody through borrowed results, tails and forks.
use super::*;
use crate::platform::execution::normalized::{
    owned_i64_cell::StorageObservation as Cells, owned_product::StorageObservation as Products,
    tests::byte_buffer_tests,
};
use crate::platform::kernel::{OwnerKey, OwnerRecord};
use crate::platform::publication::GraphRepository;
use std::sync::{Mutex, OnceLock};

const SOURCE: &str = r#"declarations.begin
(units (module create scoped-reads
  (type-alias Packet (owned-product (field cell OwnedI64Cell) (field tag I64)))
  (type-alias Nested (owned-product (field packet Packet) (field tag I64)))
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create read-cell (visibility private) (implementation core.cell.read)
    (parameter create cell (type OwnedI64Cell) (use borrow)) (returns I64))
  (external create finish-cell (visibility private) (implementation core.cell.extract)
    (parameter create cell (type OwnedI64Cell) (use consume)) (returns I64))
  (external create add (visibility private) (implementation core.i64.add)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (external create subtract (visibility private) (implementation core.i64.subtract)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (external create less (visibility private) (implementation core.i64.less)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns Bool))
  (external create divide (visibility private) (implementation core.i64.divide)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (function create deep (visibility private) (effect pure)
    (parameter create outer (type Nested) (use borrow))
    (returns OwnedI64Cell (borrow-from outer))
    (body (borrow-owned-field (type Nested) (local outer)
      (field packet (binding packet (type Packet)))
      (in (borrow-owned-field (type Packet) (local packet)
        (field cell (binding cell (type OwnedI64Cell))) (in (local cell)))))))
  (function create reader (visibility private) (effect (task))
    (parameter create fail (type Bool))
    (parameter create cell (type OwnedI64Cell) (use borrow)) (returns I64)
    (body (if (local fail) (call divide (i64 1) (i64 0))
      (call read-cell (local cell)))))
  (function create forward (visibility private) (effect (task))
    (parameter create fail (type Bool))
    (parameter create cell (type OwnedI64Cell) (use borrow)) (returns I64)
    (body (call reader (local fail) (local cell))))
  (function create branch (visibility private) (effect (task))
    (parameter create depth (type I64)) (parameter create fail (type Bool))
    (parameter create cell (type OwnedI64Cell) (use borrow)) (returns I64)
    (body (if (call less (local depth) (i64 1))
      (call forward (local fail) (local cell))
      (let
        (binding next (type I64) (call subtract (local depth) (i64 1)))
        (binding pair (type (record (left I64) (right I64)))
          (parallel (call branch (local next) (local fail) (local cell))
            (call branch (local next) (local fail) (local cell))))
        (in (call add (field (local pair) (name left))
          (field (local pair) (name right))))))))
  (function create main (visibility public) (effect (task))
    (parameter create n (type I64)) (parameter create depth (type I64))
    (parameter create fail (type Bool)) (returns I64)
    (body (let
      (binding cell (type OwnedI64Cell) (call new-cell (local n)))
      (binding packet (type Packet) (pack-owned (type Packet)
        (field cell (local cell)) (field tag (i64 19))))
      (binding outer (type Nested) (pack-owned (type Nested)
        (field packet (local packet)) (field tag (i64 23))))
      (binding observed (type I64)
        (borrow-call (call deep (local outer))
          (binding selected (type OwnedI64Cell))
          (in (let
            (binding pair (type (record (left I64) (right I64)))
              (parallel (call branch (local depth) (local fail) (local selected))
                (call branch (local depth) (local fail) (local selected))))
            (in (call add (field (local pair) (name left))
              (field (local pair) (name right))))))))
      (binding original (type I64) (unpack-owned (type Nested) (local outer)
        (field packet (binding packet (type Packet))) (field tag (binding tag (type I64)))
        (in (unpack-owned (type Packet) (local packet)
          (field cell (binding cell (type OwnedI64Cell))) (field tag (binding tag (type I64)))
          (in (call finish-cell (local cell)))))))
      (in (call add (local observed) (local original))))))))
declarations.end"#;

fn fixture() -> (Arc<NormalizedProgram>, FunctionIndex) {
    static PREPARED: OnceLock<(Arc<NormalizedProgram>, FunctionIndex)> = OnceLock::new();
    let (program, main) = PREPARED.get_or_init(fresh_fixture);
    (Arc::clone(program), *main)
}

fn fresh_fixture() -> (Arc<NormalizedProgram>, FunctionIndex) {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&temporary.path().join("scoped-reads"), &source, None)
        .unwrap()
        .repository;
    let program = crate::platform::normalized_lifecycle::prepare_repository(repository)
        .unwrap()
        .program;
    let main = source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(owner))
                if owner.name.as_str() == "main" =>
            {
                program.function(DeclarationReference {
                    package: source.root.package_id,
                    declaration: *declaration,
                })
            }
            _ => None,
        })
        .unwrap();
    (program, main)
}

#[test]
fn scoped_parallel_children_overlap_while_reading_one_guarded_source() {
    // This program has its own prepared origin so its barrier cannot intercept
    // another test's zero-worker or recursive invocation of the shared fixture.
    let (program, main) = fresh_fixture();
    let executor = crate::platform::runtime::structured::StructuredExecutor::for_test(1);
    let products = Products::start();
    let cells = Cells::start();
    let probe = ChildProbe::start(program.value_origin);
    let (value, observation) = NormalizedVm::new(
        &program,
        NormalizedRunPolicy::foreground(),
        &executor.handle(),
    )
    .invoke_entry(
        NormalizedEntryPoint::Function(main),
        vec![
            NormalizedValue::I64(137),
            NormalizedValue::I64(0),
            NormalizedValue::Bool(false),
        ],
        None,
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    // ChildProbe opens its bounded barrier only after both child invocations
    // have adopted and checked their arguments. The literal passes the same
    // source-tied cell view twice, within two live owning product ancestors.
    let children = probe.observed();
    assert_eq!(children.len(), 2);
    assert_ne!(children[0].0, children[1].0);
    assert_ne!(children[0].1, children[1].1);
    assert_eq!(observation.parallel_scopes, 1);
    assert_eq!(observation.parallel_worker_dispatches, 1);
    // Two child reads plus draining the original owner after joining.
    assert_eq!(value, NormalizedValue::I64(137 * 3));
    assert_eq!(cells.created(), 1);
    assert_eq!(products.created(), 2);
    assert_eq!(cells.live(), (0, 0));
    assert_eq!(products.live(), (0, 0));
    products.assert_owners_released_after_loans();
    products.assert_transfers_preserve_allocations();
    assert_eq!(observation.live_call_frames_after, 0);
    assert_eq!(observation.live_operands_after, 0);
    assert_eq!(observation.live_handles_after, 0);
}

#[test]
fn checked_memory_wrapper_cannot_certify_malformed_raw_storage() {
    let (program, _) = fixture();
    let packet = program
        .types
        .iter()
        .find_map(|(ty, object)| match &object.form {
            TypeForm::OwnedProduct { fields }
                if fields.len() == 2 && fields[0].name.as_str() == "cell" =>
            {
                Some(*ty)
            }
            _ => None,
        })
        .unwrap();
    let domain = super::super::value::ValueOrigin::fresh().unwrap();
    let forged = super::super::owned_product::OwnedProduct::create(
        domain,
        packet,
        vec![NormalizedValue::Bool(false), NormalizedValue::I64(19)],
        &ExecutionControl::uncancelled(),
        &mut |_| Ok(()),
    )
    .unwrap();
    assert!(
        CheckedValue::memory(&program, NormalizedValue::OwnedProduct(forged)).is_err(),
        "a wrapper must not convert unadmitted physical storage into typed lending authority"
    );
}

#[test]
fn scoped_parallel_reads_retain_ancestors_through_tails_and_nested_groups() {
    let (program, main) = fixture();
    for workers in [0, 2] {
        let executor = crate::platform::runtime::structured::StructuredExecutor::for_test(workers);
        let products = Products::start();
        let cells = Cells::start();
        let observation = Mutex::new(None);
        let vm = NormalizedVm::new(
            &program,
            NormalizedRunPolicy::foreground(),
            &executor.handle(),
        )
        .observing_checked(&observation);
        for depth in [0, 1, 3] {
            let result = vm
                .invoke_entry(
                    NormalizedEntryPoint::Function(main),
                    vec![
                        NormalizedValue::I64(137),
                        NormalizedValue::I64(depth),
                        NormalizedValue::Bool(false),
                    ],
                    None,
                    &ExecutionControl::uncancelled(),
                )
                .unwrap();
            assert_eq!(
                result.0,
                NormalizedValue::I64(137 * ((1 << (depth + 1)) + 1))
            );
            assert_eq!(result.1.parallel_scopes, (1 << (depth + 1)) - 1);
            assert!(result.1.tail_transfers > 0);
            assert_eq!(result.1.live_call_frames_after, 0);
            assert_eq!(result.1.live_operands_after, 0);
            assert_eq!(products.live(), (0, 0));
            assert_eq!(cells.live(), (0, 0));
            products.assert_owners_released_after_loans();
        }
    }
}

#[test]
fn scoped_parallel_failure_finishes_lending_before_worker_reuse() {
    let (program, main) = fixture();
    let executor = crate::platform::runtime::structured::StructuredExecutor::for_test(2);
    let products = Products::start();
    let cells = Cells::start();
    let vm = NormalizedVm::new(
        &program,
        NormalizedRunPolicy::foreground(),
        &executor.handle(),
    );
    let result = vm.invoke_entry(
        NormalizedEntryPoint::Function(main),
        vec![
            NormalizedValue::I64(137),
            NormalizedValue::I64(2),
            NormalizedValue::Bool(true),
        ],
        None,
        &ExecutionControl::uncancelled(),
    );
    assert!(result.is_err());
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
    products.assert_owners_released_after_loans();
    let healthy = vm
        .invoke_entry(
            NormalizedEntryPoint::Function(main),
            vec![
                NormalizedValue::I64(137),
                NormalizedValue::I64(2),
                NormalizedValue::Bool(false),
            ],
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    assert_eq!(healthy.0, NormalizedValue::I64(137 * 9));
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
    products.assert_owners_released_after_loans();
}

#[test]
fn scoped_parallel_quota_and_cancellation_release_every_read_before_owners() {
    let (program, main) = fixture();
    let executor = crate::platform::runtime::structured::StructuredExecutor::for_test(0);
    let products = Products::start();
    let cells = Cells::start();
    let policy = NormalizedRunPolicy::foreground();
    let arguments = || {
        vec![
            NormalizedValue::I64(137),
            NormalizedValue::I64(2),
            NormalizedValue::Bool(false),
        ]
    };
    let baseline = NormalizedVm::new(&program, policy, &executor.handle())
        .invoke_entry(
            NormalizedEntryPoint::Function(main),
            arguments(),
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap()
        .1;
    for maximum in [
        0,
        baseline.allocated_bytes / 4,
        baseline.allocated_bytes / 2,
        baseline.allocated_bytes * 3 / 4,
        baseline.allocated_bytes - 1,
    ] {
        let result = NormalizedVm::new(
            &program,
            NormalizedRunPolicy {
                maximum_allocated_bytes: Some(maximum),
                ..policy
            },
            &executor.handle(),
        )
        .invoke_entry(
            NormalizedEntryPoint::Function(main),
            arguments(),
            None,
            &ExecutionControl::uncancelled(),
        );
        assert_eq!(result.unwrap_err().class, ExecutionFailureClass::Resource);
        assert_eq!(products.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
        products.assert_owners_released_after_loans();
    }
    let mut cancelled_after_fork = false;
    for checks in [0, 64, 128, 256, 512, 1024, 2048, 4096] {
        let observation = Mutex::new(None);
        let result = NormalizedVm::new(&program, policy, &executor.handle())
            .observing_checked(&observation)
            .invoke_entry(
                NormalizedEntryPoint::Function(main),
                arguments(),
                None,
                &ExecutionControl::cancel_after_checks(checks),
            );
        match result {
            Err(error) => {
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                if observation
                    .into_inner()
                    .unwrap()
                    .is_some_and(|observation| observation.parallel_scopes != 0)
                {
                    cancelled_after_fork = true;
                }
            }
            Ok(result) => assert_eq!(result.0, NormalizedValue::I64(137 * 9)),
        }
        assert_eq!(products.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
        products.assert_owners_released_after_loans();
    }
    assert!(cancelled_after_fork);
    let result = NormalizedVm::new(&program, policy, &executor.handle()).invoke_entry(
        NormalizedEntryPoint::Function(main),
        arguments(),
        None,
        &ExecutionControl::with_deadline(std::time::Instant::now()),
    );
    assert_eq!(result.unwrap_err().code, "execution_deadline");
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
}
