//! Literal native owning declarations, independently observed custody and results.
use super::super::owned_i64_cell::StorageObservation as Cells;
use super::super::owned_product::StorageObservation as Products;
use super::super::value::NormalizedRecord;
use super::*;

const SOURCE: &str = r#"declarations.begin
(units (module create owning-records
  (owned-record create Parcel (visibility public)
    (type-parameter create T (constraint owned))
    (field create value (type T)) (field create stamp (type I64)))
  (owned-record create Empty (visibility public))
  (owned-record create Data (visibility public)
    (type-parameter create T (constraint transferable))
    (field create value (type T)))
  (external create new (visibility private) (implementation core.cell.create)
    (parameter create value (type I64)) (returns OwnedI64Cell))
  (external create read (visibility private) (implementation core.cell.read)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64))
  (external create finish (visibility private) (implementation core.cell.extract)
    (parameter create value (type OwnedI64Cell) (use consume)) (returns I64))
  (function create selected (visibility private) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create parcel (type (Parcel T)) (use borrow))
    (returns T (borrow-from parcel))
    (body (borrow-owned-field (type (Parcel T)) (local parcel)
      (field value (binding view (type T))) (in (local view)))))
  (function create main (visibility private) (effect pure)
    (parameter create n (type I64))
    (returns (record (direct I64) (payload I64) (stamp I64)))
    (body (let
      (binding value (type OwnedI64Cell) (call new (local n)))
      (binding parcel (type (Parcel OwnedI64Cell))
        (pack-owned (type (Parcel OwnedI64Cell))
          (field stamp (i64 73)) (field value (local value))))
      (binding stamp (type I64) (field (local parcel) Parcel::stamp))
      (binding direct (type I64)
        (borrow-call (call selected (types OwnedI64Cell) (local parcel))
          (binding view (type OwnedI64Cell)) (in (call read (local view)))))
      (in (unpack-owned (type (Parcel OwnedI64Cell)) (local parcel)
        (field stamp (binding ignored (type I64)))
        (field value (binding value (type OwnedI64Cell)))
        (in (let (binding payload (type I64) (call finish (local value)))
          (in (record structural (field stamp (local stamp))
            (field direct (local direct)) (field payload (local payload)))))))))))
  (function create data-main (visibility private) (effect pure)
    (parameter create n (type I64)) (returns I64)
    (body (let
      (binding empty (type Empty) (pack-owned (type Empty)))
      (binding data (type (Data I64)) (pack-owned (type (Data I64)) (field value (local n))))
      (binding read (type I64) (field (local data) Data::value))
      (in (unpack-owned (type Empty) (local empty)
        (in (unpack-owned (type (Data I64)) (local data)
          (field value (binding result (type I64))) (in (local result)))))))))
  (function create relay (visibility private) (effect (task))
    (parameter create parcel (type (Parcel OwnedI64Cell)) (use consume))
    (returns (Parcel OwnedI64Cell)) (body (local parcel)))
  (function create side (visibility private) (effect (task))
    (parameter create n (type I64)) (returns I64) (body (local n)))
  (function create inspect (visibility private) (effect (task))
    (parameter create parcel (type (Parcel OwnedI64Cell)) (use borrow))
    (returns I64)
    (body (borrow-owned-field (type (Parcel OwnedI64Cell)) (local parcel)
      (field value (binding view (type OwnedI64Cell))) (in (call read (local view))))))
  (function create moved-main (visibility private) (effect (task))
    (parameter create n (type I64))
    (returns (record (direct I64) (payload I64) (stamp I64)))
    (body (let
      (binding value (type OwnedI64Cell) (call new (local n)))
      (binding parcel (type (Parcel OwnedI64Cell))
        (pack-owned (type (Parcel OwnedI64Cell))
          (field stamp (i64 73)) (field value (local value))))
      (binding joined (type (owned-product (field left (Parcel OwnedI64Cell)) (field right I64)))
        (parallel (call relay (local parcel)) (call side (local n))))
      (in (unpack-owned (type (owned-product (field left (Parcel OwnedI64Cell)) (field right I64))) (local joined)
        (field left (binding parcel (type (Parcel OwnedI64Cell))))
        (field right (binding direct (type I64)))
        (in (let (binding stamp (type I64) (field (local parcel) Parcel::stamp))
          (in (unpack-owned (type (Parcel OwnedI64Cell)) (local parcel)
            (field value (binding value (type OwnedI64Cell)))
            (field stamp (binding ignored (type I64)))
            (in (let (binding payload (type I64) (call finish (local value)))
              (in (record structural (field direct (local direct))
                (field payload (local payload)) (field stamp (local stamp)))))))))))))))
  (function create shared-main (visibility private) (effect (task))
    (parameter create n (type I64))
    (returns (record (direct I64) (payload I64) (stamp I64)))
    (body (let
      (binding value (type OwnedI64Cell) (call new (local n)))
      (binding parcel (type (Parcel OwnedI64Cell))
        (pack-owned (type (Parcel OwnedI64Cell))
          (field stamp (i64 73)) (field value (local value))))
      (binding joined (type (record (left I64) (right I64)))
        (parallel (call inspect (local parcel)) (call side (local n))))
      (binding direct (type I64) (field (local joined) (name left)))
      (binding stamp (type I64) (field (local parcel) Parcel::stamp))
      (in (unpack-owned (type (Parcel OwnedI64Cell)) (local parcel)
        (field value (binding value (type OwnedI64Cell)))
        (field stamp (binding ignored (type I64)))
        (in (let (binding payload (type I64) (call finish (local value)))
          (in (record structural (field direct (local direct))
            (field payload (local payload)) (field stamp (local stamp)))))))))))))
