//! Literal generic child meaning and independently expected returned owners/data.
use super::super::super::owned_i64_cell::StorageObservation as Cells;
use super::*;

const DIRECT: &str = include_str!("../../../../tests/fixtures/parallel-generic-direct.lkjc");

#[test]
fn parallel_generic_direct_owned_and_composite_ordinary_results() {
    let _lane = super::super::super::parallel::isolated_test_lane();
    let source = byte_buffer_tests::author_only(DIRECT).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "joined");
    for reference in [false, true] {
        for n in [i64::MIN, -137, i64::MAX] {
            let cells = Cells::start();
            let list = || {
                NormalizedValue::list(vec![
                    NormalizedValue::I64(-3),
                    NormalizedValue::I64(0),
                    NormalizedValue::I64(7),
                ])
                .unwrap()
            };
            let arguments = vec![NormalizedValue::I64(n), list()];
            let control = ExecutionControl::uncancelled();
            let result = if reference {
                let (value, work) = NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(entry, arguments, None, &control)
                .unwrap();
                assert_eq!(work.live_call_frames_after, 0);
                assert_eq!(work.live_handles_after, 0);
                value
            } else {
                let (value, work) = NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                    .invoke(entry, arguments, None, &control)
                    .unwrap();
                assert_eq!(work.parallel_scopes, 1);
                assert_eq!(work.parallel_workers_spawned, 1);
                assert_eq!(work.live_call_frames_after, 0);
                assert_eq!(work.live_handles_after, 0);
                value
            };
            let expected =
                NormalizedValue::Record(super::super::super::value::NormalizedRecord::Structural {
                    fields: Arc::new(vec![
                        (Name::new("scalar").unwrap(), NormalizedValue::I64(n)),
                        (Name::new("values").unwrap(), list()),
                    ]),
                });
            assert_eq!(result, expected);
            assert_eq!(cells.created(), 1);
            assert_eq!(cells.live(), (0, 0));
        }
    }
}

pub(crate) fn witnessed_source() -> String {
    [
        include_str!("../../../../tests/fixtures/owned-witness-library.lkjc"),
        include_str!("../../../../tests/fixtures/parallel-result-library.lkjc"),
        include_str!("../../../../tests/fixtures/parallel-generic-workers.lkjc"),
        include_str!("../../../../tests/fixtures/parallel-generic-witness.lkjc"),
    ]
    .join("\n")
}

#[test]
fn parallel_generic_witness_identity_survives_forwarding_and_owned_returns() {
    let _lane = super::super::super::parallel::isolated_test_lane();
    let source = byte_buffer_tests::author_only(&witnessed_source()).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        for name in ["main", "owned"] {
            for n in [i64::MIN, -137, i64::MAX] {
                let cells = Cells::start();
                let entry = declaration_named(&source, name);
                let control = ExecutionControl::uncancelled();
                let args = vec![NormalizedValue::I64(n)];
                let value = if reference {
                    NormalizedReferenceInterpreter::new(
                        &source,
                        &program,
                        NormalizedRunPolicy::foreground(),
                    )
                    .invoke(entry, args, None, &control)
                    .unwrap()
                    .0
                } else {
                    let (value, work) =
                        NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                            .invoke(entry, args, None, &control)
                            .unwrap();
                    assert_eq!(work.parallel_scopes, 1);
                    assert_eq!(work.parallel_workers_spawned, 1);
                    assert_eq!(work.live_call_frames_after, 0);
                    value
                };
                assert_eq!(
                    value,
                    NormalizedValue::Record(
                        super::super::super::value::NormalizedRecord::Structural {
                            fields: Arc::new(vec![
                                (Name::new("left").unwrap(), NormalizedValue::I64(n)),
                                (Name::new("right").unwrap(), NormalizedValue::I64(99))
                            ])
                        }
                    )
                );
                assert_eq!(cells.created(), 2);
                assert_eq!(cells.live(), (0, 0));
            }
        }
    }
}

#[test]
fn parallel_generic_closed_boundary_rejects_hidden_authority_and_duplicate_custody() {
    let direct = DIRECT;
    let witnessed = witnessed_source();
    for (name, input) in [
        (
            "wrong-owned-type",
            direct.replacen(
                "(call transfer (types OwnedI64Cell)",
                "(call transfer (types I64)",
                1,
            ),
        ),
        (
            "wrong-ordinary-actual",
            direct.replacen(
                "(call keep-list (types I64)",
                "(call keep-list (types OwnedI64Cell)",
                1,
            ),
        ),
        (
            "hidden-callback-actual",
            direct.replacen(
                "(call keep-list (types I64)",
                "(call keep-list (types (option (function () I64)))",
                1,
            ),
        ),
        (
            "type-arity",
            direct.replacen(
                "(call transfer (types OwnedI64Cell)",
                "(call transfer (types OwnedI64Cell ByteBuffer)",
                1,
            ),
        ),
        (
            "witness-self",
            witnessed.replacen(
                "(implementations concrete@Alternate) (local right)",
                "(implementations concrete@buffer::Octets) (local right)",
                1,
            ),
        ),
        (
            "concrete-witness-parameter",
            witnessed.replacen(
                "ops abstraction::Storage T)",
                "ops abstraction::Storage OwnedI64Cell)",
                1,
            ),
        ),
        (
            "duplicate-owner",
            witnessed.replacen(
                "(implementations concrete@Alternate) (local right)",
                "(implementations concrete@Alternate) (local left)",
                1,
            ),
        ),
        (
            "borrow-boundary",
            direct.replacen(
                "(parameter create value (type T) (use consume))",
                "(parameter create value (type T) (use borrow))",
                1,
            ),
        ),
    ] {
        assert!(
            input != direct && input != witnessed,
            "{name} must mutate its input"
        );
        let error = byte_buffer_tests::author_only(&input).expect_err(name);
        assert!(
            error.contains("kernel_parallel_call")
                || error.contains("kernel_owned_contract")
                || error.contains("kernel_buffer_ownership"),
            "{name}: {error}"
        );
    }
}

#[path = "parallel_generic_limits_tests.rs"]
mod limits;

#[path = "parallel_transfer_tests.rs"]
mod transfer;
