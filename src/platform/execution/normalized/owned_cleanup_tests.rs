//! Both carriers clean lexical owners and active generic loans on every exit.
use super::super::{byte_buffer, owned_i64_cell};
use super::*;
fn source() -> crate::platform::kernel::KernelSnapshot {
    byte_buffer_tests::author_only(
        &[
            include_str!("../../../../tests/fixtures/owned-witness-library.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-cell.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-buffer.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-cleanup.lkjc"),
        ]
        .join("\n"),
    )
    .unwrap()
}
struct Inspect<'a> {
    buffer: &'a byte_buffer::StorageObservation,
    cell: &'a owned_i64_cell::StorageObservation,
    cell_selected: bool,
    loan: bool,
    cancel: bool,
    forge: bool,
}
impl Inspect<'_> {
    fn answer(&self, control: &ExecutionControl) -> Result<NormalizedValue, ExecutionError> {
        let (selected, unused) = if self.cell_selected {
            (self.cell.live(), self.buffer.live())
        } else {
            (self.buffer.live(), self.cell.live())
        };
        assert_eq!(unused, (0, 0));
        if self.loan {
            assert_eq!(selected.0, 1);
            assert!(selected.1 > 0);
        } else {
            assert_eq!(selected, (0, 0));
        }
        if self.cancel {
            control.cancel();
        }
        if self.forge {
            return Ok(NormalizedValue::OwnedI64Cell(
                owned_i64_cell::OwnedI64Cell::new(
                    super::super::value::ValueOrigin::fresh().unwrap(),
                    0,
                ),
            ));
        }
        Ok(NormalizedValue::I64(0))
    }
}
impl super::super::vm::NormalizedHost for Inspect<'_> {
    fn call(
        &self,
        _: &NormalizedProgram,
        _: &super::super::prepare::NormalizedFunction,
        _: &crate::platform::kernel::ImplementationName,
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
        _: &crate::platform::kernel::ImplementationName,
        _: &[TypeObjectDigest],
        _: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.answer(control)
    }
}
#[test]
fn owned_generic_lexical_drop_live_loan_cancellation_and_adapter_ingress() {
    let source = source();
    let program = prepare_snapshot(&source);
    let schema = Arc::new(
        super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&source]).unwrap(),
    );
    let reader = FaultedReferenceRead {
        source: &source,
        schema,
    };
    for reference in [false, true] {
        let cell = owned_i64_cell::StorageObservation::start();
        let buffer = byte_buffer::StorageObservation::start();
        for cell_selected in [false, true] {
            for (loan, cancel, forge) in [
                (false, false, false),
                (true, false, false),
                (true, true, false),
                (false, false, true),
                (true, false, true),
            ] {
                let name = format!(
                    "{}-{}",
                    if cell_selected { "cell" } else { "buffer" },
                    if loan { "loan" } else { "drop" }
                );
                let host = Inspect {
                    buffer: &buffer,
                    cell: &cell,
                    cell_selected,
                    loan,
                    cancel,
                    forge,
                };
                let control = ExecutionControl::uncancelled();
                let sink = Mutex::new(None);
                let d = declaration_named(&source, &name);
                let result = if reference {
                    NormalizedReferenceInterpreter::from_reader(
                        &reader,
                        &program,
                        NormalizedRunPolicy::foreground(),
                    )
                    .observing(&sink, &host)
                    .invoke(d, vec![], None, &control)
                    .map(|v| v.0)
                } else {
                    let sink = Mutex::new(None);
                    NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                        .observing(&sink, &host)
                        .invoke(d, vec![], None, &control)
                        .map(|v| v.0)
                };
                if cancel {
                    assert_eq!(
                        result.unwrap_err().class,
                        crate::platform::execution::ExecutionFailureClass::Cancelled
                    );
                } else if forge {
                    assert!(result.is_err());
                } else {
                    assert_eq!(result.unwrap(), NormalizedValue::I64(0));
                }
                assert_eq!(cell.live(), (0, 0));
                assert_eq!(buffer.live(), (0, 0));
            }
        }
    }
}
#[test]
fn owned_generic_traps_cancellation_and_quota_cleanup() {
    let source = source();
    let program = prepare_snapshot(&source);
    let schema = Arc::new(
        super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&source]).unwrap(),
    );
    let reader = FaultedReferenceRead {
        source: &source,
        schema,
    };
    for reference in [false, true] {
        let cell = owned_i64_cell::StorageObservation::start();
        let buffer = byte_buffer::StorageObservation::start();
        for carrier in ["cell", "buffer"] {
            let run = |suffix: &str, policy: NormalizedRunPolicy, control: &ExecutionControl| {
                let d = declaration_named(&source, &format!("{carrier}-{suffix}"));
                if reference {
                    NormalizedReferenceInterpreter::from_reader(&reader, &program, policy)
                        .invoke(d, vec![], None, control)
                        .map(|v| v.0)
                } else {
                    NormalizedVm::new(&program, policy)
                        .invoke(d, vec![], None, control)
                        .map(|v| v.0)
                }
            };
            assert_eq!(
                run(
                    "trap",
                    NormalizedRunPolicy::foreground(),
                    &ExecutionControl::uncancelled()
                )
                .unwrap_err()
                .class,
                crate::platform::execution::ExecutionFailureClass::Trap
            );
            assert_eq!(cell.live(), (0, 0));
            assert_eq!(buffer.live(), (0, 0));
            let mut cancelled = 0;
            let mut completed = 0;
            for checks in (0..2048).step_by(16).chain([8192]) {
                match run(
                    "loan",
                    NormalizedRunPolicy::foreground(),
                    &ExecutionControl::cancel_after_checks(checks),
                ) {
                    Ok(value) => {
                        assert_eq!(value, NormalizedValue::I64(0));
                        completed += 1;
                    }
                    Err(e) => {
                        assert_eq!(
                            e.class,
                            crate::platform::execution::ExecutionFailureClass::Cancelled
                        );
                        cancelled += 1;
                    }
                }
                assert_eq!(cell.live(), (0, 0));
                assert_eq!(buffer.live(), (0, 0));
            }
            assert!(cancelled > 0 && completed > 0);
            let mut exhausted = 0;
            let mut completed = 0;
            for quota in [0, 16, 64, 128, 256, 512, 1024, 4096, 65536, 1048576] {
                let policy = NormalizedRunPolicy {
                    maximum_allocated_bytes: Some(quota),
                    ..NormalizedRunPolicy::foreground()
                };
                match run("loan", policy, &ExecutionControl::uncancelled()) {
                    Ok(value) => {
                        assert_eq!(value, NormalizedValue::I64(0));
                        completed += 1;
                    }
                    Err(e) => {
                        assert_eq!(
                            e.class,
                            crate::platform::execution::ExecutionFailureClass::Resource
                        );
                        exhausted += 1;
                    }
                }
                assert_eq!(cell.live(), (0, 0));
                assert_eq!(buffer.live(), (0, 0));
            }
            assert!(exhausted > 0 && completed > 0);
        }
    }
}