declarations.end"#;

fn expected(n: i64) -> NormalizedValue {
    NormalizedValue::Record(NormalizedRecord::Structural {
        fields: Arc::new(vec![
            (Name::new("direct").unwrap(), NormalizedValue::I64(n)),
            (Name::new("payload").unwrap(), NormalizedValue::I64(n)),
            (Name::new("stamp").unwrap(), NormalizedValue::I64(73)),
        ]),
    })
}

#[test]
fn native_nominal_pack_borrow_unpack_preserves_one_envelope_and_child_identity() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        for n in [i64::MIN, -257, 0, 128, i64::MAX] {
            let products = Products::start();
            let cells = Cells::start();
            let main = declaration_named(&source, "main");
            let result = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(
                    main,
                    vec![NormalizedValue::I64(n)],
                    None,
                    &ExecutionControl::uncancelled(),
                )
                .unwrap()
                .0
            } else {
                NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                    .invoke(
                        main,
                        vec![NormalizedValue::I64(n)],
                        None,
                        &ExecutionControl::uncancelled(),
                    )
                    .unwrap()
                    .0
            };
            assert_eq!(result, expected(n), "reference={reference}");
            assert_eq!(
                products.created(),
                1,
                "nominality adds no outer custody envelope"
            );
            assert_eq!(cells.created(), 1);
            products.assert_transfers_preserve_allocations();
            products.assert_owners_released_after_loans();
            assert_eq!(products.live(), (0, 0));
            assert_eq!(cells.live(), (0, 0));
        }
    }
}

#[test]
fn native_empty_and_ordinary_only_nominals_keep_affine_custody() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        let products = Products::start();
        let main = declaration_named(&source, "data-main");
        let result = if reference {
            NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .invoke(
                main,
                vec![NormalizedValue::I64(97)],
                None,
                &ExecutionControl::uncancelled(),
            )
            .unwrap()
            .0
        } else {
            NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                .invoke(
                    main,
                    vec![NormalizedValue::I64(97)],
                    None,
                    &ExecutionControl::uncancelled(),
                )
                .unwrap()
                .0
        };
        assert_eq!(result, NormalizedValue::I64(97));
        assert_eq!(products.created(), 2);
        assert_eq!(products.live(), (0, 0));
    }
}

#[test]
fn nominal_task_movement_and_scoped_sharing_preserve_backing_allocations() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        for (name, envelopes) in [("moved-main", 2), ("shared-main", 1)] {
            for n in [-257, i64::MAX] {
                let products = Products::start();
                let cells = Cells::start();
                let entry = declaration_named(&source, name);
                let result = if reference {
                    NormalizedReferenceInterpreter::new(
                        &source,
                        &program,
                        NormalizedRunPolicy::foreground(),
                    )
                    .invoke(
                        entry,
                        vec![NormalizedValue::I64(n)],
                        None,
                        &ExecutionControl::uncancelled(),
                    )
                    .unwrap()
                    .0
                } else {
                    NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                        .invoke(
                            entry,
                            vec![NormalizedValue::I64(n)],
                            None,
                            &ExecutionControl::uncancelled(),
                        )
                        .unwrap()
                        .0
                };
                assert_eq!(result, expected(n), "{name} reference={reference}");
                // A mixed parallel result has its one structural envelope;
                // moving/sharing the nominal owner adds no envelope.
                assert_eq!(products.created(), envelopes);
                assert_eq!(cells.created(), 1);
                products.assert_transfers_preserve_allocations();
                products.assert_owners_released_after_loans();
                assert_eq!(products.live(), (0, 0));
                assert_eq!(cells.live(), (0, 0));
            }
        }
    }
}

#[test]
fn nominal_task_cancellation_and_quota_failures_join_custody() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        for name in ["moved-main", "shared-main"] {
            let entry = declaration_named(&source, name);
            let run = |policy, control: &ExecutionControl| {
                let arguments = vec![NormalizedValue::I64(-257)];
                if reference {
                    NormalizedReferenceInterpreter::new(&source, &program, policy)
                        .invoke(entry, arguments, None, control)
                        .map(|pair| pair.0)
                } else {
                    NormalizedVm::for_test(&program, policy)
                        .invoke(entry, arguments, None, control)
                        .map(|pair| pair.0)
                }
            };
            let products = Products::start();
            let cells = Cells::start();
            let mut completed = 0;
            let mut refused = 0;
            for checks in (0..512).step_by(16).chain([100_000]) {
                match run(
                    NormalizedRunPolicy::foreground(),
                    &ExecutionControl::cancel_after_checks(checks),
                ) {
                    Ok(value) => {
                        assert_eq!(value, expected(-257));
                        completed += 1;
                    }
                    Err(error) => {
                        assert_eq!(
                            error.class,
                            crate::platform::execution::ExecutionFailureClass::Cancelled
                        );
                        refused += 1;
                    }
                }
                assert_eq!(
                    products.live(),
                    (0, 0),
                    "{name} reference={reference} checks={checks}"
                );
                assert_eq!(cells.live(), (0, 0));
            }
            assert!(completed > 0 && refused > 0);
            completed = 0;
            refused = 0;
            for maximum in [0, 64, 1024, 65_536, 1_048_576] {
                match run(
                    NormalizedRunPolicy {
                        maximum_allocated_bytes: Some(maximum),
                        ..NormalizedRunPolicy::foreground()
                    },
                    &ExecutionControl::uncancelled(),
                ) {
                    Ok(value) => {
                        assert_eq!(value, expected(-257));
                        completed += 1;
                    }
                    Err(error) => {
                        assert_eq!(
                            error.class,
                            crate::platform::execution::ExecutionFailureClass::Resource
                        );
                        refused += 1;
                    }
                }
                assert_eq!(
                    products.live(),
                    (0, 0),
                    "{name} reference={reference} quota={maximum}"
                );
                assert_eq!(cells.live(), (0, 0));
            }
            assert!(completed > 0 && refused > 0);
            products.assert_transfers_preserve_allocations();
            products.assert_owners_released_after_loans();
        }
    }
}
